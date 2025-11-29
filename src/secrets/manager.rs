use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait SecretManager {
    async fn get_secret(&self, name: &str) -> Result<String, Box<dyn std::error::Error>>;
    
    async fn download_secret(
        &self,
        name: &str,
        dir: PathBuf,
    ) -> Result<String, Box<dyn std::error::Error>>;
    
    async fn list_secrets(
        &self,
        filters: Option<Vec<Self::Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    
    async fn select_secrets(
        &self,
        filters: Option<Vec<Self::Filter>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>>;

    type Filter;
}
