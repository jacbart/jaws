//! Integration tests for the jaws library.
//!
//! These tests verify the public API works correctly.

use jaws::config::Config;
use jaws::secrets::storage::{get_secret_path, hash_api_ref};
use std::path::PathBuf;

#[test]
fn test_config_load_with_defaults() {
    // Load config - will use defaults if no config file exists
    let config = Config::load().unwrap();

    // Check default values are sensible
    assert!(!config.editor().is_empty());
    assert!(config.cache_ttl() > 0);
}

#[test]
fn test_hash_api_ref_consistency() {
    // Same input should always produce the same hash
    let hash1 = hash_api_ref("test-secret-name");
    let hash2 = hash_api_ref("test-secret-name");
    assert_eq!(hash1, hash2);

    // Different inputs should produce different hashes
    let hash3 = hash_api_ref("different-secret");
    assert_ne!(hash1, hash3);
}

#[test]
fn test_get_secret_path() {
    let secrets_dir = PathBuf::from("/tmp/secrets");
    let filename = "my_secret_abc123def456gh_1";

    let path = get_secret_path(&secrets_dir, filename);

    assert_eq!(
        path,
        PathBuf::from("/tmp/secrets/my_secret_abc123def456gh_1")
    );
}

#[test]
fn test_config_location_options() {
    // Should return at least one option
    let options = Config::get_config_location_options();
    assert!(!options.is_empty());

    // Each option should have a path and description
    for (path, desc) in &options {
        assert!(path.to_string_lossy().contains("jaws"));
        assert!(!desc.is_empty());
    }
}
