//! Preview command handler - internal use by TUI preview panel.

use chrono::Utc;
use ff::PreviewRule;

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::{Provider, load_secret_file};
use crate::utils::parse_secret_ref;

use super::pull::fetch_and_save_secret;

/// Build a PreviewRule that uses the currently-running jaws binary path.
/// This ensures preview works correctly when testing via `cargo run`
/// or when the binary is installed outside PATH.
pub fn preview_rule() -> PreviewRule {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "jaws".to_string());
    // Wrap exe path in quotes to handle spaces, then append __preview {}
    let cmd = format!("\"{}\" __preview {{}}", exe);
    PreviewRule::parse(&cmd).expect("preview rule is always valid")
}

/// Handle the hidden __preview command: print secret value to stdout.
/// Accepts either PROVIDER://NAME or "provider | name" format.
pub async fn handle_preview(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Try "provider | name" format first (what ff sends as item text)
    let (provider_id, secret_name) = if let Some((p, n)) = secret_name.split_once(" | ") {
        (p.trim().to_string(), n.trim().to_string())
    } else {
        parse_secret_ref(secret_name, config.default_provider().as_deref())?
    };

    let provider = providers
        .iter()
        .find(|p| p.id() == provider_id)
        .ok_or_else(|| format!("Unknown provider: '{}'", provider_id))?;

    let secret = repo
        .find_secret_by_provider_and_name(&provider_id, &secret_name)?
        .ok_or_else(|| {
            format!(
                "Secret '{}' not found in provider '{}'",
                secret_name, provider_id
            )
        })?;

    let content = if let Some(download) = repo.get_latest_download(secret.id)? {
        let age = Utc::now() - download.downloaded_at;
        if age.num_seconds() < config.cache_ttl() as i64 {
            load_secret_file(&config.secrets_path(), &download.filename)?
        } else {
            fetch_and_save_secret(config, repo, provider, &secret).await?
        }
    } else {
        fetch_and_save_secret(config, repo, provider, &secret).await?
    };

    print!("{}", content);

    Ok(())
}
