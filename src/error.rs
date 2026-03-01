//! Unified error type for jaws.
//!
//! All public APIs return `Result<T, JawsError>`. The error type provides
//! specific variants for common failure categories while remaining easy to
//! construct from string messages for application-level validation errors.

use std::fmt;

/// The unified error type for all jaws operations.
#[derive(Debug)]
pub enum JawsError {
    // ── I/O & filesystem ───────────────────────────────────────────────
    /// Filesystem or I/O operation failed.
    Io(std::io::Error),

    // ── Database ───────────────────────────────────────────────────────
    /// SQLite / database operation failed.
    Db(rusqlite::Error),

    // ── Serialization ──────────────────────────────────────────────────
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),

    /// KDL config file parsing error.
    Config(String),

    // ── Provider-specific ──────────────────────────────────────────────
    /// An error originating from a secrets provider (AWS, 1Password, Bitwarden, etc.).
    Provider { provider: String, message: String },

    // ── Encryption / credentials ───────────────────────────────────────
    /// Encryption or decryption failed.
    Encryption(String),

    // ── Lookup errors ──────────────────────────────────────────────────
    /// A requested secret, version, or resource was not found.
    NotFound(String),

    /// Feature is not supported by this provider.
    Unsupported(String),

    // ── User interaction ───────────────────────────────────────────────
    /// The user cancelled an interactive operation.
    Cancelled,

    /// User input validation failed.
    Validation(String),

    // ── Catch-all ──────────────────────────────────────────────────────
    /// Any other error. Allows easy migration from string-based errors.
    Other(String),
}

// ── Display ────────────────────────────────────────────────────────────

impl fmt::Display for JawsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JawsError::Io(e) => write!(f, "{}", e),
            JawsError::Db(e) => write!(f, "database error: {}", e),
            JawsError::Json(e) => write!(f, "JSON error: {}", e),
            JawsError::Config(msg) => write!(f, "config error: {}", msg),
            JawsError::Provider { provider, message } => {
                write!(f, "provider '{}': {}", provider, message)
            }
            JawsError::Encryption(msg) => write!(f, "encryption error: {}", msg),
            JawsError::NotFound(msg) => write!(f, "{}", msg),
            JawsError::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            JawsError::Cancelled => write!(f, "cancelled"),
            JawsError::Validation(msg) => write!(f, "{}", msg),
            JawsError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for JawsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            JawsError::Io(e) => Some(e),
            JawsError::Db(e) => Some(e),
            JawsError::Json(e) => Some(e),
            _ => None,
        }
    }
}

// ── From implementations for common error types ────────────────────────

impl From<std::io::Error> for JawsError {
    fn from(e: std::io::Error) -> Self {
        JawsError::Io(e)
    }
}

impl From<rusqlite::Error> for JawsError {
    fn from(e: rusqlite::Error) -> Self {
        JawsError::Db(e)
    }
}

impl From<serde_json::Error> for JawsError {
    fn from(e: serde_json::Error) -> Self {
        JawsError::Json(e)
    }
}

impl From<std::num::ParseIntError> for JawsError {
    fn from(e: std::num::ParseIntError) -> Self {
        JawsError::Other(e.to_string())
    }
}

// Allow easy conversion from string-based errors (the most common pattern).
impl From<String> for JawsError {
    fn from(s: String) -> Self {
        JawsError::Other(s)
    }
}

impl From<&str> for JawsError {
    fn from(s: &str) -> Self {
        JawsError::Other(s.to_string())
    }
}

// Allow conversion from `Box<dyn Error>` for interop with libraries that
// return boxed errors (ff, AWS SDK, etc.).
impl From<Box<dyn std::error::Error>> for JawsError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        JawsError::Other(e.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send>> for JawsError {
    fn from(e: Box<dyn std::error::Error + Send>) -> Self {
        JawsError::Other(e.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for JawsError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        JawsError::Other(e.to_string())
    }
}

// ── Convenience constructors ───────────────────────────────────────────

impl JawsError {
    /// Create a provider-specific error.
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        JawsError::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Create an encryption/decryption error.
    pub fn encryption(message: impl Into<String>) -> Self {
        JawsError::Encryption(message.into())
    }

    /// Create a not-found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        JawsError::NotFound(message.into())
    }

    /// Create an unsupported-operation error.
    pub fn unsupported(message: impl Into<String>) -> Self {
        JawsError::Unsupported(message.into())
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        JawsError::Validation(message.into())
    }

    /// Create a config error.
    pub fn config(message: impl Into<String>) -> Self {
        JawsError::Config(message.into())
    }

    /// Create a provider error for AWS, translating common Secrets Manager
    /// errors into user-friendly messages.
    pub fn aws(e: impl std::fmt::Display) -> Self {
        let msg = e.to_string();

        let friendly = if msg.contains("ResourceNotFoundException") {
            "Secret not found (may have been deleted)".to_string()
        } else if msg.contains("AccessDeniedException") {
            "Access denied (check IAM permissions)".to_string()
        } else if msg.contains("InvalidParameterException") {
            "Invalid parameter".to_string()
        } else if msg.contains("InvalidRequestException") {
            "Invalid request".to_string()
        } else if msg.contains("DecryptionFailure") {
            "Decryption failed (KMS key issue)".to_string()
        } else if msg.contains("InternalServiceError") {
            "AWS internal error (try again later)".to_string()
        } else {
            msg
        };

        JawsError::Provider {
            provider: "aws".to_string(),
            message: friendly,
        }
    }

    /// Create a provider error for GCP, translating common Secret Manager
    /// errors into user-friendly messages.
    pub fn gcp(e: impl std::fmt::Display) -> Self {
        let msg = e.to_string();

        let friendly = if msg.contains("NOT_FOUND") || msg.contains("notFound") {
            "Secret not found".to_string()
        } else if msg.contains("PERMISSION_DENIED") || msg.contains("permissionDenied") {
            "Permission denied (check IAM roles for Secret Manager)".to_string()
        } else if msg.contains("ALREADY_EXISTS") || msg.contains("alreadyExists") {
            "Secret already exists".to_string()
        } else if msg.contains("UNAUTHENTICATED") || msg.contains("unauthenticated") {
            "Not authenticated (run 'gcloud auth application-default login')".to_string()
        } else if msg.contains("INVALID_ARGUMENT") || msg.contains("invalidArgument") {
            "Invalid argument".to_string()
        } else if msg.contains("RESOURCE_EXHAUSTED") || msg.contains("resourceExhausted") {
            "Resource exhausted (quota limit reached)".to_string()
        } else if msg.contains("FAILED_PRECONDITION") || msg.contains("failedPrecondition") {
            "Failed precondition (secret may be in an invalid state)".to_string()
        } else if msg.contains("UNAVAILABLE") {
            "GCP service unavailable (try again later)".to_string()
        } else {
            msg
        };

        JawsError::Provider {
            provider: "gcp".to_string(),
            message: friendly,
        }
    }
}

/// Convenience type alias for Results using JawsError.
pub type Result<T> = std::result::Result<T, JawsError>;
