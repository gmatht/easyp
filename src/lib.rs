//! ACME certificate management for rustls
//!
//! This crate provides ACME (Automatic Certificate Management Environment)
//! functionality for rustls, allowing automatic certificate provisioning
//! from Let's Encrypt and other ACME-compliant certificate authorities.
//!
//! The crate is designed to work with both rustls 0.23 and 0.24+ by using
//! trait abstractions that can be implemented by different rustls versions.

// This crate requires std

pub mod client;
pub mod resolver;
pub mod validation;
pub mod types;

// Re-export the main types for convenience
pub use client::AcmeClient;
pub use resolver::OnDemandCertResolver;
pub use validation::DnsValidator;
pub use types::*;
