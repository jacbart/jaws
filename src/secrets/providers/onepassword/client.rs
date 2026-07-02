use async_trait::async_trait;
use std::sync::Arc;

use super::ffi::{Item, ItemFieldType, ItemOverview, VaultOverview};

#[async_trait]
pub trait OpClient: Send + Sync {
    async fn resolve_secret(&self, reference: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_vaults(&self) -> Result<Vec<VaultOverview>, Box<dyn std::error::Error + Send + Sync>>;
    async fn find_vault(&self, name_or_id: &str) -> Result<VaultOverview, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_items(&self, vault_id: &str, vault_name: &str) -> Result<Vec<ItemOverview>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_item(&self, vault_id: &str, vault_name: &str, item_ref: &str) -> Result<Item, Box<dyn std::error::Error + Send + Sync>>;
    async fn create_item(&self, item: &Item, vault_name: &str) -> Result<Item, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_item_field(
        &self,
        vault_id: &str,
        vault_name: &str,
        item_ref: &str,
        field_id: &str,
        value: &str,
        field_type: ItemFieldType,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_item(&self, vault_id: &str, vault_name: &str, item_ref: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn archive_item(&self, vault_id: &str, vault_name: &str, item_ref: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn format_item_ref(&self, vault_name: &str, vault_id: &str, item_title: &str, item_id: &str, field_title: &str, field_id: &str) -> String;
}

pub struct SharedSdkClient {
    inner: Arc<Box<dyn OpClient>>,
}

impl SharedSdkClient {
    pub fn new(client: Box<dyn OpClient>) -> Self {
        Self {
            inner: Arc::new(client),
        }
    }

    pub fn list_vaults_sync(&self) -> Result<Vec<VaultOverview>, Box<dyn std::error::Error + Send + Sync>> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| "No tokio runtime available")?;
        rt.block_on(self.inner.list_vaults())
    }

    pub async fn find_vault(&self, name_or_id: &str) -> Result<VaultOverview, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.find_vault(name_or_id).await
    }

    pub async fn list_items(&self, vault_id: &str, vault_name: &str) -> Result<Vec<ItemOverview>, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.list_items(vault_id, vault_name).await
    }

    pub async fn get_item(&self, vault_id: &str, vault_name: &str, item_ref: &str) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.get_item(vault_id, vault_name, item_ref).await
    }

    pub async fn resolve_secret(&self, reference: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.resolve_secret(reference).await
    }

    pub async fn create_item(&self, item: &Item, vault_name: &str) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.create_item(item, vault_name).await
    }

    pub async fn update_item_field(
        &self,
        vault_id: &str,
        vault_name: &str,
        item_ref: &str,
        field_id: &str,
        value: &str,
        field_type: ItemFieldType,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner.update_item_field(vault_id, vault_name, item_ref, field_id, value, field_type).await
    }

    pub async fn delete_item(&self, vault_id: &str, vault_name: &str, item_ref: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner.delete_item(vault_id, vault_name, item_ref).await
    }

    pub async fn archive_item(&self, vault_id: &str, vault_name: &str, item_ref: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner.archive_item(vault_id, vault_name, item_ref).await
    }

    pub async fn format_item_ref(&self, vault_name: &str, vault_id: &str, item_title: &str, item_id: &str, field_title: &str, field_id: &str) -> String {
        self.inner.format_item_ref(vault_name, vault_id, item_title, item_id, field_title, field_id)
    }
}

impl Clone for SharedSdkClient {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
