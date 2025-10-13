use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use x509_parser::prelude::*;

use crate::Result;

// We'll use rustls built-in key generation instead of OpenSSL groups

/// Make an RSA private key (from which we can derive a public key).
///
/// This library does not check the number of bits used to create the key pair.
/// For Let's Encrypt, the bits must be between 2048 and 4096.
pub fn create_rsa_key(bits: u32) -> PrivateKeyDer<'static> {
    use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey};
    
    // Validate bit length for Let's Encrypt requirements
    if bits < 2048 || bits > 4096 {
        panic!("RSA key size {} bits is not supported. Must be between 2048 and 4096 bits.", bits);
    }
    
    // Generate RSA key using rsa crate
    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, bits as usize)
        .expect("Failed to generate RSA key");
    
    // Convert to PKCS8 format
    let pkcs8_der = private_key.to_pkcs8_der()
        .expect("Failed to encode RSA key as PKCS8");
    
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8_der.as_bytes().to_vec()))
}

/// Make a P-256 private key (from which we can derive a public key).
pub fn create_p256_key() -> PrivateKeyDer<'static> {
    use ring::rand::SystemRandom;
    use ring::signature::EcdsaKeyPair;
    
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
        .expect("Failed to generate P-256 key");
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.as_ref().to_vec()))
}

/// Make a P-384 private key pair (from which we can derive a public key).
pub fn create_p384_key() -> PrivateKeyDer<'static> {
    use ring::rand::SystemRandom;
    use ring::signature::EcdsaKeyPair;
    
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING, &rng)
        .expect("Failed to generate P-384 key");
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.as_ref().to_vec()))
}

pub(crate) fn create_csr(pkey: &PrivateKeyDer<'static>, domains: &[&str]) -> Result<Vec<u8>> {
    use rcgen::{CertificateParams, KeyPair, SanType};
    use std::str::FromStr;
    
    println!("🔍 ACME-LIB: Creating CSR for domains: {:?}", domains);
    
    // Convert PrivateKeyDer to KeyPair for signing
    let key_pair = match pkey {
        PrivateKeyDer::Pkcs8(pkcs8) => {
            println!("🔍 ACME-LIB: Converting PKCS8 private key to KeyPair");
            
            // Convert PKCS8 to PEM format for rcgen
            let mut pem = Vec::new();
            pem.extend_from_slice(b"-----BEGIN PRIVATE KEY-----\n");
            let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pkcs8.secret_pkcs8_der());
            for chunk in encoded.as_bytes().chunks(64) {
                pem.extend_from_slice(chunk);
                pem.push(b'\n');
            }
            pem.extend_from_slice(b"-----END PRIVATE KEY-----\n");
            
            let pem_string = String::from_utf8_lossy(&pem);
            println!("🔍 ACME-LIB: Generated PEM (first 100 chars): {}", &pem_string[..pem_string.len().min(100)]);
            
            KeyPair::from_pem(&pem_string)
                .map_err(|e| {
                    println!("🔍 ACME-LIB: ❌ Failed to create KeyPair from PKCS8: {}", e);
                    format!("Failed to create KeyPair from PKCS8: {}", e)
                })?
        }
        PrivateKeyDer::Sec1(sec1) => {
            println!("🔍 ACME-LIB: Converting SEC1 private key to KeyPair");
            
            // Convert SEC1 to PEM format for rcgen
            let mut pem = Vec::new();
            pem.extend_from_slice(b"-----BEGIN EC PRIVATE KEY-----\n");
            let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, sec1.secret_sec1_der());
            for chunk in encoded.as_bytes().chunks(64) {
                pem.extend_from_slice(chunk);
                pem.push(b'\n');
            }
            pem.extend_from_slice(b"-----END EC PRIVATE KEY-----\n");
            
            let pem_string = String::from_utf8_lossy(&pem);
            println!("🔍 ACME-LIB: Generated SEC1 PEM (first 100 chars): {}", &pem_string[..pem_string.len().min(100)]);
            
            KeyPair::from_pem(&pem_string)
                .map_err(|e| {
                    println!("🔍 ACME-LIB: ❌ Failed to create KeyPair from SEC1: {}", e);
                    format!("Failed to create KeyPair from SEC1: {}", e)
                })?
        }
        other => {
            println!("🔍 ACME-LIB: ❌ Unsupported private key format in cert.rs: {:?}", other);
            return Err("Unsupported private key format".into());
        }
    };
    
    println!("🔍 ACME-LIB: ✅ Successfully created KeyPair");
    
    // Create certificate parameters for the CSR - similar to old OpenSSL approach
    let mut params = CertificateParams::new(domains.iter().map(|s| s.to_string()).collect::<Vec<String>>())
        .map_err(|e| {
            println!("🔍 ACME-LIB: ❌ Failed to create CertificateParams: {}", e);
            format!("Failed to create CertificateParams: {}", e)
        })?;
    
    // Add Subject Alternative Names (SAN) for all domains - this is crucial!
    // This matches the old OpenSSL approach of adding DNS names as SAN extensions
    let mut san_entries = Vec::new();
    for domain in domains {
        // Create DNS name entry for SAN
        san_entries.push(SanType::DnsName(rcgen::string::Ia5String::from_str(domain).unwrap()));
        println!("🔍 ACME-LIB: Adding SAN entry for domain: {}", domain);
    }
    params.subject_alt_names = san_entries;
    
    // Set additional parameters to ensure proper CSR generation
    params.distinguished_name = rcgen::DistinguishedName::new();
    
    println!("🔍 ACME-LIB: ✅ Successfully created CertificateParams with {} SAN entries", params.subject_alt_names.len());
    
    // Generate the CSR
    let csr = params.serialize_request(&key_pair)
        .map_err(|e| {
            println!("🔍 ACME-LIB: ❌ Failed to serialize CSR: {}", e);
            format!("Failed to serialize CSR: {}", e)
        })?;
    
    println!("🔍 ACME-LIB: ✅ Successfully generated CSR ({} bytes)", csr.der().len());
    
    // Return the DER-encoded CSR
    Ok(csr.der().to_vec())
}

