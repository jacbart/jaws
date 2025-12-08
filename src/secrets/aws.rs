use super::manager::SecretManager;
use super::secrets::{download_secret, get_secret};
use async_trait::async_trait;
use aws_sdk_secretsmanager::{Client, types::Filter};
use futures::stream::{self, Stream};
use std::path::PathBuf;

pub struct AwsSecretManager {
    client: Client,
}

impl AwsSecretManager {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Helper method for TUI selection using the stream
    pub async fn select_secrets(
        &self,
        filters: Option<Vec<Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        use ff::{TuiConfig, create_items_channel, run_tui_with_config};
        use futures::StreamExt as _;

        let (tx, rx) = create_items_channel();
        let mut stream = self.list_secrets_stream(filters);

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(name) => {
                        let _ = tx_clone.send(name).await;
                    }
                    Err(_) => break,
                }
            }
        });

        let mut config = TuiConfig::fullscreen();
        config.show_help_text = false;
        let sel = run_tui_with_config(rx, true, config).await?;
        Ok(sel)
    }
}

#[async_trait]
impl SecretManager for AwsSecretManager {
    type Filter = Filter;

    async fn get_secret(&self, name: &str) -> Result<String, Box<dyn std::error::Error>> {
        get_secret(&self.client, name).await
    }

    async fn list_all(
        &self,
        filters: Option<Vec<Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let builder = self
            .client
            .list_secrets()
            .set_filters(filters)
            .into_paginator();
        let mut stream = builder.send();
        let mut secrets = Vec::new();

        while let Some(page) = stream.next().await {
            let page = page?;
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
        filters: Option<Vec<Filter>>,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin> {
        let client = self.client.clone();
        let builder = client.list_secrets().set_filters(filters).into_paginator();
        let paginator_stream = builder.send();

        // Create a stream that yields individual secret names from paginated results
        // We use unfold to maintain state (the paginator stream and current page items)
        let stream = stream::unfold(
            (paginator_stream, Vec::<String>::new()),
            |(mut paginator, mut current_items)| async move {
                // If we have items in the current page, yield the next one
                if !current_items.is_empty() {
                    let name = current_items.remove(0);
                    return Some((Ok(name), (paginator, current_items)));
                }

                // Otherwise, fetch the next page
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

    async fn download(
        &self,
        name: &str,
        dir: PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>> {
        download_secret(&self.client, name, dir).await
    }

    async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut builder = self.client.create_secret().name(name).secret_string(value);

        if let Some(desc) = description {
            builder = builder.description(desc);
        }

        let resp = builder.send().await?;
        Ok(resp
            .arn()
            .ok_or("Missing ARN in create response")?
            .to_string())
    }

    async fn update(
        &self,
        name: &str,
        value: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let resp = self
            .client
            .update_secret()
            .secret_id(name)
            .secret_string(value)
            .send()
            .await?;

        Ok(resp
            .arn()
            .ok_or("Missing ARN in update response")?
            .to_string())
    }

    async fn delete(
        &self,
        name: &str,
        force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = self.client.delete_secret().secret_id(name);

        if force {
            builder = builder.force_delete_without_recovery(true);
        }

        builder.send().await?;
        Ok(())
    }

    async fn rollback(
        &self,
        name: &str,
        version_id: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Get the secret value from the specified version (or latest if not specified)
        let mut get_builder = self.client.get_secret_value().secret_id(name);
        if let Some(vid) = version_id {
            get_builder = get_builder.version_id(vid);
        }
        let secret_value = get_builder.send().await?.secret_string()
            .ok_or("Missing secret value in rollback")?
            .to_string();

        // Update the secret with the previous version's value
        let resp = self
            .client
            .update_secret()
            .secret_id(name)
            .secret_string(&secret_value)
            .send()
            .await?;

        Ok(resp
            .arn()
            .ok_or("Missing ARN in rollback response")?
            .to_string())
    }
}
