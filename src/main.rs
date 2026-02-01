use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{Duration, Utc};
use clap::{CommandFactory, Parser};
use futures::StreamExt;

use cli::Cli;
use config::Config;
use db::{DbProvider, SecretRepository, init_db};
use secrets::{detect_providers, hash_api_ref, save_secret_file, get_secret_path, Provider};

mod archive;
mod cli;
mod config;
mod db;
mod secrets;
mod vcs;

/// Format an error in a user-friendly way
fn format_error(e: &dyn std::error::Error) -> String {
    let msg = e.to_string();
    
    // Check for common AWS Secrets Manager errors
    if msg.contains("ResourceNotFoundException") {
        return "Secret not found (may have been deleted)".to_string();
    }
    if msg.contains("AccessDeniedException") {
        return "Access denied (check IAM permissions)".to_string();
    }
    if msg.contains("InvalidParameterException") {
        return "Invalid parameter".to_string();
    }
    if msg.contains("InvalidRequestException") {
        return "Invalid request".to_string();
    }
    if msg.contains("DecryptionFailure") {
        return "Decryption failed (KMS key issue)".to_string();
    }
    if msg.contains("InternalServiceError") {
        return "AWS internal error (try again later)".to_string();
    }
    
    // Default: return the Display version (cleaner than Debug)
    msg
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load config from file
    let config = Config::load()?;

    // Handle Config command separately as it doesn't require providers or database
    if let Some(cli::Commands::Config { command }) = &cli.command {
        match command {
            cli::ConfigCommands::Generate { path, overwrite, interactive } => {
                if *interactive {
                    return handle_interactive_generate(path.clone(), *overwrite).await;
                } else {
                    let config_path = Config::generate_config_file(path.clone(), *overwrite)?;
                    println!("Config file generated at: {}", config_path.display());
                    return Ok(());
                }
            }
            cli::ConfigCommands::List => {
                println!("Current Configuration:");
                println!("  editor: {}", config.editor());
                println!("  secrets_path: {}", config.secrets_path().display());
                println!("  cache_ttl: {}s", config.cache_ttl());
                println!();
                if config.providers.is_empty() {
                    println!("Providers: (none configured)");
                } else {
                    println!("Providers:");
                    for p in &config.providers {
                        println!("  {} ({})", p.id, p.kind);
                        if let Some(profile) = &p.profile {
                            println!("    profile: {}", profile);
                        }
                        if let Some(region) = &p.region {
                            println!("    region: {}", region);
                        }
                        if let Some(vault) = &p.vault {
                            println!("    vault: {}", vault);
                        }
                        if let Some(token_env) = &p.token_env {
                            println!("    token_env: {}", token_env);
                        }
                    }
                }
                return Ok(());
            }
            cli::ConfigCommands::Get { key } => {
                match config.get_default(key) {
                    Ok(value) => println!("{}", value),
                    Err(e) => eprintln!("{}", e),
                }
                return Ok(());
            }
            cli::ConfigCommands::Set { key, value } => {
                let config_path = std::path::PathBuf::from("jaws.kdl");
                if !config_path.exists() {
                    eprintln!("Config file not found. Run 'jaws config generate' first.");
                    return Ok(());
                }
                let mut config = config;
                match config.set_default(key, value) {
                    Ok(()) => {
                        config.save(&config_path)?;
                        println!("Updated {} = {}", key, value);
                    }
                    Err(e) => eprintln!("{}", e),
                }
                return Ok(());
            }
            cli::ConfigCommands::Providers => {
                if config.providers.is_empty() {
                    println!("No providers configured.");
                    println!();
                    println!("Run 'jaws config generate --interactive' to set up providers,");
                    println!("or manually edit jaws.kdl");
                } else {
                    println!("Configured Providers:");
                    for p in &config.providers {
                        print!("  {} [{}]", p.id, p.kind);
                        if let Some(profile) = &p.profile {
                            if profile == "all" {
                                print!(" (auto-discover all AWS profiles)");
                            } else {
                                print!(" profile={}", profile);
                            }
                        }
                        if let Some(vault) = &p.vault {
                            if vault == "all" {
                                print!(" (auto-discover all vaults)");
                            } else {
                                print!(" vault={}", vault);
                            }
                        }
                        println!();
                    }
                }
                return Ok(());
            }
        }
    }

    // Handle Export command separately (doesn't require providers)
    if let Some(cli::Commands::Export { ssh_key, output, delete }) = &cli.command {
        return handle_export(&config, ssh_key.clone(), output.clone(), *delete).await;
    }

    // Handle Import command separately (doesn't require providers)
    if let Some(cli::Commands::Import { archive, ssh_key, delete }) = &cli.command {
        return handle_import(&config, archive, ssh_key.clone(), *delete).await;
    }

    // Ensure secrets directory exists
    fs::create_dir_all(config.secrets_path())?;

    // Initialize database
    let conn = init_db(&config.db_path())?;
    let repo = SecretRepository::new(conn);

    // Handle no command - default behavior (edit downloaded secrets)
    if cli.command.is_none() {
        return handle_default_command(&config, &repo).await;
    }

    // Detect and initialize all available providers
    let providers = detect_providers(&config).await?;

    if providers.is_empty() {
        eprintln!("No providers configured. Run 'jaws config generate' to create a config file.");
        return Ok(());
    }

    // Ensure providers are registered in the database
    for provider in &providers {
        repo.upsert_provider(&DbProvider {
            id: provider.id().to_string(),
            kind: provider.kind().to_string(),
            last_sync_at: None,
            config_json: None,
        })?;
    }

    match cli.command.unwrap() {
        cli::Commands::Config { .. } => unreachable!(),

        cli::Commands::Pull { secret_name, edit } => {
            handle_pull(&config, &repo, &providers, secret_name, edit).await?;
        }

        cli::Commands::Push { secret_name, edit } => {
            handle_push(&config, &repo, &providers, secret_name, edit).await?;
        }

        cli::Commands::Delete { secret_name } => {
            handle_delete(&config, &repo, secret_name).await?;
        }

        cli::Commands::Remote { command } => {
            handle_remote(&config, &providers, command).await?;
        }

        cli::Commands::Sync => {
            handle_sync(&config, &repo, &providers).await?;
        }

        cli::Commands::History {
            secret_name,
            verbose,
            limit,
        } => {
            handle_history(&config, &repo, secret_name, verbose, limit).await?;
        }

        cli::Commands::Restore {
            secret_name,
            version,
            edit,
        } => {
            handle_restore(&config, &repo, secret_name, version, edit).await?;
        }

        cli::Commands::Export { .. } => unreachable!(),
        cli::Commands::Import { .. } => unreachable!(),

        cli::Commands::Undo => {
            handle_undo(&config).await?;
        }

        cli::Commands::Log { limit } => {
            handle_log(&config, limit).await?;
        }

        cli::Commands::Diff { operation } => {
            handle_diff(&config, operation).await?;
        }
    }

    Ok(())
}

