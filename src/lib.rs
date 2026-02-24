//! jaws - A CLI tool and library for managing secrets from multiple providers.
//!
//! This crate provides functionality to:
//! - Pull secrets from AWS Secrets Manager, 1Password, and local storage
//! - Push secrets back to providers
//! - Track version history of downloaded secrets
//! - Export/import secrets as encrypted archives
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
pub mod keychain;
pub mod secrets;
pub mod utils;

// Re-export commonly used types at the crate root
pub use config::Config;
pub use db::{DbProvider, SecretRepository, init_db};
pub use secrets::{Provider, detect_providers};
