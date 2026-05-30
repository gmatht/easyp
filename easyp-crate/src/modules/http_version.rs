//! HTTP Version Support
//!
//! This module provides HTTP version parsing and handling for different HTTP protocol versions.

/// HTTP version enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    /// HTTP/0.9 - Simple request/response, no headers
    Http09,
    /// HTTP/1.0 - Status line, headers, optional Keep-Alive
    Http10,
    /// HTTP/1.1 - Persistent connections by default
    Http11,
}

impl HttpVersion {
    /// Parse HTTP version from request line
    ///
    /// # Arguments
    /// * `request_line` - The first line of the HTTP request (e.g., "GET /path HTTP/1.1")
    ///
    /// # Returns
    /// * `HttpVersion` - The parsed HTTP version, defaults to HTTP/0.9 if not found
    pub fn from_request_line(request_line: &str) -> Self {
        // Look for HTTP version in the request line
        if let Some(version_start) = request_line.find("HTTP/") {
            let version_part = &request_line[version_start..];
            if version_part.starts_with("HTTP/1.1") {
                HttpVersion::Http11
            } else if version_part.starts_with("HTTP/1.0") {
                HttpVersion::Http10
            } else {
                // Unknown version, default to HTTP/0.9
                HttpVersion::Http09
            }
        } else {
            // No version found, assume HTTP/0.9
            HttpVersion::Http09
        }
    }

    /// Get the status line prefix for this HTTP version
    ///
    /// # Returns
    /// * `&str` - The status line prefix (e.g., "HTTP/1.1", "HTTP/1.0", or "" for HTTP/0.9)
    pub fn status_line_prefix(&self) -> &'static str {
        match self {
            HttpVersion::Http09 => "",
            HttpVersion::Http10 => "HTTP/1.0",
            HttpVersion::Http11 => "HTTP/1.1",
        }
    }

    /// Check if this version supports headers
    ///
    /// # Returns
    /// * `bool` - True if headers are supported
    pub fn supports_headers(&self) -> bool {
        match self {
            HttpVersion::Http09 => false,
            HttpVersion::Http10 => true,
            HttpVersion::Http11 => true,
        }
    }

    /// Check if this version supports persistent connections by default
    ///
    /// # Returns
    /// * `bool` - True if persistent connections are default
    pub fn supports_persistent_connections(&self) -> bool {
        match self {
            HttpVersion::Http09 => false,
            HttpVersion::Http10 => false,
            HttpVersion::Http11 => true,
        }
    }
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpVersion::Http09 => write!(f, "HTTP/0.9"),
            HttpVersion::Http10 => write!(f, "HTTP/1.0"),
            HttpVersion::Http11 => write!(f, "HTTP/1.1"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http11() {
        let request = "GET /path HTTP/1.1";
        assert_eq!(HttpVersion::from_request_line(request), HttpVersion::Http11);
    }

    #[test]
    fn test_parse_http10() {
        let request = "GET /path HTTP/1.0";
        assert_eq!(HttpVersion::from_request_line(request), HttpVersion::Http10);
    }

    #[test]
    fn test_parse_http09() {
        let request = "GET /path";
        assert_eq!(HttpVersion::from_request_line(request), HttpVersion::Http09);
    }

    #[test]
    fn test_parse_unknown_version() {
        let request = "GET /path HTTP/2.0";
        assert_eq!(HttpVersion::from_request_line(request), HttpVersion::Http09);
    }

    #[test]
    fn test_status_line_prefix() {
        assert_eq!(HttpVersion::Http09.status_line_prefix(), "");
        assert_eq!(HttpVersion::Http10.status_line_prefix(), "HTTP/1.0");
        assert_eq!(HttpVersion::Http11.status_line_prefix(), "HTTP/1.1");
    }

    #[test]
    fn test_supports_headers() {
        assert!(!HttpVersion::Http09.supports_headers());
        assert!(HttpVersion::Http10.supports_headers());
        assert!(HttpVersion::Http11.supports_headers());
    }

    #[test]
    fn test_supports_persistent_connections() {
        assert!(!HttpVersion::Http09.supports_persistent_connections());
        assert!(!HttpVersion::Http10.supports_persistent_connections());
        assert!(HttpVersion::Http11.supports_persistent_connections());
    }
}