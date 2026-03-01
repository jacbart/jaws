mod ffi;

use crate::error::JawsError;
use crate::secrets::manager::SecretManager;
use async_trait::async_trait;
use ffi::{
    Item, ItemCategory, ItemField, ItemFieldType, OnePasswordSdkClient, SharedSdkClient,
    VaultOverview,
};
use futures::stream::{self, Stream, StreamExt};
use std::fmt;

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_path)
    }
}

pub struct OnePasswordSecretManager {
    sdk_client: SharedSdkClient,
    vault_id: String,
    vault_name: String,
    id: String,
}

impl OnePasswordSecretManager {
    /// Create a new 1Password secret manager
    /// If vault is None, the manager can still list vaults but cannot access secrets
    pub async fn new(
        provider_id: String,
        vault: Option<String>,
        token_env: &str,
    ) -> Result<Self, JawsError> {
        let client = OnePasswordSdkClient::from_env(token_env)
            .await
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;

        let sdk_client = SharedSdkClient::new(client);

        let (vault_id, vault_name) = if let Some(vault_ref) = vault {
            let vault_info = sdk_client
                .find_vault(&vault_ref)
                .await
                .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;
            (vault_info.id, vault_info.title)
        } else {
            (String::new(), String::new())
        };

        Ok(Self {
            sdk_client,
            vault_id,
            vault_name,
            id: provider_id,
        })
    }

    /// Create a new 1Password secret manager with a specific vault ID
    pub async fn with_vault_id(
        provider_id: String,
        vault_id: String,
        token_env: &str,
    ) -> Result<Self, JawsError> {
        let client = OnePasswordSdkClient::from_env(token_env)
            .await
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;

        let sdk_client = SharedSdkClient::new(client);

        let vault_info = sdk_client
            .find_vault(&vault_id)
            .await
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;

        Ok(Self {
            sdk_client,
            vault_id: vault_info.id,
            vault_name: vault_info.title,
            id: provider_id,
        })
    }

    /// List all vaults accessible to the service account
    pub fn list_vaults(&self) -> Result<Vec<VaultOverview>, JawsError> {
        self.sdk_client
            .list_vaults_sync()
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))
    }

    /// Resolve vault ID and item details from a path string
    /// Returns (vault_id, item_title, optional_field_name)
    async fn resolve_context(
        &self,
        path: &str,
    ) -> Result<(String, String, Option<String>), JawsError> {
        let parts: Vec<&str> = path.split('/').collect();
        match parts.len() {
            1 => {
                if self.vault_id.is_empty() {
                    return Err(JawsError::validation(
                        "No default vault configured. Please specify path as 'Vault/Item'",
                    ));
                }
                Ok((self.vault_id.clone(), parts[0].to_string(), None))
            }
            2 | 3 => {
                let vault_name = parts[0];
                let item_title = parts[1];
                let field_name = if parts.len() == 3 {
                    Some(parts[2].to_string())
                } else {
                    None
                };

                if !self.vault_name.is_empty() && vault_name == self.vault_name {
                    return Ok((self.vault_id.clone(), item_title.to_string(), field_name));
                }

                let vault = self
                    .sdk_client
                    .find_vault(vault_name)
                    .await
                    .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;
                Ok((vault.id, item_title.to_string(), field_name))
            }
            _ => Err(JawsError::validation(
                "Invalid path format. Expected 'Item', 'Vault/Item', or 'Vault/Item/Field'",
            )),
        }
    }

    /// Find an item ID by title in a specific vault
    async fn find_item_id(&self, vault_id: &str, title: &str) -> Result<String, JawsError> {
        let items = self
            .sdk_client
            .list_items(vault_id)
            .await
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;

        for item in items {
            if item.title == title {
                return Ok(item.id);
            }
        }
        Err(JawsError::not_found(format!(
            "Item '{}' not found in vault",
            title
        )))
    }
}

