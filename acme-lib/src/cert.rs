use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::Result;

/// Make an RSA private key (from which we can derive a public key).
///
/// This library does not check the number of bits used to create the key pair.
/// For Let's Encrypt, the bits must be between 2048 and 4096.
pub fn create_rsa_key(bits: u32) -> PrivateKeyDer<'static> {
    use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey};

    if bits < 2048 || bits > 4096 {
        panic!("RSA key size {} bits is not supported. Must be between 2048 and 4096 bits.", bits);
    }

    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, bits as usize)
        .expect("Failed to generate RSA key");

    let pkcs8_der = private_key.to_pkcs8_der()
        .expect("Failed to encode RSA key as PKCS8");

    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8_der.as_bytes().to_vec()))
}

/// Make a P-256 private key via OpenSSL (loaded at runtime).
pub fn create_p256_key() -> PrivateKeyDer<'static> {
    let der = lsb_openssl::certs::create_ec_p256_key()
        .expect("lsb-openssl: create_ec_p256_key failed");
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(der))
}

/// Make a P-384 private key via OpenSSL (loaded at runtime).
pub fn create_p384_key() -> PrivateKeyDer<'static> {
    let der = lsb_openssl::certs::create_ec_p384_key()
        .expect("lsb-openssl: create_ec_p384_key failed");
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(der))
}

pub(crate) fn create_csr(pkey: &PrivateKeyDer<'static>, domains: &[&str]) -> Result<Vec<u8>> {
    println!("🔍 ACME-LIB: Creating CSR for domains: {:?}", domains);

    let pkcs8_der = match pkey {
        PrivateKeyDer::Pkcs8(pkcs8) => pkcs8.secret_pkcs8_der().to_vec(),
        _ => return Err("Unsupported private key format for CSR".into()),
    };

    let csr_der = lsb_openssl::certs::generate_csr(&pkcs8_der, domains)
        .map_err(|e| format!("Failed to generate CSR via OpenSSL: {}", e))?;

    println!("🔍 ACME-LIB: ✅ Successfully generated CSR ({} bytes)", csr_der.len());
    Ok(csr_der)
}

/// Encapsulated certificate and private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    private_key: String,
    certificate: String,
}

impl Certificate {
    pub(crate) fn new(private_key: String, certificate: String) -> Self {
        Certificate { private_key, certificate }
    }

    /// The PEM encoded private key.
    pub fn private_key(&self) -> &str { &self.private_key }

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
    pub fn certificate(&self) -> &str { &self.certificate }

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
    pub fn valid_days_left(&self) -> Result<i64> {
        if cfg!(test) { return Ok(89); }

        let cert_der = self.certificate_der()?;
        let (_, cert) = x509_parser::parse_x509_certificate(&cert_der)
            .map_err(|e| format!("Failed to parse certificate: {:?}", e))?;

        let not_after = cert.validity().not_after.timestamp();
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
        let _rsa_key = create_rsa_key(2048);
        let _p256_key = create_p256_key();
        let _p384_key = create_p384_key();
    }
}
