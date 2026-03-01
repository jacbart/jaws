//! HashiCorp Vault KV v2 secrets provider.

use std::collections::HashMap;

use async_trait::async_trait;
use futures::stream::{self, Stream};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;

use crate::error::JawsError;
use crate::secrets::manager::SecretManager;

/// A secrets provider backed by HashiCorp Vault's KV v2 secrets engine.
pub struct VaultSecretManager {
    client: VaultClient,
    /// Vault server address, kept for constructing new clients in streams.
    address: String,
    /// Vault token, kept for constructing new clients in streams.
    token: String,
    mount: String,
    id: String,
}

impl VaultSecretManager {
    /// Create a new VaultSecretManager.
    ///
    /// # Arguments
    /// * `id` - Unique provider identifier (e.g. "vault-prod")
    /// * `address` - Vault server URL (e.g. "https://vault.example.com:8200")
    /// * `token` - Vault authentication token
    /// * `mount` - KV v2 secrets engine mount path (defaults to "secret")
    pub fn new(
        id: String,
        address: &str,
        token: &str,
        mount: Option<String>,
    ) -> Result<Self, JawsError> {
        let settings = VaultClientSettingsBuilder::default()
            .address(address)
            .token(token)
            .build()
            .map_err(|e| {
                JawsError::vault(format!("Failed to build Vault client settings: {}", e))
            })?;

        let client = VaultClient::new(settings)
            .map_err(|e| JawsError::vault(format!("Failed to create Vault client: {}", e)))?;

        Ok(Self {
            client,
            address: address.to_string(),
            token: token.to_string(),
            mount: mount.unwrap_or_else(|| "secret".to_string()),
            id,
        })
    }
}

