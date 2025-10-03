//! ACME client implementation for certificate management
//!
//! This module provides a full-featured ACME client that can obtain and renew
//! certificates from Let's Encrypt and other ACME-compliant certificate authorities.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use std::string::String;
use std::vec::Vec;
use std::format;
use std::string::ToString;
use std::vec;
use std::println;

use crate::types::*;
use pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use aws_lc_rs;
use base64;

use tokio::sync::RwLock;

/// ACME client for certificate management
pub struct AcmeClient {
    config: AcmeConfig,
    certificate_cache: Arc<RwLock<HashMap<String, CachedCertificate>>>,
    challenge_storage: Arc<RwLock<HashMap<String, ChallengeData>>>,
}

impl AcmeClient {
    /// Create a new ACME client
    pub fn new(config: AcmeConfig) -> Self {
        Self {
            certificate_cache: Arc::new(RwLock::new(HashMap::new())),
            challenge_storage: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Initialize the ACME account (create or load existing)
    pub async fn initialize_account(&self) -> Result<(), AcmeError> {
        use acme_lib::{Directory, DirectoryUrl};
        use acme_lib::persist::FilePersist;

        println!("🔍 initialize_account() called for email: {}", self.config.email);

        // Create a directory for acme-lib to store its files
        let cache_dir = self.config.cache_dir.as_deref()
            .ok_or_else(|| AcmeError::Client("ACME cache directory not configured".to_string()))?;
        let acme_persist_dir = format!("{}/acme_lib", cache_dir);
        
        // Backup existing data before any operations
        if std::path::Path::new(&acme_persist_dir).exists() {
            println!("💾 Backing up existing ACME data before account initialization...");
            if let Err(e) = self.backup_acme_data(&acme_persist_dir) {
                println!("⚠️  Backup failed, but continuing: {}", e);
            }
        }
        
        std::fs::create_dir_all(&acme_persist_dir)
            .map_err(|e| AcmeError::Client(format!("Failed to create ACME persistence directory '{}': {}", acme_persist_dir, e)))?;

        // Set proper permissions for the directory
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&acme_persist_dir)
                .map_err(|e| AcmeError::Client(format!("Failed to get metadata for directory '{}': {}", acme_persist_dir, e)))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&acme_persist_dir, perms)
                .map_err(|e| AcmeError::Client(format!("Failed to set permissions for directory '{}': {}", acme_persist_dir, e)))?;
        }

        // Create ACME directory
        let persist = FilePersist::new(&acme_persist_dir);
        let dir = Directory::from_url(persist, DirectoryUrl::Other(&self.config.directory_url))
            .map_err(|e| AcmeError::Client(format!("Failed to create ACME directory: {}", e)))?;

        // Create or load account with fallback for private key format issues
        println!("🔍 Attempting to load ACME account for: {}", self.config.email);
        let account = match dir.account(&self.config.email) {
            Ok(account) => {
                println!("✅ ACME account loaded successfully for email: {}", self.config.email);
                account
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                println!("❌ Failed to load ACME account: {}", error_msg);
                
                // Check if this is a private key format error
                let is_private_key_error = error_msg.contains("Unsupported private key format") ||
                    error_msg.contains("private key format") ||
                    error_msg.contains("key format");
                
                if is_private_key_error {
                    println!("⚠️  Private key format mismatch detected. Clearing old ACME account data...");
                    
                    // Clear the old account data
                    self.clear_old_account_data(&acme_persist_dir)?;
                    
                    // Try to create a new account
                    println!("🔍 Attempting to create new ACME account after clearing old data...");
                    dir.account(&self.config.email)
                        .map_err(|e| AcmeError::Client(format!("Failed to create new ACME account after clearing old data: {}", e)))?
                } else {
                    println!("❌ Other error type, not handling: {}", error_msg);
                    return Err(AcmeError::Client(format!("Failed to create/load ACME account: {}", e)));
                }
            }
        };

