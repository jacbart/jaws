//! Provider implementations and detection.
//!
//! Organized into:
//! - Provider implementations: `aws`, `bitwarden`, `gcp`, `jaws`, `onepassword`, `vault`
//! - `detect`: Provider detection and initialization from config
//! - `select`: TUI-based secret selection across providers

mod aws;
mod bitwarden;
mod detect;
mod gcp;
mod jaws;
pub mod onepassword;
mod select;
mod vault;

pub use aws::AwsSecretManager;
pub use bitwarden::BitwardenSecretManager;
pub use detect::detect_providers;
pub use gcp::GcpSecretManager;
pub use jaws::JawsSecretManager;
pub use onepassword::{OnePasswordSecretManager, SecretRef};
pub use select::select_from_all_providers;
pub use vault::VaultSecretManager;

use crate::secrets::manager::SecretManager;

/// A provider is a boxed trait object implementing SecretManager.
///
/// This allows all providers to be stored in a single `Vec<Provider>` and
/// dispatched dynamically without a large match/enum delegation layer.
pub type Provider = Box<dyn SecretManager>;
