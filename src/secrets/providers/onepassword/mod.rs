mod ffi;

use crate::secrets::manager::SecretManager;
use ffi::{ItemCategory, OnePasswordSdkClient, SharedSdkClient, VaultOverview};
use async_trait::async_trait;
use futures::stream::{self, Stream, StreamExt};
use std::fmt;
use std::path::PathBuf;

/// A unit type for 1Password filters (not currently supported by the SDK)
#[derive(Debug, Clone)]
pub struct OnePasswordFilter;

/// Separator used between display path and API reference in combined format
const REF_SEPARATOR: &str = "|||";

/// A 1Password secret reference that combines a human-readable display path
/// with an API reference using UUIDs.
///
/// Format when serialized: "display_path|||api_ref"
/// - display_path: Human-readable path like "Vault Name/Item Title/Field Name"
/// - api_ref: API reference like "op://vault_id/item_id/field_id" (uses UUIDs)
#[derive(Debug, Clone)]
pub struct SecretRef {
    /// Human-readable path for display and filesystem storage
    pub display_path: String,
    /// API reference using UUIDs for SDK calls
    pub api_ref: String,
}

impl SecretRef {
    /// Create a new SecretRef
    pub fn new(display_path: impl Into<String>, api_ref: impl Into<String>) -> Self {
        Self {
            display_path: display_path.into(),
            api_ref: api_ref.into(),
        }
    }

    /// Parse a combined reference string into a SecretRef
    ///
    /// Handles both:
    /// - Combined format: "display_path|||api_ref"  
    /// - Legacy format: "op://vault/item/field" or plain path
    pub fn parse(combined: &str) -> Self {
        if let Some((display, api)) = combined.split_once(REF_SEPARATOR) {
            Self {
                display_path: display.to_string(),
                api_ref: api.to_string(),
            }
        } else {
            // Fallback for old-style references without separator
            let display = combined.strip_prefix("op://").unwrap_or(combined);
            Self {
                display_path: display.to_string(),
                api_ref: if combined.starts_with("op://") {
                    combined.to_string()
                } else {
                    format!("op://{}", combined)
                },
            }
        }
    }

    /// Serialize to the combined format string
    pub fn to_combined(&self) -> String {
        format!("{}{}{}", self.display_path, REF_SEPARATOR, self.api_ref)
    }
}

impl fmt::Display for SecretRef {
    /// Display shows only the human-readable path
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_path)
    }
}

pub struct OnePasswordSecretManager {
    sdk_client: SharedSdkClient,
    vault_id: String,
    vault_name: String,
}

impl OnePasswordSecretManager {
    /// Create a new 1Password secret manager
    /// If vault is None, the manager can still list vaults but cannot access secrets
    pub async fn new(
        vault: Option<String>,
        token_env: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create the SDK client
        let client = OnePasswordSdkClient::from_env(token_env)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        let sdk_client = SharedSdkClient::new(client);

        // If vault is specified, resolve it; otherwise create with empty vault (for vault discovery)
        let (vault_id, vault_name) = if let Some(vault_ref) = vault {
            let vault_info = sdk_client
                .find_vault(&vault_ref)
                .await
                .map_err(|e| e as Box<dyn std::error::Error>)?;
            (vault_info.id, vault_info.title)
        } else {
            // No vault specified - manager can only be used for vault discovery
            (String::new(), String::new())
        };

        Ok(Self {
            sdk_client,
            vault_id,
            vault_name,
        })
    }

    /// Create a new 1Password secret manager with a specific vault ID
    #[allow(dead_code)]
    pub async fn with_vault_id(
        vault_id: String,
        token_env: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create the SDK client
        let client = OnePasswordSdkClient::from_env(token_env)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        let sdk_client = SharedSdkClient::new(client);

        // Find the vault to get its name
        let vault_info = sdk_client
            .find_vault(&vault_id)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        Ok(Self {
            sdk_client,
            vault_id: vault_info.id,
            vault_name: vault_info.title,
        })
    }