/// Handle the default command (no subcommand) - show picker for downloaded secrets to edit
async fn handle_default_command(
    config: &Config,
    repo: &SecretRepository,
) -> Result<(), Box<dyn std::error::Error>> {
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        // Show help if no secrets downloaded
        Cli::command().print_help()?;
        println!(); // Add newline after help
        return Ok(());
    }

    // Build list of display names for the picker
    use ff::{TuiConfig, create_items_channel, run_tui_with_config};

    let (tx, rx) = create_items_channel();

    // Send all downloaded secrets to the picker
    for (secret, _download) in &downloaded {
        let display = format!("{} | {}", secret.provider_id, secret.display_name);
        if tx.send(display).await.is_err() {
            break;
        }
    }
    drop(tx);

    let mut tui_config = TuiConfig::fullscreen();
    tui_config.show_help_text = false;

    let selected = run_tui_with_config(rx, true, tui_config)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    if selected.is_empty() {
        return Ok(());
    }

    // Collect all selected file paths
    let mut files_to_open: Vec<String> = Vec::new();
    for selected_display in &selected {
        for (secret, download) in &downloaded {
            let display = format!("{} | {}", secret.provider_id, secret.display_name);
            if &display == selected_display {
                let file_path = get_secret_path(&config.secrets_path(), &download.filename);
                files_to_open.push(file_path.to_string_lossy().to_string());
                break;
            }
        }
    }

    if !files_to_open.is_empty() {
        let _ = Command::new(config.editor())
            .args(&files_to_open)
            .status()
            .expect("failed to launch editor");
    }

    Ok(())
}

