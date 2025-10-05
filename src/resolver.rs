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
        
        // Check if this is an IP address
        if domain.parse::<std::net::IpAddr>().is_ok() {
            println!("🔍 IP address detected: {}, generating self-signed certificate", domain);
            return self.generate_self_signed_certificate(domain).await;
        }
        
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

    /// Generate a self-signed certificate for IP addresses
    async fn generate_self_signed_certificate(&self, ip_address: &str) -> Result<Arc<CertifiedKey>, AcmeError> {
        use rcgen::{CertificateParams, KeyPair, SanType};
        
        println!("🔍 Generating self-signed certificate for IP: {}", ip_address);
        
        // Parse the IP address
        let ip_addr = ip_address.parse::<std::net::IpAddr>()
            .map_err(|e| AcmeError::Validation(format!("Invalid IP address {}: {}", ip_address, e)))?;
        
        // Generate a new key pair
        let key_pair = KeyPair::generate()
            .map_err(|e| AcmeError::Validation(format!("Failed to generate key pair: {}", e)))?;
        
        // Create certificate parameters with localhost as the main subject
        let mut params = CertificateParams::new(vec!["localhost".to_string()])
            .map_err(|e| AcmeError::Validation(format!("Failed to create certificate params: {}", e)))?;
        
        // Add multiple Subject Alternative Names for better compatibility
        let mut sans = vec![
            SanType::DnsName(rcgen::string::Ia5String::try_from("localhost").unwrap()),
            SanType::IpAddress(ip_addr),
        ];
        
        // Also add common localhost IPs for better compatibility
        if ip_address != "127.0.0.1" {
            sans.push(SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))));
        }
        if ip_address != "::1" {
            sans.push(SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))));
        }
        
        params.subject_alt_names = sans;
        
        // Set additional parameters
        params.distinguished_name = rcgen::DistinguishedName::new();
        
        // Generate the certificate
        let cert = params.self_signed(&key_pair)
            .map_err(|e| AcmeError::Validation(format!("Failed to generate certificate: {}", e)))?;
        
        // Convert to DER format
        let cert_der = cert.der().to_vec();
        
        let key_der = key_pair.serialize_der();
        
        // Create the CertifiedKey using the correct API
        let certified_key = Arc::new(rustls::sign::CertifiedKey::from_der(
            vec![rustls::pki_types::CertificateDer::from(cert_der)].into(),
            rustls::pki_types::PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(key_der)),
            &rustls::crypto::ring::default_provider(),
        ).map_err(|e| AcmeError::Validation(format!("Failed to create CertifiedKey: {}", e)))?);
        
        println!("✅ Self-signed certificate generated for IP: {}", ip_address);
        Ok(certified_key)
    }

    /// Resolve certificate for IP address connections (when no server name is provided)
    fn resolve_for_ip_connection(&self, client_hello: &rustls::server::ClientHello<'_>) -> Result<rustls::sign::CertifiedSigner, rustls::Error> {
        println!("🔍 Resolving certificate for IP address connection");
        
        // For IP connections, we'll generate a self-signed certificate for localhost
        // This is a reasonable fallback since we can't determine the exact IP from the ClientHello
        let fallback_ip = "127.0.0.1";
        
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
        let result = tokio::task::block_in_place(|| {
            rt.block_on(async {
                self.generate_self_signed_certificate(fallback_ip).await
            })
        });

        match result {
            Ok(certified_key) => {
                println!("✅ Self-signed certificate generated for IP connection");
                
                // Get signature schemes from client hello
                let signature_schemes = client_hello.signature_schemes();
                println!("🔍 Client signature schemes: {:?}", signature_schemes);
                
                // Convert our CertifiedKey to a CertifiedSigner
                match certified_key.signer(signature_schemes) {
                    Some(signer) => {
                        println!("✅ Successfully created signer for IP connection");
                        Ok(signer)
                    }
                    None => {
                        println!("❌ Failed to create signer - no compatible signature schemes for IP connection");
                        Err(rustls::Error::PeerIncompatible(
                            rustls::PeerIncompatible::NoSignatureSchemesInCommon
                        ))
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to generate self-signed certificate for IP connection: {}", e);
                Err(rustls::Error::NoSuitableCertificate)
            }
        }
    }
}

impl ResolvesServerCert for OnDemandCertResolver {
    fn resolve(&self, client_hello: &rustls::server::ClientHello<'_>) -> Result<rustls::sign::CertifiedSigner, rustls::Error> {
        let server_name = match client_hello.server_name() {
            Some(name) => name,
            None => {
                println!("🔍 No server name provided in ClientHello - likely IP address connection");
                // When no server name is provided, it's likely an IP address connection
                // We'll generate a self-signed certificate for a generic IP
                // The actual IP will be determined from the connection
                return self.resolve_for_ip_connection(client_hello);
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
        println!("🔍 Resolver: About to call block_in_place for certificate resolution");
        let result = tokio::task::block_in_place(|| {
            println!("🔍 Resolver: Inside block_in_place, calling get_or_create_certificate");
            rt.block_on(async {
                println!("🔍 Resolver: Inside async block, calling get_or_create_certificate");
                self.get_or_create_certificate(server_name.as_ref()).await
            })
        });
        println!("🔍 Resolver: Certificate resolution result: {:?}", result.is_ok());

        match result {
            Ok(certified_key) => {
                println!("✅ Certificate resolved successfully for domain: {:?}", server_name);
                
                // Debug: Print signature schemes
                let signature_schemes = client_hello.signature_schemes();
                println!("🔍 Client signature schemes: {:?}", signature_schemes);
                
                // Convert our CertifiedKey to a CertifiedSigner
                match certified_key.signer(signature_schemes) {
                    Some(signer) => {
                        println!("✅ Successfully created signer for domain: {:?}", server_name);
                        Ok(signer)
                    }
                    None => {
                        println!("❌ Failed to create signer - no compatible signature schemes for domain: {:?}", server_name);
                        Err(rustls::Error::PeerIncompatible(
                            rustls::PeerIncompatible::NoSignatureSchemesInCommon
                        ))
                    }
                }
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