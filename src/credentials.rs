//! Credential encryption and decryption for provider authentication tokens.
//!
//! Wraps the `age` encryption primitives from `archive.rs` to provide a
//! token-level API for storing and retrieving encrypted credentials.
//!
//! ## Session Caching
//!
//! Three levels of caching minimize user prompts within a single jaws invocation:
//!
//! 1. **Passphrase cache**: The decryption passphrase is cached after the first
//!    successful prompt so subsequent providers reuse it without re-prompting.
//!    If a cached passphrase fails decryption (e.g. different passphrase was used
//!    for a different provider), the cache is invalidated and the user is prompted
//!    again with up to 3 total attempts.
//!
//! 2. **SSH private key path cache**: The resolved SSH private key path is cached
//!    after the first resolution so SSH users are not prompted for the path on
//!    every provider.
//!
//! 3. **Decrypted value cache**: Successfully decrypted credential values are
//!    cached by `(provider_id, credential_key)` so the same credential is never
//!    decrypted twice in a session (avoids redundant scrypt work and prompts).

use age::secrecy::SecretString;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::archive::{
    decrypt_with_passphrase, decrypt_with_ssh_privkey, encrypt_with_passphrase,
    encrypt_with_ssh_pubkey, prompt_passphrase, prompt_passphrase_with_confirm,
};
use crate::db::SecretRepository;
use crate::keychain;

/// Maximum number of passphrase attempts before giving up.
const MAX_PASSPHRASE_ATTEMPTS: usize = 3;

/// Method used to encrypt a credential.
pub enum CredentialEncryptionMethod {
    /// Encrypt with a passphrase (age scrypt)
    Passphrase(SecretString),
    /// Encrypt with an SSH public key
    SshPublicKey(PathBuf),
}

/// Method used to decrypt a credential.
pub enum CredentialDecryptionMethod {
    /// Decrypt with a passphrase
    Passphrase(SecretString),
    /// Decrypt with an SSH private key
    SshPrivateKey(PathBuf),
}

// =============================================================================
// Session caches
// =============================================================================

/// Session-scoped cache for the decryption passphrase, so the user only
/// has to enter it once per jaws invocation even if multiple providers
/// need credential fallback.
static PASSPHRASE_CACHE: Mutex<Option<SecretString>> = Mutex::new(None);

/// Session-scoped cache for the SSH private key path, so SSH users only
/// need to provide or confirm the path once per invocation.
static SSH_KEY_PATH_CACHE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Session-scoped cache of already-decrypted credential values, keyed by
/// (provider_id, credential_key). Avoids redundant decryption (and prompts)
/// when the same credential is requested multiple times in a session.
static DECRYPTED_CACHE: Mutex<Option<HashMap<(String, String), String>>> = Mutex::new(None);

// -- Passphrase cache helpers --

/// Store a passphrase in the session cache.
fn cache_passphrase(passphrase: &SecretString) {
    if let Ok(mut cache) = PASSPHRASE_CACHE.lock() {
        *cache = Some(passphrase.clone());
    }
}

/// Retrieve a cached passphrase, if one exists.
fn get_cached_passphrase() -> Option<SecretString> {
    PASSPHRASE_CACHE.lock().ok().and_then(|c| c.clone())
}

/// Clear the cached passphrase (used when a cached passphrase fails decryption).
fn clear_passphrase_cache() {
    if let Ok(mut cache) = PASSPHRASE_CACHE.lock() {
        *cache = None;
    }
}

// -- SSH key path cache helpers --

/// Store an SSH private key path in the session cache.
fn cache_ssh_key_path(path: &PathBuf) {
    if let Ok(mut cache) = SSH_KEY_PATH_CACHE.lock() {
        *cache = Some(path.clone());
    }
}

/// Retrieve the cached SSH private key path, if one exists.
fn get_cached_ssh_key_path() -> Option<PathBuf> {
    SSH_KEY_PATH_CACHE.lock().ok().and_then(|c| c.clone())
}

// -- Decrypted value cache helpers --

/// Store a decrypted credential value in the session cache.
fn cache_decrypted_value(provider_id: &str, credential_key: &str, value: &str) {
    if let Ok(mut cache) = DECRYPTED_CACHE.lock() {
        let map = cache.get_or_insert_with(HashMap::new);
        map.insert(
            (provider_id.to_string(), credential_key.to_string()),
            value.to_string(),
        );
    }
}

