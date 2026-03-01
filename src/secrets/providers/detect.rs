//! Provider detection and initialization from configuration.

use std::path::Path;

use aws_config::meta::region::RegionProviderChain;
use aws_config::profile::ProfileFileCredentialsProvider;
use aws_sdk_secretsmanager::{Client, config::Region};

use crate::config::{Config, ProviderConfig};
use crate::credentials::retrieve_credential;
use crate::db::SecretRepository;
use crate::error::JawsError;

use google_cloud_secretmanager_v1::client::SecretManagerService;

use super::{
    AwsSecretManager, BitwardenSecretManager, GcpSecretManager, JawsSecretManager,
    OnePasswordSecretManager, Provider,
};

/// Detect and initialize all available providers.
/// The jaws provider is always available and is added first.
///
/// If a `SecretRepository` is provided, stored encrypted credentials will be
/// used as a fallback when environment variables are not set.
pub async fn detect_providers(
    config: &Config,
    repo: Option<&SecretRepository>,
) -> Result<Vec<Provider>, JawsError> {
    let mut providers: Vec<Provider> = Vec::new();
    let use_keychain = config.keychain_cache();
    let cache_ttl = config.cache_ttl();
    let secrets_path = config.secrets_path();

    // Jaws provider is ALWAYS first and always available
    let jaws = JawsSecretManager::new(config.secrets_path(), "jaws".to_string());
    providers.push(Box::new(jaws));

    // Process configured remote providers
    for provider_config in &config.providers {
        match provider_config.kind.as_str() {
            "aws" => {
                if provider_config.profile.as_deref() == Some("all") {
                    match Config::discover_aws_profiles() {
                        Ok(profiles) => {
                            if profiles.is_empty() {
                                eprintln!("No AWS profiles found in ~/.aws/credentials");
                            }
                            for profile_name in profiles {
                                let region = Config::get_aws_profile_region(&profile_name);

                                let expanded_config = ProviderConfig {
                                    id: format!("aws-{}", profile_name),
                                    kind: "aws".to_string(),
                                    profile: Some(profile_name.clone()),
                                    region,
                                    vault: None,
                                    organization: None,
                                    token_env: None,
                                    project: None,
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
                                        providers.push(Box::new(aws_provider));
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
                            providers.push(Box::new(aws_provider));
                        }
                        Err(e) => eprintln!(
                            "Failed to init AWS provider '{}': {}",
                            provider_config.id, e
                        ),
                    }
                }
            }
            "onepassword" | "1password" | "op" => {
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
                            for op_provider in vault_providers {
                                providers.push(Box::new(op_provider));
                            }
                        }
                        Err(e) => eprintln!("Failed to discover 1Password vaults: {}", e),
                    }
                } else {
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
                            providers.push(Box::new(op_provider));
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
                        providers.push(Box::new(bw_provider));
                    }
                    Err(e) => eprintln!(
                        "Failed to init Bitwarden provider '{}': {}",
                        provider_config.id, e
                    ),
                }
            }
            "gcp" | "gcloud" | "google" => {
                match init_gcp_provider(provider_config).await {
                    Ok(gcp_provider) => {
                        providers.push(Box::new(gcp_provider));
                    }
                    Err(e) => eprintln!(
                        "Failed to init GCP provider '{}': {}",
                        provider_config.id, e
                    ),
                }
            }
            "jaws" => {
                eprintln!(
                    "Remote jaws providers not yet implemented. \
                     Future: Configure 'url' to connect to jaws serve."
                );
            }
            _ => eprintln!(
                "Unknown provider kind: '{}'. Valid kinds: aws, onepassword, bitwarden, gcp",
                provider_config.kind
            ),
        }
    }

    Ok(providers)
}

