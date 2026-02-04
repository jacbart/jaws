//! Jaws local secret manager - treats locally stored secrets as first-class citizens.
//!
//! The jaws provider is always available and doesn't require external configuration.
//! It stores secrets locally with full version history.

use async_trait::async_trait;
use futures::stream::{self, Stream};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::db::{SecretInput, SecretRepository, init_db};
use crate::secrets::manager::SecretManager;
use crate::secrets::storage::{get_secret_path, hash_api_ref, load_secret_file, save_secret_file};

/// Filter for jaws secrets (future use for pattern matching, tags, etc.)
#[derive(Debug, Clone, Default)]
pub struct JawsFilter {
    pub name_pattern: Option<String>,
}

/// Local secret manager that stores secrets in the jaws secrets directory.
/// This provider is always available without any external configuration.
pub struct JawsSecretManager {
    secrets_path: PathBuf,
}

impl JawsSecretManager {
    /// Create a new JawsSecretManager.
    pub fn new(secrets_path: PathBuf) -> Self {
        Self { secrets_path }
    }

    /// Generate a unique API reference for local secrets.
    pub fn generate_api_ref() -> String {
        format!("jaws://{}", Uuid::new_v4())
    }

    /// Get the secrets path.
    pub fn secrets_path(&self) -> &PathBuf {
        &self.secrets_path
    }

    /// Get a repository connection.
    fn get_repo(&self) -> Result<SecretRepository, Box<dyn std::error::Error>> {
        let db_path = self.secrets_path.join("jaws.db");
        let conn = init_db(&db_path)?;
        Ok(SecretRepository::new(conn))
    }

    /// Create a secret directly (used by the create command).
    /// This is a convenience method that wraps the SecretManager::create trait method.
    pub async fn create_secret(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        <Self as SecretManager>::create(self, name, value, description).await
    }
}

#[async_trait]
impl SecretManager for JawsSecretManager {
    type Filter = JawsFilter;