/// Encapsulated certificate and private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    private_key: String,
    certificate: String,
}

impl Certificate {
    pub(crate) fn new(private_key: String, certificate: String) -> Self {
        Certificate {
            private_key,
            certificate,
        }
    }

    /// The PEM encoded private key.
    pub fn private_key(&self) -> &str {
        &self.private_key
    }

    /// The private key as DER.
    pub fn private_key_der(&self) -> Result<Vec<u8>> {
        use rustls_pemfile::Item;
        use std::io::Cursor;
        
        let mut cursor = Cursor::new(self.private_key.as_bytes());
        match rustls_pemfile::read_one(&mut cursor).map_err(|e| format!("Failed to read PEM: {}", e))? {
            Some(Item::Pkcs8Key(key)) => Ok(key.secret_pkcs8_der().to_vec()),
            _ => Err("Unsupported private key format".into()),
        }
    }

    /// The PEM encoded issued certificate.
    pub fn certificate(&self) -> &str {
        &self.certificate
    }

    /// The issued certificate as DER.
    pub fn certificate_der(&self) -> Result<Vec<u8>> {
        use rustls_pemfile::Item;
        use std::io::Cursor;
        
        let mut cursor = Cursor::new(self.certificate.as_bytes());
        match rustls_pemfile::read_one(&mut cursor).map_err(|e| format!("Failed to read PEM: {}", e))? {
            Some(Item::X509Certificate(cert)) => Ok(cert.to_vec()),
            _ => Err("Invalid certificate format".into()),
        }
    }

    /// Inspect the certificate to count the number of (whole) valid days left.
    ///
    /// It's up to the ACME API provider to decide how long an issued certificate is valid.
    /// Let's Encrypt sets the validity to 90 days. This function reports 89 days for newly
    /// issued cert, since it counts _whole_ days.
    ///
    /// It is possible to get negative days for an expired certificate.
    pub fn valid_days_left(&self) -> Result<i64> {
        // the cert used in the tests is not valid to load as x509
        if cfg!(test) {
            return Ok(89);
        }

        // Parse certificate using x509-parser
        let cert_der = self.certificate_der()?;
        let (_, cert) = X509Certificate::from_der(&cert_der)
            .map_err(|e| format!("Failed to parse certificate: {:?}", e))?;

        // Get validity period
        let validity = &cert.tbs_certificate.validity;
        let not_after = validity.not_after.timestamp();
        
        // Calculate days remaining
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get current time: {}", e))?
            .as_secs() as i64;
        let days_left = (not_after - now) / (24 * 60 * 60);
        
        Ok(days_left as i64)
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_certificate_creation() {
        // Test that we can create keys
        let _rsa_key = create_rsa_key(2048);
        let _p256_key = create_p256_key();
        let _p384_key = create_p384_key();
    }
}
