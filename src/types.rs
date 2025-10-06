//! Common types for rustls-acme
//!
//! This module re-exports rustls types and defines ACME-specific types.

use std::sync::Arc;
use std::time::SystemTime;

// Re-export rustls types for convenience
pub use rustls::server::ResolvesServerCert;
pub use rustls::sign::{CertifiedKey, CertifiedSigner};
pub use rustls::server::ClientHello;

// Type alias for convenience
pub type CertifiedKeyType = CertifiedKey;

/// Error types for ACME operations
#[derive(Debug, thiserror::Error)]
pub enum AcmeError {
    #[error("ACME client error: {0}")]
    Client(String),
    
    #[error("DNS validation error: {0}")]
    Dns(String),
    
    #[error("Certificate error: {0}")]
    Certificate(#[from] Box<dyn std::error::Error + Send + Sync>),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Certificate not found: {0}")]
    CertificateNotFound(String),
    
    #[error("Unsupported challenge type: {0}")]
    UnsupportedChallenge(String),
    
    #[error("ACME library error: {0}")]
    AcmeLib(#[from] acme_lib::Error),
}

/// Challenge types supported by ACME
#[derive(Debug, Clone, PartialEq)]
pub enum ChallengeType {
    Http01(String, String), // token, key_authorization
    Dns01(String, String),  // token, value to set in TXT record
}

/// ACME configuration
#[derive(Debug, Clone)]
pub struct AcmeConfig {
    /// ACME directory URL (e.g., Let's Encrypt production or staging)
    pub directory_url: String,
    /// Email address for ACME account registration
    pub email: String,
    /// Allowed IP addresses for domain validation
    pub allowed_ips: Vec<std::net::IpAddr>,
    /// Challenge type preference (HTTP-01 or DNS-01)
    pub challenge_type: ChallengeType,
    /// Certificate cache directory
    pub cache_dir: Option<String>,
    /// Certificate validity threshold for renewal (days)
    pub renewal_threshold_days: u32,
    /// Whether this is a staging environment
    pub is_staging: bool,
    /// Bogus domain to use for ACME requests (workaround for rate limits)
    pub bogus_domain: Option<String>,
}

impl Default for AcmeConfig {
    fn default() -> Self {
        Self {
            directory_url: "https://acme-v02.api.letsencrypt.org/directory".to_string(),
            email: "webmaster@example.com".to_string(),
            allowed_ips: Vec::new(),
            challenge_type: ChallengeType::Http01("".to_string(), "".to_string()),
            cache_dir: Some("/var/lib/easyp/certs".to_string()),
            renewal_threshold_days: 30,
            is_staging: false,
            bogus_domain: None,
        }
    }
}

/// Cached certificate information
#[derive(Debug, Clone)]
pub struct CachedCertificate {
    pub certified_key: Arc<CertifiedKey>,
    pub expires_at: SystemTime,
    pub domain: String,
}

/// Challenge data for ACME validation
#[derive(Debug, Clone)]
pub struct ChallengeData {
    pub token: String,
    pub key_authorization: String,
    pub domain: String,
    pub challenge_type: ChallengeType,
}

/// Certificate statistics
#[derive(Debug, Clone)]
pub struct CertificateStats {
    pub total: usize,
    pub active: usize,
    pub expired: usize,
    pub expiring_soon: usize,
}