#[async_trait]
impl SecretManager for OnePasswordSecretManager {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "onepassword"
    }

    async fn get_secret(&self, name: &str) -> Result<String, JawsError> {
        let secret_ref = SecretRef::parse(name);
        self.sdk_client
            .resolve_secret(&secret_ref.api_ref)
            .await
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))
    }

    async fn list_all(&self) -> Result<Vec<String>, JawsError> {
        list_items_for_stream(
            self.sdk_client.clone(),
            self.vault_id.clone(),
            self.vault_name.clone(),
        )
        .await
        .map_err(|e| JawsError::Other(e.to_string()))
    }

    fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let sdk_client = self.sdk_client.clone();
        let vault_id = self.vault_id.clone();
        let vault_name = self.vault_name.clone();

        Box::new(
            stream::once(Box::pin(async move {
                list_items_for_stream(sdk_client, vault_id, vault_name).await
            }))
            .flat_map(
                |result: Result<Vec<String>, Box<dyn std::error::Error + Send>>| match result {
                    Ok(items) => {
                        let items_stream: Vec<Result<String, Box<dyn std::error::Error + Send>>> =
                            items.into_iter().map(Ok).collect();
                        stream::iter(items_stream)
                    }
                    Err(e) => stream::iter(vec![Err(e)]),
                },
            ),
        )
    }

    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, JawsError> {
        let (vault_id, item_title, _) = self.resolve_context(name).await?;

        let item = Item {
            id: String::new(),
            title: item_title.clone(),
            category: ItemCategory::Login,
            vault_id: vault_id.clone(),
            fields: vec![
                ItemField {
                    id: "password".to_string(),
                    title: "password".to_string(),
                    section_id: None,
                    field_type: ItemFieldType::Concealed,
                    value: value.to_string(),
                },
                ItemField {
                    id: "username".to_string(),
                    title: "username".to_string(),
                    section_id: None,
                    field_type: ItemFieldType::Text,
                    value: "jaws-generated".to_string(),
                },
            ],
            sections: vec![],
            notes: description.unwrap_or("").to_string(),
            tags: vec!["jaws".to_string()],
            websites: vec![],
            version: 0,
            files: vec![],
            document: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let created_item = self
            .sdk_client
            .create_item(&item)
            .await
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;

        Ok(format!(
            "Created item '{}' ({}) in vault {}",
            created_item.title, created_item.id, vault_id
        ))
    }

    async fn update(&self, name: &str, value: &str) -> Result<String, JawsError> {
        let (vault_id, item_id, field_name) = if name.starts_with("op://") {
            let path = name.strip_prefix("op://").unwrap();
            let parts: Vec<&str> = path.split('/').collect();
            match parts.len() {
                2 => (parts[0].to_string(), parts[1].to_string(), None),
                3 => (
                    parts[0].to_string(),
                    parts[1].to_string(),
                    Some(parts[2].to_string()),
                ),
                _ => {
                    return Err(JawsError::validation(format!(
                        "Invalid op:// reference format. Expected 'op://vault/item' or 'op://vault/item/field', got: {}",
                        name
                    )));
                }
            }
        } else {
            let (vault_id, item_title, field_name) = self.resolve_context(name).await?;
            let item_id = self.find_item_id(&vault_id, &item_title).await?;
            (vault_id, item_id, field_name)
        };

        let mut item = self
            .sdk_client
            .get_item(&vault_id, &item_id)
            .await
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;

        let target_field_id = field_name.unwrap_or_else(|| "password".to_string());
        let mut updated = false;

        for field in &mut item.fields {
            if field.id == target_field_id || field.title == target_field_id {
                field.value = value.to_string();
                updated = true;
                break;
            }
        }

        if !updated && target_field_id == "password" {
            for field in &mut item.fields {
                if let ItemFieldType::Concealed = field.field_type {
                    field.value = value.to_string();
                    updated = true;
                    break;
                }
            }
        }

        if !updated {
            item.fields.push(ItemField {
                id: target_field_id.clone(),
                title: target_field_id.clone(),
                section_id: None,
                field_type: ItemFieldType::Concealed,
                value: value.to_string(),
            });
        }

        let updated_item = self
            .sdk_client
            .put_item(&item)
            .await
            .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;

        Ok(format!(
            "Updated item '{}' ({})",
            updated_item.title, updated_item.id
        ))
    }

    async fn delete(&self, name: &str, force: bool) -> Result<(), JawsError> {
        let (vault_id, item_title, _) = self.resolve_context(name).await?;
        let item_id = self.find_item_id(&vault_id, &item_title).await?;

        if force {
            self.sdk_client
                .delete_item(&vault_id, &item_id)
                .await
                .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;
        } else {
            self.sdk_client
                .archive_item(&vault_id, &item_id)
                .await
                .map_err(|e| JawsError::provider("onepassword", e.to_string()))?;
        }

        Ok(())
    }

    async fn rollback(&self, _name: &str, _version_id: Option<&str>) -> Result<String, JawsError> {
        Err(JawsError::unsupported(
            "1Password SDK does not support rollback operations. Please use the 1Password app or CLI.",
        ))
    }
}

/// Helper function for streaming - separated to avoid lifetime issues
async fn list_items_for_stream(
    sdk_client: SharedSdkClient,
    vault_id: String,
    vault_name: String,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send>> {
    let items = sdk_client.list_items(&vault_id).await.map_err(|e| {
        Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send>
    })?;

    let mut secret_refs = Vec::new();

    for item in items {
        match sdk_client.get_item(&vault_id, &item.id).await {
            Ok(full_item) => {
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
            Err(_) => {}
        }
    }

    Ok(secret_refs)
}
