//! Public command handlers for config management.

use std::path::PathBuf;

use ff::{FuzzyFinderSession, TuiConfig};

use crate::config::{Config, Defaults};
use crate::db::{SecretRepository, init_db};
use crate::keychain;

use super::discovery::{
    discover_and_add_aws, discover_and_add_bitwarden, discover_and_add_gcp,
    discover_and_add_onepassword,
};
use super::helpers::{PendingCredential, store_pending_credentials};

/// Handle interactive config generation (`jaws config init`).
pub async fn handle_interactive_generate(
    path: Option<PathBuf>,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Interactive Configuration Setup");
    println!("================================\n");

    // Determine config path
    let config_path = if let Some(p) = path {
        p
    } else {
        println!("Select where to save the config file:\n");

        let options = Config::get_config_location_options();
        let items: Vec<String> = options
            .iter()
            .map(|(path, desc)| format!("{} — {}", path.display(), desc))
            .collect();

        let mut tui_config = TuiConfig::with_height((items.len() as u16 + 3).min(10));
        tui_config.show_help_text = false;

        let (session, tui_future) = FuzzyFinderSession::with_config(false, tui_config);

        for item in &items {
            let _ = session.add(item).await;
        }
        drop(session);

        let selected = tui_future.await.unwrap_or_default();

        if selected.is_empty() {
            println!("No location selected. Cancelled.");
            return Ok(());
        }

        let (_, selected_str) = &selected[0];
        options
            .into_iter()
            .find(|(path, desc)| {
                let display = format!("{} — {}", path.display(), desc);
                &display == selected_str
            })
            .map(|(path, _)| path)
            .unwrap_or_else(Config::default_config_path)
    };

    if config_path.exists() && !overwrite {
        return Err(format!(
            "Config file already exists at: {}. Use --overwrite to replace it.",
            config_path.display()
        )
        .into());
    }

    println!();

    // Prompt for defaults
    let default_editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".into());
    let editor = super::helpers::prompt("Editor", &default_editor);
    let secrets_path = super::helpers::prompt("Secrets path", "./.secrets");
    let cache_ttl_str = super::helpers::prompt("Cache TTL (seconds)", "900");
    let cache_ttl: u64 = cache_ttl_str.parse().unwrap_or(900);

    let mut config = Config {
        defaults: Some(Defaults {
            editor: Some(editor),
            secrets_path: Some(secrets_path),
            cache_ttl: Some(cache_ttl),
            default_provider: None,
            max_versions: None,
            keychain_cache: None,
        }),
        providers: Vec::new(),
    };

    let mut pending_credentials: Vec<PendingCredential> = Vec::new();

    println!();

    // Discover providers
    discover_and_add_aws(&mut config, &mut pending_credentials).await?;
    println!();
    discover_and_add_onepassword(&mut config, &mut pending_credentials).await?;
    println!();
    discover_and_add_bitwarden(&mut config, &mut pending_credentials).await?;
    println!();
    discover_and_add_gcp(&mut config, &mut pending_credentials).await?;
    println!();

    // Create parent directories if they don't exist
    if let Some(parent) = config_path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?;
    }

    // Save config
    config.save(&config_path)?;
    println!("Config written to: {}", config_path.display());

    // Store encrypted credentials
    store_pending_credentials(&config, &pending_credentials)?;

    if config.providers.is_empty() {
        println!();
        println!(
            "Note: No providers were added. Edit {} to add providers manually.",
            config_path.display()
        );
    }

    Ok(())
}

