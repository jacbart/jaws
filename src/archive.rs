//! Archive and encryption functionality for exporting/importing secrets.
//!
//! Uses the `age` crate for encryption (compatible with the age CLI tool)
//! and the `tar` crate for archiving.

use age::secrecy::SecretString;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;
use std::sync::Mutex;

/// Session-scoped cache for the SSH private key passphrase.
///
/// When an SSH key is passphrase-protected, the `age` crate calls
/// `SshKeyCallbacks::request_passphrase()` for every decrypt operation.
/// This cache stores the passphrase after the first successful prompt so
/// subsequent credentials encrypted to the same SSH key can be decrypted
/// without re-prompting.
static SSH_KEY_PASSPHRASE_CACHE: Mutex<Option<SecretString>> = Mutex::new(None);

/// Clear the cached SSH key passphrase.
///
/// Called when decryption fails with a cached passphrase, so the user can
/// be re-prompted on the next attempt.
pub(crate) fn clear_ssh_key_passphrase_cache() {
    if let Ok(mut cache) = SSH_KEY_PASSPHRASE_CACHE.lock() {
        *cache = None;
    }
}

/// Encryption method for export
pub enum EncryptionMethod {
    /// Encrypt with a passphrase
    Passphrase(SecretString),
    /// Encrypt to an SSH public key
    SshPublicKey(std::path::PathBuf),
}

/// Decryption method for import
pub enum DecryptionMethod {
    /// Decrypt with a passphrase
    Passphrase(SecretString),
    /// Decrypt with an SSH private key
    SshPrivateKey(std::path::PathBuf),
}

/// Create an encrypted .barrel archive from the secrets directory
pub fn export_secrets(
    secrets_path: &Path,
    output_path: &Path,
    encryption: EncryptionMethod,
) -> Result<u64, Box<dyn std::error::Error>> {
    // Validate secrets path exists and has contents
    if !secrets_path.exists() {
        return Err(format!("Secrets directory not found: {}", secrets_path.display()).into());
    }

    if !secrets_path.is_dir() {
        return Err(format!("Not a directory: {}", secrets_path.display()).into());
    }

    // Create tar archive in memory
    let tar_data = create_tar_archive(secrets_path)?;

    // Encrypt the tar data
    let encrypted_data = match encryption {
        EncryptionMethod::Passphrase(passphrase) => encrypt_with_passphrase(&tar_data, passphrase)?,
        EncryptionMethod::SshPublicKey(pubkey_path) => {
            encrypt_with_ssh_pubkey(&tar_data, &pubkey_path)?
        }
    };

    // Write to output file
    let mut output_file = File::create(output_path)?;
    output_file.write_all(&encrypted_data)?;

    Ok(encrypted_data.len() as u64)
}

/// Import and decrypt a .barrel archive to the secrets directory
pub fn import_secrets(
    archive_path: &Path,
    secrets_path: &Path,
    decryption: DecryptionMethod,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate archive exists
    if !archive_path.exists() {
        return Err(format!("Archive not found: {}", archive_path.display()).into());
    }

    // Read encrypted archive
    let encrypted_data = fs::read(archive_path)?;

    // Decrypt the data
    let tar_data = match decryption {
        DecryptionMethod::Passphrase(passphrase) => {
            decrypt_with_passphrase(&encrypted_data, passphrase)?
        }
        DecryptionMethod::SshPrivateKey(privkey_path) => {
            decrypt_with_ssh_privkey(&encrypted_data, &privkey_path)?
        }
    };

    // Extract tar archive
    extract_tar_archive(&tar_data, secrets_path)?;

    Ok(())
}

/// Create a tar archive from a directory
fn create_tar_archive(dir_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut archive_data = Vec::new();

    {
        let mut builder = tar::Builder::new(&mut archive_data);

        // Get the directory name to use as the root in the archive
        let dir_name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("secrets");

        // Add all files recursively
        builder.append_dir_all(dir_name, dir_path)?;
        builder.finish()?;
    }

    Ok(archive_data)
}

