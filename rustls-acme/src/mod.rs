//! ACME (Automatic Certificate Management Environment) client implementation
//! for on-demand certificate provisioning in rustls.
//!
//! This module provides a complete ACME client that can:
//! - Validate domain ownership through DNS resolution
//! - Obtain certificates from Let's Encrypt and other ACME providers
//! - Handle both HTTP-01 and DNS-01 challenges
//! - Manage certificate renewal and caching
//! - Integrate seamlessly with rustls' ResolvesServerCert trait

mod client;
mod resolver;
mod validation;

pub use client::AcmeClient;
pub use resolver::OnDemandCertResolver;
pub use validation::DnsValidator;

/// Re-export types for convenience
pub mod types {
    pub use super::client::{AcmeConfig, AcmeError, ChallengeType};
    pub use super::validation::ValidationResult;
    pub use super::resolver::CertificateStats;
}



