//! Create command handlers - creating new secrets.

use std::fs;
use std::io::{self, Write};

use crate::config::Config;
use crate::db::{SecretInput, SecretRepository};
use crate::secrets::Provider;
use crate::secrets::storage::hash_api_ref;
use crate::utils::edit_secret_value;

use crate::utils::parse_secret_ref;

/// Handle the create command - create a new secret
pub async fn handle_create(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    name_arg: Option<String>,
    description: Option<String>,
    file: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get or prompt for secret name
    let secret_name = if let Some(name) = name_arg {
        name
    } else {
        prompt_secret_name()?
    };

    // Determine target provider
    // 1. If name contains provider prefix (e.g., "aws://secret"), use that
    // 2. If default_provider is set in config, use that
    // 3. Otherwise, prompt user to select a provider
    let (provider_id, final_name) = if secret_name.contains("://") {
        // Name contains provider prefix
        parse_secret_ref(&secret_name, None)?
    } else if let Some(default) = config.default_provider() {
        // Use default provider from config
        (default, secret_name.clone())
    } else {
        // Prompt user to select a provider
        let provider_id = select_provider(providers).await?;
        (provider_id, secret_name.clone())
    };

    // Find the target provider
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

    // Get the value
    let value = if let Some(path) = file {
        fs::read_to_string(&path)?
    } else {
        // Open editor for input
        edit_secret_value(config, None)?
    };

    if value.trim().is_empty() {
        return Err("Secret value cannot be empty".into());
    }

    let result = provider
        .create(&final_name, &value, description.as_deref())
        .await?;

    // Register the secret in the local database so subsequent commands
    // (pull, push, etc.) can find it without requiring a sync first.
    // Use the api_ref returned by the provider's create() method (e.g. "jaws://<uuid>"
    // for local secrets, an ARN for AWS, etc.) so we don't create a duplicate row
    // when the provider already inserted one with a different api_ref.
    let input = SecretInput {
        provider_id: provider_id.clone(),
        api_ref: result.clone(),
        display_name: final_name.clone(),
        hash: hash_api_ref(&result),
        description: description.clone(),
        remote_updated_at: None,
    };
    repo.upsert_secret(&input)?;
    repo.log_operation("create", &provider_id, &final_name, None)?;

    println!("Created {}://{} ({})", provider_id, final_name, result);

    Ok(())
}

/// Prompt user to enter a secret name
fn prompt_secret_name() -> Result<String, Box<dyn std::error::Error>> {
    print!("Secret name: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let name = input.trim().to_string();

    if name.is_empty() {
        return Err("Secret name cannot be empty".into());
    }

    Ok(name)
}

/// Show TUI selector for provider selection
async fn select_provider(providers: &[Provider]) -> Result<String, Box<dyn std::error::Error>> {
    use ff::{TuiConfig, create_items_channel, run_tui_with_config};

    if providers.is_empty() {
        return Err("No providers configured. Run 'jaws config generate --interactive' to set up providers.".into());
    }

    // If only one provider (jaws), use it
    if providers.len() == 1 {
        return Ok(providers[0].id().to_string());
    }

    println!("Select a provider for the new secret:");

    let (tx, rx) = create_items_channel();

    for provider in providers {
        let display = format!("{} [{}]", provider.id(), provider.kind());
        if tx.send(display).await.is_err() {
            break;
        }
    }
    drop(tx);

    let mut tui_config = TuiConfig::with_height((providers.len() as u16 + 2).min(15));
    tui_config.show_help_text = false;

    let selected = run_tui_with_config(rx, false, tui_config)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    if selected.is_empty() {
        return Err("Cancelled".into());
    }

    // Parse provider ID from selection "PROVIDER_ID [kind]"
    let (_, selected_str) = &selected[0];
    let provider_id = selected_str
        .split(" [")
        .next()
        .ok_or("Failed to parse provider selection")?
        .to_string();

    Ok(provider_id)
}