/// Retrieve a cached decrypted credential value, if one exists.
fn get_cached_decrypted_value(provider_id: &str, credential_key: &str) -> Option<String> {
    DECRYPTED_CACHE.lock().ok().and_then(|cache| {
        cache.as_ref().and_then(|map| {
            map.get(&(provider_id.to_string(), credential_key.to_string()))
                .cloned()
        })
    })
}

// =============================================================================
// Encryption / Decryption
// =============================================================================

/// Encrypt a plaintext token string. Returns the encrypted bytes.
pub fn encrypt_token(
    plaintext: &str,
    method: &CredentialEncryptionMethod,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let data = plaintext.as_bytes();
    match method {
        CredentialEncryptionMethod::Passphrase(passphrase) => {
            encrypt_with_passphrase(data, passphrase.clone())
        }
        CredentialEncryptionMethod::SshPublicKey(pubkey_path) => {
            encrypt_with_ssh_pubkey(data, pubkey_path)
        }
    }
}

/// Decrypt encrypted token bytes back to a plaintext string.
pub fn decrypt_token(
    ciphertext: &[u8],
    method: &CredentialDecryptionMethod,
) -> Result<String, Box<dyn std::error::Error>> {
    let decrypted = match method {
        CredentialDecryptionMethod::Passphrase(passphrase) => {
            decrypt_with_passphrase(ciphertext, passphrase.clone())?
        }
        CredentialDecryptionMethod::SshPrivateKey(privkey_path) => {
            decrypt_with_ssh_privkey(ciphertext, privkey_path)?
        }
    };
    String::from_utf8(decrypted)
        .map_err(|e| format!("Decrypted token is not valid UTF-8: {}", e).into())
}

// =============================================================================
// Interactive prompts
// =============================================================================

/// Interactively prompt the user to choose an encryption method (passphrase or SSH key)
/// and collect the necessary input. Returns the method and a string tag for storage.
///
/// The returned tuple is (method, method_tag, ssh_fingerprint):
/// - method_tag: "passphrase" or "ssh"
/// - ssh_fingerprint: None for passphrase, Some(path_string) for SSH
pub fn prompt_encryption_method()
-> Result<(CredentialEncryptionMethod, String, Option<String>), Box<dyn std::error::Error>> {
    println!("  Choose encryption method:");
    println!("    1) Passphrase (age scrypt)");
    println!("    2) SSH public key");
    print!("  Selection [1]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    if choice == "2" {
        print!("  Path to SSH public key: ");
        io::stdout().flush()?;
        let mut key_path = String::new();
        io::stdin().read_line(&mut key_path)?;
        let key_path = key_path.trim();

        if key_path.is_empty() {
            return Err("SSH public key path cannot be empty".into());
        }

        let path = crate::config::expand_tilde(key_path);
        if !path.exists() {
            return Err(format!("SSH public key not found: {}", path.display()).into());
        }

        let fingerprint = path.to_string_lossy().to_string();
        Ok((
            CredentialEncryptionMethod::SshPublicKey(path),
            "ssh".to_string(),
            Some(fingerprint),
        ))
    } else {
        // Default: passphrase
        let passphrase = prompt_passphrase_with_confirm("  Enter encryption passphrase")?;
        cache_passphrase(&passphrase);
        Ok((
            CredentialEncryptionMethod::Passphrase(passphrase),
            "passphrase".to_string(),
            None,
        ))
    }
}

