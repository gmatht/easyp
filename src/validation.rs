//! DNS validation for ACME domain ownership verification

use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;
use std::string::String;
use std::vec::Vec;
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
        _resolver_config: (),
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
        
        // If no allowed IPs are configured, this is a fundamental system error
        // The server should always have at least localhost (127.0.0.1)
        if self.allowed_ips.is_empty() {
            panic!("❌ No allowed IPs configured - server has no network interfaces! This should never happen.");
        }
        
        // Use the same DNS resolution technique as ureq/minreq: std::net::ToSocketAddrs
        println!("🔍 Resolving domain {} using std::net::ToSocketAddrs", domain);
        println!("🔍 Allowed IPs: {:?}", self.allowed_ips);
        
        // Try to resolve the domain to IP addresses
        let socket_addrs = match (domain, 80).to_socket_addrs() {
            Ok(addrs) => addrs.collect::<Vec<_>>(),
            Err(e) => {
                println!("❌ DNS resolution failed for domain {}: {}", domain, e);
                return ValidationResult::Error(format!("DNS resolution failed: {}", e));
            }
        };
        
        if socket_addrs.is_empty() {
            println!("❌ No IP addresses found for domain {}", domain);
            return ValidationResult::NoResolution;
        }
        
        println!("🔍 Domain {} resolves to {} IP addresses", domain, socket_addrs.len());
        
        // Check if any of the resolved IPs are in our allowed list
        let mut found_allowed_ip = false;
        for socket_addr in &socket_addrs {
            let ip = socket_addr.ip();
            println!("🔍 Checking IP: {}", ip);
            
            if self.allowed_ips.contains(&ip) {
                println!("✅ IP {} is allowed for domain {}", ip, domain);
                found_allowed_ip = true;
                break;
            } else {
                println!("⚠️  IP {} is not in allowed list for domain {}", ip, domain);
            }
        }
        
        if found_allowed_ip {
            println!("✅ Domain {} validation successful - at least one IP is allowed", domain);
            ValidationResult::Valid
        } else {
            println!("❌ Domain {} validation failed - no allowed IPs found", domain);
            println!("💡 Tip: Ensure DNS points to this server's IP addresses: {:?}", self.allowed_ips);
            ValidationResult::InvalidIp
        }
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
    /// 
    /// Validation is always enabled - the server should always have allowed IPs.
    /// If no allowed IPs are configured, it's a system error.
    pub fn is_validation_enabled(&self) -> bool {
        true
    }
    
    /// Check if validation is properly configured (has allowed IPs)
    /// 
    /// This should always return true in normal operation.
    /// If it returns false, the server has no network interfaces.
    pub fn is_validation_configured(&self) -> bool {
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