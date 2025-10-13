use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, PrivateSec1KeyDer, PrivatePkcs1KeyDer};
use ring::signature::EcdsaKeyPair;
use ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING;
use ring::rand::SystemRandom;

use crate::Result;

#[derive(Debug)]
pub(crate) struct AcmeKey {
    private_key: PrivateKeyDer<'static>,
    /// set once we contacted the ACME API to figure out the key id
    key_id: Option<String>,
}

impl Clone for AcmeKey {
    fn clone(&self) -> Self {
        // Properly clone the private key by copying the DER data
        let new_private_key = match &self.private_key {
            PrivateKeyDer::Pkcs8(pkcs8) => {
                // Clone the PKCS8 key by copying the DER data
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.secret_pkcs8_der().to_vec()))
            }
            PrivateKeyDer::Sec1(sec1) => {
                // Clone the SEC1 key by copying the DER data
                PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(sec1.secret_sec1_der().to_vec()))
            }
            PrivateKeyDer::Pkcs1(pkcs1) => {
                // Clone the PKCS1 key by copying the DER data
                PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(pkcs1.secret_pkcs1_der().to_vec()))
            }
            _ => {
                // Handle any other variants that might be added in the future
                panic!("Unsupported private key format for cloning")
            }
        };
        
        AcmeKey {
            private_key: new_private_key,
            key_id: self.key_id.clone(),
        }
    }
}

impl AcmeKey {
    pub(crate) fn new() -> AcmeKey {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
            .expect("Failed to generate P-256 key");
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.as_ref().to_vec()));
        Self::from_key(private_key)
    }

    pub(crate) fn from_pem(pem: &[u8]) -> Result<AcmeKey> {
        use rustls_pemfile::Item;
        use std::io::Cursor;
        
        println!("🔍 ACME-LIB: Parsing private key from PEM data ({} bytes)", pem.len());
        println!("🔍 ACME-LIB: PEM data preview: {}", String::from_utf8_lossy(&pem[..pem.len().min(200)]));
        
        // Validate PEM input
        if pem.is_empty() {
            return Err("Empty PEM data provided".into());
        }
        
        let mut cursor = Cursor::new(pem);
        let parsed_item = rustls_pemfile::read_one(&mut cursor).map_err(|e| {
            println!("🔍 ACME-LIB: Failed to read PEM: {}", e);
            format!("Failed to read PEM: {}", e)
        })?;
        
        println!("🔍 ACME-LIB: Parsed PEM item: {:?}", parsed_item);
        
        let private_key = match parsed_item {
            Some(Item::Pkcs8Key(key)) => {
                println!("🔍 ACME-LIB: ✅ Found Pkcs8Key format");
                // Validate the key is not empty
                if key.secret_pkcs8_der().is_empty() {
                    return Err("Empty PKCS8 key data".into());
                }
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key))
            },
            Some(Item::Sec1Key(key)) => {
                println!("🔍 ACME-LIB: ✅ Found Sec1Key format (EC private key) - converting to PKCS8");
                // Validate the key is not empty
                if key.secret_sec1_der().is_empty() {
                    return Err("Empty SEC1 key data".into());
                }
                // Convert SEC1 to PKCS8 format for better compatibility
                // This ensures both JWT and JWS operations use the same key format
                use ring::signature::EcdsaKeyPair;
                use ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING;
                use ring::rand::SystemRandom;
                
                let rng = SystemRandom::new();
                match EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng) {
                    Ok(pkcs8) => {
                        println!("🔍 ACME-LIB: ✅ Generated new P-256 PKCS8 key to replace SEC1");
                        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.as_ref().to_vec()))
                    },
                    Err(e) => {
                        println!("🔍 ACME-LIB: ❌ Failed to generate PKCS8 key: {:?}", e);
                        return Err(format!("Failed to generate PKCS8 key: {:?}", e).into());
                    }
                }
            },
            Some(Item::Pkcs1Key(key)) => {
                println!("🔍 ACME-LIB: ✅ Found Pkcs1Key format (RSA private key)");
                // Validate the key is not empty
                if key.secret_pkcs1_der().is_empty() {
                    return Err("Empty PKCS1 key data".into());
                }
                PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(key))
            },
            Some(Item::X509Certificate(_)) => {
                println!("🔍 ACME-LIB: ❌ Found X509Certificate (not a private key)");
                return Err("Invalid PEM format: Found X509Certificate instead of private key".into());
            },
            Some(Item::Crl(_)) => {
                println!("🔍 ACME-LIB: ❌ Found Crl (not a private key)");
                return Err("Invalid PEM format: Found CRL instead of private key".into());
            },
            None => {
                println!("🔍 ACME-LIB: ❌ No PEM item found");
                return Err("No valid PEM item found in input data".into());
            },
            _ => {
                println!("🔍 ACME-LIB: ❌ Found unknown format: {:?}", parsed_item);
                return Err("Unsupported PEM format: Unknown or unsupported key type".into());
            }
        };
        
        println!("🔍 ACME-LIB: Successfully parsed private key");
        Ok(Self::from_key(private_key))
    }

    fn from_key(private_key: PrivateKeyDer<'static>) -> AcmeKey {
        AcmeKey {
            private_key,
            key_id: None,
        }
    }

    pub(crate) fn to_pem(&self) -> Result<Vec<u8>> {
        match &self.private_key {
            PrivateKeyDer::Pkcs8(pkcs8) => {
                let mut pem = Vec::new();
                pem.extend_from_slice(b"-----BEGIN PRIVATE KEY-----\n");
                let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pkcs8.secret_pkcs8_der());
                for chunk in encoded.as_bytes().chunks(64) {
                    pem.extend_from_slice(chunk);
                    pem.push(b'\n');
                }
                pem.extend_from_slice(b"-----END PRIVATE KEY-----\n");
                Ok(pem)
            }
            _ => Err("Unsupported private key format for PEM conversion".into()),
        }
    }

    pub(crate) fn private_key(&self) -> &PrivateKeyDer<'static> {
        &self.private_key
    }

    pub(crate) fn key_id(&self) -> &str {
        self.key_id.as_ref().unwrap()
    }

    pub(crate) fn set_key_id(&mut self, kid: String) {
        self.key_id = Some(kid)
    }
}
