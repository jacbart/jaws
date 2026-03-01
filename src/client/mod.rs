//! Client module for connecting to remote jaws servers.
//!
//! Provides `RemoteProvider` which implements `SecretManager` by proxying
//! operations to a remote jaws server over gRPC with mTLS.

pub mod connection;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tonic::transport::Channel;

use crate::error::JawsError;
use crate::secrets::manager::SecretManager;
use crate::server::service::proto;
use proto::jaws_service_client::JawsServiceClient;

/// A remote provider that proxies SecretManager operations to a jaws server.
///
/// Each RemoteProvider represents a single provider on the remote server.
/// For example, if the server has providers ["jaws", "aws-prod"], the client
/// will have RemoteProvider instances for each, with IDs like
/// "myserver/jaws" and "myserver/aws-prod".
pub struct RemoteProvider {
    /// The prefixed provider ID (e.g., "myserver/aws-prod").
    id: String,
    /// The remote provider's ID on the server (e.g., "aws-prod").
    remote_provider_id: String,
    /// The remote provider's kind (e.g., "aws").
    kind: String,
    /// Whether the remote provider supports rollback.
    rollback_support: bool,
    /// Whether the remote provider supports remote history.
    remote_history_support: bool,
    /// The gRPC client channel.
    client: JawsServiceClient<Channel>,
}

impl RemoteProvider {
    /// Create a new RemoteProvider.
    pub fn new(
        server_name: &str,
        remote_provider_id: String,
        kind: String,
        rollback_support: bool,
        remote_history_support: bool,
        channel: Channel,
    ) -> Self {
        let id = format!("{}/{}", server_name, remote_provider_id);
        Self {
            id,
            remote_provider_id,
            kind: format!("remote:{}", kind),
            rollback_support,
            remote_history_support,
            client: JawsServiceClient::new(channel),
        }
    }
}

#[async_trait]
impl SecretManager for RemoteProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        &self.kind
    }

    async fn get_secret(&self, name: &str) -> Result<String, JawsError> {
        let mut client = self.client.clone();
        let response = client
            .get_secret(proto::GetSecretRequest {
                provider_id: self.remote_provider_id.clone(),
                name: name.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e, "get_secret"))?;

        Ok(response.into_inner().value)
    }

    async fn list_all(&self) -> Result<Vec<String>, JawsError> {
        let mut client = self.client.clone();
        let response = client
            .list_secrets(proto::ListSecretsRequest {
                provider_id: self.remote_provider_id.clone(),
            })
            .await
            .map_err(|e| map_grpc_error(e, "list_secrets"))?;

        let mut stream = response.into_inner();
        let mut names = Vec::new();
        while let Some(entry) = stream
            .message()
            .await
            .map_err(|e| map_grpc_error(e, "list_secrets stream"))?
        {
            names.push(entry.name);
        }

        Ok(names)
    }

    fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let mut client = self.client.clone();
        let provider_id = self.remote_provider_id.clone();

        // Wrap the async gRPC call in a deferred stream: the connection is
        // established lazily when the stream is first polled.
        // 1. stream::once  → yields one Result<Vec<String>>
        // 2. flat_map       → expands each Vec into individual items
        let outer = futures::stream::once(Box::pin(async move {
            let response = client
                .list_secrets(proto::ListSecretsRequest {
                    provider_id: provider_id.clone(),
                })
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send> {
                    Box::new(map_grpc_error(e, "list_secrets"))
                })?;

            let mut inner = response.into_inner();
            let mut names = Vec::new();
            while let Some(entry) =
                inner
                    .message()
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send> {
                        Box::new(map_grpc_error(e, "list_secrets stream"))
                    })?
            {
                names.push(entry.name);
            }
            Ok::<_, Box<dyn std::error::Error + Send>>(names)
        }));

        Box::new(outer.flat_map(
            |result: Result<Vec<String>, Box<dyn std::error::Error + Send>>| match result {
                Ok(items) => futures::stream::iter(items.into_iter().map(Ok).collect::<Vec<_>>()),
                Err(e) => futures::stream::iter(vec![Err(e)]),
            },
        ))
    }

    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, JawsError> {
        let mut client = self.client.clone();
        let response = client
            .create_secret(proto::CreateSecretRequest {
                provider_id: self.remote_provider_id.clone(),
                name: name.to_string(),
                value: value.to_string(),
                description: description.unwrap_or_default().to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e, "create_secret"))?;

        Ok(response.into_inner().version_id)
    }

    async fn update(&self, name: &str, value: &str) -> Result<String, JawsError> {
        let mut client = self.client.clone();
        let response = client
            .update_secret(proto::UpdateSecretRequest {
                provider_id: self.remote_provider_id.clone(),
                name: name.to_string(),
                value: value.to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e, "update_secret"))?;

        Ok(response.into_inner().version_id)
    }

    async fn delete(&self, name: &str, force: bool) -> Result<(), JawsError> {
        let mut client = self.client.clone();
        client
            .delete_secret(proto::DeleteSecretRequest {
                provider_id: self.remote_provider_id.clone(),
                name: name.to_string(),
                force,
            })
            .await
            .map_err(|e| map_grpc_error(e, "delete_secret"))?;

        Ok(())
    }

    async fn rollback(&self, name: &str, version_id: Option<&str>) -> Result<String, JawsError> {
        let mut client = self.client.clone();
        let response = client
            .rollback_secret(proto::RollbackSecretRequest {
                provider_id: self.remote_provider_id.clone(),
                name: name.to_string(),
                version_id: version_id.unwrap_or_default().to_string(),
            })
            .await
            .map_err(|e| map_grpc_error(e, "rollback_secret"))?;

        Ok(response.into_inner().restored_version_id)
    }

    fn supports_rollback(&self) -> bool {
        self.rollback_support
    }

    fn supports_remote_history(&self) -> bool {
        self.remote_history_support
    }
}