/// Build a decryption method based on the stored encryption_method tag.
///
/// For passphrase: uses the session cache or prompts the user.
/// For SSH: uses the cached key path, tries auto-derivation from the `.pub` hint,
/// or prompts the user. The resolved path is cached for subsequent calls.
pub fn build_decryption_method(
    encryption_method: &str,
    ssh_hint: Option<&str>,
) -> Result<CredentialDecryptionMethod, Box<dyn std::error::Error>> {
    match encryption_method {
        "passphrase" => {
            // Try cache first
            if let Some(cached) = get_cached_passphrase() {
                return Ok(CredentialDecryptionMethod::Passphrase(cached));
            }
            let passphrase = prompt_passphrase("Enter passphrase to decrypt stored credentials")?;
            cache_passphrase(&passphrase);
            Ok(CredentialDecryptionMethod::Passphrase(passphrase))
        }
        "ssh" => {
            // 1. Try session cache
            if let Some(cached_path) = get_cached_ssh_key_path() {
                return Ok(CredentialDecryptionMethod::SshPrivateKey(cached_path));
            }

            // 2. Try auto-deriving from the public key hint (strip .pub)
            if let Some(hint) = ssh_hint {
                let priv_path = hint.strip_suffix(".pub").unwrap_or(hint);
                let path = crate::config::expand_tilde(priv_path);
                if path.exists() {
                    eprintln!("  Using private key: {}", path.display());
                    cache_ssh_key_path(&path);
                    return Ok(CredentialDecryptionMethod::SshPrivateKey(path));
                }
            }

            // 3. Prompt the user
            let hint_msg = ssh_hint
                .map(|h| format!(" (encrypted with: {})", h))
                .unwrap_or_default();
            print!(
                "Path to SSH private key for credential decryption{}: ",
                hint_msg
            );
            io::stdout().flush()?;
            let mut key_path = String::new();
            io::stdin().read_line(&mut key_path)?;
            let key_path = key_path.trim();

            if key_path.is_empty() {
                return Err("SSH private key path cannot be empty".into());
            }

            let path = crate::config::expand_tilde(key_path);
            cache_ssh_key_path(&path);
            Ok(CredentialDecryptionMethod::SshPrivateKey(path))
        }
        _ => Err(format!("Unknown encryption method: {}", encryption_method).into()),
    }
}

// =============================================================================
// High-level operations
// =============================================================================

/// Encrypt and store a credential for a provider.
///
/// This is the high-level function used during `config init` and provider addition.
/// When `use_keychain` is true, the plaintext is also cached in the OS keychain
/// (scoped to `secrets_path`) so the next invocation can retrieve it without
/// prompting.
pub fn store_encrypted_credential(
    repo: &SecretRepository,
    provider_id: &str,
    credential_key: &str,
    plaintext_value: &str,
    method: &CredentialEncryptionMethod,
    method_tag: &str,
    ssh_fingerprint: Option<&str>,
    use_keychain: bool,
    secrets_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let encrypted = encrypt_token(plaintext_value, method)?;
    repo.store_credential(
        provider_id,
        credential_key,
        &encrypted,
        method_tag,
        ssh_fingerprint,
    )?;

    // Also cache in the OS keychain for future invocations
    if use_keychain {
        keychain::keychain_store(secrets_path, provider_id, credential_key, plaintext_value);
    }

    Ok(())
}

/// Attempt to retrieve and decrypt a credential for a provider.
///
/// Returns `Ok(None)` if no credential is stored.
/// Returns `Ok(Some(value))` if decryption succeeds.
/// Returns `Err` if stored but all decryption attempts fail.
///
/// Caching behaviour:
/// - Checks the in-process decrypted value cache first.
/// - If `use_keychain` is true, checks the OS keychain next (respecting `cache_ttl`).
/// - For passphrase encryption: uses the cached passphrase if available. If
///   decryption fails with the cached passphrase (wrong passphrase), the cache
///   is invalidated and the user is prompted for a fresh passphrase, with up to
///   [`MAX_PASSPHRASE_ATTEMPTS`] total attempts.
/// - For SSH encryption: uses the cached key path if available.
/// - On success, caches both the passphrase/key path and the decrypted value,
///   and stores in the OS keychain if enabled.
pub fn retrieve_credential(
    repo: &SecretRepository,
    provider_id: &str,
    credential_key: &str,
    use_keychain: bool,
    cache_ttl: u64,
    secrets_path: &Path,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // 1. Check the in-process decrypted value cache
    if let Some(cached) = get_cached_decrypted_value(provider_id, credential_key) {
        return Ok(Some(cached));
    }

    // 2. Check the OS keychain cache (scoped to secrets_path)
    if use_keychain {
        if let Some(cached) =
            keychain::keychain_retrieve(secrets_path, provider_id, credential_key, cache_ttl)
        {
            // Populate the in-process cache so we don't hit the keychain again
            cache_decrypted_value(provider_id, credential_key, &cached);
            return Ok(Some(cached));
        }
    }

    // 3. Look up the stored credential in the DB
    let creds = repo.get_credentials(provider_id)?;
    let cred = match creds.iter().find(|c| c.credential_key == credential_key) {
        Some(c) => c,
        None => return Ok(None),
    };

    // 4. Decrypt with retry logic for passphrase method
    let plaintext = if cred.encryption_method == "passphrase" {
        decrypt_with_passphrase_retry(
            &cred.encrypted_value,
            cred.ssh_pubkey_fingerprint.as_deref(),
        )?
    } else {
        // SSH: single attempt (failures are key-mismatch, retrying won't help)
        let method = build_decryption_method(
            &cred.encryption_method,
            cred.ssh_pubkey_fingerprint.as_deref(),
        )?;
        decrypt_token(&cred.encrypted_value, &method)?
    };

    // 5. Cache the decrypted value for the rest of this session
    cache_decrypted_value(provider_id, credential_key, &plaintext);

    // 6. Store in OS keychain for future invocations (scoped to secrets_path)
    if use_keychain {
        keychain::keychain_store(secrets_path, provider_id, credential_key, &plaintext);
    }

    Ok(Some(plaintext))
}

