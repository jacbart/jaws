use async_trait::async_trait;
use futures::Stream;
use std::path::PathBuf;

/// Trait for managing secrets in a secrets manager backend
#[async_trait]
pub trait SecretManager {
    /// Filter type specific to the backend implementation
    type Filter;

    /// Get a secret's value by name
    async fn get_secret(&self, name: &str) -> Result<String, Box<dyn std::error::Error>>;

    /// List all secrets, optionally filtered
    async fn list_all(
        &self,
        filters: Option<Vec<Self::Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>>;

    /// Get a stream of secret names for use in TUI or async processing
    /// The stream yields secret names as Strings
    fn list_secrets_stream(
        &self,
        filters: Option<Vec<Self::Filter>>,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>;

    /// Download a secret to a local file
    async fn download(
        &self,
        name: &str,
        dir: PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>>;

    /// Create a new secret
    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>>;

    /// Update an existing secret's value
    async fn update(&self, name: &str, value: &str) -> Result<String, Box<dyn std::error::Error>>;

    /// Delete a secret
    async fn delete(&self, name: &str, force: bool) -> Result<(), Box<dyn std::error::Error>>;

    /// Rollback a secret to a previous version
    /// Returns the version ID that was restored
    async fn rollback(
        &self,
        name: &str,
        version_id: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>>;
}