    async fn get_secret(&self, api_ref: &str) -> Result<String, Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let secret = repo
            .get_secret_by_api_ref("jaws", api_ref)?
            .ok_or_else(|| format!("Secret not found: {}", api_ref))?;
        let download = repo
            .get_latest_download(secret.id)?
            .ok_or("No downloaded version found")?;
        let content = load_secret_file(&self.secrets_path, &download.filename)?;
        Ok(content)
    }

    async fn list_all(
        &self,
        _filters: Option<Vec<Self::Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let secrets = repo.list_secrets_by_provider("jaws")?;
        Ok(secrets.into_iter().map(|s| s.display_name).collect())
    }

    fn list_secrets_stream(
        &self,
        _filters: Option<Vec<Self::Filter>>,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        // Synchronously load secrets and return as a simple stream
        let db_path = self.secrets_path.join("jaws.db");

        let secrets: Vec<Result<String, Box<dyn std::error::Error + Send>>> =
            match init_db(&db_path) {
                Ok(conn) => {
                    let repo = SecretRepository::new(conn);
                    match repo.list_secrets_by_provider("jaws") {
                        Ok(secrets) => secrets.into_iter().map(|s| Ok(s.display_name)).collect(),
                        Err(e) => {
                            let msg = e.to_string();
                            vec![Err(Box::new(std::io::Error::other(msg))
                                as Box<dyn std::error::Error + Send>)]
                        }
                    }
                }
                Err(e) => {
                    vec![Err(Box::new(e) as Box<dyn std::error::Error + Send>)]
                }
            };

        Box::new(stream::iter(secrets))
    }

    async fn download(
        &self,
        api_ref: &str,
        dir: PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // For local secrets, "download" means copy to the target directory
        let content = self.get_secret(api_ref).await?;
        let repo = self.get_repo()?;
        let secret = repo
            .get_secret_by_api_ref("jaws", api_ref)?
            .ok_or("Secret not found")?;

        let dest = dir.join(&secret.display_name);
        std::fs::write(&dest, &content)?;
        Ok(dest.to_string_lossy().to_string())
    }

    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let api_ref = Self::generate_api_ref();
        let hash = hash_api_ref(&api_ref);

        // Ensure secrets directory exists
        std::fs::create_dir_all(&self.secrets_path)?;

        // Save file
        let (filename, content_hash) = save_secret_file(&self.secrets_path, name, &hash, 1, value)?;

        // Save to DB
        let repo = self.get_repo()?;
        let input = SecretInput {
            provider_id: "jaws".to_string(),
            api_ref: api_ref.clone(),
            display_name: name.to_string(),
            hash,
            description: description.map(|s| s.to_string()),
            remote_updated_at: None,
        };
        let secret = repo.upsert_secret(&input)?;
        repo.create_download(secret, &filename, &content_hash)?;
        repo.log_operation("create", "jaws", name, None)?;

        Ok(api_ref)
    }

    async fn update(
        &self,
        api_ref: &str,
        value: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let secret = repo
            .get_secret_by_api_ref("jaws", api_ref)?
            .ok_or_else(|| format!("Secret not found: {}", api_ref))?;
        let latest = repo
            .get_latest_download(secret.id)?
            .ok_or("No version exists")?;

        let new_version = latest.version + 1;
        let (filename, content_hash) = save_secret_file(
            &self.secrets_path,
            &secret.display_name,
            &secret.hash,
            new_version,
            value,
        )?;

        repo.create_download(secret.id, &filename, &content_hash)?;
        repo.log_operation(
            "update",
            "jaws",
            &secret.display_name,
            Some(&format!("{{\"version\": {}}}", new_version)),
        )?;

        Ok(format!("v{}", new_version))
    }

    async fn delete(&self, api_ref: &str, _force: bool) -> Result<(), Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let secret = repo
            .get_secret_by_api_ref("jaws", api_ref)?
            .ok_or_else(|| format!("Secret not found: {}", api_ref))?;

        // Delete all version files
        let downloads = repo.list_downloads(secret.id)?;
        for download in downloads {
            let path = get_secret_path(&self.secrets_path, &download.filename);
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
        }

        let name = secret.display_name.clone();
        repo.delete_secret(secret.id)?;
        repo.log_operation("delete", "jaws", &name, None)?;

        Ok(())
    }

    async fn rollback(
        &self,
        api_ref: &str,
        version_id: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let repo = self.get_repo()?;
        let secret = repo
            .get_secret_by_api_ref("jaws", api_ref)?
            .ok_or_else(|| format!("Secret not found: {}", api_ref))?;

        let downloads = repo.list_downloads(secret.id)?;
        if downloads.len() <= 1 {
            return Err("Only one version exists, nothing to rollback to".into());
        }

        // Determine target version
        let target = if let Some(v) = version_id {
            let version: i32 = v.parse()?;
            repo.get_download_by_version(secret.id, version)?
                .ok_or_else(|| format!("Version {} not found", v))?
        } else {
            // Default to previous version
            downloads
                .get(1)
                .cloned()
                .ok_or("No previous version found")?
        };

        // Read old content and create new version
        let content = load_secret_file(&self.secrets_path, &target.filename)?;
        let new_version = downloads[0].version + 1;

        let (filename, content_hash) = save_secret_file(
            &self.secrets_path,
            &secret.display_name,
            &secret.hash,
            new_version,
            &content,
        )?;

        repo.create_download(secret.id, &filename, &content_hash)?;
        repo.log_operation(
            "rollback",
            "jaws",
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

/// Get a secret by name (display_name) for the jaws provider.
/// This is a convenience function for looking up secrets by name rather than api_ref.
pub fn get_jaws_secret_by_name(
    secrets_path: &Path,
    name: &str,
) -> Result<Option<crate::db::DbSecret>, Box<dyn std::error::Error>> {
    let db_path = secrets_path.join("jaws.db");
    let conn = init_db(&db_path)?;
    let repo = SecretRepository::new(conn);

    let secrets = repo.list_secrets_by_provider("jaws")?;
    Ok(secrets.into_iter().find(|s| s.display_name == name))
}