    /// List all vaults accessible to the service account
    pub fn list_vaults(&self) -> Result<Vec<VaultOverview>, Box<dyn std::error::Error>> {
        self.sdk_client
            .list_vaults_sync()
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    /// Download a secret to a local file
    #[allow(dead_code)]
    async fn download_secret(
        &self,
        name: &str,
        dir: PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>> {
        use std::fs::{self, File};
        use std::io::Write;
        use std::path::Path;

        // Parse the combined reference
        let secret_ref = SecretRef::parse(name);

        // Resolve the secret using the API reference (with IDs)
        let secret_value = self
            .sdk_client
            .resolve_secret(&secret_ref.api_ref)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        // Use display path for the filename (preserves folder structure)
        let path = dir.join(Path::new(&secret_ref.display_path));
        let path_string = path.display().to_string();

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write the secret to file
        let mut file = File::create(path.as_path())?;
        file.write_all(secret_value.as_bytes())?;

        Ok(path_string)
    }
}

#[async_trait]
impl SecretManager for OnePasswordSecretManager {
    type Filter = OnePasswordFilter;

    async fn get_secret(&self, name: &str) -> Result<String, Box<dyn std::error::Error>> {
        let secret_ref = SecretRef::parse(name);
        self.sdk_client
            .resolve_secret(&secret_ref.api_ref)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)
    }

    async fn list_all(
        &self,
        _filters: Option<Vec<OnePasswordFilter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        list_items_for_stream(
            self.sdk_client.clone(),
            self.vault_id.clone(),
            self.vault_name.clone(),
        )
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)
    }

    fn list_secrets_stream(
        &self,
        _filters: Option<Vec<OnePasswordFilter>>,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let sdk_client = self.sdk_client.clone();
        let vault_id = self.vault_id.clone();
        let vault_name = self.vault_name.clone();

        Box::new(
            stream::once(Box::pin(async move {
                list_items_for_stream(sdk_client, vault_id, vault_name).await
            }))
            .flat_map(|result: Result<Vec<String>, Box<dyn std::error::Error + Send>>| match result {
                Ok(items) => {
                    let items_stream: Vec<Result<String, Box<dyn std::error::Error + Send>>> =
                        items.into_iter().map(Ok).collect();
                    stream::iter(items_stream)
                }
                Err(e) => stream::iter(vec![Err(e)]),
            }),
        )
    }

    async fn download(
        &self,
        name: &str,
        dir: PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.download_secret(name, dir).await
    }

    async fn create(
        &self,
        _name: &str,
        _value: &str,
        _description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Err(
            "1Password SDK does not support creating secrets. Please use the 1Password app or CLI."
                .into(),
        )
    }

    async fn update(
        &self,
        _name: &str,
        _value: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Err(
            "1Password SDK does not support updating secrets. Please use the 1Password app or CLI."
                .into(),
        )
    }

    async fn delete(&self, _name: &str, _force: bool) -> Result<(), Box<dyn std::error::Error>> {
        Err(
            "1Password SDK does not support deleting secrets. Please use the 1Password app or CLI."
                .into(),
        )
    }

    async fn rollback(
        &self,
        _name: &str,
        _version_id: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Err("1Password SDK does not support rollback operations. Please use the 1Password app or CLI.".into())
    }
}

/// Helper function for streaming - separated to avoid lifetime issues
/// Uses item IDs instead of titles in the op:// reference to handle items with slashes in names
async fn list_items_for_stream(
    sdk_client: SharedSdkClient,
    vault_id: String,
    vault_name: String,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send>> {
    let items = sdk_client
        .list_items(&vault_id)
        .await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send>)?;

    let mut secret_refs = Vec::new();

    for item in items {
        match sdk_client.get_item(&vault_id, &item.id).await {
            Ok(full_item) => {
                // Use item ID instead of title to avoid issues with slashes in item names
                // Format for display: "vault_name/item_title/field_title" (human readable)
                // Format for API: "op://vault_id/item_id/field_id" (uses IDs)

                for field in &full_item.fields {
                    if !field.value.is_empty() && !field.title.is_empty() {
                        let secret_ref = SecretRef::new(
                            format!("{}/{}/{}", vault_name, full_item.title, field.title),
                            format!("op://{}/{}/{}", vault_id, full_item.id, field.id),
                        );
                        secret_refs.push(secret_ref.to_combined());
                    }
                }

                if !full_item.notes.is_empty() {
                    let secret_ref = SecretRef::new(
                        format!("{}/{}/notesPlain", vault_name, full_item.title),
                        format!("op://{}/{}/notesPlain", vault_id, full_item.id),
                    );
                    secret_refs.push(secret_ref.to_combined());
                }

                if full_item.category == ItemCategory::Document
                    && let Some(doc) = &full_item.document
                {
                    let secret_ref = SecretRef::new(
                        format!("{}/{}/{}", vault_name, full_item.title, doc.name),
                        format!("op://{}/{}/{}", vault_id, full_item.id, doc.id),
                    );
                    secret_refs.push(secret_ref.to_combined());
                }
            }
            Err(_) => {
                // Silently skip items we can't access (permissions, etc.)
            }
        }
    }

    Ok(secret_refs)
}
