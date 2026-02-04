//! Direct FFI bindings to 1Password SDK Core library for listing operations.
//!
//! The 1Password SDK uses UniFFI for its FFI layer. This module implements
//! the correct UniFFI calling conventions to access vault and item listing
//! functionality.

use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// UniFFI Types
// ============================================================================

/// UniFFI RustBuffer - used for passing data across FFI boundary
#[repr(C)]
#[derive(Debug)]
struct RustBuffer {
    capacity: i32,
    len: i32,
    data: *mut u8,
}

/// UniFFI ForeignBytes - used to pass byte data to the SDK for buffer creation
#[repr(C)]
#[derive(Debug)]
struct ForeignBytes {
    len: i32,
    data: *const u8,
}

/// Function type for allocating RustBuffer from ForeignBytes
type RustBufferFromBytesFn = unsafe extern "C" fn(ForeignBytes, *mut RustCallStatus) -> RustBuffer;

impl RustBuffer {
    /// Create an empty RustBuffer
    fn empty() -> Self {
        Self {
            capacity: 0,
            len: 0,
            data: std::ptr::null_mut(),
        }
    }

    /// Extract string from RustBuffer
    /// UniFFI RustBuffer contains raw UTF-8 bytes (no length prefix)
    fn to_string(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.data.is_null() || self.len == 0 {
            return Ok(String::new());
        }

        let slice = unsafe { std::slice::from_raw_parts(self.data, self.len as usize) };
        Ok(String::from_utf8_lossy(slice).to_string())
    }

    /// Read raw bytes from RustBuffer (no length prefix)
    fn to_bytes(&self) -> Vec<u8> {
        if self.data.is_null() || self.len == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(self.data, self.len as usize).to_vec() }
    }
}

/// Helper to create a RustBuffer from a string using the SDK's allocator
fn create_rustbuffer_from_string(
    s: &str,
    from_bytes_fn: RustBufferFromBytesFn,
) -> Result<RustBuffer, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = s.as_bytes();

    let foreign_bytes = ForeignBytes {
        len: bytes.len() as i32,
        data: bytes.as_ptr(),
    };

    let mut status = RustCallStatus::new();
    let buf = unsafe { from_bytes_fn(foreign_bytes, &mut status) };

    if !status.is_success() {
        if let Some(err) = status.get_error() {
            return Err(format!("Failed to allocate RustBuffer: {}", err).into());
        }
        return Err("Failed to allocate RustBuffer".into());
    }

    Ok(buf)
}

/// UniFFI RustCallStatus - used for error reporting
#[repr(C)]
#[derive(Debug)]
struct RustCallStatus {
    code: i8,
    error_buf: RustBuffer,
}

impl RustCallStatus {
    fn new() -> Self {
        Self {
            code: 0,
            error_buf: RustBuffer::empty(),
        }
    }

    fn is_success(&self) -> bool {
        self.code == 0
    }

    fn get_error(&self) -> Option<String> {
        if self.code == 0 {
            return None;
        }

        // Error buffer contains the error message
        if self.error_buf.data.is_null() || self.error_buf.len == 0 {
            return Some(match self.code {
                1 => "SDK error (no details)".to_string(),
                2 => "SDK panic (no details)".to_string(),
                _ => format!("Unknown error code: {}", self.code),
            });
        }

        // Try to extract error message - it's a serialized Error enum
        // Format: i32 variant (big-endian) + i32 string length (big-endian) + UTF-8 bytes
        let bytes = self.error_buf.to_bytes();
        if bytes.len() >= 8 {
            // Skip variant discriminant (4 bytes), then read string length
            let str_len = i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
            if bytes.len() >= 8 + str_len {
                let msg = String::from_utf8_lossy(&bytes[8..8 + str_len]).to_string();
                return Some(msg);
            }
        }

        // Fallback: try to read the whole buffer as a string (might work for panics)
        if let Ok(msg) = self.error_buf.to_string()
            && !msg.is_empty()
        {
            return Some(msg);
        }

        Some(format!(
            "Error code {}: unable to parse details (buf len: {})",
            self.code, self.error_buf.len
        ))
    }
}

// ============================================================================
// 1Password Data Types
// ============================================================================

