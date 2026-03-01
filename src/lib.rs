//! jaws - A CLI tool and library for managing secrets from multiple providers.
//!
//! This crate provides functionality to:
//! - Pull secrets from AWS Secrets Manager, 1Password, Bitwarden, and local storage
//! - Push secrets back to providers
//! - Track version history of downloaded secrets
//! - Export/import secrets as encrypted archives
//!
//! # Architecture
//!
//! All secret providers implement the [`SecretManager`] trait, which is object-safe
//! and used as `Box<dyn SecretManager>` (aliased as [`Provider`]). This makes it
//! trivial to add new providers - just implement the trait and register in
//! `detect_providers`.
//!
//! # Example
//!
//! ```no_run
//! use jaws::{Config, detect_providers};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::load()?;
//!     let providers = detect_providers(&config, None).await?;
//!     
//!     for provider in &providers {
//!         println!("Provider: {} ({})", provider.id(), provider.kind());
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod archive;
pub mod cli;
pub mod commands;
pub mod config;
pub mod credentials;
pub mod db;
pub mod error;
pub mod keychain;
pub mod secrets;
pub mod utils;

// Re-export commonly used types at the crate root
pub use config::Config;
pub use db::{DbProvider, SecretRepository, init_db};
pub use error::JawsError;
pub use secrets::{Provider, SecretManager, detect_providers};
