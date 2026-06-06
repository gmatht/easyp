use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    Http09,
    Http10,
    Http11,
    Http2,
    Http3,
}

impl HttpVersion {
    pub fn from_request_line(request_line: &str) -> Self {
        if let Some(version_start) = request_line.find("HTTP/") {
            let version_part = &request_line[version_start..];
            if version_part.starts_with("HTTP/2") {
                HttpVersion::Http2
            } else if version_part.starts_with("HTTP/3") {
                HttpVersion::Http3
            } else if version_part.starts_with("HTTP/1.1") {
                HttpVersion::Http11
            } else if version_part.starts_with("HTTP/1.0") {
                HttpVersion::Http10
            } else {
                HttpVersion::Http09
            }
        } else {
            HttpVersion::Http09
        }
    }

    pub fn status_line_prefix(&self) -> &'static str {
        match self {
            HttpVersion::Http09 => "",
            HttpVersion::Http10 => "HTTP/1.0",
            HttpVersion::Http11 => "HTTP/1.1",
            HttpVersion::Http2 => "",
            HttpVersion::Http3 => "",
        }
    }

}

impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpVersion::Http09 => write!(f, "HTTP/0.9"),
            HttpVersion::Http10 => write!(f, "HTTP/1.0"),
            HttpVersion::Http11 => write!(f, "HTTP/1.1"),
            HttpVersion::Http2 => write!(f, "HTTP/2"),
            HttpVersion::Http3 => write!(f, "HTTP/3"),
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
    fn test_parse_http2() {
        let request = "PRI * HTTP/2.0\r\n";
        assert_eq!(HttpVersion::from_request_line(request), HttpVersion::Http2);
    }

    #[test]
    fn test_parse_http3() {
        let request = "GET /path HTTP/3";
        assert_eq!(HttpVersion::from_request_line(request), HttpVersion::Http3);
    }

    #[test]
    fn test_status_line_prefix() {
        assert_eq!(HttpVersion::Http09.status_line_prefix(), "");
        assert_eq!(HttpVersion::Http10.status_line_prefix(), "HTTP/1.0");
        assert_eq!(HttpVersion::Http11.status_line_prefix(), "HTTP/1.1");
        assert_eq!(HttpVersion::Http2.status_line_prefix(), "");
        assert_eq!(HttpVersion::Http3.status_line_prefix(), "");
    }

    #[test]
    fn test_supports_headers() {
        assert!(!HttpVersion::Http09.supports_headers());
        assert!(HttpVersion::Http10.supports_headers());
        assert!(HttpVersion::Http11.supports_headers());
        assert!(HttpVersion::Http2.supports_headers());
        assert!(HttpVersion::Http3.supports_headers());
    }

    #[test]
    fn test_supports_persistent_connections() {
        assert!(!HttpVersion::Http09.supports_persistent_connections());
        assert!(!HttpVersion::Http10.supports_persistent_connections());
        assert!(HttpVersion::Http11.supports_persistent_connections());
        assert!(HttpVersion::Http2.supports_persistent_connections());
        assert!(HttpVersion::Http3.supports_persistent_connections());
    }

    #[test]
    fn test_is_multiplexed() {
        assert!(!HttpVersion::Http09.is_multiplexed());
        assert!(!HttpVersion::Http10.is_multiplexed());
        assert!(!HttpVersion::Http11.is_multiplexed());
        assert!(HttpVersion::Http2.is_multiplexed());
        assert!(HttpVersion::Http3.is_multiplexed());
    }

    #[test]
    fn test_is_binary_framed() {
        assert!(!HttpVersion::Http09.is_binary_framed());
        assert!(!HttpVersion::Http10.is_binary_framed());
        assert!(!HttpVersion::Http11.is_binary_framed());
        assert!(HttpVersion::Http2.is_binary_framed());
        assert!(HttpVersion::Http3.is_binary_framed());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", HttpVersion::Http09), "HTTP/0.9");
        assert_eq!(format!("{}", HttpVersion::Http10), "HTTP/1.0");
        assert_eq!(format!("{}", HttpVersion::Http11), "HTTP/1.1");
        assert_eq!(format!("{}", HttpVersion::Http2), "HTTP/2");
        assert_eq!(format!("{}", HttpVersion::Http3), "HTTP/3");
    }
}
