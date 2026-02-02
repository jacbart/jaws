//! Push command handlers - uploading secrets to providers.

use std::fs;
use std::process::Command;

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::{Provider, get_secret_path};

use crate::utils::parse_secret_ref;

/// Handle the push command
pub async fn handle_push(
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
        if let Ok((provider, specific_name)) = parse_secret_ref(name, None) {
            downloaded
                .into_iter()
                .filter(|(s, _)| s.provider_id == provider && s.display_name == specific_name)
                .collect()
        } else {
            downloaded
                .into_iter()
                .filter(|(s, _)| s.display_name.contains(name) || s.hash.starts_with(name))
                .collect()
        }
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
    let mut error_count = 0;

    for (secret, download) in secrets_to_push {
        let file_path = get_secret_path(&config.secrets_path(), &download.filename);

        if !file_path.exists() {
            eprintln!("Error: File not found: {}", file_path.display());
            error_count += 1;
            continue;
        }

        let content = fs::read_to_string(&file_path)?;

        // For jaws (local) secrets, show a message about future remote push capability
        if secret.provider_id == "jaws" {
            println!(
                "{} [jaws] -> Updated locally. \
                 (Future: Configure push targets to sync to remote providers)",
                secret.display_name
            );
            // Update the local secret
            let jaws_provider = providers
                .iter()
                .find(|p| p.kind() == "jaws")
                .expect("jaws provider always exists");
            if let Err(e) = jaws_provider.update(&secret.api_ref, &content).await {
                eprintln!("Error updating local secret: {}", e);
                error_count += 1;
            } else {
                pushed_count += 1;
            }
            continue;
        }

        // Find the provider
        let provider = providers
            .iter()
            .find(|p| p.id() == secret.provider_id)
            .ok_or_else(|| format!("Provider {} not found", secret.provider_id))?;

        match provider.update(&secret.api_ref, &content).await {
            Ok(result) => {
                println!(
                    "{} [{}] -> {}",
                    secret.display_name, secret.provider_id, result
                );
                pushed_count += 1;
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("ResourceNotFoundException")
                    || error_msg.contains("not found")
                {
                    match provider.create(&secret.display_name, &content, None).await {
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
                            error_count += 1;
                        }
                    }
                } else {
                    eprintln!(
                        "Error updating {} in {}: {}",
                        secret.display_name, secret.provider_id, e
                    );
                    error_count += 1;
                }
            }
        }
    }

    if pushed_count > 0 || error_count > 0 {
        println!(
            "\nPush complete: {} succeeded, {} failed",
            pushed_count, error_count
        );
    }

    Ok(())
}
