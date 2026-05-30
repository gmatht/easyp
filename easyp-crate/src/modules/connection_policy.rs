//! Connection Policy Handler
//!
//! This module handles Keep-Alive decisions based on HTTP version and request headers.

use super::http_version::HttpVersion;

/// Connection policy for determining Keep-Alive behavior
#[derive(Debug, Clone)]
pub struct ConnectionPolicy {
    /// Maximum number of requests per connection
    pub max_requests: usize,
    /// Idle timeout in seconds
    pub idle_timeout_seconds: u64,
}

impl Default for ConnectionPolicy {
    fn default() -> Self {
        Self {
            max_requests: 100,
            idle_timeout_seconds: 5,
        }
    }
}

impl ConnectionPolicy {
    /// Create a new connection policy
    ///
    /// # Arguments
    /// * `max_requests` - Maximum number of requests per connection
    /// * `idle_timeout_seconds` - Idle timeout in seconds
    ///
    /// # Returns
    /// * `ConnectionPolicy` - New connection policy
    pub fn new(max_requests: usize, idle_timeout_seconds: u64) -> Self {
        Self {
            max_requests,
            idle_timeout_seconds,
        }
    }

    /// Determine if connection should be kept alive
    ///
    /// # Arguments
    /// * `version` - HTTP version of the request
    /// * `request_connection_header` - Connection header from request (if any)
    /// * `response_size` - Size of the response in bytes
    /// * `request_count` - Number of requests already handled on this connection
    ///
    /// # Returns
    /// * `bool` - True if connection should be kept alive
    pub fn should_keep_alive(
        &self,
        version: &HttpVersion,
        request_connection_header: Option<&str>,
        response_size: usize,
        request_count: usize,
    ) -> bool {
        // Check if we've exceeded the maximum number of requests
        if request_count >= self.max_requests {
            return false;
        }

        // Check if response is too large (prevent memory issues)
        if response_size > 10 * 1024 * 1024 { // 10MB limit
            return false;
        }

        match version {
            HttpVersion::Http09 => {
                // HTTP/0.9: Never keep alive (no header support)
                false
            },
            HttpVersion::Http10 => {
                // HTTP/1.0: Keep alive only if client explicitly requests it
                if let Some(connection_header) = request_connection_header {
                    connection_header.to_lowercase().contains("keep-alive")
                } else {
                    false
                }
            },
            HttpVersion::Http11 => {
                // HTTP/1.1: Keep alive by default, unless client requests close
                if let Some(connection_header) = request_connection_header {
                    !connection_header.to_lowercase().contains("close")
                } else {
                    true
                }
            }
        }
    }

    /// Parse HTTP version and connection header from request
    ///
    /// # Arguments
    /// * `request` - The full HTTP request as a string
    ///
    /// # Returns
    /// * `(HttpVersion, Option<String>)` - HTTP version and connection header value
    pub fn parse_request_info(request: &str) -> (HttpVersion, Option<String>) {
        let lines: Vec<&str> = request.lines().collect();

        // Parse HTTP version from first line
        let http_version = if let Some(first_line) = lines.first() {
            HttpVersion::from_request_line(first_line)
        } else {
            HttpVersion::Http09
        };

        // Parse Connection header
        let connection_header = lines.iter()
            .find(|line| line.to_lowercase().starts_with("connection:"))
            .and_then(|line| {
                line.splitn(2, ':').nth(1).map(|value| value.trim().to_string())
            });

        (http_version, connection_header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http09_never_keep_alive() {
        let policy = ConnectionPolicy::default();
        assert!(!policy.should_keep_alive(&HttpVersion::Http09, None, 1000, 1));
        assert!(!policy.should_keep_alive(&HttpVersion::Http09, Some("keep-alive"), 1000, 1));
    }

    #[test]
    fn test_http10_keep_alive_only_if_requested() {
        let policy = ConnectionPolicy::default();
        assert!(!policy.should_keep_alive(&HttpVersion::Http10, None, 1000, 1));
        assert!(!policy.should_keep_alive(&HttpVersion::Http10, Some("close"), 1000, 1));
        assert!(policy.should_keep_alive(&HttpVersion::Http10, Some("Keep-Alive"), 1000, 1));
        assert!(policy.should_keep_alive(&HttpVersion::Http10, Some("keep-alive"), 1000, 1));
    }

    #[test]
    fn test_http11_keep_alive_by_default() {
        let policy = ConnectionPolicy::default();
        assert!(policy.should_keep_alive(&HttpVersion::Http11, None, 1000, 1));
        assert!(policy.should_keep_alive(&HttpVersion::Http11, Some("keep-alive"), 1000, 1));
        assert!(!policy.should_keep_alive(&HttpVersion::Http11, Some("close"), 1000, 1));
        assert!(!policy.should_keep_alive(&HttpVersion::Http11, Some("Close"), 1000, 1));
    }

    #[test]
    fn test_max_requests_limit() {
        let policy = ConnectionPolicy::new(5, 10);
        assert!(policy.should_keep_alive(&HttpVersion::Http11, None, 1000, 4));
        assert!(!policy.should_keep_alive(&HttpVersion::Http11, None, 1000, 5));
        assert!(!policy.should_keep_alive(&HttpVersion::Http11, None, 1000, 6));
    }

    #[test]
    fn test_large_response_closes_connection() {
        let policy = ConnectionPolicy::default();
        assert!(policy.should_keep_alive(&HttpVersion::Http11, None, 5 * 1024 * 1024, 1));
        assert!(!policy.should_keep_alive(&HttpVersion::Http11, None, 11 * 1024 * 1024, 1));
    }

    #[test]
    fn test_parse_request_info() {
        let request = "GET /path HTTP/1.1\r\nHost: example.com\r\nConnection: keep-alive\r\n\r\n";
        let (version, connection) = ConnectionPolicy::parse_request_info(request);
        assert_eq!(version, HttpVersion::Http11);
        assert_eq!(connection, Some("keep-alive".to_string()));
    }

    #[test]
    fn test_parse_request_info_no_connection_header() {
        let request = "GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let (version, connection) = ConnectionPolicy::parse_request_info(request);
        assert_eq!(version, HttpVersion::Http11);
        assert_eq!(connection, None);
    }

    #[test]
    fn test_parse_request_info_http09() {
        let request = "GET /path\r\n";
        let (version, connection) = ConnectionPolicy::parse_request_info(request);
        assert_eq!(version, HttpVersion::Http09);
        assert_eq!(connection, None);
    }
}