/// Decrypt ciphertext using passphrase with retry and cache invalidation.
///
/// Flow:
/// 1. If a cached passphrase exists, try it first.
/// 2. If it fails, invalidate the cache and prompt for a fresh passphrase.
/// 3. Allow up to [`MAX_PASSPHRASE_ATTEMPTS`] total attempts.
/// 4. On success, cache the working passphrase.
fn decrypt_with_passphrase_retry(
    ciphertext: &[u8],
    ssh_hint: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut attempts = 0;
    let mut used_cache = false;

    // First attempt: try the cached passphrase if available
    if let Some(cached) = get_cached_passphrase() {
        attempts += 1;
        used_cache = true;
        let method = CredentialDecryptionMethod::Passphrase(cached);
        match decrypt_token(ciphertext, &method) {
            Ok(plaintext) => return Ok(plaintext),
            Err(_) => {
                // Cached passphrase didn't work -- clear it and fall through to prompt
                clear_passphrase_cache();
                eprintln!("  Cached passphrase failed, prompting for a new one...");
            }
        }
    }

    // Subsequent attempts: prompt the user
    let _ = ssh_hint; // unused for passphrase, but keeps the signature consistent
    while attempts < MAX_PASSPHRASE_ATTEMPTS {
        attempts += 1;
        let remaining = MAX_PASSPHRASE_ATTEMPTS - attempts;

        let passphrase = match prompt_passphrase("Enter passphrase to decrypt stored credentials") {
            Ok(p) => p,
            Err(e) => {
                if remaining > 0 {
                    eprintln!(
                        "  Error reading passphrase: {} ({} attempt(s) remaining)",
                        e, remaining
                    );
                    continue;
                }
                return Err(e);
            }
        };

        let method = CredentialDecryptionMethod::Passphrase(passphrase.clone());
        match decrypt_token(ciphertext, &method) {
            Ok(plaintext) => {
                // Success -- cache this passphrase for future use
                cache_passphrase(&passphrase);
                return Ok(plaintext);
            }
            Err(e) => {
                if remaining > 0 {
                    eprintln!(
                        "  Decryption failed: {} ({} attempt(s) remaining)",
                        e, remaining
                    );
                } else {
                    return Err(format!(
                        "Failed to decrypt credential after {} attempts: {}",
                        MAX_PASSPHRASE_ATTEMPTS, e
                    )
                    .into());
                }
            }
        }
    }

    // Should not reach here, but just in case
    Err(format!(
        "Failed to decrypt credential after {} attempts",
        if used_cache {
            "cache + prompt"
        } else {
            "prompt"
        }
    )
    .into())
}

/// Check if any stored credentials exist for a provider.
pub fn has_stored_credentials(repo: &SecretRepository, provider_id: &str) -> bool {
    repo.get_credentials(provider_id)
        .map(|creds| !creds.is_empty())
        .unwrap_or(false)
}
