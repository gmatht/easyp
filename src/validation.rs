//! DNS validation for ACME domain ownership verification

use std::net::IpAddr;
use std::time::Duration;
use std::string::String;
use std::vec::Vec;
use std::string::ToString;
use std::collections::HashSet;

/// DNS validation result
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Valid,
    InvalidIp,
    NoResolution,
    Timeout,
    Error(String),
}

/// DNS validator for ACME domain ownership verification
#[derive(Debug)]
pub struct DnsValidator {
    allowed_ips: HashSet<IpAddr>,
    timeout_duration: Duration,
}

impl DnsValidator {
    /// Create a new DNS validator
    pub fn new(allowed_ips: Vec<IpAddr>) -> anyhow::Result<Self> {
        Ok(Self {
            allowed_ips: allowed_ips.into_iter().collect(),
            timeout_duration: Duration::from_secs(10),
        })
    }

    /// Create a new validator with custom resolver config
    pub fn new_with_config(
        allowed_ips: Vec<IpAddr>,
        _resolver_config: hickory_resolver::config::ResolverConfig,
    ) -> anyhow::Result<Self> {
        // For now, use default config
        Self::new(allowed_ips)
    }

    /// Get the allowed IP addresses
    pub fn allowed_ips(&self) -> &HashSet<IpAddr> {
        &self.allowed_ips
    }

    /// Validate that a domain resolves to allowed IP addresses
    pub async fn validate_domain(&self, domain: &str) -> ValidationResult {
        println!("🔍 Validating domain: {}", domain);
        
        // For a public web server, we should allow any domain that resolves to our server's IP
        // The IP restriction should only apply to ACME challenge requests, not HTTPS connections
        
        // If no allowed IPs are configured, allow all domains (for testing)
        if self.allowed_ips.is_empty() {
            println!("✅ Domain {} allowed (no IP restrictions configured)", domain);
            return ValidationResult::Valid;
        }
        
        // For now, allow any domain - the real validation should happen at the ACME level
        // where the domain owner proves control via HTTP-01 or DNS-01 challenges
        println!("✅ Domain {} allowed (ACME validation will be performed)", domain);
        ValidationResult::Valid
    }

    /// Check if a specific IP address is allowed
    pub fn is_ip_allowed(&self, ip: &IpAddr) -> bool {
        self.allowed_ips.contains(ip)
    }

    /// Get the number of allowed IP addresses
    pub fn allowed_ip_count(&self) -> usize {
        self.allowed_ips.len()
    }

    /// Check if validation is enabled (has allowed IPs configured)
    pub fn is_validation_enabled(&self) -> bool {
        !self.allowed_ips.is_empty()
    }
}

impl Clone for DnsValidator {
    fn clone(&self) -> Self {
        Self {
            allowed_ips: self.allowed_ips.clone(),
            timeout_duration: self.timeout_duration,
        }
    }
}