use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use rustls::server::acme::{AcmeClient, OnDemandCertResolver, DnsValidator};
use rustls::server::acme::types::{AcmeConfig, ChallengeType};
use rustls::server::{Acceptor, ResolvesServerCert};
use rustls::{ServerConfig, ServerConnection};
use std::net::{IpAddr, TcpListener, TcpStream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing ACME certificate conversion...");
    
    // Create ACME client
    let acme_config = AcmeConfig {
        directory_url: "https://acme-staging-v02.api.letsencrypt.org/directory".to_string(),
        email: "test@ca.dansted.org".to_string(),
        allowed_ips: vec![IpAddr::from([72, 11, 150, 147])],
        challenge_type: ChallengeType::Http01("".to_string(), "".to_string()),
        cache_dir: None,
        renewal_threshold_days: 30,
    };

    let mut acme_client = AcmeClient::new(acme_config);
    
    // Initialize ACME account
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        acme_client.initialize_account().await
    }).map_err(|e| format!("Failed to initialize ACME account: {}", e))?;

    let acme_client = Arc::new(acme_client);
    
    // Test certificate request
    let domain = "ca.dansted.org";
    println!("Requesting certificate for domain: {}", domain);
    
    match rt.block_on(acme_client.request_acme_certificate(domain)) {
        Ok(certified_key) => {
            println!("✅ Successfully obtained and converted ACME certificate!");
            println!("Certificate type: {:?}", certified_key.cert().first().map(|c| c.0.len()));
            return Ok(());
        }
        Err(e) => {
            println!("❌ Failed to obtain certificate: {}", e);
            return Err(e.into());
        }
    }
}

