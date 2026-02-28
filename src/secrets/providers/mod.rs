//! Provider enumeration and detection.

mod aws;
mod bitwarden;
mod jaws;
pub mod onepassword;

pub use aws::AwsSecretManager;
pub use bitwarden::BitwardenSecretManager;
pub use jaws::JawsSecretManager;
pub use onepassword::{OnePasswordSecretManager, SecretRef};

use std::path::Path;

use crate::config::{Config, ProviderConfig};
use crate::credentials::retrieve_credential;
use crate::db::SecretRepository;
use crate::secrets::manager::SecretManager;

use aws_config::meta::region::RegionProviderChain;
use aws_config::profile::ProfileFileCredentialsProvider;
use aws_sdk_secretsmanager::{Client, config::Region};
use futures::StreamExt;
use futures::stream::Stream;

pub enum Provider {
    Aws(AwsSecretManager, String),                 // Manager + Provider ID
    OnePassword(OnePasswordSecretManager, String), // Manager + Provider ID
    Bitwarden(BitwardenSecretManager, String),     // Manager + Provider ID
    Jaws(JawsSecretManager, String),               // Manager + Provider ID
}

impl Provider {
    pub fn id(&self) -> &str {
        match self {
            Provider::Aws(_, id) => id,
            Provider::OnePassword(_, id) => id,
            Provider::Bitwarden(_, id) => id,
            Provider::Jaws(_, id) => id,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Provider::Aws(_, _) => "aws",
            Provider::OnePassword(_, _) => "onepassword",
            Provider::Bitwarden(_, _) => "bitwarden",
            Provider::Jaws(_, _) => "jaws",
        }
    }

    pub fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        match self {
            Provider::Aws(m, _) => m.list_secrets_stream(None),
            Provider::OnePassword(m, _) => m.list_secrets_stream(None),
            Provider::Bitwarden(m, _) => m.list_secrets_stream(None),
            Provider::Jaws(m, _) => m.list_secrets_stream(None),
        }
    }

    /// Get the value of a secret by its API reference
    pub async fn get_secret(&self, api_ref: &str) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            Provider::Aws(m, _) => m.get_secret(api_ref).await,
            Provider::OnePassword(m, _) => m.get_secret(api_ref).await,
            Provider::Bitwarden(m, _) => m.get_secret(api_ref).await,
            Provider::Jaws(m, _) => m.get_secret(api_ref).await,
        }
    }

    pub async fn create(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            Provider::Aws(m, _) => m.create(name, value, description).await,
            Provider::OnePassword(m, _) => m.create(name, value, description).await,
            Provider::Bitwarden(m, _) => m.create(name, value, description).await,
            Provider::Jaws(m, _) => m.create(name, value, description).await,
        }
    }

    pub async fn update(
        &self,
        name: &str,
        value: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            Provider::Aws(m, _) => m.update(name, value).await,
            Provider::OnePassword(m, _) => m.update(name, value).await,
            Provider::Bitwarden(m, _) => m.update(name, value).await,
            Provider::Jaws(m, _) => m.update(name, value).await,
        }
    }

    pub async fn delete(&self, name: &str, force: bool) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Provider::Aws(m, _) => m.delete(name, force).await,
            Provider::OnePassword(m, _) => m.delete(name, force).await,
            Provider::Bitwarden(m, _) => m.delete(name, force).await,
            Provider::Jaws(m, _) => m.delete(name, force).await,
        }
    }

    pub async fn rollback(
        &self,
        name: &str,
        version_id: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            Provider::Aws(m, _) => m.rollback(name, version_id).await,
            Provider::OnePassword(m, _) => m.rollback(name, version_id).await,
            Provider::Bitwarden(m, _) => m.rollback(name, version_id).await,
            Provider::Jaws(m, _) => m.rollback(name, version_id).await,
        }
    }
}