/// Handle the pull command
async fn handle_pull(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: Option<String>,
    edit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use ff::{TuiConfig, FuzzyFinderWithIndicators, ItemIndicator};
    use db::SecretInput;

    if let Some(_name) = secret_name {
        // TODO: Pull specific secret by name
        // For now, just use interactive selection
        eprintln!("Pulling specific secrets by name not yet implemented. Use interactive selection.");
        return Ok(());
    }

    // Set up TUI config
    let mut tui_config = TuiConfig::fullscreen();
    tui_config.show_help_text = false;

    // Create session with per-item indicator support
    let (session, tui_future) = FuzzyFinderWithIndicators::with_config(true, tui_config);
    let session = Arc::new(session);

    // Map from display string to secret info (provider_id, secret_id, api_ref, display_name, hash)
    let secret_map: Arc<Mutex<HashMap<String, (String, i64, String, String, String)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Spawn tasks for each provider to stream secrets to TUI
    for provider in providers {
        let session = Arc::clone(&session);
        let repo = repo.clone();
        let map = Arc::clone(&secret_map);
        let provider_id = provider.id().to_string();
        let provider_kind = provider.kind().to_string();
        let cache_ttl = config.cache_ttl();

        // Clone provider's stream capability - we need to move into the spawned task
        let mut secret_stream = provider.list_secrets_stream();

        tokio::spawn(async move {
            // Track which secrets we've already sent (by display string)
            let mut sent_displays: std::collections::HashSet<String> = std::collections::HashSet::new();

            // 1. Send cached secrets immediately (no indicator - already synced)
            let cached_secrets: Vec<_> = repo
                .list_secrets_by_provider(&provider_id)
                .unwrap_or_default();
            
            for secret in cached_secrets {
                let display = format!("{} | {}", provider_id, secret.display_name);
                if sent_displays.insert(display.clone()) {
                    map.lock().await.insert(
                        display.clone(),
                        (
                            provider_id.clone(),
                            secret.id,
                            secret.api_ref.clone(),
                            secret.display_name.clone(),
                            secret.hash.clone(),
                        ),
                    );
                    if session.add(&display).await.is_err() {
                        return;
                    }
                }
            }

            // 2. Check if refresh needed
            let should_refresh = should_refresh_cache(&repo, &provider_id, cache_ttl).unwrap_or(true);

            if should_refresh {
                // Stream from remote and update DB
                while let Some(result) = secret_stream.next().await {
                    if let Ok(secret_ref) = result {
                        // Parse based on provider type
                        let (api_ref, display_name) = if provider_kind == "onepassword" {
                            let parsed = secrets::SecretRef::parse(&secret_ref);
                            (parsed.api_ref, parsed.display_path)
                        } else {
                            (secret_ref.clone(), secret_ref.clone())
                        };
                        let hash = hash_api_ref(&api_ref);

                        let display = format!("{} | {}", provider_id, display_name);
                        let is_new = sent_displays.insert(display.clone());

                        // Add to TUI with spinner if new item
                        if is_new {
                            if session.add_with_indicator(&display, ItemIndicator::Spinner).await.is_err() {
                                return;
                            }
                        }

                        // Upsert to DB
                        let input = SecretInput {
                            provider_id: provider_id.clone(),
                            api_ref: api_ref.clone(),
                            display_name: display_name.clone(),
                            hash: hash.clone(),
                            remote_updated_at: None,
                        };

                        // Get secret_id - upsert returns the id
                        let secret_id = match repo.upsert_secret(&input) {
                            Ok(id) => id,
                            Err(_) => continue,
                        };

                        // Update map and clear indicator (sync complete for this item)
                        if is_new {
                            map.lock().await.insert(
                                display.clone(),
                                (
                                    provider_id.clone(),
                                    secret_id,
                                    api_ref,
                                    display_name,
                                    hash,
                                ),
                            );
                            // Clear the spinner now that sync is complete
                            let _ = session.clear_indicator(&display).await;
                        }
                    }
                }

                // Update sync time
                let _ = repo.update_provider_sync_time(&provider_id);
            }
        });
    }

    // Drop session reference so channel closes when all tasks finish
    drop(session);

    // Run TUI and wait for selection
    let selected = tui_future
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    if selected.is_empty() {
        println!("No secrets selected.");
        return Ok(());
    }

    // Look up secret info for selected items
    let map = secret_map.lock().await;
    let selected_secrets: Vec<_> = selected
        .iter()
        .filter_map(|display| map.get(display).cloned())
        .collect();
    drop(map);

    if selected_secrets.is_empty() {
        println!("No secrets selected.");
        return Ok(());
    }

    // Download selected secrets
    let success_count = AtomicUsize::new(0);
    let fail_count = AtomicUsize::new(0);
    let total = selected_secrets.len();

    let stream = futures::stream::iter(selected_secrets);

    let results: Vec<Option<String>> = stream
        .map(|(provider_id, secret_id, api_ref, display_name, hash)| {
            let providers = providers;
            let config = config;
            let repo = repo.clone();
            let success_count = &success_count;
            let fail_count = &fail_count;

            async move {
                if let Some(provider) = providers.iter().find(|p| p.id() == provider_id) {
                    match provider.get_secret(&api_ref).await {
                        Ok(content) => {
                            // Get next version number
                            let version = repo
                                .get_latest_download(secret_id)
                                .ok()
                                .flatten()
                                .map(|d| d.version + 1)
                                .unwrap_or(1);

                            // Save to file
                            match save_secret_file(
                                &config.secrets_path(),
                                &display_name,
                                &hash,
                                version,
                                &content,
                            ) {
                                Ok((filename, content_hash)) => {
                                    // Record download in database
                                    if let Err(e) =
                                        repo.create_download(secret_id, &filename, &content_hash)
                                    {
                                        eprintln!("Warning: Failed to record download: {}", e);
                                    }
                                    success_count.fetch_add(1, Ordering::Relaxed);
                                    return Some(
                                        get_secret_path(&config.secrets_path(), &filename)
                                            .to_string_lossy()
                                            .to_string(),
                                    );
                                }
                                Err(e) => {
                                    eprintln!("✗ {} [{}]: Failed to save - {}", display_name, provider_id, e);
                                    fail_count.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("✗ {} [{}]: {}", display_name, provider_id, format_error(e.as_ref()));
                            fail_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                None
            }
        })
        .buffer_unordered(10)
        .collect()
        .await;

    let downloaded_files: Vec<String> = results.into_iter().flatten().collect();

    // Print summary
    let succeeded = success_count.load(Ordering::Relaxed);
    let failed = fail_count.load(Ordering::Relaxed);
    if failed == 0 {
        println!("Downloaded {} secret(s)", succeeded);
    } else {
        println!(
            "Downloaded {} of {} secret(s) ({} failed)",
            succeeded, total, failed
        );
    }

    // Commit to VCS if any secrets were downloaded
    if succeeded > 0 {
        let commit_msg = format!("pull: downloaded {} secret(s)", succeeded);
        try_vcs_commit(&config.secrets_path(), &commit_msg);
    }

    // Open in editor if requested
    if edit && !downloaded_files.is_empty() {
        let _ = Command::new(config.editor())
            .args(&downloaded_files)
            .status()
            .expect("failed to launch editor");
    }

    Ok(())
}

/// Handle the push command
async fn handle_push(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: Option<String>,
    edit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get all downloaded secrets
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        return Err("No secrets downloaded. Use 'jaws pull' first.".into());
    }

    // Filter by name if provided
    let secrets_to_push: Vec<_> = if let Some(name) = &secret_name {
        downloaded
            .into_iter()
            .filter(|(s, _)| s.display_name.contains(name) || s.hash.starts_with(name))
            .collect()
    } else {
        downloaded
    };

    if secrets_to_push.is_empty() {
        return Err(format!(
            "No matching secrets found{}",
            secret_name
                .map(|n| format!(" for '{}'", n))
                .unwrap_or_default()
        )
        .into());
    }

    // Collect file paths for editing
    let files: Vec<String> = secrets_to_push
        .iter()
        .map(|(_, d)| {
            get_secret_path(&config.secrets_path(), &d.filename)
                .to_string_lossy()
                .to_string()
        })
        .collect();

    // Open in editor if requested
    if edit && !files.is_empty() {
        let _ = Command::new(config.editor())
            .args(&files)
            .status()
            .expect("failed to launch editor");
    }

    // Push each secret
    let mut pushed_count = 0;

    for (secret, download) in secrets_to_push {
        let file_path = get_secret_path(&config.secrets_path(), &download.filename);

        if !file_path.exists() {
            eprintln!("Error: File not found: {}", file_path.display());
            continue;
        }

        let content = fs::read_to_string(&file_path)?;

        // Find the provider
        let provider = providers
            .iter()
            .find(|p| p.id() == secret.provider_id)
            .ok_or_else(|| format!("Provider {} not found", secret.provider_id))?;

        match provider.update(&secret.api_ref, &content).await {
            Ok(result) => {
                println!("{} [{}] -> {}", secret.display_name, secret.provider_id, result);
                pushed_count += 1;
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("ResourceNotFoundException")
                    || error_msg.contains("not found")
                {
                    match provider
                        .create(&secret.display_name, &content, None)
                        .await
                    {
                        Ok(result) => {
                            println!(
                                "{} [{}] -> {} (created)",
                                secret.display_name, secret.provider_id, result
                            );
                            pushed_count += 1;
                        }
                        Err(create_err) => {
                            eprintln!(
                                "Error creating {} in {}: {}",
                                secret.display_name, secret.provider_id, create_err
                            );
                        }
                    }
                } else {
                    eprintln!(
                        "Error updating {} in {}: {}",
                        secret.display_name, secret.provider_id, e
                    );
                }
            }
        }
    }

    // Commit to VCS if any secrets were pushed
    if pushed_count > 0 {
        let commit_msg = format!("push: updated {} secret(s)", pushed_count);
        try_vcs_commit(&config.secrets_path(), &commit_msg);
    }

    Ok(())
}

/// Handle the remote delete command - delete from provider
async fn handle_remote_delete(
    providers: &[Provider],
    secret_name: Option<String>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected = if let Some(name) = secret_name {
        // Find which provider might have this secret
        vec![(providers[0].id().to_string(), name)]
    } else {
        secrets::select_from_all_providers(providers).await?
    };

    for (provider_id, secret_ref) in selected {
        let provider = providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| format!("Provider {} not found", provider_id))?;

        match provider.delete(&secret_ref, force).await {
            Ok(()) => {
                if force {
                    println!("{} [{}] deleted (force)", secret_ref, provider_id);
                } else {
                    println!(
                        "{} [{}] deleted (recovery period: 7-30 days)",
                        secret_ref, provider_id
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Error deleting {} from {}: {}",
                    secret_ref, provider_id, e
                );
            }
        }
    }

    Ok(())
}

/// Handle the remote rollback command - rollback on provider
async fn handle_remote_rollback(
    providers: &[Provider],
    secret_name: Option<String>,
    version_id: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected = if let Some(name) = secret_name {
        vec![(providers[0].id().to_string(), name)]
    } else {
        secrets::select_from_all_providers(providers).await?
    };

    for (provider_id, secret_ref) in selected {
        let provider = providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| format!("Provider {} not found", provider_id))?;

        match provider.rollback(&secret_ref, version_id.as_deref()).await {
            Ok(result) => {
                if let Some(vid) = &version_id {
                    println!(
                        "{} [{}] rolled back to version {} -> {}",
                        secret_ref, provider_id, vid, result
                    );
                } else {
                    println!(
                        "{} [{}] rolled back to previous version -> {}",
                        secret_ref, provider_id, result
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Error rolling back {} in {}: {}",
                    secret_ref, provider_id, e
                );
            }
        }
    }

    Ok(())
}

/// Handle the sync command
async fn handle_sync(
    _config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Syncing remote secret listings...");

    for provider in providers {
        match sync_provider(repo, provider).await {
            Ok(count) => {
                println!("  {} [{}]: {} secrets", provider.id(), provider.kind(), count);
            }
            Err(e) => {
                eprintln!("  {} [{}]: Error - {}", provider.id(), provider.kind(), e);
            }
        }
    }

    println!("Sync complete.");
    Ok(())
}

/// Check if the cache for a provider should be refreshed
fn should_refresh_cache(
    repo: &SecretRepository,
    provider_id: &str,
    cache_ttl: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let provider = repo.get_provider(provider_id)?;

    let should_refresh = provider
        .and_then(|p| p.last_sync_at)
        .map(|last_sync| {
            let ttl = Duration::seconds(cache_ttl as i64);
            last_sync + ttl < Utc::now()
        })
        .unwrap_or(true);

    Ok(should_refresh)
}

/// Sync a provider's secrets to the database
async fn sync_provider(
    repo: &SecretRepository,
    provider: &Provider,
) -> Result<usize, Box<dyn std::error::Error>> {
    use db::SecretInput;

    let mut stream = provider.list_secrets_stream();
    let mut count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(secret_ref) => {
                // For 1Password, parse the combined format "display_path|||api_ref"
                // For AWS, the secret name is both the API ref and display name
                let (api_ref, display_name) = if provider.kind() == "onepassword" {
                    let parsed = secrets::SecretRef::parse(&secret_ref);
                    (parsed.api_ref, parsed.display_path)
                } else {
                    (secret_ref.clone(), secret_ref.clone())
                };
                let hash = hash_api_ref(&api_ref);

                let input = SecretInput {
                    provider_id: provider.id().to_string(),
                    api_ref,
                    display_name,
                    hash,
                    remote_updated_at: None,
                };

                repo.upsert_secret(&input)?;
                count += 1;
            }
            Err(e) => {
                eprintln!("Warning: Error fetching secret: {}", e);
            }
        }
    }

    repo.update_provider_sync_time(provider.id())?;

    Ok(count)
}

/// Handle interactive config generation
async fn handle_interactive_generate(
    path: Option<std::path::PathBuf>,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use config::{Config, Defaults, ProviderConfig};
    use ff::{FuzzyFinderSession, TuiConfig};
    use std::io::{self, Write};

    let config_path = path.unwrap_or_else(|| std::path::PathBuf::from("./jaws.kdl"));

    // Check if file exists and overwrite flag
    if config_path.exists() && !overwrite {
        return Err(format!(
            "Config file already exists at: {}. Use --overwrite to replace it.",
            config_path.display()
        )
        .into());
    }

    println!("Interactive Configuration Setup");
    println!("================================\n");

    // Helper function to read input with a default
    fn prompt(message: &str, default: &str) -> String {
        print!("{} [{}]: ", message, default);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        if input.is_empty() {
            default.to_string()
        } else {
            input.to_string()
        }
    }

    // Get defaults
    let default_editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".into());
    let editor = prompt("Editor", &default_editor);
    let secrets_path = prompt("Secrets path", "./.secrets");
    let cache_ttl_str = prompt("Cache TTL (seconds)", "900");
    let cache_ttl: u64 = cache_ttl_str.parse().unwrap_or(900);

    let mut config = Config {
        defaults: Some(Defaults {
            editor: Some(editor),
            secrets_path: Some(secrets_path),
            cache_ttl: Some(cache_ttl),
        }),
        providers: Vec::new(),
    };

    println!();

    // Discover AWS profiles
    println!("Discovering AWS profiles...");
    match Config::discover_aws_profiles() {
        Ok(profiles) if !profiles.is_empty() => {
            println!("Found {} AWS profile(s). Select which to add (or none to skip):", profiles.len());
            println!("  Tip: Use 'all' option to auto-discover profiles at runtime\n");

            // Create items for ff selection
            let mut items: Vec<String> = vec!["[all] - Auto-discover all profiles at runtime".to_string()];
            for profile in &profiles {
                let region = Config::get_aws_profile_region(profile)
                    .map(|r| format!(" ({})", r))
                    .unwrap_or_default();
                items.push(format!("{}{}", profile, region));
            }

            // Use ff for multi-select
            let mut tui_config = TuiConfig::with_height(15.min(items.len() as u16 + 3));
            tui_config.show_help_text = true;

            let (session, tui_future) = FuzzyFinderSession::with_config(true, tui_config);

            for item in &items {
                let _ = session.add(item).await;
            }
            drop(session);

            let selected = tui_future.await.unwrap_or_default();

            if !selected.is_empty() {
                // Check if "all" was selected
                if selected.iter().any(|s| s.starts_with("[all]")) {
                    config.providers.push(ProviderConfig::new_aws(
                        "aws".to_string(),
                        Some("all".to_string()),
                        None,
                    ));
                    println!("Added AWS provider with auto-discovery");
                } else {
                    // Add individual profiles
                    for selection in &selected {
                        // Extract profile name (before any region in parentheses)
                        let profile_name = selection.split(" (").next().unwrap_or(selection).trim();
                        let region = Config::get_aws_profile_region(profile_name);
                        config.providers.push(ProviderConfig::new_aws(
                            format!("aws-{}", profile_name),
                            Some(profile_name.to_string()),
                            region,
                        ));
                    }
                    println!("Added {} AWS provider(s)", selected.len());
                }
            } else {
                println!("No AWS profiles selected");
            }
        }
        Ok(_) => println!("No AWS profiles found in ~/.aws/credentials"),
        Err(e) => println!("Could not discover AWS profiles: {}", e),
    }

    println!();

    // Check for 1Password
    println!("Checking for 1Password...");
    let op_token_env = "OP_SERVICE_ACCOUNT_TOKEN";
    if std::env::var(op_token_env).is_ok() {
        println!("Found {}. Discovering vaults...", op_token_env);

        match secrets::OnePasswordSecretManager::new(None, op_token_env).await {
            Ok(manager) => {
                match manager.list_vaults() {
                    Ok(vaults) if !vaults.is_empty() => {
                        println!("Found {} vault(s). Select which to add (or none to skip):", vaults.len());
                        println!("  Tip: Use 'all' option to auto-discover vaults at runtime\n");

                        let mut items: Vec<String> = vec!["[all] - Auto-discover all vaults at runtime".to_string()];
                        for vault in &vaults {
                            items.push(format!("{} ({})", vault.title, vault.id));
                        }

                        let mut tui_config = TuiConfig::with_height(15.min(items.len() as u16 + 3));
                        tui_config.show_help_text = true;

                        let (session, tui_future) = FuzzyFinderSession::with_config(true, tui_config);

                        for item in &items {
                            let _ = session.add(item).await;
                        }
                        drop(session);

                        let selected = tui_future.await.unwrap_or_default();

                        if !selected.is_empty() {
                            if selected.iter().any(|s| s.starts_with("[all]")) {
                                config.providers.push(ProviderConfig::new_onepassword(
                                    "op".to_string(),
                                    Some("all".to_string()),
                                    None,
                                ));
                                println!("Added 1Password provider with auto-discovery");
                            } else {
                                for selection in &selected {
                                    // Extract vault name and ID
                                    if let Some((name, rest)) = selection.split_once(" (") {
                                        let vault_id = rest.trim_end_matches(')');
                                        let provider_id = format!("op-{}", name.to_lowercase().replace(' ', "-"));
                                        config.providers.push(ProviderConfig::new_onepassword(
                                            provider_id,
                                            Some(vault_id.to_string()),
                                            None,
                                        ));
                                    }
                                }
                                println!("Added {} 1Password provider(s)", selected.len());
                            }
                        } else {
                            println!("No 1Password vaults selected");
                        }
                    }
                    Ok(_) => println!("No 1Password vaults accessible"),
                    Err(e) => println!("Could not list 1Password vaults: {}", e),
                }
            }
            Err(e) => println!("Could not initialize 1Password: {}", e),
        }
    } else {
        println!("{} not set, skipping 1Password setup", op_token_env);
        println!("  Tip: Set this environment variable and re-run to add 1Password providers");
    }

    println!();

    // Save config
    config.save(&config_path)?;
    println!("Config written to: {}", config_path.display());

    if config.providers.is_empty() {
        println!();
        println!("Note: No providers were added. Edit {} to add providers manually.", config_path.display());
    }

    Ok(())
}

/// Handle the export command - archive and encrypt secrets
async fn handle_export(
    config: &Config,
    ssh_key: Option<std::path::PathBuf>,
    output: Option<std::path::PathBuf>,
    delete: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use archive::{export_secrets, format_size, prompt_passphrase_with_confirm, EncryptionMethod};

    let secrets_path = config.secrets_path();
    let output_path = output.unwrap_or_else(|| std::path::PathBuf::from("./jaws.barrel"));

    // Validate secrets directory exists
    if !secrets_path.exists() {
        return Err(format!(
            "Secrets directory not found: {}\nNothing to export.",
            secrets_path.display()
        )
        .into());
    }

    // Determine encryption method
    let encryption = if let Some(pubkey_path) = ssh_key {
        if !pubkey_path.exists() {
            return Err(format!("SSH public key not found: {}", pubkey_path.display()).into());
        }
        println!("Encrypting with SSH key: {}", pubkey_path.display());
        EncryptionMethod::SshPublicKey(pubkey_path)
    } else {
        // Default: passphrase
        let passphrase = prompt_passphrase_with_confirm("Enter passphrase")?;
        EncryptionMethod::Passphrase(passphrase)
    };

    // Create the archive
    let size = export_secrets(&secrets_path, &output_path, encryption)?;

    println!(
        "Exported {} to {} ({})",
        secrets_path.display(),
        output_path.display(),
        format_size(size)
    );

    // Delete original if requested
    if delete {
        fs::remove_dir_all(&secrets_path)?;
        println!("Deleted {}", secrets_path.display());
    }

    Ok(())
}

/// Handle the import command - decrypt and extract archive
async fn handle_import(
    config: &Config,
    archive_path: &std::path::Path,
    ssh_key: Option<std::path::PathBuf>,
    delete: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use archive::{import_secrets, prompt_passphrase, DecryptionMethod};

    let secrets_path = config.secrets_path();

    // Validate archive exists
    if !archive_path.exists() {
        return Err(format!("Archive not found: {}", archive_path.display()).into());
    }

    // Warn if secrets directory already exists
    if secrets_path.exists() {
        eprintln!(
            "Warning: {} already exists and will be overwritten",
            secrets_path.display()
        );
    }

    // Determine decryption method
    let decryption = if let Some(privkey_path) = ssh_key {
        if !privkey_path.exists() {
            return Err(format!("SSH private key not found: {}", privkey_path.display()).into());
        }
        println!("Decrypting with SSH key: {}", privkey_path.display());
        DecryptionMethod::SshPrivateKey(privkey_path)
    } else {
        // Default: passphrase
        let passphrase = prompt_passphrase("Enter passphrase")?;
        DecryptionMethod::Passphrase(passphrase)
    };

    // Import the archive
    import_secrets(archive_path, &secrets_path, decryption)?;

    println!("Imported to {}", secrets_path.display());

    // Delete archive if requested
    if delete {
        fs::remove_file(archive_path)?;
        println!("Deleted {}", archive_path.display());
    }

    Ok(())
}

/// Handle the history command - show version history for downloaded secrets
async fn handle_history(
    _config: &Config,
    repo: &SecretRepository,
    secret_name: Option<String>,
    verbose: bool,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    use chrono_humanize::HumanTime;

    // Get all downloaded secrets
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        println!("No secrets downloaded. Use 'jaws pull' first.");
        return Ok(());
    }

    // If secret_name provided, filter; otherwise show picker
    let selected_secrets: Vec<_> = if let Some(name) = &secret_name {
        downloaded
            .into_iter()
            .filter(|(s, _)| {
                s.display_name.to_lowercase().contains(&name.to_lowercase())
                    || s.hash.starts_with(name)
            })
            .collect()
    } else {
        // Show picker for selecting a secret
        use ff::{create_items_channel, run_tui_with_config, TuiConfig};

        let (tx, rx) = create_items_channel();

        for (secret, _download) in &downloaded {
            let display = format!("{} | {}", secret.provider_id, secret.display_name);
            if tx.send(display).await.is_err() {
                break;
            }
        }
        drop(tx);

        let mut tui_config = TuiConfig::fullscreen();
        tui_config.show_help_text = false;

        let selected = run_tui_with_config(rx, false, tui_config) // single select
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        if selected.is_empty() {
            return Ok(());
        }

        // Find the selected secret
        downloaded
            .into_iter()
            .filter(|(s, _)| {
                let display = format!("{} | {}", s.provider_id, s.display_name);
                selected.contains(&display)
            })
            .collect()
    };

    if selected_secrets.is_empty() {
        println!(
            "No matching secrets found{}",
            secret_name
                .map(|n| format!(" for '{}'", n))
                .unwrap_or_default()
        );
        return Ok(());
    }

    // Show history for each selected secret
    for (secret, _latest_download) in selected_secrets {
        let downloads = repo.list_downloads(secret.id)?;

        if downloads.is_empty() {
            println!("{}: No download history", secret.display_name);
            continue;
        }

        println!("\n{}", secret.display_name);
        println!("{}", "-".repeat(secret.display_name.len().min(60)));

        let versions_to_show: Vec<_> = if let Some(n) = limit {
            downloads.into_iter().take(n).collect()
        } else {
            downloads
        };

        for (i, download) in versions_to_show.iter().enumerate() {
            let age = HumanTime::from(download.downloaded_at);
            let current_marker = if i == 0 { " (current)" } else { "" };

            if verbose {
                println!(
                    "  v{}: {} | {} | {}{}",
                    download.version,
                    download.downloaded_at.format("%Y-%m-%d %H:%M:%S"),
                    download.file_hash.as_deref().unwrap_or("no hash"),
                    download.filename,
                    current_marker
                );
            } else {
                println!(
                    "  v{}: {}{}",
                    download.version,
                    age,
                    current_marker
                );
            }
        }
    }

    Ok(())
}

/// Attempt to commit changes to VCS, auto-initializing if needed
fn try_vcs_commit(secrets_path: &std::path::Path, message: &str) {
    if !secrets_path.exists() {
        return;
    }

    // Auto-initialize if not already initialized, otherwise load
    let vcs_result = if vcs::SecretsVcs::exists(secrets_path) {
        vcs::SecretsVcs::load(secrets_path)
    } else {
        vcs::SecretsVcs::init(secrets_path)
    };

    if let Ok(mut vcs) = vcs_result {
        if let Err(e) = vcs.commit(message) {
            // Log but don't fail - VCS is supplementary
            eprintln!("Warning: VCS commit failed: {}", e);
        }
    }
}

/// Handle the local delete command - delete local secret files and DB records
async fn handle_delete(
    config: &Config,
    repo: &SecretRepository,
    secret_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, Write};

    // Get all downloaded secrets
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        println!("No local secrets to delete.");
        return Ok(());
    }

    // Select secret to delete
    let selected_secret = if let Some(name) = &secret_name {
        let matches: Vec<_> = downloaded
            .iter()
            .filter(|(s, _)| {
                s.display_name.to_lowercase().contains(&name.to_lowercase())
                    || s.hash.starts_with(name)
            })
            .collect();

        if matches.is_empty() {
            return Err(format!("No secret found matching '{}'", name).into());
        } else if matches.len() > 1 {
            return Err(format!(
                "Multiple secrets match '{}'. Be more specific.",
                name
            )
            .into());
        }
        matches[0].clone()
    } else {
        // Show picker for selecting a secret
        use ff::{create_items_channel, run_tui_with_config, TuiConfig};

        let (tx, rx) = create_items_channel();

        for (secret, _download) in &downloaded {
            let display = format!("{} | {}", secret.provider_id, secret.display_name);
            if tx.send(display).await.is_err() {
                break;
            }
        }
        drop(tx);

        let mut tui_config = TuiConfig::fullscreen();
        tui_config.show_help_text = false;

        let selected = run_tui_with_config(rx, false, tui_config)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        if selected.is_empty() {
            return Ok(());
        }

        downloaded
            .into_iter()
            .find(|(s, _)| {
                let display = format!("{} | {}", s.provider_id, s.display_name);
                selected.contains(&display)
            })
            .ok_or("Secret not found")?
    };

    let (secret, _latest_download) = selected_secret;

    // Get all versions for this secret
    let all_downloads = repo.list_downloads(secret.id)?;
    let version_count = all_downloads.len();

    // Prompt for confirmation
    print!(
        "Delete '{}' and {} local version(s)? [y/N]: ",
        secret.display_name, version_count
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input != "y" && input != "yes" {
        println!("Cancelled.");
        return Ok(());
    }

    // Delete all version files
    let mut deleted_files = 0;
    for download in &all_downloads {
        let file_path = get_secret_path(&config.secrets_path(), &download.filename);
        if file_path.exists() {
            fs::remove_file(&file_path)?;
            deleted_files += 1;
        }
    }

    // Delete DB records (downloads are deleted via CASCADE when secret is deleted)
    repo.delete_secret(secret.id)?;

    println!(
        "Deleted '{}' ({} file(s), {} version record(s))",
        secret.display_name, deleted_files, version_count
    );

    // Commit deletion to VCS
    try_vcs_commit(
        &config.secrets_path(),
        &format!("delete: {}", secret.display_name),
    );

    Ok(())
}

