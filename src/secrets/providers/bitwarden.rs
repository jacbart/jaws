use crate::secrets::manager::SecretManager;
use async_trait::async_trait;
use bitwarden::{
    Client, ClientSettings, DeviceType,
    auth::login::AccessTokenLoginRequest,
    secrets_manager::{
        ClientProjectsExt, ClientSecretsExt,
        projects::ProjectsListRequest,
        secrets::{
            SecretCreateRequest, SecretGetRequest, SecretIdentifiersByProjectRequest,
            SecretIdentifiersRequest, SecretPutRequest, SecretsDeleteRequest,
        },
    },
};
use futures::stream::{self, Stream, StreamExt};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// A unit type for Bitwarden filters (not currently supported)
#[derive(Debug, Clone)]
pub struct BitwardenFilter;

pub struct BitwardenSecretManager {
    client: Arc<Client>,
    project_id: Option<Uuid>,
    organization_id: Option<Uuid>,
}

impl BitwardenSecretManager {
    /// Create a new Bitwarden secret manager
    pub async fn new(
        project_id: Option<String>,
        token_env: &str,
        organization_id: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Get token from environment
        let token = env::var(token_env).map_err(|_| {
            format!(
                "Environment variable '{}' not set. Please set it to your Bitwarden Access Token.",
                token_env
            )
        })?;

        if token.is_empty() {
            return Err(format!(
                "Environment variable '{}' is empty. Please set it to your Bitwarden Access Token.",
                token_env
            )
            .into());
        }

        // Try to get organization ID from argument first, then environment
        let organization_id = if let Some(id) = organization_id {
            Some(
                Uuid::parse_str(&id)
                    .map_err(|e| format!("Invalid Organization ID '{}': {}", id, e))?,
            )
        } else {
            env::var("BWS_ORGANIZATION_ID")
                .ok()
                .and_then(|id| Uuid::parse_str(&id).ok())
        };

        // Initialize client
        let settings = ClientSettings {
            device_type: DeviceType::SDK,
            user_agent: format!("jaws/{}", env!("CARGO_PKG_VERSION")),
            bitwarden_client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            ..Default::default()
        };

        let client = Client::new(Some(settings));

        // Authenticate
        let request = AccessTokenLoginRequest {
            access_token: token,
            state_file: None,
        };
        client.auth().login_access_token(&request).await?;

        // Parse project ID if provided
        let parsed_project_id = if let Some(id_str) = project_id {
            Some(
                Uuid::parse_str(&id_str)
                    .map_err(|e| format!("Invalid Project ID '{}': {}", id_str, e))?,
            )
        } else {
            None
        };

        Ok(Self {
            client: Arc::new(client),
            project_id: parsed_project_id,
            organization_id,
        })
    }

    /// List secret identifiers, preferring project-scoped listing when a project_id is configured.
    ///
    /// The Bitwarden API returns 404 when an access token only has project-level access
    /// but `secrets().list()` (organization-scoped) is called. Using `list_by_project()`
    /// avoids this by querying only the secrets within the configured project.
    async fn list_secret_identifiers(
        &self,
    ) -> Result<
        Vec<bitwarden::secrets_manager::secrets::SecretIdentifiersResponse>,
        Box<dyn std::error::Error>,
    > {
        if let Some(project_id) = self.project_id {
            // Project-scoped listing -- avoids 404 for project-only access tokens
            let request = SecretIdentifiersByProjectRequest { project_id };
            let response = self.client.secrets().list_by_project(&request).await?;
            Ok(vec![response])
        } else {
            // Organization-scoped listing -- requires org-wide access
            let request = SecretIdentifiersRequest {
                organization_id: self.organization_id.unwrap_or(Uuid::nil()),
            };
            let response = self.client.secrets().list(&request).await?;
            Ok(vec![response])
        }
    }

    /// Helper to find a secret ID by name
    /// This is necessary because the SDK operates on UUIDs, but jaws uses human-readable names
    async fn find_secret_id(&self, name: &str) -> Result<Uuid, Box<dyn std::error::Error>> {
        let responses = self.list_secret_identifiers().await?;

        for response in responses {
            for secret in response.data {
                if secret.key == name {
                    return Ok(secret.id);
                }
            }
        }

        Err(format!("Secret '{}' not found", name).into())
    }

