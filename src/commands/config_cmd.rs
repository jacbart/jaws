//! Config command handlers - managing configuration.

use std::io::{self, Write};

use crate::config::{Config, Defaults, ProviderConfig};
use crate::secrets::{BitwardenSecretManager, OnePasswordSecretManager};

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
        let selected_str = &selected[0];
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
        }),
        providers: Vec::new(),
    };

    println!();

    // Discover AWS profiles
    println!("Discovering AWS profiles...");
    match Config::discover_aws_profiles() {
        Ok(profiles) if !profiles.is_empty() => {
            println!(
                "Found {} AWS profile(s). Select which to add (or none to skip):",
                profiles.len()
            );
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

        match OnePasswordSecretManager::new(None, op_token_env).await {
            Ok(manager) => {
                let vaults = match manager.list_vaults() {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Could not list 1Password vaults: {}", e);
                        return Ok(()); // Continue to next provider check instead of aborting
                    }
                };

                if !vaults.is_empty() {
                    println!(
                        "Found {} vault(s). Select which to add (or none to skip):",
                        vaults.len()
                    );
                    println!("  Tip: Use 'all' option to auto-discover vaults at runtime\n");

                    let mut items: Vec<String> =
                        vec!["[all] - Auto-discover all vaults at runtime".to_string()];
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
                                    let provider_id =
                                        format!("op-{}", name.to_lowercase().replace(' ', "-"));
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
                    println!("No 1Password vaults accessible");
                }
            }
            Err(e) => println!("Could not initialize 1Password: {}", e),
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
                println!(
                    "Found {} project(s). Select which to add (or none to skip):",
                    projects.len()
                );

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
                    for selection in &selected {
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

    if config.providers.is_empty() {
        println!();
        println!(
            "Note: No providers were added. Edit {} to add providers manually.",
            config_path.display()
        );
    }

    Ok(())
}