/// Handle remote subcommands
async fn handle_remote(
    _config: &Config,
    providers: &[Provider],
    command: cli::RemoteCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        cli::RemoteCommands::Delete { secret_name, force } => {
            handle_remote_delete(providers, secret_name, force).await?;
        }
        cli::RemoteCommands::Rollback {
            secret_name,
            version_id,
        } => {
            handle_remote_rollback(providers, secret_name, version_id).await?;
        }
        cli::RemoteCommands::History { secret_name: _ } => {
            handle_remote_history().await?;
        }
    }
    Ok(())
}

/// Handle the remote history command - placeholder
async fn handle_remote_history() -> Result<(), Box<dyn std::error::Error>> {
    println!("Remote history is not yet implemented.");
    println!("Use 'jaws history' to view local version history.");
    Ok(())
}

/// Handle the undo command - undo the last VCS operation
async fn handle_undo(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let secrets_path = config.secrets_path();

    if !vcs::SecretsVcs::exists(&secrets_path) {
        println!("No history yet. Run 'jaws pull' to download secrets first.");
        return Ok(());
    }

    let mut vcs = vcs::SecretsVcs::load(&secrets_path)?;

    match vcs.undo() {
        Ok(undone_desc) => {
            println!("Undid operation: {}", undone_desc);
        }
        Err(e) => {
            return Err(format!("Failed to undo: {}", e).into());
        }
    }

    Ok(())
}

