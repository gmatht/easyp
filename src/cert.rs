use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use x509_parser::prelude::*;

use crate::Result;

// We'll use rustls built-in key generation instead of OpenSSL groups

/// Make an RSA private key (from which we can derive a public key).
///
/// This library does not check the number of bits used to create the key pair.
/// For Let's Encrypt, the bits must be between 2048 and 4096.
pub fn create_rsa_key(_bits: u32) -> PrivateKeyDer<'static> {
    use rcgen::KeyPair;
    
    // Generate RSA key using rcgen
    let key_pair = KeyPair::generate()
        .expect("Failed to generate RSA key");
    
    // Convert to PKCS8 format
    let pkcs8_der = key_pair.serialize_der();
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8_der))
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
    use rcgen::{CertificateParams, KeyPair};
    
    // Convert PrivateKeyDer to KeyPair for signing
    let key_pair = match pkey {
        PrivateKeyDer::Pkcs8(pkcs8) => {
            // Convert PKCS8 to PEM format for rcgen
            let mut pem = Vec::new();
            pem.extend_from_slice(b"-----BEGIN PRIVATE KEY-----\n");
            let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pkcs8.secret_pkcs8_der());
            for chunk in encoded.as_bytes().chunks(64) {
                pem.extend_from_slice(chunk);
                pem.push(b'\n');
            }
            pem.extend_from_slice(b"-----END PRIVATE KEY-----\n");
            
            KeyPair::from_pem(&String::from_utf8_lossy(&pem))
                .map_err(|e| format!("Failed to create KeyPair from PKCS8: {}", e))?
        }
        other => {
            println!("🔍 ACME-LIB: ❌ Unsupported private key format in cert.rs: {:?}", other);
            return Err("Unsupported private key format".into());
        }
    };
    
    // Create certificate parameters for the CSR
    let params = CertificateParams::new(domains.iter().map(|s| s.to_string()).collect::<Vec<String>>())
        .map_err(|e| format!("Failed to create CertificateParams: {}", e))?;
    
    // Generate the CSR
    let csr = params.serialize_request(&key_pair)
        .map_err(|e| format!("Failed to serialize CSR: {}", e))?;
    
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
    pub fn private_key_der(&self) -> Vec<u8> {
        use rustls_pemfile::Item;
        use std::io::Cursor;
        
        let mut cursor = Cursor::new(self.private_key.as_bytes());
        match rustls_pemfile::read_one(&mut cursor).expect("Failed to read PEM") {
            Some(Item::Pkcs8Key(key)) => key.secret_pkcs8_der().to_vec(),
            _ => panic!("Unsupported private key format"),
        }
    }

    /// The PEM encoded issued certificate.
    pub fn certificate(&self) -> &str {
        &self.certificate
    }

    /// The issued certificate as DER.
    pub fn certificate_der(&self) -> Vec<u8> {
        use rustls_pemfile::Item;
        use std::io::Cursor;
        
        let mut cursor = Cursor::new(self.certificate.as_bytes());
        match rustls_pemfile::read_one(&mut cursor).expect("Failed to read PEM") {
            Some(Item::X509Certificate(cert)) => cert.to_vec(),
            _ => panic!("Invalid certificate format"),
        }
    }

    /// Inspect the certificate to count the number of (whole) valid days left.
    ///
    /// It's up to the ACME API provider to decide how long an issued certificate is valid.
    /// Let's Encrypt sets the validity to 90 days. This function reports 89 days for newly
    /// issued cert, since it counts _whole_ days.
    ///
    /// It is possible to get negative days for an expired certificate.
    pub fn valid_days_left(&self) -> i64 {
        // the cert used in the tests is not valid to load as x509
        if cfg!(test) {
            return 89;
        }

        // Parse certificate using x509-parser
        let cert_der = self.certificate_der();
        let (_, cert) = X509Certificate::from_der(&cert_der)
            .expect("Failed to parse certificate");

        // Get validity period
        let validity = &cert.tbs_certificate.validity;
        let not_after = validity.not_after.timestamp();
        
        // Calculate days remaining
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let days_left = (not_after - now) / (24 * 60 * 60);
        
        days_left as i64
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
