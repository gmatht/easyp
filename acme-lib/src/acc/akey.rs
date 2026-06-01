use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, PrivateSec1KeyDer, PrivatePkcs1KeyDer};

use crate::Result;

#[derive(Debug)]
pub(crate) struct AcmeKey {
    private_key: PrivateKeyDer<'static>,
    key_id: Option<String>,
}

impl Clone for AcmeKey {
    fn clone(&self) -> Self {
        let new_private_key = match &self.private_key {
            PrivateKeyDer::Pkcs8(pkcs8) => {
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.secret_pkcs8_der().to_vec()))
            }
            PrivateKeyDer::Sec1(sec1) => {
                PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(sec1.secret_sec1_der().to_vec()))
            }
            PrivateKeyDer::Pkcs1(pkcs1) => {
                PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(pkcs1.secret_pkcs1_der().to_vec()))
            }
            _ => panic!("Unsupported private key format for cloning"),
        };
        AcmeKey { private_key: new_private_key, key_id: self.key_id.clone() }
    }
}

impl AcmeKey {
    pub(crate) fn new() -> AcmeKey {
        let der = lsb_openssl::certs::create_ec_p256_key()
            .expect("lsb-openssl: failed to generate EC P-256 key");
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(der));
        Self::from_key(private_key)
    }

    pub(crate) fn from_pem(pem: &[u8]) -> Result<AcmeKey> {
        use rustls_pemfile::Item;
        use std::io::Cursor;

        let mut cursor = Cursor::new(pem);
        let parsed_item = rustls_pemfile::read_one(&mut cursor)
            .map_err(|e| format!("Failed to read PEM: {}", e))?;

        let private_key = match parsed_item {
            Some(Item::Pkcs8Key(key)) => {
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key))
            },
            Some(Item::Sec1Key(_key)) => {
                // Generate new P-256 key instead of trying to convert SEC1
                let der = lsb_openssl::certs::create_ec_p256_key()
                    .expect("lsb-openssl: failed to generate EC P-256 key");
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(der))
            },
            Some(Item::Pkcs1Key(key)) => {
                PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(key))
            },
            _ => return Err("Invalid or unsupported PEM format".into()),
        };

        Ok(Self::from_key(private_key))
    }

    fn from_key(private_key: PrivateKeyDer<'static>) -> AcmeKey {
        AcmeKey { private_key, key_id: None }
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

    pub(crate) fn private_key(&self) -> &PrivateKeyDer<'static> { &self.private_key }
    pub(crate) fn key_id(&self) -> &str { self.key_id.as_ref().unwrap() }
    pub(crate) fn set_key_id(&mut self, kid: String) { self.key_id = Some(kid) }
}
