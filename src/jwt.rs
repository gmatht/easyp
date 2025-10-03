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
            rustls_pki_types::PrivateKeyDer::Pkcs8(pkcs8) => pkcs8.secret_pkcs8_der(),
            _ => panic!("Unsupported private key format for JWT"),
        };
        
        use ring::rand::SystemRandom;
        use ring::signature::KeyPair;
        
        let rng = SystemRandom::new();
        let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8, &rng)
            .expect("Failed to create EcdsaKeyPair");
        
        // Extract public key coordinates
        let public_key = key_pair.public_key();
        let public_key_bytes = public_key.as_ref();
        
        // For P-256, the public key is 65 bytes: 0x04 + 32 bytes x + 32 bytes y
        if public_key_bytes.len() != 65 || public_key_bytes[0] != 0x04 {
            panic!("Invalid P-256 public key format");
        }
        
        let x = &public_key_bytes[1..33];
        let y = &public_key_bytes[33..65];
        
        Jwk {
            alg: "ES256".into(),
            kty: "EC".into(),
            crv: "P-256".into(),
            _use: "sig".into(),
            x: base64url(x),
            y: base64url(y),
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