/// Handle the log command - show jj operation history
async fn handle_log(
    config: &Config,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    use chrono_humanize::HumanTime;

    let secrets_path = config.secrets_path();

    if !vcs::SecretsVcs::exists(&secrets_path) {
        println!("No history yet. Run 'jaws pull' to download secrets first.");
        return Ok(());
    }

    let vcs = vcs::SecretsVcs::load(&secrets_path)?;
    let history = vcs.history()?;

    if history.is_empty() {
        println!("No operations in history.");
        return Ok(());
    }

    let entries_to_show: Vec<_> = if let Some(n) = limit {
        history.into_iter().take(n).collect()
    } else {
        history
    };

    println!("Operation log:");
    for (i, entry) in entries_to_show.iter().enumerate() {
        let age = HumanTime::from(entry.timestamp);
        let current_marker = if i == 0 { " <- current" } else { "" };

        println!(
            "  {} {} | {}{}",
            entry.id_short,
            age,
            entry.description,
            current_marker
        );
    }

    Ok(())
}

/// Handle the diff command - show diff between operations (placeholder)
async fn handle_diff(
    config: &Config,
    operation: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let secrets_path = config.secrets_path();

    if !vcs::SecretsVcs::exists(&secrets_path) {
        println!("No history yet. Run 'jaws pull' to download secrets first.");
        return Ok(());
    }

    // TODO: Implement proper diff functionality
    // For now, just show that we're comparing against an operation
    if let Some(op) = operation {
        println!("Diff against operation {} (not yet implemented)", op);
    } else {
        println!("Diff against previous operation (not yet implemented)");
    }

    println!("Tip: Use 'jaws log' to see operation history");

    Ok(())
}