/// Try to restore a credential from the database into an environment variable.
fn try_restore_credential_to_env(
    repo: Option<&SecretRepository>,
    provider_id: &str,
    credential_key: &str,
    env_var: &str,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> bool {
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

async fn init_onepassword_all_vaults(
    config: &ProviderConfig,
    repo: Option<&SecretRepository>,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> Result<Vec<OnePasswordSecretManager>, JawsError> {
    let token_env = config
        .token_env
        .as_deref()
        .unwrap_or("OP_SERVICE_ACCOUNT_TOKEN");

    try_restore_credential_to_env(
        repo,
        &config.id,
        "token",
        token_env,
        use_keychain,
        cache_ttl,
        secrets_path,
    );

    let temp_manager = OnePasswordSecretManager::new(config.id.clone(), None, token_env).await?;
    let vaults = temp_manager.list_vaults()?;

    if vaults.is_empty() {
        return Err(JawsError::provider(
            "onepassword",
            "No 1Password vaults accessible with the current service account token",
        ));
    }

    let mut providers = Vec::new();
    for vault in vaults {
        let vault_id = vault.id.clone();
        let provider_id = format!("op-{}", vault.title.to_lowercase().replace(' ', "-"));

        match OnePasswordSecretManager::new(provider_id, Some(vault_id), token_env).await {
            Ok(manager) => {
                providers.push(manager);
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
) -> Result<AwsSecretManager, JawsError> {
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

    let region_provider = config
        .region
        .clone()
        .map(Region::new)
        .map(RegionProviderChain::first_try)
        .unwrap_or_else(|| RegionProviderChain::default_provider())
        .or_else(Region::new("us-west-2"));

    let mut config_loader = aws_config::from_env().region(region_provider);

    if let Some(profile_name) = &config.profile {
        let credentials_provider = ProfileFileCredentialsProvider::builder()
            .profile_name(profile_name)
            .build();
        config_loader = config_loader
            .profile_name(profile_name)
            .credentials_provider(credentials_provider);
    }

    let shared_config = config_loader.load().await;
    let client = Client::new(&shared_config);

    Ok(AwsSecretManager::new(client, config.id.clone()))
}

async fn init_onepassword_provider(
    config: &ProviderConfig,
    repo: Option<&SecretRepository>,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> Result<OnePasswordSecretManager, JawsError> {
    let vault = config.vault.clone();
    let token_env = config
        .token_env
        .as_deref()
        .unwrap_or("OP_SERVICE_ACCOUNT_TOKEN");

    try_restore_credential_to_env(
        repo,
        &config.id,
        "token",
        token_env,
        use_keychain,
        cache_ttl,
        secrets_path,
    );

    OnePasswordSecretManager::new(config.id.clone(), vault, token_env).await
}

async fn init_bitwarden_provider(
    config: &ProviderConfig,
    repo: Option<&SecretRepository>,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> Result<BitwardenSecretManager, JawsError> {
    let project_id = config.vault.clone();
    let token_env = config.token_env.as_deref().unwrap_or("BWS_ACCESS_TOKEN");
    let organization_id = config.organization.clone();

    try_restore_credential_to_env(
        repo,
        &config.id,
        "token",
        token_env,
        use_keychain,
        cache_ttl,
        secrets_path,
    );

    BitwardenSecretManager::new(config.id.clone(), project_id, token_env, organization_id).await
}

async fn init_gcp_provider(config: &ProviderConfig) -> Result<GcpSecretManager, JawsError> {
    let project_id = config.project.clone().ok_or_else(|| {
        JawsError::config(format!(
            "GCP provider '{}' requires a 'project' field with the GCP project ID",
            config.id
        ))
    })?;

    // Expand ~ in GOOGLE_APPLICATION_CREDENTIALS if present.
    // The GCP SDK does not handle tilde expansion, so users who set
    // GOOGLE_APPLICATION_CREDENTIALS="~/path/to/key.json" (with quotes
    // preventing shell expansion) would get a confusing "could not create
    // default credentials" error.
    if let Ok(creds_path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        if creds_path.starts_with("~/") || creds_path == "~" {
            let expanded = crate::config::expand_tilde(&creds_path);
            unsafe {
                std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", &expanded);
            }
        }
    }

    // Build the GCP Secret Manager client using Application Default Credentials.
    // This automatically picks up:
    // - GOOGLE_APPLICATION_CREDENTIALS env var (service account key JSON)
    // - gcloud CLI credentials (gcloud auth application-default login)
    // - GCE/GKE metadata server (when running on GCP)
    let client = SecretManagerService::builder()
        .build()
        .await
        .map_err(|e| {
            let mut msg = format!("Failed to initialize GCP client: {}", e);
            if std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err() {
                msg.push_str(
                    "\n  Hint: Set GOOGLE_APPLICATION_CREDENTIALS to a service account key JSON file,\n  \
                     or run 'gcloud auth application-default login'",
                );
            } else {
                let path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").unwrap_or_default();
                if !std::path::Path::new(&path).exists() {
                    msg.push_str(&format!(
                        "\n  Hint: GOOGLE_APPLICATION_CREDENTIALS is set to '{}' but the file does not exist",
                        path
                    ));
                }
            }
            JawsError::provider("gcp", msg)
        })?;

    Ok(GcpSecretManager::new(client, project_id, config.id.clone()))
}