/// Vault overview returned by VaultsList
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultOverview {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Item category enum
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ItemCategory {
    Login,
    SecureNote,
    CreditCard,
    CryptoWallet,
    Identity,
    Password,
    Document,
    ApiCredentials,
    BankAccount,
    Database,
    DriverLicense,
    Email,
    MedicalRecord,
    Membership,
    OutdoorLicense,
    Passport,
    Rewards,
    Router,
    Server,
    SshKey,
    SocialSecurityNumber,
    SoftwareLicense,
    Person,
    Unsupported,
}

/// Item state enum
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemState {
    Active,
    Archived,
}

/// Website info for items
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Website {
    pub url: String,
    pub label: String,
    pub autofill_behavior: String,
}

/// Item overview returned by ItemsList (without full field data)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemOverview {
    pub id: String,
    pub title: String,
    pub category: ItemCategory,
    pub vault_id: String,
    pub websites: Vec<Website>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub state: ItemState,
}

/// Field type enum
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ItemFieldType {
    Text,
    Concealed,
    CreditCardType,
    CreditCardNumber,
    Phone,
    Url,
    Totp,
    Email,
    Reference,
    SshKey,
    Menu,
    MonthYear,
    Address,
    Date,
    Unsupported,
}

/// Item field
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemField {
    pub id: String,
    pub title: String,
    pub section_id: Option<String>,
    pub field_type: ItemFieldType,
    pub value: String,
}

/// Item section
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ItemSection {
    pub id: String,
    pub title: String,
}

/// File attributes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileAttributes {
    pub name: String,
    pub id: String,
    pub size: i64,
}

/// Item file
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFile {
    pub attributes: FileAttributes,
    pub section_id: String,
    pub field_id: String,
}

/// Full item returned by ItemsGet
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub title: String,
    pub category: ItemCategory,
    pub vault_id: String,
    pub fields: Vec<ItemField>,
    pub sections: Vec<ItemSection>,
    pub notes: String,
    pub tags: Vec<String>,
    pub websites: Vec<Website>,
    pub version: i32,
    pub files: Vec<ItemFile>,
    pub document: Option<FileAttributes>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// SDK Invocation Types
// ============================================================================

/// Client configuration for initialization
/// Must match the format expected by the 1Password SDK
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientConfig {
    service_account_token: String,
    programming_language: String,
    sdk_version: String,
    integration_name: String,
    integration_version: String,
    request_library_name: String,
    request_library_version: String,
    os: String,
    os_version: String,
    architecture: String,
}

/// Invocation wrapper
#[derive(Serialize)]
struct Invocation {
    invocation: InvocationInner,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InvocationInner {
    client_id: u64,
    parameters: InvocationParameters,
}

#[derive(Serialize)]
struct InvocationParameters {
    name: String,
    parameters: serde_json::Value,
}

// ============================================================================
// FFI Library Interface
// ============================================================================

// Response from init_client - it's just the client ID as a string number, not JSON
// The error case is handled by the RustCallStatus
type InitClientFn = unsafe extern "C" fn(RustBuffer) -> *mut std::ffi::c_void;
type InvokeSyncFn = unsafe extern "C" fn(RustBuffer, *mut RustCallStatus) -> RustBuffer;
type ReleaseClientFn = unsafe extern "C" fn(RustBuffer, *mut RustCallStatus);
type RustBufferFreeFn = unsafe extern "C" fn(RustBuffer, *mut RustCallStatus);
type FuturePollFn = unsafe extern "C" fn(*mut std::ffi::c_void, extern "C" fn(usize, i8), usize);
type FutureCompleteFn =
    unsafe extern "C" fn(*mut std::ffi::c_void, *mut RustCallStatus) -> RustBuffer;
type FutureFreeFn = unsafe extern "C" fn(*mut std::ffi::c_void);

/// 1Password SDK client with direct FFI access
pub struct OnePasswordSdkClient {
    client_id: u64,
    _library: Arc<libloading::Library>,
    invoke_sync_fn: InvokeSyncFn,
    rustbuffer_free_fn: RustBufferFreeFn,
    rustbuffer_from_bytes_fn: RustBufferFromBytesFn,
}

// SAFETY: The SDK client handles synchronization internally
unsafe impl Send for OnePasswordSdkClient {}
unsafe impl Sync for OnePasswordSdkClient {}

impl OnePasswordSdkClient {
    /// Create a new SDK client from environment token
    pub async fn from_env(
        token_env: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let token = env::var(token_env)
            .map_err(|_| {
                format!(
                    "Environment variable '{}' not set. Please set it to your 1Password Service Account token.",
                    token_env
                )
            })?
            .trim()
            .to_string();

        if token.is_empty() {
            return Err(format!(
                "Environment variable '{}' is empty. Please set it to your 1Password Service Account token.",
                token_env
            ).into());
        }

        Self::new(&token, "jaws", env!("CARGO_PKG_VERSION")).await
    }

