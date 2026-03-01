//! TUI-based secret selection across all providers.

use futures::StreamExt;

use crate::error::JawsError;

use super::Provider;
use super::onepassword::SecretRef;

/// Select secrets from all providers using a unified TUI.
///
/// Returns a list of (provider_id, secret_reference) pairs.
pub async fn select_from_all_providers(
    providers: &[Provider],
) -> Result<Vec<(String, String)>, JawsError> {
    use ff::{TuiConfig, create_items_channel, run_tui_with_config};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let (tx, rx) = create_items_channel();

    // Map from display string to full reference (for 1Password combined refs)
    let display_to_full: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    // Spawn tasks to fetch secrets from each provider
    for provider in providers {
        let tx_clone = tx.clone();
        let provider_id = provider.id().to_string();
        let provider_kind = provider.kind().to_string();
        let mut stream = provider.list_secrets_stream();
        let map_clone = Arc::clone(&display_to_full);

        tokio::spawn(async move {
            while let Some(result) = stream.next().await {
                if let Ok(secret) = result {
                    let display_secret = if provider_kind == "onepassword" {
                        let secret_ref = SecretRef::parse(&secret);
                        let mut map = map_clone.lock().await;
                        let display_key = format!("{} | {}", provider_id, secret_ref.display_path);
                        map.insert(display_key, secret.clone());
                        secret_ref.display_path
                    } else {
                        secret.clone()
                    };

                    let item = format!("{} | {}", provider_id, display_secret);
                    if tx_clone.send(item).await.is_err() {
                        break;
                    }
                }
            }
        });
    }

    drop(tx);

    let mut tui_config = TuiConfig::fullscreen();
    tui_config.show_help_text = false;

    let selected_items = run_tui_with_config(rx, true, tui_config)
        .await
        .map_err(|e| JawsError::Other(e.to_string()))?;

    let map = display_to_full.lock().await;
    let mut result = Vec::new();
    for (_, item) in selected_items {
        if let Some((prov_id, _secret_display)) = item.split_once(" | ") {
            let full_ref = map.get(&item).cloned().unwrap_or_else(|| {
                item.split_once(" | ")
                    .map(|(_, s)| s.to_string())
                    .unwrap_or(item.clone())
            });
            result.push((prov_id.to_string(), full_ref));
        }
    }

    Ok(result)
}
