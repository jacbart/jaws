//! Provider auto-discovery functions for interactive config setup.
//!
//! These are reusable by both `jaws config init` and `jaws config provider add`.

use std::process::Command;

use ff::{FuzzyFinderSession, TuiConfig};

use crate::config::{Config, ProviderConfig};
use crate::secrets::{BitwardenSecretManager, OnePasswordSecretManager};

use super::helpers::{PendingCredential, confirm, prompt};

/// Discover AWS profiles and interactively add them to the config.
/// Returns the number of providers added.
pub(super) async fn discover_and_add_aws(
    config: &mut Config,
    pending_credentials: &mut Vec<PendingCredential>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let initial_count = config.providers.len();

    println!("Discovering AWS profiles...");
    match Config::discover_aws_profiles() {
        Ok(profiles) if !profiles.is_empty() => {
            println!("Found {} AWS profile(s).", profiles.len());

            if confirm("Add AWS provider(s)?") {
                println!("  Tip: Use 'all' option to auto-discover profiles at runtime\n");

                let mut items: Vec<String> =
                    vec!["[all] - Auto-discover all profiles at runtime".to_string()];
                for profile in &profiles {
                    let region = Config::get_aws_profile_region(profile)
                        .map(|r| format!(" ({})", r))
                        .unwrap_or_default();
                    items.push(format!("{}{}", profile, region));
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
                    if selected.iter().any(|(_, s)| s.starts_with("[all]")) {
                        config.add_provider(ProviderConfig::new_aws(
                            "aws".to_string(),
                            Some("all".to_string()),
                            None,
                        ));
                        println!("Added AWS provider with auto-discovery");
                    } else {
                        for (_, selection) in &selected {
                            let profile_name =
                                selection.split(" (").next().unwrap_or(selection).trim();
                            let region = Config::get_aws_profile_region(profile_name);
                            config.add_provider(ProviderConfig::new_aws(
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
                    config.add_provider(ProviderConfig::new_aws("aws".to_string(), None, region));
                    println!("Added AWS provider (env var credentials)");

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

    Ok(config.providers.len() - initial_count)
}

/// Discover 1Password vaults and interactively add them to the config.
/// Returns the number of providers added.
pub(super) async fn discover_and_add_onepassword(
    config: &mut Config,
    pending_credentials: &mut Vec<PendingCredential>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let initial_count = config.providers.len();
    let op_token_env = "OP_SERVICE_ACCOUNT_TOKEN";

    println!("Checking for 1Password...");
    if std::env::var(op_token_env).is_ok() {
        println!("Found {}. Discovering vaults...", op_token_env);

        match OnePasswordSecretManager::new("op-discovery".to_string(), None, op_token_env).await {
            Ok(manager) => match manager.list_vaults() {
                Ok(vaults) if !vaults.is_empty() => {
                    println!("Found {} vault(s).", vaults.len());

                    if confirm("Add 1Password provider(s)?") {
                        println!("  Tip: Use 'all' option to auto-discover vaults at runtime\n");

                        let mut items: Vec<String> =
                            vec!["[all] - Auto-discover all vaults at runtime".to_string()];
                        for vault in &vaults {
                            items.push(format!("{} ({})", vault.title, vault.id));
                        }

                        let mut tui_config = TuiConfig::with_height(15.min(items.len() as u16 + 3));
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
                                config.add_provider(ProviderConfig::new_onepassword(
                                    "op".to_string(),
                                    Some("all".to_string()),
                                    None,
                                ));
                                println!("Added 1Password provider with auto-discovery");
                            } else {
                                for (_, selection) in &selected {
                                    if let Some((name, rest)) = selection.split_once(" (") {
                                        let vault_id = rest.trim_end_matches(')');
                                        let provider_id =
                                            format!("op-{}", name.to_lowercase().replace(' ', "-"));
                                        config.add_provider(ProviderConfig::new_onepassword(
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
            },
            Err(e) => println!("Could not initialize 1Password: {}", e),
        }

        // Offer to store the token if any OP providers were added
        if config
            .providers
            .iter()
            .any(|p| matches!(p.kind.as_str(), "onepassword" | "1password" | "op"))
        {
            if let Ok(token) = std::env::var(op_token_env) {
                if confirm("Store encrypted copy of 1Password service account token?") {
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

    Ok(config.providers.len() - initial_count)
}

/// Discover Bitwarden projects and interactively add them to the config.
/// Returns the number of providers added.
pub(super) async fn discover_and_add_bitwarden(
    config: &mut Config,
    pending_credentials: &mut Vec<PendingCredential>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let initial_count = config.providers.len();
    let bw_token_env = "BWS_ACCESS_TOKEN";

    println!("Checking for Bitwarden...");
    if std::env::var(bw_token_env).is_ok() {
        println!("Found {}. Discovering projects...", bw_token_env);

        let mut manager =
            BitwardenSecretManager::new("bw-discovery".to_string(), None, bw_token_env, None)
                .await?;
        let mut projects_result = manager.list_projects().await;
        let mut organization_id: Option<String> = None;

        if projects_result.is_err() {
            println!("Could not list Bitwarden projects.");
            println!("This usually means your access token requires an explicit Organization ID.");

            let org_input = prompt("Enter Organization ID (optional, press Enter to skip)", "");
            if !org_input.is_empty() {
                println!("Retrying with Organization ID: {}", org_input);
                organization_id = Some(org_input.clone());

                manager = BitwardenSecretManager::new(
                    "bw-discovery".to_string(),
                    None,
                    bw_token_env,
                    Some(org_input),
                )
                .await?;
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
                            if let Some((name, rest)) = selection.split_once(" (") {
                                let project_id = rest.trim_end_matches(')');
                                let provider_id =
                                    format!("bw-{}", name.to_lowercase().replace(' ', "-"));
                                config.add_provider(ProviderConfig::new_bitwarden(
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

        // Offer to store the token if any BW providers were added
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

    Ok(config.providers.len() - initial_count)
}

/// Discover GCP project and interactively add a GCP Secret Manager provider.
/// Returns the number of providers added.
///
/// Discovery attempts:
/// 1. Check GOOGLE_CLOUD_PROJECT / GCLOUD_PROJECT / CLOUDSDK_CORE_PROJECT env vars
/// 2. Run `gcloud config get-value project` to detect the active project
/// 3. Fall back to manual entry
pub(super) async fn discover_and_add_gcp(
    config: &mut Config,
    _pending_credentials: &mut Vec<PendingCredential>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let initial_count = config.providers.len();

    println!("Checking for GCP...");

    // Try to discover the default project from env vars or gcloud CLI
    let project_id = discover_gcp_project();

    match project_id {
        Some(project) => {
            println!("Found GCP project: {}", project);
            if confirm("Add GCP Secret Manager provider?") {
                let id_input = prompt("Provider ID", &format!("gcp-{}", project));
                let provider_id = if id_input.is_empty() {
                    format!("gcp-{}", project)
                } else {
                    id_input
                };

                config.add_provider(ProviderConfig::new_gcp(
                    provider_id,
                    Some(project.clone()),
                ));
                println!("Added GCP Secret Manager provider for project '{}'", project);

                // Check if user has additional projects to add
                while confirm("Add another GCP project?") {
                    let extra_project = prompt("GCP project ID", "");
                    if extra_project.is_empty() {
                        break;
                    }
                    let extra_id = prompt(
                        "Provider ID",
                        &format!("gcp-{}", extra_project),
                    );
                    let provider_id = if extra_id.is_empty() {
                        format!("gcp-{}", extra_project)
                    } else {
                        extra_id
                    };
                    config.add_provider(ProviderConfig::new_gcp(
                        provider_id,
                        Some(extra_project.clone()),
                    ));
                    println!("Added GCP provider for project '{}'", extra_project);
                }
            } else {
                println!("Skipping GCP");
            }
        }
        None => {
            println!("No GCP project detected.");
            println!("  Tip: Run 'gcloud auth application-default login' and 'gcloud config set project <PROJECT_ID>'");
            println!("  Or set the GOOGLE_CLOUD_PROJECT environment variable.");

            if confirm("Enter a GCP project ID manually?") {
                let manual_project = prompt("GCP project ID", "");
                if !manual_project.is_empty() {
                    let id_input = prompt(
                        "Provider ID",
                        &format!("gcp-{}", manual_project),
                    );
                    let provider_id = if id_input.is_empty() {
                        format!("gcp-{}", manual_project)
                    } else {
                        id_input
                    };
                    config.add_provider(ProviderConfig::new_gcp(
                        provider_id,
                        Some(manual_project.clone()),
                    ));
                    println!("Added GCP provider for project '{}'", manual_project);
                }
            }
        }
    }

    Ok(config.providers.len() - initial_count)
}

/// Try to discover the GCP project ID from environment variables or gcloud CLI.
fn discover_gcp_project() -> Option<String> {
    // 1. Check common environment variables
    for env_var in &[
        "GOOGLE_CLOUD_PROJECT",
        "GCLOUD_PROJECT",
        "CLOUDSDK_CORE_PROJECT",
    ] {
        if let Ok(project) = std::env::var(env_var) {
            if !project.is_empty() {
                return Some(project);
            }
        }
    }

    // 2. Try gcloud CLI
    match Command::new("gcloud")
        .args(["config", "get-value", "project"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let project = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !project.is_empty() && project != "(unset)" {
                return Some(project);
            }
        }
        _ => {}
    }

    None
}