    /// Create a new SDK client with explicit token
    pub async fn new(
        token: &str,
        integration_name: &str,
        integration_version: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Load the native library
        let library = Self::load_library()?;
        let library = Arc::new(library);

        // Get function pointers - using correct UniFFI names
        let init_client_fn: libloading::Symbol<InitClientFn> =
            unsafe { library.get(b"uniffi_op_uniffi_core_fn_func_init_client\0")? };

        let invoke_sync_fn: libloading::Symbol<InvokeSyncFn> =
            unsafe { library.get(b"uniffi_op_uniffi_core_fn_func_invoke_sync\0")? };

        let rustbuffer_free_fn: libloading::Symbol<RustBufferFreeFn> =
            unsafe { library.get(b"ffi_op_uniffi_core_rustbuffer_free\0")? };

        let rustbuffer_from_bytes_fn: libloading::Symbol<RustBufferFromBytesFn> =
            unsafe { library.get(b"ffi_op_uniffi_core_rustbuffer_from_bytes\0")? };

        // For async init, we need the future polling functions
        let future_poll_fn: libloading::Symbol<FuturePollFn> =
            unsafe { library.get(b"ffi_op_uniffi_core_rust_future_poll_rust_buffer\0")? };

        let future_complete_fn: libloading::Symbol<FutureCompleteFn> =
            unsafe { library.get(b"ffi_op_uniffi_core_rust_future_complete_rust_buffer\0")? };

        let future_free_fn: libloading::Symbol<FutureFreeFn> =
            unsafe { library.get(b"ffi_op_uniffi_core_rust_future_free_rust_buffer\0")? };

        // Initialize client - this is an async function that returns a future
        // SDK version format: 7 digits like "0030201" (major * 10000 + minor * 100 + patch)
        // We match the Python SDK version 0.3.2 = 0030201
        let config = ClientConfig {
            service_account_token: token.trim().to_string(),
            programming_language: "Rust".to_string(),
            sdk_version: "0030201".to_string(), // Match SDK version format
            integration_name: integration_name.trim().to_string(),
            integration_version: integration_version.trim().to_string(),
            request_library_name: "reqwest".to_string(),
            request_library_version: "0.12".to_string(),
            os: std::env::consts::OS.to_string(),
            os_version: "0.0.0".to_string(),
            architecture: std::env::consts::ARCH.to_string(),
        };

        let config_json = serde_json::to_string(&config)?;
        let config_buf = create_rustbuffer_from_string(&config_json, *rustbuffer_from_bytes_fn)?;

        // Call init_client - returns a future handle
        let future_handle = unsafe { init_client_fn(config_buf) };
        if future_handle.is_null() {
            return Err("Failed to initialize 1Password client: null future".into());
        }

        // Poll the future to completion using a simple blocking approach
        let result_buf = Self::poll_future_blocking(
            future_handle,
            *future_poll_fn,
            *future_complete_fn,
            *future_free_fn,
        )?;

        // Parse the result - it's just the client ID as a string number
        let result_str = result_buf.to_string()?;

        // Free the result buffer
        let mut status = RustCallStatus::new();
        unsafe { rustbuffer_free_fn(result_buf, &mut status) };

        // Parse client ID - it's returned as a plain integer string
        let client_id: u64 = result_str.trim().parse().map_err(|e| {
            format!(
                "Failed to parse client ID: {}. Raw response: '{}'",
                e, result_str
            )
        })?;

        // Store function pointers with 'static lifetime
        // SAFETY: Library is kept alive in Arc, so pointers remain valid
        let invoke_sync_fn: InvokeSyncFn = *invoke_sync_fn;
        let rustbuffer_free_fn: RustBufferFreeFn = *rustbuffer_free_fn;
        let rustbuffer_from_bytes_fn: RustBufferFromBytesFn = *rustbuffer_from_bytes_fn;

        Ok(Self {
            client_id,
            _library: library,
            invoke_sync_fn,
            rustbuffer_free_fn,
            rustbuffer_from_bytes_fn,
        })
    }

