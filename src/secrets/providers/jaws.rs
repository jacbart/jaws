//! Jaws local secret manager - treats locally stored secrets as first-class citizens.
//!
//! The jaws provider is always available and doesn't require external configuration.
//! It stores secrets locally with full version history under:
//!
//! - `{secrets_path}/secrets/jaws/{name}`              — user-editable working copy
//! - `{secrets_path}/.versions/jaws/{name}/v{N}`       — per-version archive

use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{self, Stream};
use std::path::PathBuf;
use uuid::Uuid;

use crate::db::{SecretInput, SecretRepository, init_db};
use crate::error::JawsError;
use crate::secrets::manager::SecretManager;
use crate::secrets::storage::{
    compute_content_hash, delete_all_archives, delete_working_file, hash_api_ref,
    load_secret_file, read_working_file, write_secret_version,
};

/// Local secret manager that stores secrets in the jaws secrets directory.
/// This provider is always available without any external configuration.
pub struct JawsSecretManager {
    secrets_path: PathBuf,
    id: String,
}

impl JawsSecretManager {
    pub fn new(secrets_path: PathBuf, id: String) -> Self {
        Self { secrets_path, id }
    }

    /// Generate a unique API reference for local secrets.
    pub fn generate_api_ref() -> String {
        format!("jaws://{}", Uuid::new_v4())
    }

    pub fn secrets_path(&self) -> &PathBuf {
        &self.secrets_path
    }

    fn get_repo(&self) -> Result<SecretRepository, JawsError> {
        let db_path = self.secrets_path.join("jaws.db");
        let conn = init_db(&db_path)?;
        Ok(SecretRepository::new(conn))
    }

    /// Create a secret directly (used by the create command).
    pub async fn create_secret(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, JawsError> {
        <Self as SecretManager>::create(self, name, value, description).await
    }
}

#[async_trait]
impl SecretManager for JawsSecretManager {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "jaws"
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    fn supports_remote_history(&self) -> bool {
        true
    }

    async fn get_secret(&self, api_ref: &str) -> Result<String, JawsError> {
        let repo = self.get_repo()?;
        // Look up by api_ref first, then fall back to display_name. Remote
        // clients (via gRPC) send display names through `get_secret`.
        let secret = repo
            .get_secret_by_api_ref(&self.id, api_ref)?
            .or(repo.find_secret_by_provider_and_name(&self.id, api_ref)?)
            .ok_or_else(|| JawsError::not_found(format!("Secret not found: {}", api_ref)))?;
        // Prefer the user-editable working file; fall back to the latest archive
        // (e.g. if the user removed the working file but kept history).
        match read_working_file(&self.secrets_path, &secret.provider_id, &secret.display_name) {
            Ok(c) => Ok(c),
            Err(_) => {
                let download = repo
                    .get_latest_download(secret.id)?
                    .ok_or_else(|| JawsError::not_found("No downloaded version found"))?;
                load_secret_file(&self.secrets_path, &download.filename)
            }
        }
    }

    async fn list_all(&self) -> Result<Vec<String>, JawsError> {
        let repo = self.get_repo()?;
        let secrets = repo.list_secrets_by_provider(&self.id)?;
        Ok(secrets.into_iter().map(|s| s.display_name).collect())
    }

    fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let db_path = self.secrets_path.join("jaws.db");
        let provider_id = self.id.clone();

        let secrets: Vec<Result<String, Box<dyn std::error::Error + Send>>> =
            match init_db(&db_path) {
                Ok(conn) => {
                    let repo = SecretRepository::new(conn);
                    match repo.list_secrets_by_provider(&provider_id) {
                        Ok(secrets) => secrets.into_iter().map(|s| Ok(s.display_name)).collect(),
                        Err(e) => {
                            let msg = e.to_string();
                            vec![Err(Box::new(std::io::Error::other(msg))
                                as Box<dyn std::error::Error + Send>)]
                        }
                    }
                }
                Err(e) => {
                    vec![Err(Box::new(std::io::Error::other(e.to_string()))
                        as Box<dyn std::error::Error + Send>)]
                }
            };