/// Select secrets from all providers using a unified TUI
pub async fn select_from_all_providers(
    providers: &[Provider],
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    use ff::{TuiConfig, create_items_channel, run_tui_with_config};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let (tx, rx) = create_items_channel();

    // Map from display string to full reference (for 1Password combined refs)
    let display_to_full: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    // Spawn tasks to fetch secrets from each provider
    for provider in providers {
        let tx_clone = tx.clone();
        let provider_id = provider.id().to_string();
        let provider_kind = provider.kind().to_string();
        let mut stream = provider.list_secrets_stream();
        let map_clone = Arc::clone(&display_to_full);

        tokio::spawn(async move {
            while let Some(result) = stream.next().await {
                if let Ok(secret) = result {
                    // For 1Password, secrets are in combined format "display_path|||api_ref"
                    // For AWS, secrets are just the secret name
                    let display_secret = if provider_kind == "onepassword" {
                        let secret_ref = SecretRef::parse(&secret);
                        // Store the mapping from display key to full reference
                        let mut map = map_clone.lock().await;
                        let display_key = format!("{} | {}", provider_id, secret_ref.display_path);
                        map.insert(display_key, secret.clone());
                        secret_ref.display_path
                    } else {
                        secret.clone()
                    };

                    // Format: "provider_id | secret_name"
                    let item = format!("{} | {}", provider_id, display_secret);
                    if tx_clone.send(item).await.is_err() {
                        break;
                    }
                }
            }
        });
    }

    // Drop original tx so channel closes when tasks finish
    drop(tx);

    let mut tui_config = TuiConfig::fullscreen();
    tui_config.show_help_text = false;

    let selected_items = run_tui_with_config(rx, true, tui_config)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    let map = display_to_full.lock().await;
    let mut result = Vec::new();
    for (_, item) in selected_items {
        if let Some((prov_id, _secret_display)) = item.split_once(" | ") {
            // Look up the full reference from our map
            let full_ref = map.get(&item).cloned().unwrap_or_else(|| {
                // Fallback for non-1Password providers
                item.split_once(" | ")
                    .map(|(_, s)| s.to_string())
                    .unwrap_or(item.clone())
            });
            result.push((prov_id.to_string(), full_ref));
        }
    }

    Ok(result)
}

