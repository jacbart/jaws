use crate::error::JawsError;
use crate::secrets::manager::SecretManager;
use async_trait::async_trait;
use futures::stream::{self, Stream};
use google_cloud_gax::paginator::ItemPaginator as _;
use google_cloud_secretmanager_v1::client::SecretManagerService;
use google_cloud_secretmanager_v1::model::{Replication, Secret, SecretPayload};

pub struct GcpSecretManager {
    client: SecretManagerService,
    project_id: String,
    id: String,
}

impl GcpSecretManager {
    pub fn new(client: SecretManagerService, project_id: String, id: String) -> Self {
        Self {
            client,
            project_id,
            id,
        }
    }

    /// Build the parent resource name for the project.
    /// Format: `projects/{project_id}`
    fn parent(&self) -> String {
        format!("projects/{}", self.project_id)
    }

    /// Build the full secret resource name.
    /// Format: `projects/{project_id}/secrets/{secret_id}`
    fn secret_name(&self, name: &str) -> String {
        format!("projects/{}/secrets/{}", self.project_id, name)
    }

    /// Build the full secret version resource name.
    /// Format: `projects/{project_id}/secrets/{secret_id}/versions/{version}`
    fn version_name(&self, name: &str, version: &str) -> String {
        format!(
            "projects/{}/secrets/{}/versions/{}",
            self.project_id, name, version
        )
    }

    /// Extract the short secret name from a full resource name.
    /// `projects/my-project/secrets/my-secret` -> `my-secret`
    fn extract_short_name(full_name: &str) -> &str {
        // Resource name format: projects/{project}/secrets/{secret_id}
        // We want just the last component after "secrets/"
        if let Some(idx) = full_name.rfind("/secrets/") {
            &full_name[idx + "/secrets/".len()..]
        } else {
            full_name.rsplit('/').next().unwrap_or(full_name)
        }
    }
}

#[async_trait]
impl SecretManager for GcpSecretManager {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "gcp"
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    async fn get_secret(&self, name: &str) -> Result<String, JawsError> {
        let version_name = self.version_name(name, "latest");
        let resp = self
            .client
            .access_secret_version()
            .set_name(&version_name)
            .send()
            .await
            .map_err(JawsError::gcp)?;

        let payload = resp
            .payload
            .ok_or_else(|| JawsError::provider("gcp", "Secret version has no payload"))?;

        let data = payload.data;
        String::from_utf8(data.into())
            .map_err(|_| JawsError::provider("gcp", "Secret payload is not valid UTF-8"))
    }

    async fn list_all(&self) -> Result<Vec<String>, JawsError> {
        let parent = self.parent();
        let mut items = self
            .client
            .list_secrets()
            .set_parent(&parent)
            .by_item();

        let mut secrets = Vec::new();
        while let Some(item) = items.next().await {
            let secret = item.map_err(JawsError::gcp)?;
            let name = &secret.name;
            if !name.is_empty() {
                secrets.push(Self::extract_short_name(name).to_string());
            }
        }

        Ok(secrets)
    }

    fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let parent = self.parent();
        let client = self.client.clone();

        // We batch-fetch all secrets and then yield them one by one through the
        // stream. The GCP SDK paginator is not easily convertible to a
        // futures::Stream while remaining object-safe, so we fetch all names in
        // the first poll and then drain the buffer.
        let stream = stream::unfold(
            (Some((client, parent)), Vec::<String>::new()),
            |(state, mut buffer)| async move {
                // Drain buffer first
                if !buffer.is_empty() {
                    let name = buffer.remove(0);
                    return Some((Ok(name), (state, buffer)));
                }

                // If we already fetched everything, we're done
                let (client, parent) = state?;

                let mut items = client
                    .list_secrets()
                    .set_parent(&parent)
                    .by_item();

                let mut names = Vec::new();
                while let Some(item) = items.next().await {
                    match item {
                        Ok(secret) => {
                            let name = &secret.name;
                            if !name.is_empty() {
                                names.push(
                                    Self::extract_short_name(name).to_string(),
                                );
                            }
                        }
                        Err(e) => {
                            return Some((
                                Err(Box::new(e) as Box<dyn std::error::Error + Send>),
                                (None, Vec::new()),
                            ));
                        }
                    }
                }

                if names.is_empty() {
                    return None;
                }

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
        let parent = self.parent();

        // Step 1: Create the secret (metadata only, no payload yet)
        // GCP requires a replication policy to be specified
        let replication = Replication::default()
            .set_automatic(
                google_cloud_secretmanager_v1::model::replication::Automatic::default(),
            );

        let secret = Secret::default().set_replication(replication);

        self.client
            .create_secret()
            .set_parent(&parent)
            .set_secret_id(name)
            .set_secret(secret)
            .send()
            .await
            .map_err(JawsError::gcp)?;

        // Step 2: Add the first secret version with the actual payload
        let secret_name = self.secret_name(name);
        let payload = SecretPayload::default()
            .set_data(bytes::Bytes::from(value.to_string()));

        let version_resp = self
            .client
            .add_secret_version()
            .set_parent(&secret_name)
            .set_payload(payload)
            .send()
            .await
            .map_err(JawsError::gcp)?;

        let resp_name = &version_resp.name;
        if resp_name.is_empty() {
            Ok(secret_name)
        } else {
            Ok(resp_name.clone())
        }
    }

    async fn update(&self, name: &str, value: &str) -> Result<String, JawsError> {
        let secret_name = self.secret_name(name);
        let payload = SecretPayload::default()
            .set_data(bytes::Bytes::from(value.to_string()));

        let version_resp = self
            .client
            .add_secret_version()
            .set_parent(&secret_name)
            .set_payload(payload)
            .send()
            .await
            .map_err(JawsError::gcp)?;

        let resp_name = &version_resp.name;
        if resp_name.is_empty() {
            Ok(secret_name)
        } else {
            Ok(resp_name.clone())
        }
    }

    async fn delete(&self, name: &str, _force: bool) -> Result<(), JawsError> {
        let secret_name = self.secret_name(name);

        self.client
            .delete_secret()
            .set_name(&secret_name)
            .send()
            .await
            .map_err(JawsError::gcp)?;

        Ok(())
    }

    async fn rollback(&self, name: &str, version_id: Option<&str>) -> Result<String, JawsError> {
        // Access the specified version (or latest if none specified)
        let version = version_id.unwrap_or("latest");
        let version_name = self.version_name(name, version);

        let resp = self
            .client
            .access_secret_version()
            .set_name(&version_name)
            .send()
            .await
            .map_err(JawsError::gcp)?;

        let payload = resp
            .payload
            .ok_or_else(|| JawsError::provider("gcp", "Secret version has no payload"))?;

        let value = String::from_utf8(payload.data.into())
            .map_err(|_| JawsError::provider("gcp", "Secret payload is not valid UTF-8"))?;

        // Add it as a new version (effectively "rolling back" by making the old value current)
        self.update(name, &value).await
    }
}