        Box::new(stream::iter(secrets))
    }

    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, JawsError> {
        let api_ref = Self::generate_api_ref();
        let hash = hash_api_ref(&api_ref);

        let (filename, content_hash) =
            write_secret_version(&self.secrets_path, &self.id, name, 1, value)?;

        let repo = self.get_repo()?;
        let input = SecretInput {
            provider_id: self.id.clone(),
            api_ref: api_ref.clone(),
            display_name: name.to_string(),
            hash,
            description: description.map(|s| s.to_string()),
            remote_updated_at: None,
        };
        let secret = repo.upsert_secret(&input)?;
        // Local jaws has no remote — stamp pushed_at so push is a no-op.
        repo.create_download(secret, &filename, &content_hash, Some(Utc::now()))?;
        repo.log_operation("create", &self.id, name, None)?;

        Ok(api_ref)
    }

    async fn update(&self, api_ref: &str, value: &str) -> Result<String, JawsError> {
        let repo = self.get_repo()?;
        let secret = repo
            .get_secret_by_api_ref(&self.id, api_ref)?
            .ok_or_else(|| JawsError::not_found(format!("Secret not found: {}", api_ref)))?;
        let latest = repo
            .get_latest_download(secret.id)?
            .ok_or_else(|| JawsError::not_found("No version exists"))?;

        let new_version = latest.version + 1;
        let (filename, content_hash) = write_secret_version(
            &self.secrets_path,
            &secret.provider_id,
            &secret.display_name,
            new_version,
            value,
        )?;

        repo.create_download(secret.id, &filename, &content_hash, Some(Utc::now()))?;
        repo.log_operation(
            "update",
            &self.id,
            &secret.display_name,
            Some(&format!("{{\"version\": {}}}", new_version)),
        )?;

        Ok(format!("v{}", new_version))
    }

    async fn delete(&self, api_ref: &str, _force: bool) -> Result<(), JawsError> {
        let repo = self.get_repo()?;
        let secret = repo
            .get_secret_by_api_ref(&self.id, api_ref)?
            .ok_or_else(|| JawsError::not_found(format!("Secret not found: {}", api_ref)))?;

        delete_working_file(&self.secrets_path, &secret.provider_id, &secret.display_name)?;
        delete_all_archives(&self.secrets_path, &secret.provider_id, &secret.display_name)?;

        let name = secret.display_name.clone();
        repo.delete_secret(secret.id)?;
        repo.log_operation("delete", &self.id, &name, None)?;

        Ok(())
    }

    async fn rollback(&self, api_ref: &str, version_id: Option<&str>) -> Result<String, JawsError> {
        let repo = self.get_repo()?;
        let secret = repo
            .get_secret_by_api_ref(&self.id, api_ref)?
            .ok_or_else(|| JawsError::not_found(format!("Secret not found: {}", api_ref)))?;

        let downloads = repo.list_downloads(secret.id)?;
        if downloads.len() <= 1 {
            return Err(JawsError::Validation(
                "Only one version exists, nothing to rollback to".into(),
            ));
        }

        let target = if let Some(v) = version_id {
            let version: i32 = v.parse()?;
            repo.get_download_by_version(secret.id, version)?
                .ok_or_else(|| JawsError::not_found(format!("Version {} not found", v)))?
        } else {
            downloads
                .get(1)
                .cloned()
                .ok_or_else(|| JawsError::not_found("No previous version found"))?
        };

        let content = load_secret_file(&self.secrets_path, &target.filename)?;
        let target_content_hash = compute_content_hash(&content);

        let current = &downloads[0];
        if let Some(current_hash) = &current.file_hash
            && current_hash == &target_content_hash
        {
            return Ok(format!(
                "No changes - content identical to v{}.",
                target.version
            ));
        }

        let new_version = current.version + 1;
        let (filename, content_hash) = write_secret_version(
            &self.secrets_path,
            &secret.provider_id,
            &secret.display_name,
            new_version,
            &content,
        )?;

        repo.create_download(secret.id, &filename, &content_hash, Some(Utc::now()))?;
        repo.log_operation(
            "rollback",
            &self.id,
            &secret.display_name,
            Some(&format!(
                "{{\"from_version\": {}, \"to_version\": {}}}",
                target.version, new_version
            )),
        )?;

        Ok(format!(
            "Rolled back v{} -> v{} (new current)",
            target.version, new_version
        ))
    }
}
