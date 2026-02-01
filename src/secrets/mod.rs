pub mod aws;
pub mod manager;
pub mod onepassword;
pub mod onepassword_ffi;
pub mod providers;
pub mod secret;
pub mod storage;

pub use aws::AwsSecretManager;
pub use onepassword::{OnePasswordSecretManager, SecretRef};
pub use providers::{detect_providers, select_from_all_providers, Provider};
pub use storage::{hash_api_ref, save_secret_file, get_secret_path};
