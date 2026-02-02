//! Create command handlers - creating new local secrets.

use std::fs;

use crate::config::Config;
use crate::secrets::Provider;
use crate::utils::edit_secret_value;

use crate::utils::parse_secret_ref;

/// Handle the create command - create a new secret
pub async fn handle_create(
    config: &Config,
    providers: &[Provider],
    name_arg: String,
    description: Option<String>,
    file: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the secret reference
    // If no provider specified in name or config, default to "jaws" (local)
    let default_provider_string = config.default_provider();
    let default_provider = default_provider_string.as_deref().or(Some("jaws"));
    let (provider_id, secret_name) = parse_secret_ref(&name_arg, default_provider)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

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
        .create(&secret_name, &value, description.as_deref())
        .await?;

    println!("Created {}://{} ({})", provider_id, secret_name, result);

    Ok(())
}