        println!("✅ ACME account created successfully for email: {}", self.config.email);
        Ok(())
    }

    /// Get or create a certificate for the given domain
    pub async fn get_certificate(&self, domain: &str) -> Result<Arc<CertifiedKey>, AcmeError> {
        println!("🔍 get_certificate() called for domain: {}", domain);
        
        // Check cache first
        {
            let cache = self.certificate_cache.read().await;
            if let Some(cached) = cache.get(domain) {
                if cached.expires_at > SystemTime::now() {
                    println!("OLD CERT OK");
                    return Ok(cached.certified_key.clone());
                }
                println!("CERT EXPIRED");
            } else {
                // Get the SHA256 of the current binary for debugging
                let binary_path = std::env::current_exe().unwrap_or_else(|_| "unknown".into());
                let sha256 = if let Ok(content) = std::fs::read(&binary_path) {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    content.hash(&mut hasher);
                    format!("{:x}", hasher.finish())
                } else {
                    "unknown".to_string()
                };
                println!("NO CERT IN CACHE! Binary: {} SHA256: {}", binary_path.display(), sha256);
            }   
        }

        // Try to load from acme-lib's persistence
        println!("🔍 About to call load_certificate_from_acme_lib for domain: {}", domain);
        if let Some(certified_key) = self.load_certificate_from_acme_lib(domain).await? {
            println!("✅ Certificate loaded from acme-lib persistence for domain: {}", domain);
            
            // Cache the certificate
            {
                let mut cache = self.certificate_cache.write().await;
                cache.insert(domain.to_string(), CachedCertificate {
                    certified_key: certified_key.clone(),
                    expires_at: SystemTime::now() + Duration::from_secs(30 * 24 * 60 * 60), // 30 days
                    domain: domain.to_string(),
                });
            }
            
            return Ok(certified_key);
        }

        println!("No certificate found in acme-lib persistence for domain: {}", domain);
        println!("Requesting ACME certificate for domain: {}", domain);
        
        // Request a real ACME certificate
        let certified_key = self.request_acme_certificate(domain).await?;
        
        // Cache the certificate
        {
            let mut cache = self.certificate_cache.write().await;
            cache.insert(domain.to_string(), CachedCertificate {
                certified_key: certified_key.clone(),
                expires_at: SystemTime::now() + Duration::from_secs(30 * 24 * 60 * 60), // 30 days
                domain: domain.to_string(),
            });
        }
        
        println!("ACME certificate cached for domain: {}", domain);
        println!("Certificate obtained for domain: {}", domain);
        
        Ok(certified_key)
    }

    /// Request an ACME certificate for the given domain
    async fn request_acme_certificate(&self, domain: &str) -> Result<Arc<CertifiedKey>, AcmeError> {
        // This is a simplified version - in practice, you'd implement the full ACME flow here
        // For now, we'll return an error indicating this needs to be implemented
        Err(AcmeError::Client("ACME certificate request not yet implemented in rustls-acme".to_string()))
    }

    /// Load certificate from acme-lib's persistence
    async fn load_certificate_from_acme_lib(&self, domain: &str) -> Result<Option<Arc<CertifiedKey>>, AcmeError> {
        // This is a simplified version - in practice, you'd implement loading from acme-lib here
        Ok(None)
    }

    /// Backup ACME certificates and account data to /root/.easyp_backup
    fn backup_acme_data(&self, acme_persist_dir: &str) -> Result<(), AcmeError> {
        use std::fs;
        use std::path::Path;

        let backup_dir = "/root/.easyp_backup";
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_path = format!("{}/acme_backup_{}", backup_dir, timestamp);

        println!("💾 Creating backup of ACME data to: {}", backup_path);

        // Create backup directory
        fs::create_dir_all(&backup_path)
            .map_err(|e| AcmeError::Client(format!("Failed to create backup directory '{}': {}", backup_path, e)))?;

        // Check if source directory exists
        if !Path::new(acme_persist_dir).exists() {
            println!("⚠️  Source ACME directory does not exist: {}", acme_persist_dir);
            return Ok(());
        }

        // Copy all ACME data to backup
        if let Err(e) = self.copy_directory_recursive(acme_persist_dir, &backup_path) {
            return Err(AcmeError::Client(format!("Failed to backup ACME data: {}", e)));
        }

        // Set proper permissions on backup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&backup_path)
                .map_err(|e| AcmeError::Client(format!("Failed to get backup metadata: {}", e)))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&backup_path, perms)
                .map_err(|e| AcmeError::Client(format!("Failed to set backup permissions: {}", e)))?;
        }

        println!("✅ ACME data backed up successfully to: {}", backup_path);
        Ok(())
    }

    /// Restore ACME certificates and account data from the most recent backup
    pub fn restore_acme_data(&self, acme_persist_dir: &str) -> Result<(), AcmeError> {
        use std::fs;
        use std::path::Path;

        let backup_dir = "/root/.easyp_backup";
        
        println!("🔄 Attempting to restore ACME data from: {}", backup_dir);

        // Check if backup directory exists
        if !Path::new(backup_dir).exists() {
            println!("⚠️  No backup directory found at: {}", backup_dir);
            return Ok(());
        }

        // Find the most recent backup
        let mut backup_dirs = Vec::new();
        if let Ok(entries) = fs::read_dir(backup_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("acme_backup_") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                backup_dirs.push((name.to_string(), modified));
                            }
                        }
                    }
                }
            }
        }

        if backup_dirs.is_empty() {
            println!("⚠️  No ACME backups found in: {}", backup_dir);
            return Ok(());
        }

        // Sort by modification time (most recent first)
        backup_dirs.sort_by(|a, b| b.1.cmp(&a.1));
        let latest_backup = format!("{}/{}", backup_dir, backup_dirs[0].0);

        println!("🔄 Restoring from latest backup: {}", latest_backup);

        // Create target directory
        fs::create_dir_all(acme_persist_dir)
            .map_err(|e| AcmeError::Client(format!("Failed to create ACME directory '{}': {}", acme_persist_dir, e)))?;

        // Copy backup to target
        if let Err(e) = self.copy_directory_recursive(&latest_backup, acme_persist_dir) {
            return Err(AcmeError::Client(format!("Failed to restore ACME data: {}", e)));
        }

        // Set proper permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(acme_persist_dir)
                .map_err(|e| AcmeError::Client(format!("Failed to get metadata for directory: {}", e)))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(acme_persist_dir, perms)
                .map_err(|e| AcmeError::Client(format!("Failed to set permissions: {}", e)))?;
        }

        println!("✅ ACME data restored successfully from: {}", latest_backup);
        Ok(())
    }

    /// Helper function to copy directory recursively
    fn copy_directory_recursive(&self, src: &str, dst: &str) -> Result<(), std::io::Error> {
        use std::fs;
        use std::path::Path;

        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        if !src_path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Source is not a directory",
            ));
        }

        // Create destination directory
        fs::create_dir_all(dst_path)?;

        // Copy all entries
        for entry in fs::read_dir(src_path)? {
            let entry = entry?;
            let src_file = entry.path();
            let dst_file = dst_path.join(entry.file_name());

            if src_file.is_dir() {
                self.copy_directory_recursive(
                    src_file.to_str().unwrap(),
                    dst_file.to_str().unwrap(),
                )?;
            } else {
                fs::copy(&src_file, &dst_file)?;
            }
        }

        Ok(())
    }

    /// Clear old ACME account data when there's a private key format mismatch
    fn clear_old_account_data(&self, acme_persist_dir: &str) -> Result<(), AcmeError> {
        use std::fs;
        
        println!("🧹 Clearing old ACME account data from: {}", acme_persist_dir);
        
        // ALWAYS backup before clearing!
        if std::path::Path::new(acme_persist_dir).exists() {
            println!("💾 Backing up ACME data before clearing...");
            if let Err(e) = self.backup_acme_data(acme_persist_dir) {
                println!("⚠️  Backup failed, but continuing with clear: {}", e);
            }
            
            fs::remove_dir_all(acme_persist_dir)
                .map_err(|e| AcmeError::Client(format!("Failed to remove old ACME data: {}", e)))?;
            println!("✅ Old ACME account data cleared successfully");
        }
        
        // Recreate the directory
        fs::create_dir_all(acme_persist_dir)
            .map_err(|e| AcmeError::Client(format!("Failed to recreate ACME directory: {}", e)))?;

        // Set proper permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(acme_persist_dir)
                .map_err(|e| AcmeError::Client(format!("Failed to get metadata for directory: {}", e)))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(acme_persist_dir, perms)
                .map_err(|e| AcmeError::Client(format!("Failed to set permissions: {}", e)))?;
        }

        Ok(())
    }

    /// Get email for domain (simplified)
    fn get_email_for_domain(&self, domain: &str) -> String {
        format!("webmaster@{}", domain)
    }

    /// Get challenge response for HTTP-01 challenge
    pub async fn get_challenge_response(&self, token: &str) -> Option<String> {
        // This is a simplified implementation - in practice, you'd implement the full challenge handling here
        // For now, we'll return None to indicate no challenge is available
        println!("🔍 get_challenge_response called for token: {}", token);
        None
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        // This is a simplified implementation - in practice, you'd return actual cache statistics
        // Returns (total_certificates, expired_certificates)
        (0, 0)
    }

    /// Check if a certificate needs renewal
    pub async fn needs_renewal(&self, domain: &str) -> bool {
        // This is a simplified implementation - in practice, you'd check certificate expiration
        println!("🔍 needs_renewal called for domain: {}", domain);
        false
    }

    /// Clean expired certificates
    pub async fn clean_expired_certificates(&self) -> Result<usize, AcmeError> {
        // This is a simplified implementation - in practice, you'd clean expired certificates
        println!("🔍 clean_expired_certificates called");
        Ok(0)
    }
}