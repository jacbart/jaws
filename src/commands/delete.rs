//! Delete command handlers - deleting secrets (local, remote, or both).

use std::fs;
use std::io::{self, Write};

use crate::cli::DeleteScope;
use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::{Provider, get_secret_path};
use crate::utils::parse_secret_ref;

/// Handle the unified delete command - can delete local, remote, or both
pub async fn handle_delete(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: Option<String>,
    scope: Option<DeleteScope>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get all downloaded secrets for selection
    let downloaded = repo.list_all_downloaded_secrets()?;

    // Select secret to delete
    let (provider_id, secret_display_name, db_secret) = if let Some(name) = &secret_name {
        // Parse the secret reference
        let (pid, sname) = parse_secret_ref(name, config.default_provider().as_deref())
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        // Try to find in downloaded secrets first
        let db_secret = downloaded
            .iter()
            .find(|(s, _)| s.provider_id == pid && s.display_name == sname)
            .map(|(s, _)| s.clone());

        (pid, sname, db_secret)
    } else {
        // Show TUI picker - combine local and remote secrets
        use ff::{TuiConfig, create_items_channel, run_tui_with_config};

        let (tx, rx) = create_items_channel();

        // Add downloaded secrets
        let mut items: Vec<(String, String)> = Vec::new();
        for (secret, _download) in &downloaded {
            items.push((secret.provider_id.clone(), secret.display_name.clone()));
        }

        // Also sync and add remote secrets that aren't downloaded
        for provider in providers {
            let cached = repo.list_secrets_by_provider(provider.id()).unwrap_or_default();
            for secret in cached {
                if !items.iter().any(|(p, n)| p == &secret.provider_id && n == &secret.display_name) {
                    items.push((secret.provider_id.clone(), secret.display_name.clone()));
                }
            }
        }

        if items.is_empty() {
            println!("No secrets found. Run 'jaws sync' first to discover secrets from providers.");
            return Ok(());
        }

        for (provider_id, display_name) in &items {
            let display = format!("{} | {}", provider_id, display_name);
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

        // Parse selection "PROVIDER | NAME"
        let (_, selected_str) = &selected[0];
        let parts: Vec<&str> = selected_str.split(" | ").collect();
        if parts.len() != 2 {
            return Err("Failed to parse selection".into());
        }

        let pid = parts[0].to_string();
        let sname = parts[1].to_string();

        // Find the DB secret if it exists locally
        let db_secret = downloaded
            .iter()
            .find(|(s, _)| s.provider_id == pid && s.display_name == sname)
            .map(|(s, _)| s.clone());

        (pid, sname, db_secret)
    };

    // Determine scope - prompt if not provided
    let delete_scope = if let Some(s) = scope {
        s
    } else {
        prompt_delete_scope(&provider_id, &secret_display_name, db_secret.is_some())?
    };

    // Execute deletion based on scope
    match delete_scope {
        DeleteScope::Local => {
            delete_local(config, repo, &provider_id, &secret_display_name, db_secret.as_ref()).await?;
        }
        DeleteScope::Remote => {
            delete_remote(providers, &provider_id, &secret_display_name, force).await?;
        }
        DeleteScope::Both => {
            // Delete remote first, then local
            delete_remote(providers, &provider_id, &secret_display_name, force).await?;
            if db_secret.is_some() {
                delete_local(config, repo, &provider_id, &secret_display_name, db_secret.as_ref()).await?;
            }
        }
    }

    Ok(())
}

/// Prompt user to select delete scope
fn prompt_delete_scope(
    provider_id: &str,
    secret_name: &str,
    has_local: bool,
) -> Result<DeleteScope, Box<dyn std::error::Error>> {
    use ff::{TuiConfig, create_items_channel, run_tui_with_config};

    println!("Delete '{}://{}'", provider_id, secret_name);
    println!();

    // Build options based on what's available
    let mut options = Vec::new();
    
    if has_local {
        options.push(("local", "Delete local cached files only"));
    }
    
    // Remote is always an option (assuming secret exists on provider)
    if provider_id != "jaws" {
        options.push(("remote", "Delete from remote provider only"));
    }
    
    if has_local && provider_id != "jaws" {
        options.push(("both", "Delete from both local and remote"));
    } else if provider_id == "jaws" {
        // For jaws provider, "local" is effectively "both" since it's only local
        if !has_local {
            return Err("Secret not found locally".into());
        }
    }

    if options.is_empty() {
        return Err("No delete options available for this secret".into());
    }

    // If only one option, use it
    if options.len() == 1 {
        return match options[0].0 {
            "local" => Ok(DeleteScope::Local),
            "remote" => Ok(DeleteScope::Remote),
            "both" => Ok(DeleteScope::Both),
            _ => unreachable!(),
        };
    }

    // Use TUI for selection
    let rt = tokio::runtime::Handle::current();
    let result = rt.block_on(async {
        let (tx, rx) = create_items_channel();

        for (key, desc) in &options {
            let display = format!("{} - {}", key, desc);
            if tx.send(display).await.is_err() {
                break;
            }
        }
        drop(tx);

        let mut tui_config = TuiConfig::with_height((options.len() as u16 + 2).min(10));
        tui_config.show_help_text = false;

        run_tui_with_config(rx, false, tui_config).await
    }).map_err(|e| e as Box<dyn std::error::Error>)?;

    if result.is_empty() {
        return Err("Cancelled".into());
    }

    let (_, selected) = &result[0];
    if selected.starts_with("local") {
        Ok(DeleteScope::Local)
    } else if selected.starts_with("remote") {
        Ok(DeleteScope::Remote)
    } else if selected.starts_with("both") {
        Ok(DeleteScope::Both)
    } else {
        Err("Invalid selection".into())
    }
}

/// Delete local secret files and DB records
async fn delete_local(
    config: &Config,
    repo: &SecretRepository,
    provider_id: &str,
    secret_name: &str,
    db_secret: Option<&crate::db::DbSecret>,
) -> Result<(), Box<dyn std::error::Error>> {
    let secret = match db_secret {
        Some(s) => s.clone(),
        None => {
            println!("No local files found for '{}://{}'", provider_id, secret_name);
            return Ok(());
        }
    };

    // Get all versions for this secret
    let all_downloads = repo.list_downloads(secret.id)?;
    let version_count = all_downloads.len();

    // Prompt for confirmation
    print!(
        "Delete local '{}' and {} version(s)? [y/N]: ",
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
        "Deleted local '{}' ({} file(s), {} version record(s))",
        secret.display_name, deleted_files, version_count
    );

    // Log the operation
    repo.log_operation("delete_local", provider_id, &secret.display_name, None)?;

    Ok(())
}

/// Delete secret from remote provider
async fn delete_remote(
    providers: &[Provider],
    provider_id: &str,
    secret_name: &str,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find the provider
    let provider = providers
        .iter()
        .find(|p| p.id() == provider_id)
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    // For jaws provider, remote delete is the same as local delete
    if provider_id == "jaws" {
        println!("Note: 'jaws' is a local-only provider. Use --scope=local to delete.");
        return Ok(());
    }

    match provider.delete(secret_name, force).await {
        Ok(()) => {
            if force {
                println!("{}://{} deleted from remote (force)", provider_id, secret_name);
            } else {
                println!(
                    "{}://{} deleted from remote (recovery period: 7-30 days)",
                    provider_id, secret_name
                );
            }
        }
        Err(e) => {
            return Err(format!(
                "Error deleting '{}' from '{}': {}",
                secret_name, provider_id, e
            )
            .into());
        }
    }

    Ok(())
}
