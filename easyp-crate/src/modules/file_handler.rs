//! File Handler Module
//!
//! This module provides centralized file serving logic for both HTTP and HTTPS connections.
//! It handles domain-based document root selection, file serving, and response generation.


/// Extract domain from HTTP Host header
pub fn extract_domain_from_host_header(request: &str) -> Option<String> {
    for line in request.lines() {
        if line.to_lowercase().starts_with("host:") {
            let host = line.split(':').nth(1)?.trim();
            // Remove port if present
            let domain = host.split(':').next()?.to_string();
            return Some(domain);
        }
    }
    None
}


