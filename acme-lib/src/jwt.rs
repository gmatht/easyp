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
        let (alg, kty, crv, x, y) = match a.private_key() {
            rustls_pki_types::PrivateKeyDer::Pkcs8(pkcs8) => {
                let pkcs8_der = pkcs8.secret_pkcs8_der().to_vec();
                match lsb_openssl::certs::ec_public_key_bytes(&pkcs8_der) {
                    Ok(pubkey) if pubkey.len() == 65 && pubkey[0] == 0x04 => {
                        let x_val = base64url(&pubkey[1..33]);
                        let y_val = base64url(&pubkey[33..65]);
                        ("ES256".to_string(), "EC".to_string(), "P-256".to_string(), x_val, y_val)
                    }
                    _ => {
                        ("ES256".to_string(), "EC".to_string(), "P-256".to_string(),
                         "invalid_key".to_string(), "invalid_key".to_string())
                    }
                }
            },
            _ => {
                ("ES256".to_string(), "EC".to_string(), "P-256".to_string(),
                 "unsupported_format".to_string(), "unsupported_format".to_string())
            },
        };
        
        Jwk { alg, kty, crv, _use: "sig".into(), x, y }
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
