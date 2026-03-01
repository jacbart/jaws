use async_trait::async_trait;
use futures::Stream;

use crate::error::JawsError;

/// Trait for managing secrets in a secrets manager backend.
///
/// This trait is object-safe and can be used as `Box<dyn SecretManager>`.
/// All providers (AWS, 1Password, Bitwarden, local jaws) implement this trait.
///
/// # Capability methods
///
/// Providers can override `supports_rollback()` and `supports_remote_history()`
/// to indicate which operations they support. Default implementations return `false`.
#[async_trait]
pub trait SecretManager: Send + Sync {
    /// The unique identifier for this provider instance (e.g., "aws-prod", "op-vault1", "jaws").
    fn id(&self) -> &str;

    /// The provider kind (e.g., "aws", "onepassword", "bitwarden", "jaws").
    fn kind(&self) -> &str;

    /// Get a secret's value by name or API reference.
    async fn get_secret(&self, name: &str) -> Result<String, JawsError>;

    /// List all secret names/references.
    async fn list_all(&self) -> Result<Vec<String>, JawsError>;

    /// Get a stream of secret names for use in TUI or async processing.
    /// The stream yields secret names as Strings.
    fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>;

    /// Create a new secret.
    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, JawsError>;

    /// Update an existing secret's value.
    async fn update(&self, name: &str, value: &str) -> Result<String, JawsError>;

    /// Delete a secret.
    async fn delete(&self, name: &str, force: bool) -> Result<(), JawsError>;

    /// Rollback a secret to a previous version.
    /// Returns the version ID that was restored.
    ///
    /// Providers that don't support rollback should return `JawsError::Unsupported`.
    async fn rollback(&self, name: &str, version_id: Option<&str>) -> Result<String, JawsError>;

    /// Whether this provider supports rollback to previous versions.
    fn supports_rollback(&self) -> bool {
        false
    }

    /// Whether this provider supports remote version history.
    fn supports_remote_history(&self) -> bool {
        false
    }
}
