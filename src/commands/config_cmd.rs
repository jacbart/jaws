//! Config command handlers - managing configuration.

use std::io::{self, Write};

use crate::config::{Config, Defaults, ProviderConfig};
use crate::credentials::{prompt_encryption_method, store_encrypted_credential};
use crate::db::{SecretRepository, init_db};
use crate::keychain;
use crate::secrets::{BitwardenSecretManager, OnePasswordSecretManager};

/// Credentials pending storage, collected during interactive config setup.
/// Each entry is (provider_id, credential_key, plaintext_value).
struct PendingCredential {
    provider_id: String,
    credential_key: String,
    plaintext_value: String,
}

/// Handle interactive config generation
pub async fn handle_interactive_generate(
    path: Option<std::path::PathBuf>,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use ff::{FuzzyFinderSession, TuiConfig};

    println!("Interactive Configuration Setup");
    println!("================================\n");

    // Determine config path - either from --path flag or interactive selection
    let config_path = if let Some(p) = path {
        p
    } else {
        // Show location picker
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

        // Find the selected path
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

    // Check if file exists and overwrite flag
    if config_path.exists() && !overwrite {
        return Err(format!(
            "Config file already exists at: {}. Use --overwrite to replace it.",
            config_path.display()
        )
        .into());
    }

    println!();

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

    // Helper function for y/N confirmation prompts
    fn confirm(message: &str) -> bool {
        print!("{} [y/N]: ", message);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
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
            default_provider: None,
            max_versions: None,   // Use default (10)
            keychain_cache: None, // Use default (true)
        }),
        providers: Vec::new(),
    };

    // Collect credentials to store after all providers are configured
    let mut pending_credentials: Vec<PendingCredential> = Vec::new();

    println!();

    // Discover AWS profiles
    println!("Discovering AWS profiles...");
    match Config::discover_aws_profiles() {
        Ok(profiles) if !profiles.is_empty() => {
            println!("Found {} AWS profile(s).", profiles.len());

            if confirm("Add AWS provider(s)?") {
                println!("  Tip: Use 'all' option to auto-discover profiles at runtime\n");

                // Create items for ff selection
                let mut items: Vec<String> =
                    vec!["[all] - Auto-discover all profiles at runtime".to_string()];
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
                    if selected.iter().any(|(_, s)| s.starts_with("[all]")) {
                        config.providers.push(ProviderConfig::new_aws(
                            "aws".to_string(),
                            Some("all".to_string()),
                            None,
                        ));
                        println!("Added AWS provider with auto-discovery");
                    } else {
                        // Add individual profiles
                        for (_, selection) in &selected {
                            // Extract profile name (before any region in parentheses)
                            let profile_name =
                                selection.split(" (").next().unwrap_or(selection).trim();
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
            } else {
                println!("Skipping AWS");
            }
        }
        Ok(_) => {
            println!("No AWS profiles found in ~/.aws/credentials");
            // Check if AWS env vars are set (env-var-only auth)
            if let (Ok(access_key), Ok(secret_key)) = (
                std::env::var("AWS_ACCESS_KEY_ID"),
                std::env::var("AWS_SECRET_ACCESS_KEY"),
            ) {
                println!("Found AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY in environment.");
                if confirm("Add AWS provider using environment credentials?") {
                    let region_input = prompt("AWS region", "us-west-2");
                    let region = if region_input.is_empty() {
                        None
                    } else {
                        Some(region_input)
                    };
                    config.providers.push(ProviderConfig::new_aws(
                        "aws".to_string(),
                        None, // No profile -- uses env vars
                        region,
                    ));
                    println!("Added AWS provider (env var credentials)");

                    // Check for session token (skip if present -- it's temporary)
                    if std::env::var("AWS_SESSION_TOKEN").is_ok() {
                        println!(
                            "  Note: AWS_SESSION_TOKEN detected but will NOT be stored (temporary credential)"
                        );
                    }

                    if confirm("Store encrypted copy of AWS access key credentials?") {
                        pending_credentials.push(PendingCredential {
                            provider_id: "aws".to_string(),
                            credential_key: "access_key_id".to_string(),
                            plaintext_value: access_key,
                        });
                        pending_credentials.push(PendingCredential {
                            provider_id: "aws".to_string(),
                            credential_key: "secret_access_key".to_string(),
                            plaintext_value: secret_key,
                        });
                    }
                }
            }
        }
        Err(e) => println!("Could not discover AWS profiles: {}", e),
    }

    println!();

    // Check for 1Password
    println!("Checking for 1Password...");
    let op_token_env = "OP_SERVICE_ACCOUNT_TOKEN";
    if std::env::var(op_token_env).is_ok() {
        println!("Found {}. Discovering vaults...", op_token_env);

        match OnePasswordSecretManager::new(None, op_token_env).await {
            Ok(manager) => {
                match manager.list_vaults() {
                    Ok(vaults) if !vaults.is_empty() => {
                        println!("Found {} vault(s).", vaults.len());

                        if confirm("Add 1Password provider(s)?") {
                            println!(
                                "  Tip: Use 'all' option to auto-discover vaults at runtime\n"
                            );

                            let mut items: Vec<String> =
                                vec!["[all] - Auto-discover all vaults at runtime".to_string()];
                            for vault in &vaults {
                                items.push(format!("{} ({})", vault.title, vault.id));
                            }

                            let mut tui_config =
                                TuiConfig::with_height(15.min(items.len() as u16 + 3));
                            tui_config.show_help_text = true;

                            let (session, tui_future) =
                                FuzzyFinderSession::with_config(true, tui_config);

                            for item in &items {
                                let _ = session.add(item).await;
                            }
                            drop(session);

                            let selected = tui_future.await.unwrap_or_default();

                            if !selected.is_empty() {
                                if selected.iter().any(|(_, s)| s.starts_with("[all]")) {
                                    config.providers.push(ProviderConfig::new_onepassword(
                                        "op".to_string(),
                                        Some("all".to_string()),
                                        None,
                                    ));
                                    println!("Added 1Password provider with auto-discovery");
                                } else {
                                    for (_, selection) in &selected {
                                        // Extract vault name and ID
                                        if let Some((name, rest)) = selection.split_once(" (") {
                                            let vault_id = rest.trim_end_matches(')');
                                            let provider_id = format!(
                                                "op-{}",
                                                name.to_lowercase().replace(' ', "-")
                                            );
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
                        } else {
                            println!("Skipping 1Password");
                        }
                    }
                    Ok(_) => println!("No 1Password vaults accessible"),
                    Err(e) => println!("Could not list 1Password vaults: {}", e),
                }
            }
            Err(e) => println!("Could not initialize 1Password: {}", e),
        }

        // Offer to store the 1Password token if any OP providers were added
        if config
            .providers
            .iter()
            .any(|p| matches!(p.kind.as_str(), "onepassword" | "1password" | "op"))
        {
            if let Ok(token) = std::env::var(op_token_env) {
                if confirm("Store encrypted copy of 1Password service account token?") {
                    // For all OP providers, store against a canonical provider ID.
                    // If "all" mode, use "op"; otherwise use first OP provider ID.
                    let store_id = config
                        .providers
                        .iter()
                        .find(|p| matches!(p.kind.as_str(), "onepassword" | "1password" | "op"))
                        .map(|p| p.id.clone())
                        .unwrap_or_else(|| "op".to_string());
                    pending_credentials.push(PendingCredential {
                        provider_id: store_id,
                        credential_key: "token".to_string(),
                        plaintext_value: token,
                    });
                }
            }
        }
    } else {
        println!("{} not set, skipping 1Password setup", op_token_env);
        println!("  Tip: Set this environment variable and re-run to add 1Password providers");
    }

    println!();

    // Check for Bitwarden
    println!("Checking for Bitwarden...");
    let bw_token_env = "BWS_ACCESS_TOKEN";
    if std::env::var(bw_token_env).is_ok() {
        println!("Found {}. Discovering projects...", bw_token_env);

        let mut manager = BitwardenSecretManager::new(None, bw_token_env, None).await?;
        let mut projects_result = manager.list_projects().await;
        let mut organization_id: Option<String> = None;

        // If listing failed, try prompting for Organization ID
        if projects_result.is_err() {
            println!("Could not list Bitwarden projects.");
            println!("This usually means your access token requires an explicit Organization ID.");

            let org_input = prompt("Enter Organization ID (optional, press Enter to skip)", "");
            if !org_input.is_empty() {
                println!("Retrying with Organization ID: {}", org_input);
                organization_id = Some(org_input.clone());

                // Re-initialize manager with Org ID
                manager = BitwardenSecretManager::new(None, bw_token_env, Some(org_input)).await?;
                projects_result = manager.list_projects().await;
            }
        }

        match projects_result {
            Ok(projects) if !projects.is_empty() => {
                println!("Found {} project(s).", projects.len());

                if confirm("Add Bitwarden provider(s)?") {
                    let mut items: Vec<String> = Vec::new();
                    for (name, id) in &projects {
                        items.push(format!("{} ({})", name, id));
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
                        for (_, selection) in &selected {
                            // Extract project name and ID
                            if let Some((name, rest)) = selection.split_once(" (") {
                                let project_id = rest.trim_end_matches(')');
                                let provider_id =
                                    format!("bw-{}", name.to_lowercase().replace(' ', "-"));
                                config.providers.push(ProviderConfig::new_bitwarden(
                                    provider_id,
                                    Some(project_id.to_string()),
                                    organization_id.clone(),
                                    Some(bw_token_env.to_string()),
                                ));
                            }
                        }
                        println!("Added {} Bitwarden provider(s)", selected.len());
                    } else {
                        println!("No Bitwarden projects selected");
                    }
                } else {
                    println!("Skipping Bitwarden");
                }
            }
            Ok(_) => println!("No Bitwarden projects found"),
            Err(e) => {
                println!("Could not list Bitwarden projects: {}", e);
                if std::env::var("BWS_ORGANIZATION_ID").is_err() {
                    println!(
                        "  Hint: Ensure BWS_ORGANIZATION_ID is set if your token requires it."
                    );
                }
            }
        }

        // Offer to store the Bitwarden token if any BW providers were added
        if config
            .providers
            .iter()
            .any(|p| matches!(p.kind.as_str(), "bw" | "bitwarden" | "bws"))
        {
            if let Ok(token) = std::env::var(bw_token_env) {
                if confirm("Store encrypted copy of Bitwarden access token?") {
                    let store_id = config
                        .providers
                        .iter()
                        .find(|p| matches!(p.kind.as_str(), "bw" | "bitwarden" | "bws"))
                        .map(|p| p.id.clone())
                        .unwrap_or_else(|| "bw".to_string());
                    pending_credentials.push(PendingCredential {
                        provider_id: store_id,
                        credential_key: "token".to_string(),
                        plaintext_value: token,
                    });
                }
            }
        }
    } else {
        println!("{} not set, skipping Bitwarden setup", bw_token_env);
        println!("  Tip: Set this environment variable and re-run to add Bitwarden providers");
    }

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

    // Store encrypted credentials if any were collected
    if !pending_credentials.is_empty() {
        println!();
        println!(
            "Encrypting {} credential(s) for secure storage...",
            pending_credentials.len()
        );

        // Prompt for encryption method once (reused for all credentials)
        let (method, method_tag, ssh_fingerprint) = prompt_encryption_method()?;

        // Initialize the database so we can store credentials
        let secrets_path = config.secrets_path();
        std::fs::create_dir_all(&secrets_path)?;
        let conn = init_db(&config.db_path())?;
        let repo = SecretRepository::new(conn);

        let use_keychain = config.keychain_cache();
        for cred in &pending_credentials {
            match store_encrypted_credential(
                &repo,
                &cred.provider_id,
                &cred.credential_key,
                &cred.plaintext_value,
                &method,
                &method_tag,
                ssh_fingerprint.as_deref(),
                use_keychain,
                &secrets_path,
            ) {
                Ok(()) => {
                    println!(
                        "  Stored encrypted {} for provider '{}'",
                        cred.credential_key, cred.provider_id
                    );
                }
                Err(e) => {
                    eprintln!(
                        "  Failed to store {} for '{}': {}",
                        cred.credential_key, cred.provider_id, e
                    );
                }
            }
        }
        println!("Credential storage complete.");
    }

    if config.providers.is_empty() {
        println!();
        println!(
            "Note: No providers were added. Edit {} to add providers manually.",
            config_path.display()
        );
    }

    Ok(())
}

/// Handle `jaws config clear-cache` -- remove all jaws entries from the OS keychain.
pub fn handle_clear_cache(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if !keychain::keychain_available() {
        eprintln!("OS keychain is not available on this system.");
        return Ok(());
    }

    // We need the database to enumerate known credential keys
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
