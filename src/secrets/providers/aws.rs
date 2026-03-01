use crate::error::JawsError;
use crate::secrets::manager::SecretManager;
use async_trait::async_trait;
use aws_sdk_secretsmanager::Client;
use futures::stream::{self, Stream};

pub struct AwsSecretManager {
    client: Client,
    id: String,
}

impl AwsSecretManager {
    pub fn new(client: Client, id: String) -> Self {
        Self { client, id }
    }
}

#[async_trait]
impl SecretManager for AwsSecretManager {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "aws"
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    async fn get_secret(&self, name: &str) -> Result<String, JawsError> {
        let resp = self
            .client
            .get_secret_value()
            .secret_id(name)
            .send()
            .await
            .map_err(JawsError::aws)?;
        let secret_value = resp.secret_string().ok_or(
            "Secret is stored as binary, not a string. Binary secrets are not yet supported.",
        )?;
        Ok(secret_value.to_string())
    }

    async fn list_all(&self) -> Result<Vec<String>, JawsError> {
        let builder = self.client.list_secrets().into_paginator();
        let mut stream = builder.send();
        let mut secrets = Vec::new();

        while let Some(page) = stream.next().await {
            let page = page.map_err(JawsError::aws)?;
            for secret in page.secret_list() {
                if let Some(name) = secret.name() {
                    secrets.push(name.to_string());
                }
            }
        }

        Ok(secrets)
    }

    fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let client = self.client.clone();
        let builder = client.list_secrets().into_paginator();
        let paginator_stream = builder.send();

        let stream = stream::unfold(
            (paginator_stream, Vec::<String>::new()),
            |(mut paginator, mut current_items)| async move {
                if !current_items.is_empty() {
                    let name = current_items.remove(0);
                    return Some((Ok(name), (paginator, current_items)));
                }

                match paginator.next().await {
                    Some(Ok(page)) => {
                        let names: Vec<String> = page
                            .secret_list()
                            .iter()
                            .filter_map(|secret| secret.name().map(|s| s.to_string()))
                            .collect();

                        if names.is_empty() {
                            None
                        } else {
                            current_items = names[1..].to_vec();
                            Some((Ok(names[0].clone()), (paginator, current_items)))
                        }
                    }
                    Some(Err(e)) => Some((
                        Err(Box::new(e) as Box<dyn std::error::Error + Send>),
                        (paginator, current_items),
                    )),
                    None => None,
                }
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
        let mut builder = self.client.create_secret().name(name).secret_string(value);

        if let Some(desc) = description {
            builder = builder.description(desc);
        }

        let resp = builder.send().await.map_err(JawsError::aws)?;
        Ok(resp
            .arn()
            .ok_or("Missing ARN in create response")?
            .to_string())
    }

    async fn update(&self, name: &str, value: &str) -> Result<String, JawsError> {
        let resp = self
            .client
            .update_secret()
            .secret_id(name)
            .secret_string(value)
            .send()
            .await
            .map_err(JawsError::aws)?;

        Ok(resp
            .arn()
            .ok_or("Missing ARN in update response")?
            .to_string())
    }

    async fn delete(&self, name: &str, force: bool) -> Result<(), JawsError> {
        let mut builder = self.client.delete_secret().secret_id(name);

        if force {
            builder = builder.force_delete_without_recovery(true);
        }

        builder.send().await.map_err(JawsError::aws)?;
        Ok(())
    }

    async fn rollback(&self, name: &str, version_id: Option<&str>) -> Result<String, JawsError> {
        let mut get_builder = self.client.get_secret_value().secret_id(name);
        if let Some(vid) = version_id {
            get_builder = get_builder.version_id(vid);
        }
        let secret_value = get_builder
            .send()
            .await
            .map_err(JawsError::aws)?
            .secret_string()
            .ok_or("Missing secret value in rollback")?
            .to_string();

        let resp = self
            .client
            .update_secret()
            .secret_id(name)
            .secret_string(&secret_value)
            .send()
            .await
            .map_err(JawsError::aws)?;

        Ok(resp
            .arn()
            .ok_or("Missing ARN in rollback response")?
            .to_string())
    }
}
