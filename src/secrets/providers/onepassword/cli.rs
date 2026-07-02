use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;

use super::client::OpClient;
use super::ffi::{
    FileAttributes, Item, ItemCategory, ItemField, ItemFieldType, ItemFile, ItemOverview,
    ItemSection, ItemState, VaultOverview, Website,
};
use crate::debug_eprintln;

const READ_TIMEOUT_SECS: u64 = 30;
const WRITE_TIMEOUT_SECS: u64 = 60;

pub struct OpCliClient;

impl OpCliClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn is_available() -> bool {
        let args = vec!["vault".to_string(), "list".to_string(), "--format".to_string(), "json".to_string()];
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            run_op(args),
        )
        .await
        {
            Ok(Ok(_)) => true,
            _ => false,
        }
    }
}

async fn run_op(args: Vec<String>) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        let output = std::process::Command::new("op")
            .args(&args)
            .stdin(Stdio::inherit())
            .env("OP_BIOMETRIC_UNLOCK_ENABLED", "true")
            .output()
            .map_err(|e| format!("Failed to run op CLI: {}", e))?;

        if output.status.success() {
            Ok(output.stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("op CLI error: {}", stderr.trim()))
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

async fn run_op_with_retry(args: Vec<String>, timeout_secs: u64) -> Result<Vec<u8>, String> {
    let max_retries = 3;
    let mut delay = std::time::Duration::from_secs(1);
    
    for attempt in 0..=max_retries {
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            run_op(args.clone()),
        )
        .await
        {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) => {
                // Check if it's an auth error
                if e.contains("account is not signed in") || e.contains("not signed in") {
                    if attempt < max_retries {
                        debug_eprintln!("  Authentication required, retrying in {}s... (attempt {}/{})", 
                            delay.as_secs(), attempt + 1, max_retries);
                        tokio::time::sleep(delay).await;
                        delay *= 2; // Exponential backoff
                        continue;
                    } else {
                        return Err(format!(
                            "{}\n\nHint: Ensure 1Password desktop app is running and 'Integrate with 1Password CLI' is enabled in Settings > Developer",
                            e
                        ));
                    }
                } else {
                    return Err(e);
                }
            }
            Err(_) => return Err(format!("op CLI timed out after {}s", timeout_secs)),
        }
    }
    
    Err("Max retries exceeded".to_string())
}

fn parse_op_json<T: serde::de::DeserializeOwned>(stdout: &[u8]) -> Result<T, String> {
    serde_json::from_slice(stdout).map_err(|e| format!("Failed to parse op CLI JSON: {}", e))
}

#[derive(Deserialize)]
struct OpVault {
    id: String,
    name: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Deserialize)]
struct OpItemOverview {
    id: String,
    title: String,
    category: String,
    vault: OpVaultRef,
    #[serde(default)]
    urls: Vec<OpUrl>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Deserialize)]