#[async_trait]
impl SecretManager for VaultSecretManager {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "vault"
    }

    fn supports_rollback(&self) -> bool {
        true // KV v2 supports versioning
    }

    async fn get_secret(&self, name: &str) -> Result<String, JawsError> {
        let secret: HashMap<String, String> = kv2::read(&self.client, &self.mount, name)
            .await
            .map_err(JawsError::vault)?;

        // If the secret has a single "value" key (written by jaws), return it directly.
        // Otherwise, serialize the entire map as JSON so the user sees all key-value pairs.
        if secret.len() == 1 {
            if let Some(value) = secret.get("value") {
                return Ok(value.clone());
            }
        }

        serde_json::to_string_pretty(&secret)
            .map_err(|e| JawsError::vault(format!("Failed to serialize secret: {}", e)))
    }

    async fn list_all(&self) -> Result<Vec<String>, JawsError> {
        // List secrets at the root path of the mount.
        // vaultrs::kv2::list returns keys at a given path; an empty string
        // means the root of the mount.
        match kv2::list(&self.client, &self.mount, "").await {
            Ok(keys) => Ok(keys),
            Err(e) => {
                let msg = e.to_string();
                // A 404 from Vault means no secrets exist yet -- return empty list
                if msg.contains("404") || msg.contains("Not Found") {
                    Ok(Vec::new())
                } else {
                    Err(JawsError::vault(e))
                }
            }
        }
    }

    fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        // We cannot easily clone VaultClient into an async stream closure
        // because it doesn't implement Clone. Instead we batch-fetch all keys
        // and yield them one by one, matching the GCP provider pattern.
        //
        // We store the address and token in the struct so we can construct a
        // fresh client inside the stream.

        let address = self.address.clone();
        let token = self.token.clone();
        let mount = self.mount.clone();

        let stream = stream::unfold(
            (Some((address, token, mount)), Vec::<String>::new()),
            |(state, mut buffer): (Option<(String, String, String)>, Vec<String>)| async move {
                // Drain buffer first
                if !buffer.is_empty() {
                    let name = buffer.remove(0);
                    return Some((Ok(name), (state, buffer)));
                }

                // If we already fetched everything, we're done
                let (address, token, mount) = state?;

                let settings = match VaultClientSettingsBuilder::default()
                    .address(&address)
                    .token(&token)
                    .build()
                {
                    Ok(s) => s,
                    Err(e) => {
                        return Some((
                            Err(Box::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            ))
                                as Box<dyn std::error::Error + Send>),
                            (None, Vec::new()),
                        ));
                    }
                };

                let client = match VaultClient::new(settings) {
                    Ok(c) => c,
                    Err(e) => {
                        return Some((
                            Err(Box::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            ))
                                as Box<dyn std::error::Error + Send>),
                            (None, Vec::new()),
                        ));
                    }
                };

                let names = match kv2::list(&client, &mount, "").await {
                    Ok(keys) => keys,
                    Err(e) => {
                        let msg = e.to_string();
                        // 404 means no secrets -- just return empty
                        if msg.contains("404") || msg.contains("Not Found") {
                            return None;
                        }
                        return Some((
                            Err(
                                Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg))
                                    as Box<dyn std::error::Error + Send>,
                            ),
                            (None, Vec::new()),
                        ));
                    }
                };

                if names.is_empty() {
                    return None;
                }

                let mut names = names;
                let first = names.remove(0);
                Some((Ok(first), (None, names)))
            },
        );

        Box::new(Box::pin(stream))
    }

    async fn create(
        &self,
        name: &str,
        value: &str,
        _description: Option<&str>,
    ) -> Result<String, JawsError> {
        let mut data = HashMap::new();
        data.insert("value".to_string(), value.to_string());

        kv2::set(&self.client, &self.mount, name, &data)
            .await
            .map_err(JawsError::vault)?;

        Ok(format!("{}/data/{}", self.mount, name))
    }

    async fn update(&self, name: &str, value: &str) -> Result<String, JawsError> {
        // KV v2 create and update use the same API endpoint --
        // writing to an existing path creates a new version.
        let mut data = HashMap::new();
        data.insert("value".to_string(), value.to_string());

        kv2::set(&self.client, &self.mount, name, &data)
            .await
            .map_err(JawsError::vault)?;

        Ok(format!("{}/data/{}", self.mount, name))
    }

    async fn delete(&self, name: &str, force: bool) -> Result<(), JawsError> {
        if force {
            // Permanently delete all versions and metadata
            kv2::delete_metadata(&self.client, &self.mount, name)
                .await
                .map_err(JawsError::vault)?;
        } else {
            // Soft-delete the latest version (can be undeleted)
            kv2::delete_latest(&self.client, &self.mount, name)
                .await
                .map_err(JawsError::vault)?;
        }

        Ok(())
    }

    async fn rollback(&self, name: &str, version_id: Option<&str>) -> Result<String, JawsError> {
        // Read the specified version, then write it as a new version
        let version: u64 = match version_id {
            Some(v) => v.parse().map_err(|_| {
                JawsError::validation(format!(
                    "Invalid version number '{}'. Vault KV v2 versions are positive integers.",
                    v
                ))
            })?,
            None => {
                return Err(JawsError::validation(
                    "Vault rollback requires a version number (e.g., --version-id 2)",
                ));
            }
        };

        // Read the old version
        let secret: HashMap<String, String> =
            kv2::read_version(&self.client, &self.mount, name, version)
                .await
                .map_err(JawsError::vault)?;

        // Extract the value -- if it was written by jaws it has a "value" key,
        // otherwise re-serialize the whole map.
        let value = if secret.len() == 1 {
            if let Some(v) = secret.get("value") {
                v.clone()
            } else {
                serde_json::to_string_pretty(&secret)
                    .map_err(|e| JawsError::vault(format!("Failed to serialize secret: {}", e)))?
            }
        } else {
            serde_json::to_string_pretty(&secret)
                .map_err(|e| JawsError::vault(format!("Failed to serialize secret: {}", e)))?
        };

        // Write it back as a new version
        self.update(name, &value).await
    }
}
