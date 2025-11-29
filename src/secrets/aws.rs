use async_trait::async_trait;
use aws_sdk_secretsmanager::{Client, types::Filter};
use std::path::PathBuf;
use super::manager::SecretManager;
use super::secrets_list::tui_selector;
use super::secrets::download_secret;

pub struct AwsSecretManager {
    client: Client,
}

impl AwsSecretManager {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SecretManager for AwsSecretManager {
    type Filter = Filter;

    async fn get_secret(&self, name: &str) -> Result<String, Box<dyn std::error::Error>> {
        super::secrets::get_secret(&self.client, name).await
    }

    async fn download_secret(
        &self,
        name: &str,
        dir: PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>> {
        download_secret(&self.client, name, dir).await
    }

    async fn list_secrets(
        &self,
        filters: Option<Vec<Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let builder = self.client.list_secrets().set_filters(filters).into_paginator();
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

    async fn select_secrets(
        &self,
        filters: Option<Vec<Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        tui_selector(&self.client, filters).await
    }
}
