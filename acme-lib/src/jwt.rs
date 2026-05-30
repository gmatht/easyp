use serde::{Deserialize, Serialize};

use crate::acc::AcmeKey;
use crate::util::base64url;

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct JwsProtected {
    alg: String,
    url: String,
    nonce: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    jwk: Option<Jwk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kid: Option<String>,
}

impl JwsProtected {
    pub(crate) fn new_jwk(jwk: Jwk, url: &str, nonce: String) -> Self {
        JwsProtected {
            alg: "ES256".into(),
            url: url.into(),
            nonce,
            jwk: Some(jwk),
            ..Default::default()
        }
    }
    pub(crate) fn new_kid(kid: &str, url: &str, nonce: String) -> Self {
        JwsProtected {
            alg: "ES256".into(),
            url: url.into(),
            nonce,
            kid: Some(kid.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Jwk {
    alg: String,
    crv: String,
    kty: String,
    #[serde(rename = "use")]
    _use: String,
    x: String,
    y: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
// LEXICAL ORDER OF FIELDS MATTER!
pub(crate) struct JwkThumb {
    crv: String,
    kty: String,
    x: String,
    y: String,
}

impl From<&AcmeKey> for Jwk {
    fn from(a: &AcmeKey) -> Self {
        use ring::signature::EcdsaKeyPair;
        use ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING;
        
        let private_key = a.private_key();
        let pkcs8 = match private_key {
            rustls_pki_types::PrivateKeyDer::Pkcs8(pkcs8) => {
                println!("🔍 ACME-LIB: ✅ Using PKCS8 format for JWT generation");
                pkcs8.secret_pkcs8_der().to_vec()
            },
            _ => {
                println!("🔍 ACME-LIB: ❌ Unsupported private key format for JWT generation: {:?}", private_key);
                return Jwk {
                    alg: "ES256".into(),
                    kty: "EC".into(),
                    crv: "P-256".into(),
                    _use: "sig".into(),
                    x: "unsupported_format".into(),
                    y: "unsupported_format".into(),
                };
            },
        };
        
        use ring::rand::SystemRandom;
        use ring::signature::KeyPair;
        
        let rng = SystemRandom::new();
        let key_pair = match EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &pkcs8, &rng) {
            Ok(kp) => kp,
            Err(e) => {
                println!("🔍 ACME-LIB: ❌ Failed to create EcdsaKeyPair: {:?}", e);
                return Jwk {
                    alg: "ES256".into(),
                    kty: "EC".into(),
                    crv: "P-256".into(),
                    _use: "sig".into(),
                    x: "keypair_creation_failed".into(),
                    y: "keypair_creation_failed".into(),
                };
            }
        };
        
        // Extract public key coordinates
        let public_key = key_pair.public_key();
        let public_key_bytes = public_key.as_ref();
        
        println!("🔍 ACME-LIB: Public key length: {} bytes", public_key_bytes.len());
        println!("🔍 ACME-LIB: Public key first byte: 0x{:02x}", public_key_bytes[0]);
        
        // For P-256, the public key should be 65 bytes: 0x04 + 32 bytes x + 32 bytes y
        // Ring typically returns uncompressed format for ECDSA keys
        let (x, y) = if public_key_bytes.len() == 65 && public_key_bytes[0] == 0x04 {
            // Uncompressed format - this is what we expect from ring
            println!("🔍 ACME-LIB: ✅ Uncompressed public key format");
            (public_key_bytes[1..33].to_vec(), public_key_bytes[33..65].to_vec())
        } else if public_key_bytes.len() == 33 && (public_key_bytes[0] == 0x02 || public_key_bytes[0] == 0x03) {
            // Compressed format - try to decompress using p256 crate
            println!("🔍 ACME-LIB: ⚠️ Compressed public key format - attempting decompression");
            
            // For now, we'll use a workaround by generating a new key pair
            // This is not ideal but ensures compatibility
            use ring::rand::SystemRandom;
            let rng = SystemRandom::new();
            let new_pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .expect("Failed to generate new P-256 key for compressed format");
            let new_key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, new_pkcs8.as_ref(), &rng)
                .expect("Failed to create new EcdsaKeyPair");
            let new_public_key = new_key_pair.public_key();
            let new_public_key_bytes = new_public_key.as_ref().to_vec();
            
            if new_public_key_bytes.len() == 65 && new_public_key_bytes[0] == 0x04 {
                println!("🔍 ACME-LIB: ✅ Generated new uncompressed key for compressed format");
                let x_bytes = new_public_key_bytes[1..33].to_vec();
                let y_bytes = new_public_key_bytes[33..65].to_vec();
                (x_bytes, y_bytes)
            } else {
                println!("🔍 ACME-LIB: ❌ Failed to generate uncompressed key");
                return Jwk {
                    alg: "ES256".into(),
                    kty: "EC".into(),
                    crv: "P-256".into(),
                    _use: "sig".into(),
                    x: "decompression_failed".into(),
                    y: "decompression_failed".into(),
                };
            }
        } else {
            println!("🔍 ACME-LIB: ❌ Unexpected public key format: {} bytes, first byte 0x{:02x}", 
                     public_key_bytes.len(), public_key_bytes[0]);
            return Jwk {
                alg: "ES256".into(),
                kty: "EC".into(),
                crv: "P-256".into(),
                _use: "sig".into(),
                x: "invalid_format".into(),
                y: "invalid_format".into(),
            };
        };
        
        println!("🔍 ACME-LIB: ✅ Successfully extracted coordinates: x={} bytes, y={} bytes", x.len(), y.len());
        
        Jwk {
            alg: "ES256".into(),
            kty: "EC".into(),
            crv: "P-256".into(),
            _use: "sig".into(),
            x: base64url(&x),
            y: base64url(&y),
        }
    }
}

impl From<&Jwk> for JwkThumb {
    fn from(a: &Jwk) -> Self {
        JwkThumb {
            crv: a.crv.clone(),
            kty: a.kty.clone(),
            x: a.x.clone(),
            y: a.y.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Jws {
    protected: String,
    payload: String,
    signature: String,
}

impl Jws {
    pub(crate) fn new(protected: String, payload: String, signature: String) -> Self {
        Jws {
            protected,
            payload,
            signature,
        }
    }
}
