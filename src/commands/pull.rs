//! Pull command handlers - downloading secrets from providers.

use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::Utc;
use futures::StreamExt;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::db::{DbSecret, SecretInput, SecretRepository};
use crate::secrets::{
    Provider, SecretRef, get_secret_path, hash_api_ref, load_secret_file, save_secret_file,
};
use crate::utils::{format_error, parse_secret_ref};

/// Handle the pull command
pub async fn handle_pull(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: Option<String>,
    edit: bool,
    print: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use ff::{FuzzyFinderWithIndicators, ItemIndicator, TuiConfig};

    // Handle print mode - requires a secret name
    if print {
        let name = secret_name
            .ok_or("--print requires a secret name. Usage: jaws pull PROVIDER://SECRET_NAME -p")?;
        return handle_pull_print(config, repo, providers, &name).await;
    }

    // Handle direct pull by name (non-print mode)
    if let Some(name) = secret_name {
        return handle_pull_by_name(config, repo, providers, &name, edit).await;
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
            let mut sent_displays: std::collections::HashSet<String> =
                std::collections::HashSet::new();

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
            let should_refresh =
                super::sync::should_refresh_cache(&repo, &provider_id, cache_ttl).unwrap_or(true);

            if should_refresh {
                // Stream from remote and update DB
                while let Some(result) = secret_stream.next().await {
                    if let Ok(secret_ref) = result {
                        // Parse based on provider type
                        let (api_ref, display_name) = if provider_kind == "onepassword" {
                            let parsed = SecretRef::parse(&secret_ref);
                            (parsed.api_ref, parsed.display_path)
                        } else {
                            (secret_ref.clone(), secret_ref.clone())
                        };
                        let hash = hash_api_ref(&api_ref);

                        let display = format!("{} | {}", provider_id, display_name);
                        let is_new = sent_displays.insert(display.clone());

                        // Add to TUI with spinner if new item
                        if is_new
                            && session
                                .add_with_indicator(&display, ItemIndicator::Spinner)
                                .await
                                .is_err()
                        {
                            return;
                        }

                        // Upsert to DB
                        let input = SecretInput {
                            provider_id: provider_id.clone(),
                            api_ref: api_ref.clone(),
                            display_name: display_name.clone(),
                            hash: hash.clone(),
                            description: None,
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
                                (provider_id.clone(), secret_id, api_ref, display_name, hash),
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
                                    eprintln!(
                                        "✗ {} [{}]: Failed to save - {}",
                                        display_name, provider_id, e
                                    );
                                    fail_count.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "✗ {} [{}]: {}",
                                display_name,
                                provider_id,
                                format_error(e.as_ref())
                            );
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

    // Open in editor if requested
    if edit && !downloaded_files.is_empty() {
        let _ = Command::new(config.editor())
            .args(&downloaded_files)
            .status()
            .expect("failed to launch editor");
    }

    Ok(())
}

/// Handle pull with --print flag: fetch and print secret to stdout
async fn handle_pull_print(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the secret reference
    let (provider_id, secret_name) = parse_secret_ref(name, config.default_provider().as_deref())
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // Find the provider
    let provider = providers
        .iter()
        .find(|p| p.id() == provider_id)
        .ok_or_else(|| {
            format!(
                "Unknown provider: '{}'. Available providers: {}",
                provider_id,
                providers
                    .iter()
                    .map(|p| p.id())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    // Look up secret in DB by provider and display_name
    let secret = repo.find_secret_by_provider_and_name(&provider_id, &secret_name)?;

    let content = if let Some(secret) = &secret {
        // Check if we have a cached copy and if it's still fresh
        if let Some(download) = repo.get_latest_download(secret.id)? {
            let age = Utc::now() - download.downloaded_at;
            if age.num_seconds() < config.cache_ttl() as i64 {
                // Use cached version - read from file
                load_secret_file(&config.secrets_path(), &download.filename)?
            } else {
                // Cache expired - fetch fresh and save
                fetch_and_save_secret(config, repo, provider, secret).await?
            }
        } else {
            // No download record - fetch and save
            fetch_and_save_secret(config, repo, provider, secret).await?
        }
    } else {
        // Secret not in DB - try to sync this provider first
        return Err(format!(
            "Secret '{}' not found in provider '{}'. Run 'jaws sync' first or check the name.\n\
             Hint: Use 'jaws list --provider {}' to see available secrets.",
            secret_name, provider_id, provider_id
        )
        .into());
    };

    // Print to stdout without extra newline (let the content speak for itself)
    print!("{}", content);

    Ok(())
}

/// Fetch a secret from the provider and save it locally
pub async fn fetch_and_save_secret(
    config: &Config,
    repo: &SecretRepository,
    provider: &Provider,
    secret: &DbSecret,
) -> Result<String, Box<dyn std::error::Error>> {
    // Fetch from provider
    let content = provider.get_secret(&secret.api_ref).await?;

    // Get next version number
    let version = repo
        .get_latest_download(secret.id)
        .ok()
        .flatten()
        .map(|d| d.version + 1)
        .unwrap_or(1);

    // Save to file
    let (filename, content_hash) = save_secret_file(
        &config.secrets_path(),
        &secret.display_name,
        &secret.hash,
        version,
        &content,
    )?;

    // Record download in database
    repo.create_download(secret.id, &filename, &content_hash)?;

    Ok(content)
}

/// Handle pull by name (non-print mode) - download and optionally edit
async fn handle_pull_by_name(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    name: &str,
    edit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the secret reference
    let (provider_id, secret_name) = parse_secret_ref(name, config.default_provider().as_deref())
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // Find the provider
    let provider = providers
        .iter()
        .find(|p| p.id() == provider_id)
        .ok_or_else(|| {
            format!(
                "Unknown provider: '{}'. Available providers: {}",
                provider_id,
                providers
                    .iter()
                    .map(|p| p.id())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    // Look up secret in DB
    let secret = repo
        .find_secret_by_provider_and_name(&provider_id, &secret_name)?
        .ok_or_else(|| {
            format!(
                "Secret '{}' not found in provider '{}'. Run 'jaws sync' first or check the name.",
                secret_name, provider_id
            )
        })?;

    // Fetch and save
    let content = fetch_and_save_secret(config, repo, provider, &secret).await?;

    // Get the file path for the downloaded secret
    let download = repo
        .get_latest_download(secret.id)?
        .ok_or("Failed to get download record")?;
    let file_path = get_secret_path(&config.secrets_path(), &download.filename);

    println!("Downloaded: {}://{}", provider_id, secret_name);

    // Open in editor if requested
    if edit {
        // Make sure the content is written before opening editor
        drop(content);
        let _ = Command::new(config.editor())
            .arg(&file_path)
            .status()
            .expect("failed to launch editor");
    }

    Ok(())
}

/// Handle pull with --inject flag: process a template file and replace secret placeholders
pub async fn handle_pull_inject(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    template_path: &std::path::Path,
    output_path: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    use regex::Regex;

    // Read the template file
    let template_content = std::fs::read_to_string(template_path).map_err(|e| {
        format!(
            "Failed to read template file '{}': {}",
            template_path.display(),
            e
        )
    })?;

    // Pattern to match {{ EXPRESSION }}
    // Captures everything inside the braces non-greedily
    let pattern = Regex::new(r"\{\{(.+?)\}\}").expect("Invalid regex pattern");

    // Find all unique secret references/expressions
    let mut secret_refs: Vec<String> = pattern
        .captures_iter(&template_content)
        .map(|cap| cap[1].to_string())
        .collect();
    secret_refs.sort();
    secret_refs.dedup();

    if secret_refs.is_empty() {
        // No secrets to inject, just output the template as-is
        if let Some(output) = output_path {
            std::fs::write(output, &template_content)?;
            eprintln!(
                "No secret placeholders found. Output written to: {}",
                output.display()
            );
        } else {
            print!("{}", template_content);
        }
        return Ok(());
    }

    // Resolve all secrets
    let mut resolved: HashMap<String, String> = HashMap::new();
    let mut errors: Vec<String> = Vec::new();

    for full_ref in &secret_refs {
        let mut found_value: Option<String> = None;
        let mut candidate_errors: Vec<String> = Vec::new();

        // Split by || and iterate through candidates
        let candidates: Vec<&str> = full_ref.split("||").map(|s| s.trim()).collect();

        for candidate in candidates {
            // Check for string literals (quoted with " or ')
            if (candidate.starts_with('"') && candidate.ends_with('"'))
                || (candidate.starts_with('\'') && candidate.ends_with('\''))
                    && candidate.len() >= 2
            {
                found_value = Some(candidate[1..candidate.len() - 1].to_string());
                break;
            }

            // Try to resolve as a secret reference
            // Parse the secret reference
            let (provider_id, secret_name) =
                match parse_secret_ref(candidate, config.default_provider().as_deref()) {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        // Not a valid secret ref (and not a string literal), probably malformed
                        // Only log if it's the last candidate or clearly meant to be a secret?
                        // For now, we log it as an error for this candidate
                        candidate_errors.push(format!("Invalid format: {}", candidate));
                        continue;
                    }
                };

            // Find the provider
            let provider = match providers.iter().find(|p| p.id() == provider_id) {
                Some(p) => p,
                None => {
                    candidate_errors.push(format!("Unknown provider: {}", provider_id));
                    continue;
                }
            };

            // Look up secret in DB
            let secret = match repo.find_secret_by_provider_and_name(&provider_id, &secret_name) {
                Ok(Some(s)) => s,
                Ok(None) => {
                    candidate_errors.push(format!(
                        "Secret not found: {}://{}",
                        provider_id, secret_name
                    ));
                    continue;
                }
                Err(e) => {
                    candidate_errors.push(format!("DB error: {}", e));
                    continue;
                }
            };

            // Fetch the secret value (respecting TTL cache)
            let fetch_result = if let Ok(Some(download)) = repo.get_latest_download(secret.id) {
                let age = Utc::now() - download.downloaded_at;
                if age.num_seconds() < config.cache_ttl() as i64 {
                    // Use cached version
                    match load_secret_file(&config.secrets_path(), &download.filename) {
                        Ok(content) => Ok(content),
                        Err(e) => Err(format!("Failed to load cache: {}", e)),
                    }
                } else {
                    // Cache expired - fetch fresh
                    match fetch_and_save_secret(config, repo, provider, &secret).await {
                        Ok(content) => Ok(content),
                        Err(e) => Err(format!("Failed to fetch: {}", e)),
                    }
                }
            } else {
                // No download record - fetch and save
                match fetch_and_save_secret(config, repo, provider, &secret).await {
                    Ok(content) => Ok(content),
                    Err(e) => Err(format!("Failed to fetch: {}", e)),
                }
            };

            match fetch_result {
                Ok(content) => {
                    found_value = Some(content);
                    break; // Successfully found a value, stop checking other candidates
                }
                Err(e) => {
                    candidate_errors.push(e);
                }
            }
        }

        if let Some(value) = found_value {
            resolved.insert(full_ref.clone(), value);
        } else {
            // All candidates failed
            errors.push(format!(
                "  {{{{{}}}}}: All candidates failed:\n    - {}",
                full_ref,
                candidate_errors.join("\n    - ")
            ));
        }
    }

    // Check for errors
    if !errors.is_empty() {
        return Err(format!(
            "Failed to resolve {} placeholder(s) in template:\n{}",
            errors.len(),
            errors.join("\n")
        )
        .into());
    }

    // Replace all placeholders with resolved values
    let mut output_content = template_content;
    for (full_ref, value) in &resolved {
        let placeholder = format!("{{{{{}}}}}", full_ref);
        output_content = output_content.replace(&placeholder, value);
    }

    // Write output
    if let Some(output) = output_path {
        std::fs::write(output, &output_content)?;
        eprintln!(
            "Injected {} value(s). Output written to: {}",
            resolved.len(),
            output.display()
        );
    } else {
        print!("{}", output_content);
    }

    Ok(())
}