    /// Poll a UniFFI async future to completion (blocking)
    fn poll_future_blocking(
        future_handle: *mut std::ffi::c_void,
        poll_fn: FuturePollFn,
        complete_fn: FutureCompleteFn,
        free_fn: FutureFreeFn,
    ) -> Result<RustBuffer, Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicI8, Ordering};

        // Shared state for the callback
        let poll_result = StdArc::new(AtomicI8::new(-1));
        let poll_result_clone = StdArc::clone(&poll_result);

        // Store the Arc pointer to pass through the callback
        let poll_result_ptr = StdArc::into_raw(poll_result_clone) as usize;

        // Callback function that will be called when the future is ready
        extern "C" fn continuation_callback(data: usize, poll_code: i8) {
            let poll_result = unsafe { StdArc::from_raw(data as *const AtomicI8) };
            poll_result.store(poll_code, Ordering::SeqCst);
            // Don't drop - we'll retrieve it later
            let _ = StdArc::into_raw(poll_result);
        }

        // Poll loop
        loop {
            // Reset poll result
            poll_result.store(-1, Ordering::SeqCst);

            // Poll the future
            unsafe { poll_fn(future_handle, continuation_callback, poll_result_ptr) };

            // Wait for callback (simple spin wait - in practice this is nearly instant)
            let mut attempts = 0;
            while poll_result.load(Ordering::SeqCst) == -1 {
                std::thread::sleep(std::time::Duration::from_micros(100));
                attempts += 1;
                if attempts > 100000 {
                    // 10 second timeout
                    unsafe { free_fn(future_handle) };
                    // Clean up the Arc
                    unsafe { StdArc::from_raw(poll_result_ptr as *const AtomicI8) };
                    return Err("Timeout waiting for 1Password SDK response".into());
                }
            }

            let code = poll_result.load(Ordering::SeqCst);

            // 0 = READY, 1 = MAYBE_READY (poll again)
            if code == 0 {
                break;
            }
        }

        // Clean up the Arc we passed to the callback
        unsafe { StdArc::from_raw(poll_result_ptr as *const AtomicI8) };

        // Complete the future and get the result
        let mut status = RustCallStatus::new();
        let result = unsafe { complete_fn(future_handle, &mut status) };

        // Free the future
        unsafe { free_fn(future_handle) };

        if !status.is_success() {
            if let Some(err) = status.get_error() {
                return Err(format!("1Password SDK error: {}", err).into());
            }
            return Err("1Password SDK error (unknown)".into());
        }

