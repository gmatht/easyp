use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
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
        // Create a new AcmeKey with the same key_id but regenerate the private key
        // This is a workaround since PrivateKeyDer doesn't implement Clone
        let new_private_key = match &self.private_key {
            PrivateKeyDer::Pkcs8(pkcs8) => {
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.secret_pkcs8_der().to_vec()))
            }
            _ => panic!("Unsupported private key format for cloning"),
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
        
        let mut cursor = Cursor::new(pem);
        let parsed_item = rustls_pemfile::read_one(&mut cursor).map_err(|e| {
            println!("🔍 ACME-LIB: Failed to read PEM: {}", e);
            format!("Failed to read PEM: {}", e)
        })?;
        
        println!("🔍 ACME-LIB: Parsed PEM item: {:?}", parsed_item);
        
        let private_key = match parsed_item {
            Some(Item::Pkcs8Key(key)) => {
                println!("🔍 ACME-LIB: ✅ Found Pkcs8Key format (expected)");
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key))
            },
            Some(Item::RSAKey(key)) => {
                println!("🔍 ACME-LIB: ❌ Found RSAKey format (not supported)");
                return Err("Unsupported private key format: RSAKey (expected Pkcs8Key)".into());
            },
            Some(Item::ECKey(key)) => {
                println!("🔍 ACME-LIB: ❌ Found ECKey format (not supported)");
                return Err("Unsupported private key format: ECKey (expected Pkcs8Key)".into());
            },
            Some(Item::Ed25519Key(key)) => {
                println!("🔍 ACME-LIB: ❌ Found Ed25519Key format (not supported)");
                return Err("Unsupported private key format: Ed25519Key (expected Pkcs8Key)".into());
            },
            Some(Item::Ed448Key(key)) => {
                println!("🔍 ACME-LIB: ❌ Found Ed448Key format (not supported)");
                return Err("Unsupported private key format: Ed448Key (expected Pkcs8Key)".into());
            },
            Some(Item::X509Certificate(_)) => {
                println!("🔍 ACME-LIB: ❌ Found X509Certificate (not a private key)");
                return Err("Unsupported private key format: X509Certificate (expected Pkcs8Key)".into());
            },
            Some(Item::Crl(_)) => {
                println!("🔍 ACME-LIB: ❌ Found Crl (not a private key)");
                return Err("Unsupported private key format: Crl (expected Pkcs8Key)".into());
            },
            Some(Item::Pkcs1Key(key)) => {
                println!("🔍 ACME-LIB: ❌ Found Pkcs1Key format (not supported)");
                return Err("Unsupported private key format: Pkcs1Key (expected Pkcs8Key)".into());
            },
            Some(Item::Sec1Key(key)) => {
                println!("🔍 ACME-LIB: ❌ Found Sec1Key format (not supported)");
                return Err("Unsupported private key format: Sec1Key (expected Pkcs8Key)".into());
            },
            None => {
                println!("🔍 ACME-LIB: ❌ No PEM item found");
                return Err("Unsupported private key format: No PEM item found (expected Pkcs8Key)".into());
            },
            _ => {
                println!("🔍 ACME-LIB: ❌ Found unknown format: {:?}", parsed_item);
                return Err("Unsupported private key format: Unknown format (expected Pkcs8Key)".into());
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

    pub(crate) fn to_pem(&self) -> Vec<u8> {
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
                pem
            }
            _ => panic!("Unsupported private key format"),
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
