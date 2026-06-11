use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use acme_lib::persist::{FilePersist, Persist, PersistKey};
use acme_lib::{Directory, DirectoryUrl, Error as AcmeLibError};

pub struct AcmeLsbClient {
    directory_url: String,
    email: String,
    cache_dir: String,
    is_staging: bool,
    challenge_storage: Arc<Mutex<BTreeMap<String, String>>>,
    cert_pem_cache: Arc<Mutex<BTreeMap<String, (String, String)>>>,
    renewal_threshold: Duration,
}

impl AcmeLsbClient {
    pub fn new(
        directory_url: String,
        email: String,
        cache_dir: String,
        is_staging: bool,
        challenge_storage: Arc<Mutex<BTreeMap<String, String>>>,
    ) -> Self {
        AcmeLsbClient {
            directory_url,
            email,
            cache_dir,
            is_staging,
            challenge_storage,
            cert_pem_cache: Arc::new(Mutex::new(BTreeMap::new())),
            renewal_threshold: Duration::from_secs(30 * 24 * 60 * 60),
        }
    }

    pub fn challenge_storage(&self) -> Arc<Mutex<BTreeMap<String, String>>> {
        self.challenge_storage.clone()
    }

    fn dir(&self) -> Result<Directory<FilePersist>, AcmeLibError> {
        let persist = FilePersist::new(&format!("{}/acme_lib", self.cache_dir));
        let url = if self.is_staging {
            DirectoryUrl::LetsEncryptStaging
        } else {
            DirectoryUrl::LetsEncrypt
        };
        Directory::from_url(persist, url)
    }

    fn email_for(&self, domain: &str) -> String {
        if self.email.is_empty() {
            format!("webmaster@{}", domain)
        } else {
            self.email.clone()
        }
    }

    fn load_from_persist(&self, domain: &str) -> Option<(String, String)> {
        let email = self.email_for(domain);
        let persist = FilePersist::new(&format!("{}/acme_lib", self.cache_dir));
        let key_key = PersistKey::new(&email, acme_lib::persist::PersistKind::PrivateKey, domain);
        let crt_key = PersistKey::new(&email, acme_lib::persist::PersistKind::Certificate, domain);
        match (persist.get(&key_key), persist.get(&crt_key)) {
            (Ok(Some(key)), Ok(Some(cert))) => {
                let key_pem = String::from_utf8(key).ok()?;
                let cert_pem = String::from_utf8(cert).ok()?;
                Some((cert_pem, key_pem))
            }
            _ => None,
        }
    }

    fn save_to_cache(&self, domain: &str, cert_pem: &str, key_pem: &str) {
        if let Ok(mut cache) = self.cert_pem_cache.lock() {
            cache.insert(domain.to_string(), (cert_pem.to_string(), key_pem.to_string()));
        }
        let domain_dir = format!("{}/{}", self.cache_dir, domain);
        let _ = std::fs::create_dir_all(&domain_dir);
        let _ = std::fs::write(format!("{}/fullchain.pem", domain_dir), cert_pem);
        let _ = std::fs::write(format!("{}/privkey.pem", domain_dir), key_pem);
    }

    pub async fn initialize_account(&self) -> Result<(), String> {
        let email = if self.email.is_empty() {
            "webmaster@localhost".to_string()
        } else {
            self.email.clone()
        };
        let _account = self.dir()
            .map_err(|e| format!("directory error: {}", e))?
            .account(&email)
            .map_err(|e| format!("account error: {}", e))?;
        Ok(())
    }

    pub async fn get_certificate_pem(&self, domain: &str) -> Result<(String, String), String> {
        // Check in-memory cache
        {
            let cache = self.cert_pem_cache.lock().map_err(|e| e.to_string())?;
            if let Some(pair) = cache.get(domain) {
                return Ok(pair.clone());
            }
        }

        // Try persistence
        if let Some(pair) = self.load_from_persist(domain) {
            self.save_to_cache(domain, &pair.0, &pair.1);
            return Ok(pair);
        }

        // Request new certificate
        self.request_acme_certificate(domain)
    }

    fn request_acme_certificate(&self, domain: &str) -> Result<(String, String), String> {
        let email = self.email_for(domain);
        let dir = self.dir().map_err(|e| format!("directory: {}", e))?;
        let account = dir.account(&email).map_err(|e| format!("account: {}", e))?;

        let mut order = account.new_order(domain, &[])
            .map_err(|e| format!("new_order: {}", e))?;

        let auths = order.authorizations()
            .map_err(|e| format!("authorizations: {}", e))?;

        for authz in auths {
            if !authz.need_challenge() {
                continue;
            }

            let challenge = authz.http_challenge();
            let token = challenge.http_token().to_string();
            let key_auth = challenge.http_proof();

            {
                let mut storage = self.challenge_storage.lock().map_err(|e| e.to_string())?;
                storage.insert(token.clone(), key_auth.clone());
            }

            challenge.validate(5000)
                .map_err(|e| format!("challenge validate: {}", e))?;

            {
                let mut storage = self.challenge_storage.lock().map_err(|e| e.to_string())?;
                storage.remove(&token);
            }
        }

        order.refresh().map_err(|e| format!("refresh: {}", e))?;

        let csr_order = order.confirm_validations()
            .ok_or_else(|| "validations not confirmed".to_string())?;

        let pkey = acme_lib::create_p384_key();
        let cert_order = csr_order.finalize_pkey(pkey, 5000)
            .map_err(|e| format!("finalize: {}", e))?;

        let cert = cert_order.download_and_save_cert()
            .map_err(|e| format!("download: {}", e))?;

        let cert_pem = cert.certificate().to_string();
        let key_pem = cert.private_key().to_string();

        self.save_to_cache(domain, &cert_pem, &key_pem);
        Ok((cert_pem, key_pem))
    }

    pub fn needs_renewal(&self, domain: &str) -> bool {
        let cert_path = format!("{}/{}/fullchain.pem", self.cache_dir, domain);
        match std::fs::metadata(&cert_path) {
            Ok(meta) => {
                match meta.modified() {
                    Ok(modified) => {
                        SystemTime::now().duration_since(modified).ok()
                            .map(|age| age >= self.renewal_threshold)
                            .unwrap_or(true)
                    }
                    Err(_) => true,
                }
            }
            Err(_) => true,
        }
    }

    pub async fn get_challenge_response(&self, token: &str) -> Option<String> {
        self.challenge_storage.lock().ok()
            .and_then(|s| s.get(token).cloned())
    }
}