        Ok(result)
    }

    fn load_library() -> Result<libloading::Library, Box<dyn std::error::Error + Send + Sync>> {
        // Check environment variable first (set by nix wrapper)
        if let Ok(path) = env::var("ONEPASSWORD_LIB_PATH") {
            // The path might point to the directory or directly to the file
            let paths_to_try = if path.ends_with(".so") || path.ends_with(".dylib") {
                vec![path.clone()]
            } else {
                let lib_name = if cfg!(target_os = "macos") {
                    "libop_uniffi_core.dylib"
                } else {
                    "libop_uniffi_core.so"
                };
                vec![format!("{}/{}", path, lib_name), path.clone()]
            };

            for p in &paths_to_try {
                if let Ok(lib) = unsafe { libloading::Library::new(p) } {
                    return Ok(lib);
                }
            }
        }

        // Try the target directory (where corteq-onepassword puts it)
        let lib_name = if cfg!(target_os = "macos") {
            "libop_uniffi_core.dylib"
        } else {
            "libop_uniffi_core.so"
        };

        let target_paths = [
            format!("target/release/{}", lib_name),
            format!("target/debug/{}", lib_name),
            format!("target/release/deps/{}", lib_name),
            format!("target/debug/deps/{}", lib_name),
        ];

        for path in &target_paths {
            if let Ok(lib) = unsafe { libloading::Library::new(path) } {
                return Ok(lib);
            }
        }

        // Try system paths
        if let Ok(lib) = unsafe { libloading::Library::new(lib_name) } {
            return Ok(lib);
        }

        Err(format!(
            "Could not find {}. Set ONEPASSWORD_LIB_PATH environment variable.",
            lib_name
        )
        .into())
    }

    /// Invoke an SDK operation synchronously
    fn invoke(
        &self,
        invocation: &Invocation,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string(invocation)?;
        let buf = create_rustbuffer_from_string(&json, self.rustbuffer_from_bytes_fn)?;

        let mut status = RustCallStatus::new();
        let result_buf = unsafe { (self.invoke_sync_fn)(buf, &mut status) };

        if !status.is_success() {
            if let Some(err) = status.get_error() {
                return Err(format!("1Password SDK error: {}", err).into());
            }
            return Err("1Password SDK error (unknown)".into());
        }

        let result_str = result_buf.to_string()?;

        // Free the result buffer
        let mut free_status = RustCallStatus::new();
        unsafe { (self.rustbuffer_free_fn)(result_buf, &mut free_status) };

        Ok(result_str)
    }

    /// List all vaults accessible to the service account
    pub fn list_vaults(
        &self,
    ) -> Result<Vec<VaultOverview>, Box<dyn std::error::Error + Send + Sync>> {
        let invocation = Invocation {
            invocation: InvocationInner {
                client_id: self.client_id,
                parameters: InvocationParameters {
                    name: "VaultsList".to_string(),
                    parameters: serde_json::json!({}),
                },
            },
        };

        let response = self.invoke(&invocation)?;
        let vaults: Vec<VaultOverview> = serde_json::from_str(&response)?;
        Ok(vaults)
    }

    /// Find a vault by name or ID
    pub fn find_vault(
        &self,
        name_or_id: &str,
    ) -> Result<VaultOverview, Box<dyn std::error::Error + Send + Sync>> {
        let vaults = self.list_vaults()?;

        // First try exact ID match
        if let Some(vault) = vaults.iter().find(|v| v.id == name_or_id) {
            return Ok(vault.clone());
        }

        // Then try title match (case-insensitive)
        if let Some(vault) = vaults
            .iter()
            .find(|v| v.title.to_lowercase() == name_or_id.to_lowercase())
        {
            return Ok(vault.clone());
        }

        Err(format!(
            "Vault '{}' not found. Available vaults: {}",
            name_or_id,
            vaults
                .iter()
                .map(|v| format!("{} ({})", v.title, v.id))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .into())
    }

    /// List all items in a vault
    pub fn list_items(
        &self,
        vault_id: &str,
    ) -> Result<Vec<ItemOverview>, Box<dyn std::error::Error + Send + Sync>> {
        let invocation = Invocation {
            invocation: InvocationInner {
                client_id: self.client_id,
                parameters: InvocationParameters {
                    name: "ItemsList".to_string(),
                    parameters: serde_json::json!({
                        "vault_id": vault_id,
                        "filters": []
                    }),
                },
            },
        };

        let response = self.invoke(&invocation)?;
        let items: Vec<ItemOverview> = serde_json::from_str(&response)?;
        Ok(items)
    }

    /// Get full item details including all fields
    pub fn get_item(
        &self,
        vault_id: &str,
        item_id: &str,
    ) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        let invocation = Invocation {
            invocation: InvocationInner {
                client_id: self.client_id,
                parameters: InvocationParameters {
                    name: "ItemsGet".to_string(),
                    parameters: serde_json::json!({
                        "vault_id": vault_id,
                        "item_id": item_id
                    }),
                },
            },
        };

        let response = self.invoke(&invocation)?;
        let item: Item = serde_json::from_str(&response)?;
        Ok(item)
    }

    /// Resolve a secret reference (op://vault/item/field)
    pub fn resolve_secret(
        &self,
        reference: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let invocation = Invocation {
            invocation: InvocationInner {
                client_id: self.client_id,
                parameters: InvocationParameters {
                    name: "SecretsResolve".to_string(),
                    parameters: serde_json::json!({
                        "secret_reference": reference
                    }),
                },
            },
        };

        let response = self.invoke(&invocation)?;

        // The response is the secret value directly as a string
        // But it might be wrapped - try to parse as JSON first
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response) {
            // Case 1: It's a JSON object with a "secret" field
            if let Some(secret) = value.get("secret").and_then(|s| s.as_str()) {
                return Ok(secret.to_string());
            }

            // Case 2: It's just a JSON string (the SDK returns "value" with quotes)
            if let Some(secret_str) = value.as_str() {
                return Ok(secret_str.to_string());
            }
        }

        // Otherwise return the raw response (it's likely just the secret value)
        Ok(response)
    }

    /// Create a new item
    pub fn create_item(
        &self,
        item: &Item,
    ) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        let invocation = Invocation {
            invocation: InvocationInner {
                client_id: self.client_id,
                parameters: InvocationParameters {
                    name: "ItemsCreate".to_string(),
                    parameters: serde_json::json!({
                        "item": item
                    }),
                },
            },
        };

        let response = self.invoke(&invocation)?;
        let created_item: Item = serde_json::from_str(&response)?;
        Ok(created_item)
    }

    /// Update an existing item
    pub fn put_item(&self, item: &Item) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        let invocation = Invocation {
            invocation: InvocationInner {
                client_id: self.client_id,
                parameters: InvocationParameters {
                    name: "ItemsPut".to_string(),
                    parameters: serde_json::json!({
                        "item": item
                    }),
                },
            },
        };

        let response = self.invoke(&invocation)?;
        let updated_item: Item = serde_json::from_str(&response)?;
        Ok(updated_item)
    }

    /// Delete an item
    pub fn delete_item(
        &self,
        vault_id: &str,
        item_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let invocation = Invocation {
            invocation: InvocationInner {
                client_id: self.client_id,
                parameters: InvocationParameters {
                    name: "ItemsDelete".to_string(),
                    parameters: serde_json::json!({
                        "vault_id": vault_id,
                        "item_id": item_id
                    }),
                },
            },
        };

        self.invoke(&invocation)?;
        Ok(())
    }

    /// Archive an item
    pub fn archive_item(
        &self,
        vault_id: &str,
        item_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let invocation = Invocation {
            invocation: InvocationInner {
                client_id: self.client_id,
                parameters: InvocationParameters {
                    name: "ItemsArchive".to_string(),
                    parameters: serde_json::json!({
                        "vault_id": vault_id,
                        "item_id": item_id
                    }),
                },
            },
        };

        self.invoke(&invocation)?;
        Ok(())
    }
}