/// Detect and initialize all available providers.
/// The jaws provider is always available and is added first.
///
/// If a `SecretRepository` is provided, stored encrypted credentials will be
/// used as a fallback when environment variables are not set.
pub async fn detect_providers(
    config: &Config,
    repo: Option<&SecretRepository>,
) -> Result<Vec<Provider>, Box<dyn std::error::Error>> {
    let mut providers = Vec::new();
    let use_keychain = config.keychain_cache();
    let cache_ttl = config.cache_ttl();
    let secrets_path = config.secrets_path();

    // Jaws provider is ALWAYS first and always available
    let jaws = JawsSecretManager::new(config.secrets_path());
    providers.push(Provider::Jaws(jaws, "jaws".to_string()));

    // Process configured remote providers
    for provider_config in &config.providers {
        match provider_config.kind.as_str() {
            "aws" => {
                // Check if profile is "all" - discover all AWS profiles
                if provider_config.profile.as_deref() == Some("all") {
                    match Config::discover_aws_profiles() {
                        Ok(profiles) => {
                            if profiles.is_empty() {
                                eprintln!("No AWS profiles found in ~/.aws/credentials");
                            }
                            for profile_name in profiles {
                                // Get region for this profile if available
                                let region = Config::get_aws_profile_region(&profile_name);

                                let expanded_config = ProviderConfig {
                                    id: format!("aws-{}", profile_name),
                                    kind: "aws".to_string(),
                                    profile: Some(profile_name.clone()),
                                    region,
                                    vault: None,
                                    organization: None,
                                    token_env: None,
                                };

                                match init_aws_provider(
                                    &expanded_config,
                                    repo,
                                    use_keychain,
                                    cache_ttl,
                                    &secrets_path,
                                )
                                .await
                                {
                                    Ok(aws_provider) => {
                                        providers
                                            .push(Provider::Aws(aws_provider, expanded_config.id));
                                    }
                                    Err(e) => eprintln!(
                                        "Failed to init AWS profile '{}': {}",
                                        profile_name, e
                                    ),
                                }
                            }
                        }
                        Err(e) => eprintln!("Failed to discover AWS profiles: {}", e),
                    }
                } else {
                    // Normal single profile
                    match init_aws_provider(
                        provider_config,
                        repo,
                        use_keychain,
                        cache_ttl,
                        &secrets_path,
                    )
                    .await
                    {
                        Ok(aws_provider) => {
                            providers.push(Provider::Aws(aws_provider, provider_config.id.clone()));
                        }
                        Err(e) => eprintln!(
                            "Failed to init AWS provider '{}': {}",
                            provider_config.id, e
                        ),
                    }
                }
            }
            "onepassword" | "1password" | "op" => {
                // Check if vault is "all" - discover all 1Password vaults
                if provider_config.vault.as_deref() == Some("all") {
                    match init_onepassword_all_vaults(
                        provider_config,
                        repo,
                        use_keychain,
                        cache_ttl,
                        &secrets_path,
                    )
                    .await
                    {
                        Ok(vault_providers) => {
                            for (op_provider, vault_id) in vault_providers {
                                providers.push(Provider::OnePassword(op_provider, vault_id));
                            }
                        }
                        Err(e) => eprintln!("Failed to discover 1Password vaults: {}", e),
                    }
                } else {
                    // Normal single vault
                    match init_onepassword_provider(
                        provider_config,
                        repo,
                        use_keychain,
                        cache_ttl,
                        &secrets_path,
                    )
                    .await
                    {
                        Ok(op_provider) => {
                            providers.push(Provider::OnePassword(
                                op_provider,
                                provider_config.id.clone(),
                            ));
                        }
                        Err(e) => eprintln!(
                            "Failed to init 1Password provider '{}': {}",
                            provider_config.id, e
                        ),
                    }
                }
            }
            "bitwarden" | "bws" | "bw" => {
                match init_bitwarden_provider(
                    provider_config,
                    repo,
                    use_keychain,
                    cache_ttl,
                    &secrets_path,
                )
                .await
                {
                    Ok(bw_provider) => {
                        providers
                            .push(Provider::Bitwarden(bw_provider, provider_config.id.clone()));
                    }
                    Err(e) => eprintln!(
                        "Failed to init Bitwarden provider '{}': {}",
                        provider_config.id, e
                    ),
                }
            }
            "jaws" => {
                // Future: remote jaws instance
                eprintln!(
                    "Remote jaws providers not yet implemented. \
                     Future: Configure 'url' to connect to jaws serve."
                );
            }
            _ => eprintln!(
                "Unknown provider kind: '{}'. Valid kinds: aws, onepassword, bitwarden",
                provider_config.kind
            ),
        }
    }

    // Always succeeds since jaws provider is always available
    Ok(providers)
}

