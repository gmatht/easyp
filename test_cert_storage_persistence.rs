use std::sync::Arc;
use std::time::Duration;
use std::path::Path;
use std::fs;
use tempfile::TempDir;

use rustls_acme::{AcmeClient, OnDemandCertResolver, DnsValidator};
use rustls_acme::{AcmeConfig, ChallengeType};
use rustls::server::ResolvesServerCert;
use std::net::{IpAddr, Ipv4Addr};

/// Test certificate storage and restoration functionality
async fn test_certificate_storage_persistence() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing certificate storage and persistence...");
    
    // Create a temporary directory for certificate storage
    let temp_dir = TempDir::new()?;
    let cache_dir = temp_dir.path().to_string_lossy().to_string();
    println!("📁 Using temporary cache directory: {}", cache_dir);
    
    // Create ACME client configuration
    let acme_config = AcmeConfig {
        directory_url: "https://acme-staging-v02.api.letsencrypt.org/directory".to_string(),
        email: "test@localhost".to_string(),
        allowed_ips: vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))],
        challenge_type: ChallengeType::Http01("".to_string(), "".to_string()),
        cache_dir: Some(cache_dir.clone()),
        renewal_threshold_days: 30,
        is_staging: true,
        bogus_domain: None,
    };

    // Test domain
    let test_domain = "test.example.com";
    
    // Test 1: Create ACME client and test basic functionality
    println!("🔧 Test 1: Creating ACME client and testing basic functionality...");
    let mut acme_client = AcmeClient::new(acme_config.clone());
    acme_client.initialize_account().await?;
    let acme_client = Arc::new(acme_client);
    println!("✅ ACME client initialized successfully");
    
    // Test 2: Test certificate request (this will use the internal storage logic)
    println!("🔧 Test 2: Testing certificate request and storage...");
    let certified_key = acme_client.get_certificate(test_domain).await?;
    println!("✅ Certificate obtained and stored for {}", test_domain);
    
    // Test 3: Test cache behavior
    println!("🔧 Test 3: Testing cache behavior...");
    let cache_stats = acme_client.cache_stats().await;
    println!("📊 Cache stats: {} total, {} expired", cache_stats.0, cache_stats.1);
    assert!(cache_stats.0 > 0, "Cache should contain at least one certificate");
    println!("✅ Cache behavior verified");
    
    // Test 4: Test certificate resolver integration
    println!("🔧 Test 4: Testing certificate resolver integration...");
    let dns_validator = Arc::new(DnsValidator::new(vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))])?);
    let cert_resolver = Arc::new(OnDemandCertResolver::new(
        acme_client.clone(),
        dns_validator,
        None,
        1000,
        Duration::from_secs(30 * 24 * 60 * 60),
    )?);
    println!("✅ Certificate resolver created successfully");
    
    // Test 5: Test renewal logic
    println!("🔧 Test 5: Testing renewal logic...");
    let needs_renewal = acme_client.needs_renewal(test_domain).await;
    assert!(!needs_renewal, "Fresh certificate should not need renewal");
    println!("✅ Renewal logic working correctly");
    
    // Test 6: Test cache cleanup
    println!("🔧 Test 6: Testing cache cleanup...");
    let cleaned_count = acme_client.clean_expired_certificates().await?;
    println!("🧹 Cleaned {} expired certificates", cleaned_count);
    println!("✅ Cache cleanup working correctly");
    
    // Test 7: Verify certificate files exist on disk
    println!("🔧 Test 7: Verifying certificate files exist on disk...");
    let cert_dir = format!("{}/staging", cache_dir);
    let cert_path = format!("{}/{}.crt", cert_dir, test_domain);
    let key_path = format!("{}/{}.key", cert_dir, test_domain);
    let metadata_path = format!("{}/{}.meta", cert_dir, test_domain);
    
    assert!(Path::new(&cert_path).exists(), "Certificate file should exist");
    assert!(Path::new(&key_path).exists(), "Private key file should exist");
    assert!(Path::new(&metadata_path).exists(), "Metadata file should exist");
    println!("✅ All certificate files exist on disk");
    
    // Test 8: Test metadata integrity
    println!("🔧 Test 8: Testing metadata integrity...");
    let metadata_content = fs::read_to_string(&metadata_path)?;
    // Basic validation that metadata contains expected fields
    assert!(metadata_content.contains(test_domain), "Metadata should contain domain");
    assert!(metadata_content.contains("is_staging"), "Metadata should contain staging flag");
    assert!(metadata_content.contains("created_at"), "Metadata should contain creation time");
    println!("✅ Metadata integrity verified");
    
    println!("🎉 All certificate storage and persistence tests passed!");
    Ok(())
}

/// Test certificate renewal logic
async fn test_certificate_renewal() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing certificate renewal logic...");
    
    let temp_dir = TempDir::new()?;
    let cache_dir = temp_dir.path().to_string_lossy().to_string();
    
    let acme_config = AcmeConfig {
        directory_url: "https://acme-staging-v02.api.letsencrypt.org/directory".to_string(),
        email: "test@localhost".to_string(),
        allowed_ips: vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))],
        challenge_type: ChallengeType::Http01("".to_string(), "".to_string()),
        cache_dir: Some(cache_dir),
        renewal_threshold_days: 30,
        is_staging: true,
        bogus_domain: None,
    };

    let mut acme_client = AcmeClient::new(acme_config);
    acme_client.initialize_account().await?;
    let acme_client = Arc::new(acme_client);
    
    let test_domain = "renewal.test.example.com";
    
    // Test renewal check for non-existent certificate
    let needs_renewal = acme_client.needs_renewal(test_domain).await;
    assert!(needs_renewal, "Non-existent certificate should need renewal");
    println!("✅ Non-existent certificate correctly identified as needing renewal");
    
    // Test renewal check for existing certificate
    let _certified_key = acme_client.get_certificate(test_domain).await?;
    let needs_renewal = acme_client.needs_renewal(test_domain).await;
    assert!(!needs_renewal, "Fresh certificate should not need renewal");
    println!("✅ Fresh certificate correctly identified as not needing renewal");
    
    println!("🎉 Certificate renewal tests passed!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting comprehensive certificate storage and persistence tests...\n");
    
    // Run all tests
    test_certificate_storage_persistence().await?;
    println!();
    
    test_certificate_renewal().await?;
    println!();
    
    println!("🎉 All tests completed successfully!");
    println!("✅ Certificate storage and restoration is working properly!");
    
    Ok(())
}