/// Handle `jaws config provider add [--kind <type>]`.
pub async fn handle_add_provider(kind: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = match Config::find_existing_config() {
        Some(path) => path,
        None => {
            return Err("No config file found. Run 'jaws config init' first.".into());
        }
    };

    let mut config = Config::load_from(Some(&config_path))?;
    let mut pending_credentials: Vec<PendingCredential> = Vec::new();

    let provider_kind = if let Some(k) = kind {
        k.to_lowercase()
    } else {
        let provider_types = vec![
            "aws - Amazon Web Services Secrets Manager".to_string(),
            "onepassword - 1Password".to_string(),
            "bitwarden - Bitwarden Secrets Manager".to_string(),
            "gcp - Google Cloud Secret Manager".to_string(),
        ];

        let mut tui_config = TuiConfig::with_height(6);
        tui_config.show_help_text = false;

        let (session, tui_future) = FuzzyFinderSession::with_config(false, tui_config);

        for item in &provider_types {
            let _ = session.add(item).await;
        }
        drop(session);

        let selected = tui_future.await.unwrap_or_default();

        if selected.is_empty() {
            println!("No provider type selected. Cancelled.");
            return Ok(());
        }

        let (_, selected_str) = &selected[0];
        selected_str
            .split(" - ")
            .next()
            .unwrap_or("aws")
            .trim()
            .to_string()
    };

    println!();

    let added = match provider_kind.as_str() {
        "aws" => discover_and_add_aws(&mut config, &mut pending_credentials).await?,
        "onepassword" | "1password" | "op" => {
            discover_and_add_onepassword(&mut config, &mut pending_credentials).await?
        }
        "bitwarden" | "bw" | "bws" => {
            discover_and_add_bitwarden(&mut config, &mut pending_credentials).await?
        }
        "gcp" | "gcloud" | "google" => {
            discover_and_add_gcp(&mut config, &mut pending_credentials).await?
        }
        other => {
            return Err(format!(
                "Unknown provider kind: '{}'. Valid kinds: aws, onepassword, bitwarden, gcp",
                other
            )
            .into());
        }
    };

    if added > 0 {
        config.save(&config_path)?;
        println!();
        println!(
            "Config updated: {} provider(s) added to {}",
            added,
            config_path.display()
        );

        store_pending_credentials(&config, &pending_credentials)?;
    } else {
        println!("No providers were added.");
    }

    Ok(())
}

/// Handle `jaws config provider remove [id]`.
pub async fn handle_remove_provider(
    config: &Config,
    id: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = match Config::find_existing_config() {
        Some(path) => path,
        None => {
            return Err("No config file found. Run 'jaws config init' first.".into());
        }
    };

    let mut config = config.clone();

    if config.providers.is_empty() {
        println!("No providers configured.");
        return Ok(());
    }

    let provider_id = if let Some(id) = id {
        id
    } else {
        let items: Vec<String> = config
            .providers
            .iter()
            .map(|p| format!("{} ({})", p.id, p.kind))
            .collect();

        let mut tui_config = TuiConfig::with_height((items.len() as u16 + 3).min(15));
        tui_config.show_help_text = false;

        let (session, tui_future) = FuzzyFinderSession::with_config(false, tui_config);

        for item in &items {
            let _ = session.add(item).await;
        }
        drop(session);

        let selected = tui_future.await.unwrap_or_default();

        if selected.is_empty() {
            println!("No provider selected. Cancelled.");
            return Ok(());
        }

        let (_, selected_str) = &selected[0];
        selected_str
            .split(" (")
            .next()
            .unwrap_or(selected_str)
            .trim()
            .to_string()
    };

    if config.remove_provider(&provider_id) {
        config.save(&config_path)?;
        println!(
            "Removed provider '{}' from {}",
            provider_id,
            config_path.display()
        );
    } else {
        let available: Vec<String> = config.providers.iter().map(|p| p.id.clone()).collect();
        return Err(format!(
            "Provider '{}' not found in config. Available: {}",
            provider_id,
            if available.is_empty() {
                "(none)".to_string()
            } else {
                available.join(", ")
            }
        )
        .into());
    }

    Ok(())
}

/// Handle `jaws config clear-cache` -- remove all jaws entries from the OS keychain.
pub fn handle_clear_cache(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if !keychain::keychain_available() {
        eprintln!("OS keychain is not available on this system.");
        return Ok(());
    }

    let db_path = config.db_path();
    if !db_path.exists() {
        println!("No database found -- nothing to clear.");
        return Ok(());
    }

    let conn = init_db(&db_path)?;
    let repo = SecretRepository::new(conn);

    let cleared = keychain::keychain_clear_all(&config.secrets_path(), &repo);
    if cleared > 0 {
        println!("Cleared {} cached credential(s) from OS keychain.", cleared);
    } else {
        println!("No cached credentials found in OS keychain.");
    }
    Ok(())
}
