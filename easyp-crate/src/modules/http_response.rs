//! HTTP Response Builder
//!
//! This module provides a protocol-agnostic HTTP response builder that can encode
//! responses for different HTTP versions.

use std::collections::HashMap;
use super::http_version::HttpVersion;

/// HTTP response representation that is protocol-agnostic
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code (e.g., 200, 404, 500)
    pub status_code: u16,
    /// HTTP status text (e.g., "OK", "Not Found", "Internal Server Error")
    pub status_text: String,
    /// HTTP headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Create a new HTTP response
    ///
    /// # Arguments
    /// * `status_code` - HTTP status code
    /// * `status_text` - HTTP status text
    /// * `body` - Response body as bytes
    ///
    /// # Returns
    /// * `HttpResponse` - New response instance
    pub fn new(status_code: u16, status_text: &str, body: Vec<u8>) -> Self {
        Self {
            status_code,
            status_text: status_text.to_string(),
            headers: HashMap::new(),
            body,
        }
    }

    /// Create a 200 OK response
    ///
    /// # Arguments
    /// * `body` - Response body as bytes
    ///
    /// # Returns
    /// * `HttpResponse` - New 200 OK response
    pub fn ok(body: Vec<u8>) -> Self {
        Self::new(200, "OK", body)
    }

    /// Create a 404 Not Found response
    ///
    /// # Arguments
    /// * `body` - Response body as bytes
    ///
    /// # Returns
    /// * `HttpResponse` - New 404 Not Found response
    pub fn not_found(body: Vec<u8>) -> Self {
        Self::new(404, "Not Found", body)
    }

    /// Create a 500 Internal Server Error response
    ///
    /// # Arguments
    /// * `body` - Response body as bytes
    ///
    /// # Returns
    /// * `HttpResponse` - New 500 Internal Server Error response
    pub fn internal_server_error(body: Vec<u8>) -> Self {
        Self::new(500, "Internal Server Error", body)
    }

    /// Create a 301 Moved Permanently response
    ///
    /// # Arguments
    /// * `location` - The new location URL
    ///
    /// # Returns
    /// * `HttpResponse` - New 301 Moved Permanently response
    pub fn moved_permanently(location: &str) -> Self {
        let mut response = Self::new(301, "Moved Permanently", Vec::new());
        response.set_header("Location", location);
        response
    }

    /// Create a 304 Not Modified response for conditional requests
    ///
    /// # Arguments
    /// * `last_modified` - Last modified date in HTTP format
    /// * `etag` - ETag value for cache validation
    ///
    /// # Returns
    /// * `HttpResponse` - New 304 Not Modified response
    pub fn not_modified(last_modified: &str, etag: &str) -> Self {
        let mut response = Self::new(304, "Not Modified", Vec::new());
        response.set_last_modified(last_modified);
        response.set_etag(etag);
        response
    }

    /// Set a header
    ///
    /// # Arguments
    /// * `name` - Header name
    /// * `value` - Header value
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }

    /// Set Content-Type header
    ///
    /// # Arguments
    /// * `content_type` - MIME type (e.g., "text/html", "application/json")
    pub fn set_content_type(&mut self, content_type: &str) {
        self.set_header("Content-Type", content_type);
    }

    /// Set Content-Length header based on body size
    pub fn set_content_length(&mut self) {
        self.set_header("Content-Length", &self.body.len().to_string());
    }

    /// Set Cache-Control header
    ///
    /// # Arguments
    /// * `cache_control` - Cache control directive (e.g., "no-cache", "max-age=3600")
    pub fn set_cache_control(&mut self, cache_control: &str) {
        self.set_header("Cache-Control", cache_control);
    }

    /// Set Last-Modified header
    ///
    /// # Arguments
    /// * `last_modified` - Last modified date in HTTP format
    pub fn set_last_modified(&mut self, last_modified: &str) {
        self.set_header("Last-Modified", last_modified);
    }

    /// Set ETag header for cache validation
    ///
    /// # Arguments
    /// * `etag` - ETag value (e.g., "1234567890" or "W/\"weak-etag\"")
    pub fn set_etag(&mut self, etag: &str) {
        self.set_header("ETag", etag);
    }

    /// Add caching headers for static files
    ///
    /// # Arguments
    /// * `last_modified` - Last modified date in HTTP format
    /// * `etag` - ETag value for cache validation
    /// * `cache_duration_seconds` - How long to cache (0 = no cache, -1 = cache forever)
    pub fn add_caching_headers(&mut self, last_modified: &str, etag: &str, cache_duration_seconds: i64) {
        self.set_last_modified(last_modified);
        self.set_etag(etag);

        match cache_duration_seconds {
            -1 => {
                // Cache forever (1 year)
                self.set_cache_control("public, max-age=31536000, immutable");
            },
            0 => {
                // No cache
                self.set_cache_control("no-cache, no-store, must-revalidate");
            },
            duration if duration > 0 => {
                // Cache for specified duration
                self.set_cache_control(&format!("public, max-age={}", duration));
            },
            _ => {
                // Invalid duration, default to no cache
                self.set_cache_control("no-cache");
            }
        }
    }

    /// Add security headers
    pub fn add_security_headers(&mut self) {
        self.set_header("X-Content-Type-Options", "nosniff");
        self.set_header("X-Frame-Options", "DENY");
        self.set_header("X-XSS-Protection", "1; mode=block");
        self.set_cache_control("no-cache");
    }

    /// Encode the response for a specific HTTP version
    ///
    /// # Arguments
    /// * `version` - HTTP version to encode for
    /// * `keep_alive` - Whether to keep the connection alive
    ///
    /// # Returns
    /// * `Vec<u8>` - Encoded response bytes
    pub fn encode(&self, version: &HttpVersion, keep_alive: bool) -> Vec<u8> {
        let mut response = Vec::new();

        if *version == HttpVersion::Http09 {
            response.extend_from_slice(&self.body);
            return response;
        }

        let status_line = format!("{} {} {}\r\n",
            version.status_line_prefix(),
            self.status_code,
            self.status_text
        );
        response.extend_from_slice(status_line.as_bytes());

        for (name, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", name, value);
            response.extend_from_slice(header_line.as_bytes());
        }

        match version {
            HttpVersion::Http11 => {
                if !keep_alive {
                    response.extend_from_slice(b"Connection: close\r\n");
                }
            },
            HttpVersion::Http10 => {
                if keep_alive {
                    response.extend_from_slice(b"Connection: Keep-Alive\r\n");
                }
            },
            HttpVersion::Http2 | HttpVersion::Http3 => {
                // Binary-framed protocols don't use text-based Connection headers.
                // The caller should use the protocol library for the actual framing.
                // This text form is used for HTTP/1 upgrade responses or fallback only.
            },
            HttpVersion::Http09 => {}
        }

        response.extend_from_slice(b"\r\n");
        response.extend_from_slice(&self.body);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http09_encoding() {
        let mut response = HttpResponse::ok(b"Hello World".to_vec());
        response.set_content_type("text/plain");

        let encoded = response.encode(&HttpVersion::Http09, false);
        assert_eq!(encoded, b"Hello World");
    }

    fn check_headers(encoded: &[u8], status: &str, headers: &[&str], body: &str) {
        let s = std::str::from_utf8(encoded).unwrap();
        assert!(s.starts_with(status), "status line mismatch in:\n{}", s);
        for h in headers {
            assert!(s.contains(h), "header '{}' not found in:\n{}", h, s);
        }
        assert!(s.ends_with(body), "body mismatch in:\n{}", s);
    }

    #[test]
    fn test_http10_encoding() {
        let mut response = HttpResponse::ok(b"Hello World".to_vec());
        response.set_content_type("text/plain");
        response.set_content_length();
        let encoded = response.encode(&HttpVersion::Http10, false);
        check_headers(&encoded,
            "HTTP/1.0 200 OK\r\n",
            &["Content-Type: text/plain", "Content-Length: 11"],
            "\r\nHello World");
    }

    #[test]
    fn test_http10_keep_alive() {
        let mut response = HttpResponse::ok(b"Hello World".to_vec());
        response.set_content_type("text/plain");
        response.set_content_length();
        let encoded = response.encode(&HttpVersion::Http10, true);
        check_headers(&encoded,
            "HTTP/1.0 200 OK\r\n",
            &["Content-Type: text/plain", "Content-Length: 11", "Connection: Keep-Alive"],
            "\r\nHello World");
    }

    #[test]
    fn test_http11_encoding() {
        let mut response = HttpResponse::ok(b"Hello World".to_vec());
        response.set_content_type("text/plain");
        response.set_content_length();
        let encoded = response.encode(&HttpVersion::Http11, true);
        check_headers(&encoded,
            "HTTP/1.1 200 OK\r\n",
            &["Content-Type: text/plain", "Content-Length: 11"],
            "\r\nHello World");
    }

    #[test]
    fn test_http11_close() {
        let mut response = HttpResponse::ok(b"Hello World".to_vec());
        response.set_content_type("text/plain");
        response.set_content_length();
        let encoded = response.encode(&HttpVersion::Http11, false);
        check_headers(&encoded,
            "HTTP/1.1 200 OK\r\n",
            &["Content-Type: text/plain", "Content-Length: 11", "Connection: close"],
            "\r\nHello World");
    }
}