//! File Caching Utilities
//!
//! This module provides utilities for handling file caching, including
//! Last-Modified timestamps, ETag generation, and conditional request handling.

use std::fs::Metadata;
use std::time::{SystemTime, UNIX_EPOCH};

/// File cache information
#[derive(Debug, Clone)]
pub struct FileCacheInfo {
    /// Last modified timestamp as Unix timestamp
    pub last_modified: u64,
    /// File size in bytes
    pub size: u64,
    /// ETag value for cache validation
    pub etag: String,
}

impl FileCacheInfo {
    /// Create cache info from file metadata
    ///
    /// # Arguments
    /// * `metadata` - File system metadata
    ///
    /// # Returns
    /// * `FileCacheInfo` - Cache information for the file
    pub fn from_metadata(metadata: &Metadata) -> Self {
        let last_modified = metadata
            .modified()
            .unwrap_or_else(|_| SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let size = metadata.len();

        // Generate ETag based on modification time and size
        let etag = format!("\"{}-{}\"", last_modified, size);

        Self {
            last_modified,
            size,
            etag,
        }
    }

    /// Format Last-Modified header value in HTTP format
    ///
    /// # Returns
    /// * `String` - HTTP-formatted date (RFC 7231 format)
    pub fn last_modified_http(&self) -> String {
        // Convert Unix timestamp to HTTP date format (RFC 7231)
        format_http_date_from_timestamp(self.last_modified)
    }

    /// Get cache duration for different file types
    ///
    /// # Arguments
    /// * `content_type` - MIME type of the file
    ///
    /// # Returns
    /// * `i64` - Cache duration in seconds (-1 = cache forever, 0 = no cache)
    pub fn get_cache_duration(&self, content_type: &str) -> i64 {
        match content_type {
            // Static assets - cache for 1 year
            t if t.starts_with("image/") => 31536000, // 1 year
            t if t.starts_with("text/css") => 31536000, // 1 year
            t if t.starts_with("application/javascript") => 31536000, // 1 year
            t if t.starts_with("application/font-") => 31536000, // 1 year
            t if t.starts_with("font/") => 31536000, // 1 year

            // Archives and downloads - cache for 1 day
            t if t.starts_with("application/gzip") => 86400, // 1 day
            t if t.starts_with("application/zip") => 86400, // 1 day
            t if t.starts_with("application/x-tar") => 86400, // 1 day
            t if t.starts_with("application/octet-stream") => 86400, // 1 day

            // HTML files - cache for 1 hour
            t if t.starts_with("text/html") => 3600, // 1 hour

            // JSON/XML - cache for 1 hour
            t if t.starts_with("application/json") => 3600, // 1 hour
            t if t.starts_with("application/xml") => 3600, // 1 hour
            t if t.starts_with("text/xml") => 3600, // 1 hour

            // Default - no cache
            _ => 0,
        }
    }
}

/// Check if a conditional request should return 304 Not Modified
///
/// # Arguments
/// * `cache_info` - File cache information
/// * `if_modified_since` - If-Modified-Since header value
/// * `if_none_match` - If-None-Match header value
///
/// # Returns
/// * `bool` - True if should return 304 Not Modified
pub fn should_return_not_modified(
    cache_info: &FileCacheInfo,
    if_modified_since: Option<&str>,
    if_none_match: Option<&str>,
) -> bool {
    // Check If-None-Match (ETag) first
    if let Some(if_none_match) = if_none_match {
        // Remove quotes if present
        let client_etag = if_none_match.trim_matches('"');
        let server_etag = cache_info.etag.trim_matches('"');

        if client_etag == server_etag {
            return true;
        }
    }

    // Check If-Modified-Since
    if let Some(if_modified_since) = if_modified_since {
        // Parse the If-Modified-Since header
        // For simplicity, we'll compare Unix timestamps
        // In production, you'd want to parse the HTTP date format properly
        if let Ok(client_timestamp) = if_modified_since.parse::<u64>() {
            if client_timestamp >= cache_info.last_modified {
                return true;
            }
        }
    }

    false
}

/// Parse conditional request headers from HTTP request
///
/// # Arguments
/// * `request` - Raw HTTP request string
///
/// # Returns
/// * `(Option<String>, Option<String>)` - (If-Modified-Since, If-None-Match)
pub fn parse_conditional_headers(request: &str) -> (Option<String>, Option<String>) {
    let mut if_modified_since = None;
    let mut if_none_match = None;

    for line in request.lines() {
        if line.starts_with("If-Modified-Since:") {
            if_modified_since = Some(line["If-Modified-Since:".len()..].trim().to_string());
        } else if line.starts_with("If-None-Match:") {
            if_none_match = Some(line["If-None-Match:".len()..].trim().to_string());
        }
    }

    (if_modified_since, if_none_match)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_file_cache_info() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create a test file
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        file.sync_all().unwrap();

        // Get metadata and create cache info
        let metadata = file_path.metadata().unwrap();
        let cache_info = FileCacheInfo::from_metadata(&metadata);

        assert_eq!(cache_info.size, 13);
        assert!(cache_info.last_modified > 0);
        assert!(!cache_info.etag.is_empty());
    }