/// Extract a tar archive to a directory
fn extract_tar_archive(
    tar_data: &[u8],
    dest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let cursor = io::Cursor::new(tar_data);
    let mut archive = tar::Archive::new(cursor);

    // Get the parent directory of the destination
    let parent = dest_path.parent().unwrap_or(Path::new("."));

    // Extract to parent directory (the archive contains the secrets dir name)
    archive.unpack(parent)?;

    Ok(())
}

/// Encrypt data with a passphrase using age
pub(crate) fn encrypt_with_passphrase(
    data: &[u8],
    passphrase: SecretString,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let encryptor = age::Encryptor::with_user_passphrase(passphrase);

    let mut encrypted = Vec::new();
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(data)?;
    writer.finish()?;

    Ok(encrypted)
}

/// Encrypt data with an SSH public key using age
pub(crate) fn encrypt_with_ssh_pubkey(
    data: &[u8],
    key_path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Read the key file
    let key_content =
        fs::read_to_string(key_path).map_err(|e| format!("Failed to read SSH key file: {}", e))?;

    // Check if this looks like a private key instead of a public key
    let pubkey_str = if key_content.contains("PRIVATE KEY") {
        // User provided a private key - try to find the corresponding public key
        let pub_path = find_public_key_path(key_path);

        if let Some(ref pub_path) = pub_path {
            if pub_path.exists() {
                eprintln!(
                    "Note: Using public key {} (you provided the private key)",
                    pub_path.display()
                );
                fs::read_to_string(pub_path)
                    .map_err(|e| format!("Failed to read SSH public key: {}", e))?
            } else {
                return Err(format!(
                    "Expected an SSH public key for encryption, but you provided a private key.\n\
                     Provided: {}\n\
                     Try: {} (file not found)",
                    key_path.display(),
                    pub_path.display()
                )
                .into());
            }
        } else {
            return Err(format!(
                "Expected an SSH public key for encryption, but you provided a private key.\n\
                 Provided: {}\n\
                 Try using the corresponding .pub file",
                key_path.display()
            )
            .into());
        }
    } else {
        key_content
    };

    // Parse the SSH public key as an age recipient
    let recipient: age::ssh::Recipient = pubkey_str
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse SSH public key: {:?}", e))?;

    // Create encryptor with the recipient - use iterator as required by API
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|_| "Failed to create encryptor")?;

    let mut encrypted = Vec::new();
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(data)?;
    writer.finish()?;

    Ok(encrypted)
}

/// Find the public key path for a given private key path
pub(crate) fn find_public_key_path(private_key_path: &Path) -> Option<std::path::PathBuf> {
    // First, try appending .pub to the path
    let pub_path = std::path::PathBuf::from(format!("{}.pub", private_key_path.display()));
    if pub_path.exists() {
        return Some(pub_path);
    }

    // If the path already ends with something like .pem, try replacing the extension
    if let Some(ext) = private_key_path.extension() {
        let ext_str = ext.to_string_lossy();
        if ext_str != "pub" {
            let mut pub_path = private_key_path.to_path_buf();
            pub_path.set_extension("pub");
            if pub_path.exists() {
                return Some(pub_path);
            }
        }
    }

    // Return the most likely path even if it doesn't exist (for error message)
    Some(std::path::PathBuf::from(format!(
        "{}.pub",
        private_key_path.display()
    )))
}

/// Decrypt data with a passphrase using age
pub(crate) fn decrypt_with_passphrase(
    encrypted_data: &[u8],
    passphrase: SecretString,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let decryptor = age::Decryptor::new(encrypted_data)?;

    let identity = age::scrypt::Identity::new(passphrase);

    let mut decrypted = Vec::new();
    let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))?;
    reader.read_to_end(&mut decrypted)?;

    Ok(decrypted)
}