    /// List all projects accessible to the service account
    pub async fn list_projects(&self) -> Result<Vec<(String, Uuid)>, Box<dyn std::error::Error>> {
        let request = ProjectsListRequest {
            organization_id: self.organization_id.unwrap_or(Uuid::nil()),
        };
        let response = self.client.projects().list(&request).await?;

        let mut projects = Vec::new();
        for project in response.data {
            projects.push((project.name, project.id));
        }
        Ok(projects)
    }
}

#[async_trait]
impl SecretManager for BitwardenSecretManager {
    type Filter = BitwardenFilter;

    async fn get_secret(&self, name: &str) -> Result<String, Box<dyn std::error::Error>> {
        // If the name looks like a UUID, try to use it directly
        let id = if let Ok(uuid) = Uuid::parse_str(name) {
            uuid
        } else {
            self.find_secret_id(name).await?
        };

        let response = self.client.secrets().get(&SecretGetRequest { id }).await?;
        Ok(response.value)
    }

    async fn list_all(
        &self,
        _filters: Option<Vec<Self::Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let responses = self.list_secret_identifiers().await?;

        let mut names = Vec::new();
        for response in responses {
            for secret in response.data {
                names.push(secret.key);
            }
        }

        Ok(names)
    }

    fn list_secrets_stream(
        &self,
        _filters: Option<Vec<Self::Filter>>,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let client = self.client.clone();
        let project_id = self.project_id;
        let organization_id = self.organization_id.unwrap_or(Uuid::nil());

        // Create a stream that yields the list of secrets.
        // Prefer project-scoped listing when a project_id is configured to avoid
        // 404 errors from the Bitwarden API for project-only access tokens.
        let stream = stream::once(async move {
            let result = if let Some(pid) = project_id {
                let request = SecretIdentifiersByProjectRequest { project_id: pid };
                client
                    .secrets()
                    .list_by_project(&request)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)
            } else {
                let request = SecretIdentifiersRequest { organization_id };
                client
                    .secrets()
                    .list(&request)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)
            };

            match result {
                Ok(response) => {
                    let names: Vec<String> = response.data.into_iter().map(|s| s.key).collect();
                    Ok(names)
                }
                Err(e) => Err(e),
            }
        })
        .flat_map(
            |result: Result<Vec<String>, Box<dyn std::error::Error + Send>>| match result {
                Ok(names) => {
                    let items: Vec<Result<String, Box<dyn std::error::Error + Send>>> =
                        names.into_iter().map(Ok).collect();
                    stream::iter(items)
                }
                Err(e) => stream::iter(vec![Err(e)]),
            },
        );

        Box::new(Box::pin(stream))
    }

    async fn download(
        &self,
        name: &str,
        dir: PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>> {
        use std::fs::{self, File};
        use std::io::Write;

        let content = self.get_secret(name).await?;

        // Use secret name for filename
        let path = dir.join(name);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(&path)?;
        file.write_all(content.as_bytes())?;

        Ok(path.display().to_string())
    }

    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let project_id = self.project_id.ok_or(
            "Project ID is required to create secrets. Configure 'vault' (project_id) in jaws.kdl",
        )?;

        let request = SecretCreateRequest {
            organization_id: self.organization_id.unwrap_or(Uuid::nil()),
            project_ids: Some(vec![project_id]),
            key: name.to_string(),
            value: value.to_string(),
            note: description.unwrap_or("").to_string(),
        };

        let response = self.client.secrets().create(&request).await?;
        Ok(response.id.to_string())
    }

    async fn update(&self, name: &str, value: &str) -> Result<String, Box<dyn std::error::Error>> {
        let id = self.find_secret_id(name).await?;

        // We need to get the secret first to preserve other fields
        let current = self.client.secrets().get(&SecretGetRequest { id }).await?;

        let request = SecretPutRequest {
            id,
            organization_id: current.organization_id,
            project_ids: current.project_id.map(|id| vec![id]),
            key: current.key,
            value: value.to_string(),
            note: current.note,
        };

        let response = self.client.secrets().update(&request).await?;
        Ok(response.id.to_string())
    }

    async fn delete(&self, name: &str, _force: bool) -> Result<(), Box<dyn std::error::Error>> {
        let id = self.find_secret_id(name).await?;
        let request = SecretsDeleteRequest { ids: vec![id] };
        self.client.secrets().delete(request).await?;
        Ok(())
    }

    async fn rollback(
        &self,
        _name: &str,
        _version_id: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Err("Bitwarden SDK does not support rollback/history operations.".into())
    }
}
