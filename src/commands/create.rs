//! Create command handlers - creating new local secrets.

use std::fs;

use crate::config::Config;
use crate::secrets::Provider;
use crate::utils::edit_secret_value;

/// Handle the create command - create a new local secret
pub async fn handle_create(
    config: &Config,
    providers: &[Provider],
    name: String,
    description: Option<String>,
    file: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find the jaws provider (always available)
    let jaws_provider = providers
        .iter()
        .find(|p| p.kind() == "jaws")
        .expect("jaws provider is always available");

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

    let api_ref = jaws_provider
        .create(&name, &value, description.as_deref())
        .await?;
    println!("Created secret '{}' ({})", name, api_ref);

    Ok(())
}