/// Decrypt data with an SSH private key using age
pub(crate) fn decrypt_with_ssh_privkey(
    encrypted_data: &[u8],
    privkey_path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Read the private key file
    let privkey_data =
        fs::read(privkey_path).map_err(|e| format!("Failed to read SSH private key: {}", e))?;

    // Parse the SSH private key
    let identity = age::ssh::Identity::from_buffer(
        BufReader::new(&privkey_data[..]),
        Some(privkey_path.to_string_lossy().to_string()),
    )
    .map_err(|e| format!("Failed to parse SSH private key: {}", e))?;

    // Handle encrypted vs unencrypted keys using callbacks
    let identity = match identity {
        age::ssh::Identity::Unencrypted(_) | age::ssh::Identity::Encrypted(_) => {
            // Use with_callbacks for encrypted key support
            identity.with_callbacks(SshKeyCallbacks {
                key_path: privkey_path.to_path_buf(),
            })
        }
        age::ssh::Identity::Unsupported(key) => {
            return Err(format!("Unsupported SSH key type: {:?}", key).into());
        }
    };

    // Decrypt
    let decryptor = age::Decryptor::new(encrypted_data)?;

    let mut decrypted = Vec::new();
    let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))?;
    reader.read_to_end(&mut decrypted)?;

    Ok(decrypted)
}

/// Callbacks for SSH key passphrase prompting
#[derive(Clone)]
struct SshKeyCallbacks {
    key_path: std::path::PathBuf,
}

impl age::Callbacks for SshKeyCallbacks {
    fn display_message(&self, message: &str) {
        eprintln!("{}", message);
    }

    fn confirm(&self, _message: &str, _yes_string: &str, _no_string: Option<&str>) -> Option<bool> {
        Some(true)
    }

    fn request_public_string(&self, description: &str) -> Option<String> {
        eprint!("{}: ", description);
        io::stderr().flush().ok()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok()?;
        Some(input.trim().to_string())
    }

    fn request_passphrase(&self, description: &str) -> Option<SecretString> {
        // Return the cached passphrase if available (avoids re-prompting
        // when multiple credentials are encrypted to the same SSH key).
        if let Ok(cache) = SSH_KEY_PASSPHRASE_CACHE.lock() {
            if let Some(cached) = cache.as_ref() {
                return Some(cached.clone());
            }
        }

        let prompt = if description.is_empty() {
            format!("Enter passphrase for SSH key {}", self.key_path.display())
        } else {
            description.to_string()
        };
        eprint!("{}: ", prompt);
        io::stderr().flush().ok()?;
        let passphrase = rpassword::read_password().ok()?;
        let secret = SecretString::from(passphrase);

        // Cache for subsequent decrypt calls in this session
        if let Ok(mut cache) = SSH_KEY_PASSPHRASE_CACHE.lock() {
            *cache = Some(secret.clone());
        }

        Some(secret)
    }
}

/// Prompt for a passphrase with hidden input
pub fn prompt_passphrase(prompt: &str) -> Result<SecretString, Box<dyn std::error::Error>> {
    eprint!("{}: ", prompt);
    io::stderr().flush()?;

    let passphrase = rpassword::read_password()?;

    if passphrase.is_empty() {
        return Err("Passphrase cannot be empty".into());
    }

    Ok(SecretString::from(passphrase))
}

/// Prompt for a passphrase with confirmation (for encryption)
pub fn prompt_passphrase_with_confirm(
    prompt: &str,
) -> Result<SecretString, Box<dyn std::error::Error>> {
    eprint!("{}: ", prompt);
    io::stderr().flush()?;
    let passphrase1 = rpassword::read_password()?;

    if passphrase1.is_empty() {
        return Err("Passphrase cannot be empty".into());
    }

    eprint!("Confirm passphrase: ");
    io::stderr().flush()?;
    let passphrase2 = rpassword::read_password()?;

    if passphrase1 != passphrase2 {
        return Err("Passphrases do not match".into());
    }

    Ok(SecretString::from(passphrase1))
}

/// Format a byte size in human-readable format
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passphrase_encryption_roundtrip() {
        let data = b"test secret data";
        let passphrase = SecretString::from("test-passphrase".to_string());

        let encrypted = encrypt_with_passphrase(data, passphrase.clone()).unwrap();
        assert_ne!(encrypted, data);

        let decrypted = decrypt_with_passphrase(&encrypted, passphrase).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1572864), "1.5 MB");
    }
}
