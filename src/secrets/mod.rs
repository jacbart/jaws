//! Secrets management module.

pub mod manager;
pub mod providers;
pub mod secret;
pub mod storage;

pub use providers::{
    BitwardenSecretManager, OnePasswordSecretManager, Provider, SecretRef, detect_providers,
    select_from_all_providers,
};
#[allow(unused_imports)]
pub use storage::{get_secret_path, hash_api_ref, load_secret_file, save_secret_file};
