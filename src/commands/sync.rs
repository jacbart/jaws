//! Sync command handlers - synchronizing secret listings from providers.

use chrono::Duration;
use chrono::Utc;
use futures::StreamExt;

use crate::config::Config;
use crate::db::{SecretInput, SecretRepository};
use crate::secrets::{hash_api_ref, Provider, SecretRef};

/// Handle the sync command
pub async fn handle_sync(
    _config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Syncing remote secret listings...");

    for provider in providers {
        match sync_provider(repo, provider).await {
            Ok(count) => {
                println!(
                    "  {} [{}]: {} secrets",
                    provider.id(),
                    provider.kind(),
                    count
                );
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
pub fn should_refresh_cache(
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
pub async fn sync_provider(
    repo: &SecretRepository,
    provider: &Provider,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut stream = provider.list_secrets_stream();
    let mut count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(secret_ref) => {
                // For 1Password, parse the combined format "display_path|||api_ref"
                // For AWS, the secret name is both the API ref and display name
                let (api_ref, display_name) = if provider.kind() == "onepassword" {
                    let parsed = SecretRef::parse(&secret_ref);
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
                    description: None,
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
