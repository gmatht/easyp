//! On-demand certificate resolver with ACME integration

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::string::String;
use std::format;
use std::string::ToString;
use std::boxed::Box;
use std::vec;
use std::println;

use tokio::sync::RwLock;

use crate::types::*;
use crate::client::AcmeClient;
use crate::validation::{DnsValidator, ValidationResult};
use rustls::server::ResolvesServerCert;

/// On-demand certificate resolver that uses ACME to obtain certificates
pub struct OnDemandCertResolver {
    acme_client: Arc<AcmeClient>,
    dns_validator: Arc<DnsValidator>,
    cert_cache: Arc<RwLock<HashMap<String, CachedCertificate>>>,
}

impl OnDemandCertResolver {
    /// Create a new on-demand certificate resolver
    pub fn new(
        acme_client: Arc<AcmeClient>,
        dns_validator: Arc<DnsValidator>,
        _fallback_resolver: Option<Box<dyn ResolvesServerCert>>,
        _max_cache_size: usize,
        _renewal_threshold: Duration,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            acme_client,
            dns_validator,
            cert_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a new resolver with additional parameters
    pub fn new_with_params(
        acme_client: Arc<AcmeClient>,
        dns_validator: Arc<DnsValidator>,
        _fallback_resolver: Option<Box<dyn ResolvesServerCert>>,
        _max_cache_size: usize,
        _renewal_threshold: Duration,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            acme_client,
            dns_validator,
            cert_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Clean expired certificates from the cache
    pub async fn clean_expired_certificates(&self) -> Result<usize, AcmeError> {
        let mut cache = self.cert_cache.write().await;
        let initial_count = cache.len();
        let now = SystemTime::now();
        
        cache.retain(|_, cached| cached.expires_at > now);
        
        Ok(initial_count - cache.len())
    }

    /// Get certificate statistics
    pub async fn get_certificate_stats(&self) -> CertificateStats {
        let cache = self.cert_cache.read().await;
        let now = SystemTime::now();
        let renewal_threshold = Duration::from_secs(30 * 24 * 60 * 60); // 30 days
        
        let mut stats = CertificateStats {
            total: cache.len(),
            active: 0,
            expired: 0,
            expiring_soon: 0,
        };
        
        for cached in cache.values() {
            if cached.expires_at > now {
                stats.active += 1;
                if cached.expires_at.duration_since(now).unwrap_or_default() < renewal_threshold {
                    stats.expiring_soon += 1;
                }
            } else {
                stats.expired += 1;
            }
        }
        
        stats
    }

    /// Get or create a certificate for the given domain
    async fn get_or_create_certificate(&self, domain: &str) -> Result<Arc<CertifiedKey>, AcmeError> {
        println!("🔍 get_or_create_certificate called for domain: {}", domain);
        
        // Check cache first
        {
            let cache = self.cert_cache.read().await;
            if let Some(cached) = cache.get(domain) {
                if cached.expires_at > SystemTime::now() {
                    println!("✅ Certificate found in cache for domain: {}", domain);
                    return Ok(cached.certified_key.clone());
                }
                println!("⚠️  Certificate expired in cache for domain: {}", domain);
            }
        }

        // Validate domain
        println!("🔍 Validating domain: {}", domain);
        match self.dns_validator.validate_domain(domain).await {
            ValidationResult::Valid => {
                println!("✅ Domain validation successful for: {}", domain);
            }
            ValidationResult::InvalidIp => Err(AcmeError::Validation(format!("Domain {} resolves to unauthorized IPs", domain)))?,
            ValidationResult::NoResolution => Err(AcmeError::Validation(format!("Domain {} does not resolve to any IP address", domain)))?,
            ValidationResult::Timeout => Err(AcmeError::Validation(format!("DNS resolution timeout for domain {}", domain)))?,
            ValidationResult::Error(msg) => Err(AcmeError::Validation(format!("DNS validation error for domain {}: {}", domain, msg)))?,
        }

        // Request certificate from ACME client
        println!("🔍 Requesting certificate from ACME client for domain: {}", domain);
        let certified_key = self.acme_client.get_certificate(domain).await?;

                        // Cache the certificate
        {
                        let mut cache = self.cert_cache.write().await;
                        cache.insert(domain.to_string(), CachedCertificate {
                            certified_key: certified_key.clone(),
                expires_at: SystemTime::now() + Duration::from_secs(30 * 24 * 60 * 60), // 30 days
                            domain: domain.to_string(),
            });
        }

        println!("✅ Certificate obtained and cached for domain: {}", domain);
        Ok(certified_key)
    }

    /// Renew certificate if needed
    pub async fn renew_if_needed(&self, domain: &str) -> Result<Option<Arc<CertifiedKey>>, AcmeError> {
        // Check if renewal is needed
        let needs_renewal = {
            let cache = self.cert_cache.read().await;
            if let Some(cached) = cache.get(domain) {
                let renewal_threshold = Duration::from_secs(30 * 24 * 60 * 60); // 30 days
                cached.expires_at.duration_since(SystemTime::now()).unwrap_or_default() < renewal_threshold
            } else {
                true // No certificate in cache, needs renewal
            }
        };

        if needs_renewal {
            println!("🔄 Certificate renewal needed for domain: {}", domain);
            let new_cert = self.get_or_create_certificate(domain).await?;
            Ok(Some(new_cert))
        } else {
            println!("✅ Certificate is still valid for domain: {}", domain);
            Ok(None)
        }
    }

    /// Generate a self-signed certificate (fallback)
    fn generate_self_signed_certificate(&self, domain: &str) -> Result<Arc<CertifiedKey>, AcmeError> {
        // This is a simplified version - in practice, you'd implement self-signed certificate generation here
        Err(AcmeError::Client("Self-signed certificate generation not yet implemented in rustls-acme".to_string()))
    }
}

impl ResolvesServerCert for OnDemandCertResolver {
    fn resolve(&self, client_hello: &rustls::server::ClientHello<'_>) -> Result<rustls::sign::CertifiedSigner, rustls::Error> {
        let server_name = match client_hello.server_name() {
            Some(name) => name,
            None => {
                println!("❌ No server name provided in ClientHello");
                return Err(rustls::Error::NoSuitableCertificate);
            }
        };

        println!("🔍 Resolver: About to call get_or_create_certificate for domain: {:?}", server_name);
        
        // We need to handle the async call synchronously
        // Create a new tokio runtime for this call
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => {
                // If we're not in a tokio context, create a new runtime
                match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt.handle().clone(),
                    Err(e) => {
                        println!("❌ Failed to create tokio runtime: {}", e);
                        return Err(rustls::Error::NoSuitableCertificate);
                    }
                }
            }
        };

        // Use block_in_place to handle the async call
        let result = rt.block_on(async {
            self.get_or_create_certificate(server_name.as_ref()).await
        });

        match result {
            Ok(certified_key) => {
                println!("✅ Certificate resolved successfully for domain: {:?}", server_name);
                // Convert our CertifiedKey to a CertifiedSigner
                certified_key.signer(client_hello.signature_schemes())
                    .ok_or(rustls::Error::PeerIncompatible(
                        rustls::PeerIncompatible::NoSignatureSchemesInCommon
                    ))
            }
            Err(e) => {
                println!("❌ Failed to get certificate for domain {:?}: {}", server_name, e);
                Err(rustls::Error::NoSuitableCertificate)
            }
        }
    }
}

impl std::fmt::Debug for OnDemandCertResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnDemandCertResolver")
            .field("acme_client", &"<AcmeClient>")
            .field("dns_validator", &"<DnsValidator>")
            .finish()
    }
}