    #[test]
    fn test_cache_duration() {
        let cache_info = FileCacheInfo {
            last_modified: 1234567890,
            size: 1024,
            etag: "\"1234567890-1024\"".to_string(),
        };

        assert_eq!(cache_info.get_cache_duration("image/png"), 31536000);
        assert_eq!(cache_info.get_cache_duration("text/html"), 3600);
        assert_eq!(cache_info.get_cache_duration("application/gzip"), 86400);
        assert_eq!(cache_info.get_cache_duration("text/plain"), 0);
    }

    #[test]
    fn test_conditional_headers() {
        let request = "GET /test.txt HTTP/1.1\r\n\
                      Host: example.com\r\n\
                      If-Modified-Since: 1234567890\r\n\
                      If-None-Match: \"abc123\"\r\n\
                      \r\n";

        let (if_modified_since, if_none_match) = parse_conditional_headers(request);

        assert_eq!(if_modified_since, Some("1234567890".to_string()));
        assert_eq!(if_none_match, Some("\"abc123\"".to_string()));
    }

    #[test]
    fn test_should_return_not_modified() {
        let cache_info = FileCacheInfo {
            last_modified: 1234567890,
            size: 1024,
            etag: "\"1234567890-1024\"".to_string(),
        };

        // Test ETag match
        assert!(should_return_not_modified(
            &cache_info,
            None,
            Some("\"1234567890-1024\"")
        ));

        // Test timestamp match
        assert!(should_return_not_modified(
            &cache_info,
            Some("1234567890"),
            None
        ));

        // Test no match
        assert!(!should_return_not_modified(
            &cache_info,
            Some("1234567889"),
            Some("\"different-etag\"")
        ));
    }
}

/// Format a Unix timestamp as an HTTP date (RFC 7231)
/// Returns a string in the format: "Day, DD Mon YYYY HH:MM:SS GMT"
fn format_http_date_from_timestamp(timestamp: u64) -> String {
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

    let secs_per_day = 86400u64;
    let secs_per_hour = 3600u64;
    let secs_per_minute = 60u64;

    // Calculate days since epoch
    let days_since_epoch = timestamp / secs_per_day;

    // Calculate day of week (Jan 1, 1970 was a Thursday = 4)
    let day_of_week = ((days_since_epoch + 4) % 7) as usize;

    // Calculate year, month, and day
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Calculate month and day
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0;
    let mut day = remaining_days + 1;

    for (m, &days_in_month) in days_in_months.iter().enumerate() {
        if day <= days_in_month as u64 {
            month = m;
            break;
        }
        day -= days_in_month as u64;
    }

    // Calculate time
    let secs_today = timestamp % secs_per_day;
    let hour = secs_today / secs_per_hour;
    let minute = (secs_today % secs_per_hour) / secs_per_minute;
    let second = secs_today % secs_per_minute;

    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        DAYS[day_of_week],
        day,
        MONTHS[month],
        year,
        hour,
        minute,
        second
    )
}

/// Check if a year is a leap year
fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