struct OpVaultRef {
    id: String,
    #[allow(dead_code)]
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct OpUrl {
    #[serde(default)]
    url: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct OpItem {
    id: String,
    title: String,
    category: String,
    #[serde(default)]
    version: i32,
    vault: OpVaultRef,
    #[serde(default)]
    fields: Vec<OpField>,
    #[serde(default, alias = "notesPlain")]
    notes_plain: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    urls: Vec<OpUrl>,
    #[serde(default)]
    files: Vec<OpFile>,
    #[serde(default)]
    sections: Vec<OpSectionCli>,
    #[serde(default, alias = "createdAt")]
    created_at: Option<String>,
    #[serde(default, alias = "updatedAt")]
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct OpField {
    id: String,
    #[serde(default, rename = "type")]
    field_type: String,
    #[allow(dead_code)]
    #[serde(default)]
    purpose: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    value: String,
}

#[derive(Deserialize)]
struct OpFile {
    name: String,
    id: String,
    #[serde(default)]
    size: i64,
}

#[derive(Deserialize)]
struct OpSectionCli {
    id: String,
    #[serde(default)]
    label: String,
}

fn map_category(cat: &str) -> ItemCategory {
    match cat.to_uppercase().as_str() {
        "LOGIN" => ItemCategory::Login,
        "SECURE_NOTE" | "SECURENOTE" => ItemCategory::SecureNote,
        "CREDIT_CARD" | "CREDITCARD" => ItemCategory::CreditCard,
        "CRYPTO_WALLET" | "CRYPTOWALLET" => ItemCategory::CryptoWallet,
        "IDENTITY" => ItemCategory::Identity,
        "PASSWORD" => ItemCategory::Password,
        "DOCUMENT" => ItemCategory::Document,
        "API_CREDENTIAL" | "APICREDENTIAL" => ItemCategory::ApiCredentials,
        "BANK_ACCOUNT" | "BANKACCOUNT" => ItemCategory::BankAccount,
        "DATABASE" => ItemCategory::Database,
        "DRIVER_LICENSE" | "DRIVERLICENSE" => ItemCategory::DriverLicense,
        "EMAIL" => ItemCategory::Email,
        "MEDICAL_RECORD" | "MEDICALRECORD" => ItemCategory::MedicalRecord,
        "MEMBERSHIP" => ItemCategory::Membership,
        "OUTDOOR_LICENSE" | "OUTDOORLICENSE" => ItemCategory::OutdoorLicense,
        "PASSPORT" => ItemCategory::Passport,
        "REWARDS" => ItemCategory::Rewards,
        "ROUTER" => ItemCategory::Router,
        "SERVER" => ItemCategory::Server,
        "SSH_KEY" | "SSHKEY" => ItemCategory::SshKey,
        "SOCIAL_SECURITY_NUMBER" | "SOCIALSECURITYNUMBER" => ItemCategory::SocialSecurityNumber,
        "SOFTWARE_LICENSE" | "SOFTWARELICENSE" => ItemCategory::SoftwareLicense,
        _ => ItemCategory::Unsupported,
    }
}

fn map_field_type(ft: &str) -> ItemFieldType {
    match ft.to_uppercase().as_str() {
        "STRING" | "TEXT" => ItemFieldType::Text,
        "CONCEALED" => ItemFieldType::Concealed,
        "URL" => ItemFieldType::Url,
        "EMAIL" => ItemFieldType::Email,
        "TOTP" => ItemFieldType::Totp,
        "DATE" => ItemFieldType::Date,
        "MONTH_YEAR" | "MONTHYEAR" => ItemFieldType::MonthYear,
        "ADDRESS" => ItemFieldType::Address,
        "PHONE" => ItemFieldType::Phone,
        "REFERENCE" => ItemFieldType::Reference,
        "SSHKEY" | "SSH_KEY" => ItemFieldType::SshKey,
        "MENU" => ItemFieldType::Menu,
        "CREDIT_CARD_NUMBER" | "CREDITCARDNUMBER" => ItemFieldType::CreditCardNumber,
        "CREDIT_CARD_TYPE" | "CREDITCARDTYPE" => ItemFieldType::CreditCardType,
        _ => ItemFieldType::Unsupported,
    }
}

fn category_to_op_string(cat: &ItemCategory) -> &str {
    match cat {
        ItemCategory::Login => "Login",
        ItemCategory::SecureNote => "Secure Note",
        ItemCategory::CreditCard => "Credit Card",
        ItemCategory::CryptoWallet => "Crypto Wallet",
        ItemCategory::Identity => "Identity",
        ItemCategory::Password => "Password",
        ItemCategory::Document => "Document",
        ItemCategory::ApiCredentials => "API Credential",
        ItemCategory::BankAccount => "Bank Account",
        ItemCategory::Database => "Database",
        ItemCategory::DriverLicense => "Driver License",
        ItemCategory::Email => "Email Account",
        ItemCategory::MedicalRecord => "Medical Record",
        ItemCategory::Membership => "Membership",
        ItemCategory::OutdoorLicense => "Outdoor License",
        ItemCategory::Passport => "Passport",
        ItemCategory::Rewards => "Rewards",
        ItemCategory::Router => "Router",
        ItemCategory::Server => "Server",
        ItemCategory::SshKey => "SSH Key",
        ItemCategory::SocialSecurityNumber => "Social Security Number",
        ItemCategory::SoftwareLicense => "Software License",
        ItemCategory::Person => "Person",
        ItemCategory::Unsupported => "Login",
    }
}

fn op_vault_to_overview(v: &OpVault) -> VaultOverview {
    VaultOverview {
        id: v.id.clone(),
        title: v.name.clone(),
        created_at: v.created_at.clone(),
        updated_at: v.updated_at.clone(),
    }
}

fn op_item_overview_to_item_overview(item: &OpItemOverview) -> ItemOverview {
    let websites = item
        .urls
        .iter()
        .map(|u| Website {
            url: u.url.clone(),
            label: if u.primary { "primary".to_string() } else { String::new() },
            autofill_behavior: String::new(),
        })
        .collect();

    ItemOverview {
        id: item.id.clone(),
        title: item.title.clone(),
        category: map_category(&item.category),
        vault_id: item.vault.id.clone(),
        websites,
        tags: item.tags.clone(),
        created_at: item.created_at.clone(),
        updated_at: item.updated_at.clone(),
        state: ItemState::Active,
    }
}

fn op_item_to_item(item: &OpItem) -> Item {
    let fields = item
        .fields
        .iter()
        .map(|f| ItemField {
            id: f.id.clone(),
            title: if f.label.is_empty() { f.id.clone() } else { f.label.clone() },
            section_id: None,
            field_type: map_field_type(&f.field_type),
            value: f.value.clone(),
        })
        .collect();

    let websites = item
        .urls
        .iter()
        .map(|u| Website {
            url: u.url.clone(),
            label: if u.primary { "primary".to_string() } else { String::new() },
            autofill_behavior: String::new(),
        })
        .collect();

    let files = item
        .files
        .iter()
        .map(|f| ItemFile {
            attributes: FileAttributes {
                name: f.name.clone(),
                id: f.id.clone(),
                size: f.size,
            },
            section_id: String::new(),
            field_id: String::new(),
        })
        .collect();

    let sections = item
        .sections
        .iter()
        .map(|s| ItemSection {
            id: s.id.clone(),
            title: s.label.clone(),
        })
        .collect();

    let document = if item.category.to_uppercase() == "DOCUMENT" {
        item.files.first().map(|f| FileAttributes {
            name: f.name.clone(),
            id: f.id.clone(),
            size: f.size,
        })
    } else {
        None
    };

    Item {
        id: item.id.clone(),
        title: item.title.clone(),
        category: map_category(&item.category),
        vault_id: item.vault.id.clone(),
        fields,
        sections,
        notes: item.notes_plain.clone().unwrap_or_default(),
        tags: item.tags.clone(),
        websites,
        version: item.version,
        files,
        document,
        created_at: item.created_at.clone().unwrap_or_default(),
        updated_at: item.updated_at.clone().unwrap_or_default(),
    }
}

#[async_trait]
impl OpClient for OpCliClient {
    async fn resolve_secret(&self, reference: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args = vec!["read".to_string(), reference.to_string()];
        let stdout = run_op_with_retry(args, READ_TIMEOUT_SECS).await.map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        Ok(String::from_utf8_lossy(&stdout).trim().to_string())
    }

    async fn list_vaults(&self) -> Result<Vec<VaultOverview>, Box<dyn std::error::Error + Send + Sync>> {
        let args = vec!["vault".to_string(), "list".to_string(), "--format".to_string(), "json".to_string()];
        let stdout = run_op_with_retry(args, READ_TIMEOUT_SECS).await.map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        let op_vaults: Vec<OpVault> = parse_op_json(&stdout).map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        Ok(op_vaults.iter().map(op_vault_to_overview).collect())
    }

    async fn find_vault(&self, name_or_id: &str) -> Result<VaultOverview, Box<dyn std::error::Error + Send + Sync>> {
        let vaults = self.list_vaults().await?;

        if let Some(vault) = vaults.iter().find(|v| v.id == name_or_id) {
            return Ok(vault.clone());
        }

        if let Some(vault) = vaults.iter().find(|v| v.title.to_lowercase() == name_or_id.to_lowercase()) {
            return Ok(vault.clone());
        }

        Err(format!(
            "Vault '{}' not found. Available vaults: {}",
            name_or_id,
            vaults.iter().map(|v| format!("{} ({})", v.title, v.id)).collect::<Vec<_>>().join(", ")
        ).into())
    }

    async fn list_items(&self, _vault_id: &str, vault_name: &str) -> Result<Vec<ItemOverview>, Box<dyn std::error::Error + Send + Sync>> {
        let args = vec![
            "item".to_string(),
            "list".to_string(),
            "--vault".to_string(),
            vault_name.to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let stdout = run_op_with_retry(args, READ_TIMEOUT_SECS).await.map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        let op_items: Vec<OpItemOverview> = parse_op_json(&stdout).map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        Ok(op_items.iter().map(op_item_overview_to_item_overview).collect())
    }

    async fn get_item(&self, _vault_id: &str, vault_name: &str, item_ref: &str) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        let args = vec![
            "item".to_string(),
            "get".to_string(),
            item_ref.to_string(),
            "--vault".to_string(),
            vault_name.to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let stdout = run_op_with_retry(args, READ_TIMEOUT_SECS).await.map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        let op_item: OpItem = parse_op_json(&stdout).map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        Ok(op_item_to_item(&op_item))
    }

    async fn create_item(&self, item: &Item, vault_name: &str) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        let category_str = category_to_op_string(&item.category);

        let field_args: Vec<String> = item
            .fields
            .iter()
            .filter(|f| !f.value.is_empty())
            .map(|f| {
                let type_suffix = match f.field_type {
                    ItemFieldType::Concealed => "[concealed]",
                    _ => "",
                };
                format!("{}{}={}", f.title, type_suffix, f.value)
            })
            .collect();

        let mut args: Vec<String> = vec![
            "item".to_string(),
            "create".to_string(),
            "--category".to_string(),
            category_str.to_string(),
            "--title".to_string(),
            item.title.clone(),
            "--vault".to_string(),
            vault_name.to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];

        if !item.notes.is_empty() {
            args.push("--notes".to_string());
            args.push(item.notes.clone());
        }

        if !item.tags.is_empty() {
            args.push("--tags".to_string());
            args.push(item.tags.join(","));
        }

        args.extend(field_args);

        let stdout = run_op_with_retry(args, WRITE_TIMEOUT_SECS).await.map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        let op_item: OpItem = parse_op_json(&stdout).map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        Ok(op_item_to_item(&op_item))
    }

    async fn update_item_field(
        &self,
        _vault_id: &str,
        vault_name: &str,
        item_ref: &str,
        field_id: &str,
        value: &str,
        field_type: ItemFieldType,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let type_suffix = match field_type {
            ItemFieldType::Concealed => "[concealed]",
            _ => "",
        };
        let field_assignment = format!("{}{}={}", field_id, type_suffix, value);

        let args = vec![
            "item".to_string(),
            "edit".to_string(),
            item_ref.to_string(),
            "--vault".to_string(),
            vault_name.to_string(),
            field_assignment,
        ];

        run_op_with_retry(args, WRITE_TIMEOUT_SECS).await.map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        Ok(())
    }

    async fn delete_item(&self, _vault_id: &str, vault_name: &str, item_ref: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let args = vec![
            "item".to_string(),
            "delete".to_string(),
            item_ref.to_string(),
            "--vault".to_string(),
            vault_name.to_string(),
        ];

        run_op_with_retry(args, READ_TIMEOUT_SECS).await.map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        Ok(())
    }

    async fn archive_item(&self, _vault_id: &str, vault_name: &str, item_ref: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let args = vec![
            "item".to_string(),
            "delete".to_string(),
            item_ref.to_string(),
            "--vault".to_string(),
            vault_name.to_string(),
            "--archive".to_string(),
        ];

        run_op_with_retry(args, READ_TIMEOUT_SECS).await.map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        Ok(())
    }

    fn format_item_ref(&self, _vault_name: &str, vault_id: &str, _item_title: &str, item_id: &str, _field_title: &str, field_id: &str) -> String {
        format!("op://{}/{}/{}", vault_id, item_id, field_id)
    }
}