/// Handle the restore command - restore a previous version of a secret
async fn handle_restore(
    config: &Config,
    repo: &SecretRepository,
    secret_name: Option<String>,
    version: Option<i32>,
    edit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get all downloaded secrets
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        println!("No secrets downloaded. Use 'jaws pull' first.");
        return Ok(());
    }

    // If secret_name provided, filter; otherwise show picker
    let selected_secret = if let Some(name) = &secret_name {
        let matches: Vec<_> = downloaded
            .iter()
            .filter(|(s, _)| {
                s.display_name.to_lowercase().contains(&name.to_lowercase())
                    || s.hash.starts_with(name)
            })
            .collect();

        if matches.is_empty() {
            return Err(format!("No secret found matching '{}'", name).into());
        } else if matches.len() > 1 {
            return Err(format!(
                "Multiple secrets match '{}'. Be more specific.",
                name
            )
            .into());
        }
        matches[0].clone()
    } else {
        // Show picker for selecting a secret
        use ff::{create_items_channel, run_tui_with_config, TuiConfig};

        let (tx, rx) = create_items_channel();

        for (secret, _download) in &downloaded {
            let display = format!("{} | {}", secret.provider_id, secret.display_name);
            if tx.send(display).await.is_err() {
                break;
            }
        }
        drop(tx);

        let mut tui_config = TuiConfig::fullscreen();
        tui_config.show_help_text = false;

        let selected = run_tui_with_config(rx, false, tui_config)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        if selected.is_empty() {
            return Ok(());
        }

        downloaded
            .into_iter()
            .find(|(s, _)| {
                let display = format!("{} | {}", s.provider_id, s.display_name);
                selected.contains(&display)
            })
            .ok_or("Secret not found")?
    };

    let (secret, latest_download) = selected_secret;

    // Get all versions for this secret
    let all_downloads = repo.list_downloads(secret.id)?;

    if all_downloads.len() <= 1 {
        println!("Only one version exists for '{}'. Nothing to restore.", secret.display_name);
        return Ok(());
    }

    // Select version to restore
    let target_download = if let Some(v) = version {
        repo.get_download_by_version(secret.id, v)?
            .ok_or_else(|| format!("Version {} not found for '{}'", v, secret.display_name))?
    } else {
        // Show picker for version selection
        use ff::{create_items_channel, run_tui_with_config, TuiConfig};
        use chrono_humanize::HumanTime;

        let (tx, rx) = create_items_channel();

        // Skip the current (latest) version - we want to restore to something else
        for download in all_downloads.iter().skip(1) {
            let age = HumanTime::from(download.downloaded_at);
            let display = format!(
                "v{} - {} - {}",
                download.version,
                age,
                download.file_hash.as_deref().map(|h| &h[..8]).unwrap_or("?")
            );
            if tx.send(display).await.is_err() {
                break;
            }
        }
        drop(tx);

        if all_downloads.len() <= 1 {
            println!("Only one version exists. Nothing to restore.");
            return Ok(());
        }

        let mut tui_config = TuiConfig::with_height(10.min(all_downloads.len() as u16 + 2));
        tui_config.show_help_text = false;

        let selected = run_tui_with_config(rx, false, tui_config)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        if selected.is_empty() {
            return Ok(());
        }

        // Parse version from selection "v{version} - ..."
        let selected_str = &selected[0];
        let version_str = selected_str
            .strip_prefix("v")
            .and_then(|s| s.split(" - ").next())
            .ok_or("Failed to parse version")?;
        let selected_version: i32 = version_str.parse()?;

        repo.get_download_by_version(secret.id, selected_version)?
            .ok_or("Selected version not found")?
    };

    // Read the old version's content
    let old_file_path = get_secret_path(&config.secrets_path(), &target_download.filename);
    if !old_file_path.exists() {
        return Err(format!(
            "Version {} file not found at: {}\nThe file may have been deleted.",
            target_download.version,
            old_file_path.display()
        )
        .into());
    }

    let content = fs::read_to_string(&old_file_path)?;

    // Create a new version with this content (next version number after latest)
    let new_version = latest_download.version + 1;
    let (new_filename, content_hash) = save_secret_file(
        &config.secrets_path(),
        &secret.display_name,
        &secret.hash,
        new_version,
        &content,
    )?;

    // Record the new download
    repo.create_download(secret.id, &new_filename, &content_hash)?;

    println!(
        "Restored '{}' from v{} -> v{} (new current)",
        secret.display_name, target_download.version, new_version
    );

    // Commit to VCS
    let commit_msg = format!(
        "restore: {} from v{} to v{}",
        secret.display_name, target_download.version, new_version
    );
    try_vcs_commit(&config.secrets_path(), &commit_msg);

    // Open in editor if requested
    if edit {
        let file_path = get_secret_path(&config.secrets_path(), &new_filename);
        let _ = Command::new(config.editor())
            .arg(file_path.to_string_lossy().to_string())
            .status()
            .expect("failed to launch editor");
    }

    Ok(())
}
