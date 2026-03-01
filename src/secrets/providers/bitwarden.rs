use crate::error::JawsError;
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
use std::sync::Arc;
use uuid::Uuid;

pub struct BitwardenSecretManager {
    client: Arc<Client>,
    project_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    id: String,
}

impl BitwardenSecretManager {
    /// Create a new Bitwarden secret manager
    pub async fn new(
        provider_id: String,
        project_id: Option<String>,
        token_env: &str,
        organization_id: Option<String>,
    ) -> Result<Self, JawsError> {
        // Get token from environment
        let token = env::var(token_env).map_err(|_| {
            JawsError::provider(
                "bitwarden",
                format!(
                    "Environment variable '{}' not set. Please set it to your Bitwarden Access Token.",
                    token_env
                ),
            )
        })?;

        if token.is_empty() {
            return Err(JawsError::provider(
                "bitwarden",
                format!(
                    "Environment variable '{}' is empty. Please set it to your Bitwarden Access Token.",
                    token_env
                ),
            ));
        }

        let organization_id = if let Some(id) = organization_id {
            Some(Uuid::parse_str(&id).map_err(|e| {
                JawsError::provider(
                    "bitwarden",
                    format!("Invalid Organization ID '{}': {}", id, e),
                )
            })?)
        } else {
            env::var("BWS_ORGANIZATION_ID")
                .ok()
                .and_then(|id| Uuid::parse_str(&id).ok())
        };

        let settings = ClientSettings {
            device_type: DeviceType::SDK,
            user_agent: format!("jaws/{}", env!("CARGO_PKG_VERSION")),
            bitwarden_client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            ..Default::default()
        };

        let client = Client::new(Some(settings));

        let request = AccessTokenLoginRequest {
            access_token: token,
            state_file: None,
        };
        client
            .auth()
            .login_access_token(&request)
            .await
            .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;

        let parsed_project_id = if let Some(id_str) = project_id {
            Some(Uuid::parse_str(&id_str).map_err(|e| {
                JawsError::provider(
                    "bitwarden",
                    format!("Invalid Project ID '{}': {}", id_str, e),
                )
            })?)
        } else {
            None
        };

        Ok(Self {
            client: Arc::new(client),
            project_id: parsed_project_id,
            organization_id,
            id: provider_id,
        })
    }

    /// List secret identifiers, preferring project-scoped listing when a project_id is configured.
    async fn list_secret_identifiers(
        &self,
    ) -> Result<Vec<bitwarden::secrets_manager::secrets::SecretIdentifiersResponse>, JawsError>
    {
        if let Some(project_id) = self.project_id {
            let request = SecretIdentifiersByProjectRequest { project_id };
            let response = self
                .client
                .secrets()
                .list_by_project(&request)
                .await
                .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;
            Ok(vec![response])
        } else {
            let request = SecretIdentifiersRequest {
                organization_id: self.organization_id.unwrap_or(Uuid::nil()),
            };
            let response = self
                .client
                .secrets()
                .list(&request)
                .await
                .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;
            Ok(vec![response])
        }
    }

    /// Helper to find a secret ID by name
    async fn find_secret_id(&self, name: &str) -> Result<Uuid, JawsError> {
        let responses = self.list_secret_identifiers().await?;

        for response in responses {
            for secret in response.data {
                if secret.key == name {
                    return Ok(secret.id);
                }
            }
        }

        Err(JawsError::not_found(format!("Secret '{}' not found", name)))
    }

    /// List all projects accessible to the service account
    pub async fn list_projects(&self) -> Result<Vec<(String, Uuid)>, JawsError> {
        let request = ProjectsListRequest {
            organization_id: self.organization_id.unwrap_or(Uuid::nil()),
        };
        let response = self
            .client
            .projects()
            .list(&request)
            .await
            .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;

        let mut projects = Vec::new();
        for project in response.data {
            projects.push((project.name, project.id));
        }
        Ok(projects)
    }
}

#[async_trait]
impl SecretManager for BitwardenSecretManager {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "bitwarden"
    }

    async fn get_secret(&self, name: &str) -> Result<String, JawsError> {
        let id = if let Ok(uuid) = Uuid::parse_str(name) {
            uuid
        } else {
            self.find_secret_id(name).await?
        };

        let response = self
            .client
            .secrets()
            .get(&SecretGetRequest { id })
            .await
            .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;
        Ok(response.value)
    }

    async fn list_all(&self) -> Result<Vec<String>, JawsError> {
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
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let client = self.client.clone();
        let project_id = self.project_id;
        let organization_id = self.organization_id.unwrap_or(Uuid::nil());

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

    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, JawsError> {
        let project_id = self.project_id.ok_or_else(|| {
            JawsError::validation(
                "Project ID is required to create secrets. Configure 'vault' (project_id) in jaws.kdl",
            )
        })?;

        let request = SecretCreateRequest {
            organization_id: self.organization_id.unwrap_or(Uuid::nil()),
            project_ids: Some(vec![project_id]),
            key: name.to_string(),
            value: value.to_string(),
            note: description.unwrap_or("").to_string(),
        };

        let response = self
            .client
            .secrets()
            .create(&request)
            .await
            .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;
        Ok(response.id.to_string())
    }

    async fn update(&self, name: &str, value: &str) -> Result<String, JawsError> {
        let id = self.find_secret_id(name).await?;

        let current = self
            .client
            .secrets()
            .get(&SecretGetRequest { id })
            .await
            .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;

        let request = SecretPutRequest {
            id,
            organization_id: current.organization_id,
            project_ids: current.project_id.map(|id| vec![id]),
            key: current.key,
            value: value.to_string(),
            note: current.note,
        };

        let response = self
            .client
            .secrets()
            .update(&request)
            .await
            .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;
        Ok(response.id.to_string())
    }

    async fn delete(&self, name: &str, _force: bool) -> Result<(), JawsError> {
        let id = self.find_secret_id(name).await?;
        let request = SecretsDeleteRequest { ids: vec![id] };
        self.client
            .secrets()
            .delete(request)
            .await
            .map_err(|e| JawsError::provider("bitwarden", e.to_string()))?;
        Ok(())
    }

    async fn rollback(&self, _name: &str, _version_id: Option<&str>) -> Result<String, JawsError> {
        Err(JawsError::unsupported(
            "Bitwarden SDK does not support rollback/history operations.",
        ))
    }
}