/// Discover remote providers by calling ListProviders on the server.
/// Returns a Vec of RemoteProvider instances.
pub async fn discover_remote_providers(
    server_name: &str,
    channel: Channel,
) -> Result<Vec<RemoteProvider>, JawsError> {
    let mut client = JawsServiceClient::new(channel.clone());

    let response = client
        .list_providers(proto::ListProvidersRequest {})
        .await
        .map_err(|e| map_grpc_error(e, "list_providers"))?;

    let providers = response
        .into_inner()
        .providers
        .into_iter()
        .map(|info| {
            RemoteProvider::new(
                server_name,
                info.id,
                info.kind,
                info.supports_rollback,
                info.supports_remote_history,
                channel.clone(),
            )
        })
        .collect();

    Ok(providers)
}

/// Map a tonic::Status to a JawsError with context.
fn map_grpc_error(status: tonic::Status, operation: &str) -> JawsError {
    match status.code() {
        tonic::Code::NotFound => JawsError::not_found(status.message()),
        tonic::Code::PermissionDenied => {
            JawsError::provider("remote", format!("Permission denied: {}", status.message()))
        }
        tonic::Code::Unavailable => JawsError::provider(
            "remote",
            format!("Server unavailable ({}): {}", operation, status.message()),
        ),
        tonic::Code::Unimplemented => JawsError::unsupported(status.message()),
        _ => JawsError::provider(
            "remote",
            format!(
                "{} failed: {} ({})",
                operation,
                status.message(),
                status.code()
            ),
        ),
    }
}

/// Client connection paths for a specific server.
#[derive(Debug, Clone)]
pub struct ClientPaths {
    pub dir: PathBuf,
    pub ca_cert: PathBuf,
    pub client_cert: PathBuf,
    pub client_key: PathBuf,
}

impl ClientPaths {
    /// Compute client cert paths for a named server under the given config dir.
    pub fn new(config_dir: &Path, server_name: &str) -> Self {
        let dir = config_dir.join("clients").join(server_name);
        Self {
            ca_cert: dir.join("ca.pem"),
            client_cert: dir.join("client.pem"),
            client_key: dir.join("client-key.pem"),
            dir,
        }
    }

    /// Whether all client cert files exist.
    pub fn exists(&self) -> bool {
        self.ca_cert.exists() && self.client_cert.exists() && self.client_key.exists()
    }

    /// Save certificates to disk.
    pub fn save(
        &self,
        ca_cert_pem: &str,
        client_cert_pem: &str,
        client_key_pem: &str,
    ) -> Result<(), JawsError> {
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(&self.ca_cert, ca_cert_pem)?;
        std::fs::write(&self.client_cert, client_cert_pem)?;
        std::fs::write(&self.client_key, client_key_pem)?;
        crate::utils::restrict_file_permissions(&self.client_key)?;
        crate::utils::restrict_file_permissions(&self.ca_cert)?;
        crate::utils::restrict_file_permissions(&self.client_cert)?;
        Ok(())
    }
}
