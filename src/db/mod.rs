//! Database module for managing secret metadata and download history.

mod models;
mod repository;
mod schema;

#[allow(unused_imports)]
pub use models::{DbDownload, DbProvider, DbSecret, SecretInput};
pub use repository::SecretRepository;
pub use schema::init_db;