/// Try to restore a credential from the database into an environment variable.
/// Returns true if the env var was set from stored credentials.
///
/// When `use_keychain` is true, the OS keychain is checked first (and populated
/// after a successful decryption) to avoid prompting on subsequent invocations.
fn try_restore_credential_to_env(
    repo: Option<&SecretRepository>,
    provider_id: &str,
    credential_key: &str,
    env_var: &str,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> bool {
    // Only attempt if the env var is not already set
    if std::env::var(env_var).is_ok() {
        return false;
    }

    let repo = match repo {
        Some(r) => r,
        None => return false,
    };

    match retrieve_credential(
        repo,
        provider_id,
        credential_key,
        use_keychain,
        cache_ttl,
        secrets_path,
    ) {
        Ok(Some(value)) => {
            // SAFETY: This is called during single-threaded provider initialization
            // before any provider threads are spawned. The env var is set to provide
            // credentials to provider constructors that read from the environment.
            unsafe {
                std::env::set_var(env_var, &value);
            }
            eprintln!(
                "  Restored {} from encrypted storage for provider '{}'",
                env_var, provider_id
            );
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!(
                "  Failed to decrypt stored {} for '{}': {}",
                credential_key, provider_id, e
            );
            false
        }
    }
}

/// Initialize 1Password providers for all accessible vaults
async fn init_onepassword_all_vaults(
    config: &ProviderConfig,
    repo: Option<&SecretRepository>,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> Result<Vec<(OnePasswordSecretManager, String)>, Box<dyn std::error::Error>> {
    let token_env = config
        .token_env
        .as_deref()
        .unwrap_or("OP_SERVICE_ACCOUNT_TOKEN");

    // Try credential fallback if env var is missing
    try_restore_credential_to_env(
        repo,
        &config.id,
        "token",
        token_env,
        use_keychain,
        cache_ttl,
        secrets_path,
    );

    // First, create a temporary manager to list vaults
    let temp_manager = OnePasswordSecretManager::new(None, token_env).await?;
    let vaults = temp_manager.list_vaults()?;

    if vaults.is_empty() {
        return Err("No 1Password vaults accessible with the current service account token".into());
    }

    let mut providers = Vec::new();
    for vault in vaults {
        let vault_id = vault.id.clone();
        let provider_id = format!("op-{}", vault.title.to_lowercase().replace(' ', "-"));

        match OnePasswordSecretManager::new(Some(vault_id), token_env).await {
            Ok(manager) => {
                providers.push((manager, provider_id));
            }
            Err(e) => eprintln!("Failed to init 1Password vault '{}': {}", vault.title, e),
        }
    }

    Ok(providers)
}

async fn init_aws_provider(
    config: &ProviderConfig,
    repo: Option<&SecretRepository>,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> Result<AwsSecretManager, Box<dyn std::error::Error>> {
    // For AWS without a profile, try credential fallback for env-var-based auth
    if config.profile.is_none() {
        try_restore_credential_to_env(
            repo,
            &config.id,
            "access_key_id",
            "AWS_ACCESS_KEY_ID",
            use_keychain,
            cache_ttl,
            secrets_path,
        );
        try_restore_credential_to_env(
            repo,
            &config.id,
            "secret_access_key",
            "AWS_SECRET_ACCESS_KEY",
            use_keychain,
            cache_ttl,
            secrets_path,
        );
    }

    // Set up region with fallback
    let region_provider = config
        .region
        .clone()
        .map(Region::new)
        .map(RegionProviderChain::first_try)
        .unwrap_or_else(|| RegionProviderChain::default_provider())
        .or_else(Region::new("us-west-2"));

    // Start building config
    let mut config_loader = aws_config::from_env().region(region_provider);

    // If profile is specified, force its use by explicitly setting credentials provider.
    // This bypasses environment variables (AWS_ACCESS_KEY_ID, etc.) in the default
    // credential chain, ensuring the profile credentials are used instead.
    if let Some(profile_name) = &config.profile {
        let credentials_provider = ProfileFileCredentialsProvider::builder()
            .profile_name(profile_name)
            .build();
        config_loader = config_loader
            .profile_name(profile_name)
            .credentials_provider(credentials_provider);
    }

    // Load final config
    let shared_config = config_loader.load().await;
    let client = Client::new(&shared_config);

    Ok(AwsSecretManager::new(client))
}

async fn init_onepassword_provider(
    config: &ProviderConfig,
    repo: Option<&SecretRepository>,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> Result<OnePasswordSecretManager, Box<dyn std::error::Error>> {
    let vault = config.vault.clone();
    let token_env = config
        .token_env
        .as_deref()
        .unwrap_or("OP_SERVICE_ACCOUNT_TOKEN");

    // Try credential fallback if env var is missing
    try_restore_credential_to_env(
        repo,
        &config.id,
        "token",
        token_env,
        use_keychain,
        cache_ttl,
        secrets_path,
    );

    OnePasswordSecretManager::new(vault, token_env).await
}

async fn init_bitwarden_provider(
    config: &ProviderConfig,
    repo: Option<&SecretRepository>,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> Result<BitwardenSecretManager, Box<dyn std::error::Error>> {
    let project_id = config.vault.clone(); // Map vault to project_id for config consistency
    let token_env = config.token_env.as_deref().unwrap_or("BWS_ACCESS_TOKEN");
    let organization_id = config.organization.clone();

    // Try credential fallback if env var is missing
    try_restore_credential_to_env(
        repo,
        &config.id,
        "token",
        token_env,
        use_keychain,
        cache_ttl,
        secrets_path,
    );

    BitwardenSecretManager::new(project_id, token_env, organization_id).await
}