// ============================================================================
// Thread-safe Wrapper
// ============================================================================

/// Thread-safe wrapper for the SDK client
pub struct SharedSdkClient {
    inner: Arc<Mutex<OnePasswordSdkClient>>,
}

impl SharedSdkClient {
    pub fn new(client: OnePasswordSdkClient) -> Self {
        Self {
            inner: Arc::new(Mutex::new(client)),
        }
    }

    pub async fn list_vaults(
        &self,
    ) -> Result<Vec<VaultOverview>, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.list_vaults()
    }

    /// Synchronous version of list_vaults for use in non-async contexts
    pub fn list_vaults_sync(
        &self,
    ) -> Result<Vec<VaultOverview>, Box<dyn std::error::Error + Send + Sync>> {
        // Use try_lock to avoid blocking if possible
        match self.inner.try_lock() {
            Ok(client) => client.list_vaults(),
            Err(_) => Err("Could not acquire lock on 1Password client".into()),
        }
    }

    pub async fn find_vault(
        &self,
        name_or_id: &str,
    ) -> Result<VaultOverview, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.find_vault(name_or_id)
    }

    pub async fn list_items(
        &self,
        vault_id: &str,
    ) -> Result<Vec<ItemOverview>, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.list_items(vault_id)
    }

    pub async fn get_item(
        &self,
        vault_id: &str,
        item_id: &str,
    ) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.get_item(vault_id, item_id)
    }

    pub async fn resolve_secret(
        &self,
        reference: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.resolve_secret(reference)
    }

    pub async fn create_item(
        &self,
        item: &Item,
    ) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.create_item(item)
    }

    pub async fn put_item(
        &self,
        item: &Item,
    ) -> Result<Item, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.put_item(item)
    }

    pub async fn delete_item(
        &self,
        vault_id: &str,
        item_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.delete_item(vault_id, item_id)
    }

    pub async fn archive_item(
        &self,
        vault_id: &str,
        item_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = self.inner.lock().await;
        client.archive_item(vault_id, item_id)
    }
}

impl Clone for SharedSdkClient {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
