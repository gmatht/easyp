// logs.admin.rs - Admin panel for viewing server logs and output messages
// Provides a comprehensive log viewer with filtering, search, and real-time updates

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// Import file logger to get the log file path
#[path = "../src/modules/file_logger.rs"]
mod file_logger;

// Log entry structure
#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
    source: String,
}

// Log storage structure
#[derive(Debug)]
struct LogStorage {
    entries: Vec<LogEntry>,
    max_entries: usize,
}

impl LogStorage {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    fn add_entry(&mut self, level: String, message: String, source: String) {
        let timestamp = get_current_timestamp();
        let entry = LogEntry {
            timestamp,
            level,
            message,
            source,
        };

        self.entries.push(entry);

        // Keep only the most recent entries
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    fn get_entries(&self, filter: Option<&str>, level_filter: Option<&str>, limit: Option<usize>) -> Vec<LogEntry> {
        let mut filtered_entries = self.entries.clone();

        // Apply level filter
        if let Some(level) = level_filter {
            if level != "all" {
                filtered_entries.retain(|entry| entry.level.to_lowercase() == level.to_lowercase());
            }
        }

        // Apply text filter
        if let Some(filter_text) = filter {
            if !filter_text.is_empty() {
                let filter_lower = filter_text.to_lowercase();
                filtered_entries.retain(|entry|
                    entry.message.to_lowercase().contains(&filter_lower) ||
                    entry.source.to_lowercase().contains(&filter_lower)
                );
            }
        }

        // Apply limit
        if let Some(limit) = limit {
            let start = if filtered_entries.len() > limit {
                filtered_entries.len() - limit
            } else {
                0
            };
            filtered_entries = filtered_entries[start..].to_vec();
        }

        filtered_entries
    }

    fn clear_old_entries(&mut self, older_than_hours: u64) {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - (older_than_hours * 3600);

        self.entries.retain(|entry| {
            // Parse timestamp and compare
            if let Ok(entry_time) = parse_timestamp(&entry.timestamp) {
                entry_time >= cutoff_time
            } else {
                true // Keep entries with unparseable timestamps
            }
        });
    }
}

// Global log storage
use std::sync::OnceLock;

static LOG_STORAGE: OnceLock<Arc<Mutex<LogStorage>>> = OnceLock::new();

fn get_log_storage() -> &'static Arc<Mutex<LogStorage>> {
    LOG_STORAGE.get_or_init(|| Arc::new(Mutex::new(LogStorage::new(10000))))
}

// Custom logger that captures logs
pub struct LogCaptureLogger {
    level: log::Level,
}

impl LogCaptureLogger {
    pub fn new(level: log::Level) -> Self {
        Self { level }
    }
}

impl log::Log for LogCaptureLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level = record.level().to_string();
            let message = format!("{}", record.args());
            let source = format!("{}:{}", record.file().unwrap_or("unknown"), record.line().unwrap_or(0));

            // Add to storage
            if let Ok(mut storage) = get_log_storage().lock() {
                storage.add_entry(level, message, source);
            }

            // Also print to console
            println!("{}: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

// Function to add a log entry manually (for non-log crate messages)
pub fn add_log_entry(level: &str, message: &str, source: &str) {
    if let Ok(mut storage) = get_log_storage().lock() {
        storage.add_entry(level.to_string(), message.to_string(), source.to_string());
    }
}

// Function to read logs from log files
fn read_log_files() -> Vec<LogEntry> {
    let mut entries = Vec::new();

    // Get the actual log file path from the file logger
    let log_file_path = crate::file_logger::get_log_file_path().unwrap_or_else(|| "/tmp/easyp.log".to_string());

    // Common log file locations to check
    let log_files = [
        "/var/log/easyp/server.log",
        "/var/log/easyp/error.log",
        "server.log",
        "server_error.log",
        "/tmp/easyp.log",
        &log_file_path, // Add the actual log file path
    ];

    for log_file in &log_files {
        if let Ok(file) = fs::File::open(log_file) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Some(entry) = parse_log_line(&line, log_file) {
                        entries.push(entry);
                    }
                }
            }
        }
    }

    entries
}

// Parse a log line into a LogEntry
fn parse_log_line(line: &str, source_file: &str) -> Option<LogEntry> {
    // Try to parse common log formats
    // Format: [timestamp] LEVEL message
    if let Some(bracket_end) = line.find(']') {
        if line.starts_with('[') {
            let timestamp_part = &line[1..bracket_end];
            let rest = &line[bracket_end + 1..].trim();

            if let Some(space_pos) = rest.find(' ') {
                let level = rest[..space_pos].to_string();
                let message = rest[space_pos + 1..].to_string();

                return Some(LogEntry {
                    timestamp: timestamp_part.to_string(),
                    level,
                    message,
                    source: source_file.to_string(),
                });
            }
        }
    }

    // Try to parse format: timestamp LEVEL message
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 3 {
        // Check if first part looks like a timestamp (numbers or time format)
        let first_part = parts[0];
        let second_part = parts[1];

        // Check if second part is a log level
        if ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"].contains(&second_part) {
            let timestamp = if first_part.parse::<u64>().is_ok() {
                // It's a Unix timestamp, format it
                format_unix_timestamp(first_part.parse::<u64>().unwrap_or(0))
            } else {
                first_part.to_string()
            };
            let level = second_part.to_string();
            let message = parts[2..].join(" ");

            return Some(LogEntry {
                timestamp,
                level,
                message,
                source: source_file.to_string(),
            });
        }
    }

    // Try to parse format: timestamp LEVEL message (with colon)
    if let Some(colon_pos) = line.find(':') {
        let before_colon = &line[..colon_pos];
        let after_colon = &line[colon_pos + 1..].trim();

        // Check if before_colon contains a timestamp and after_colon starts with a level
        let parts: Vec<&str> = before_colon.split_whitespace().collect();
        if parts.len() >= 2 {
            let timestamp = if parts[0].parse::<u64>().is_ok() {
                // It's a Unix timestamp, format it
                format_unix_timestamp(parts[0].parse::<u64>().unwrap_or(0))
            } else {
                parts[0].to_string()
            };
            let level = parts[1].to_string();

            if ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"].contains(&level.as_str()) {
                let message = after_colon.to_string();

                return Some(LogEntry {
                    timestamp,
                    level,
                    message,
                    source: source_file.to_string(),
                });
            }
        }
    }

    // Fallback: treat entire line as message
    Some(LogEntry {
        timestamp: get_current_timestamp(),
        level: "INFO".to_string(),
        message: line.to_string(),
        source: source_file.to_string(),
    })
}

// Parse timestamp string to unix timestamp
fn parse_timestamp(timestamp_str: &str) -> Result<u64, std::num::ParseIntError> {
    // Try to parse as unix timestamp first
    if let Ok(ts) = timestamp_str.parse::<u64>() {
        return Ok(ts);
    }

    // Try to parse common timestamp formats
    // This is a simplified parser - you might want to use a proper date parsing library
    Ok(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
}

// Get current timestamp as string
fn get_current_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let total_seconds = now.as_secs();
    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// Format Unix timestamp to readable time
fn format_unix_timestamp(timestamp: u64) -> String {
    let hours = (timestamp / 3600) % 24;
    let minutes = (timestamp / 60) % 60;
    let seconds = timestamp % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// Escape HTML content while preserving emojis and other Unicode characters
fn escape_html_preserve_unicode(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#x27;".to_string(),
            _ => c.to_string(), // Preserve all other characters including emojis
        })
        .collect()
}

// Generate the logs admin panel HTML
fn generate_logs_panel(admin_key: &str, filter: Option<&str>, level_filter: Option<&str>, limit: Option<usize>) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<title>Server Logs</title>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<meta http-equiv=\"refresh\" content=\"10\">\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: 'Courier New', monospace; margin: 0; background-color: #1e1e1e; color: #d4d4d4; }\n");
    html.push_str(".container { max-width: 1400px; margin: 0 auto; padding: 20px; }\n");
    html.push_str("h1 { color: #569cd6; border-bottom: 2px solid #007acc; padding-bottom: 10px; margin-bottom: 20px; }\n");
    html.push_str(".controls { background-color: #2d2d30; padding: 15px; border-radius: 5px; margin-bottom: 20px; display: flex; gap: 15px; align-items: center; flex-wrap: wrap; }\n");
    html.push_str(".control-group { display: flex; flex-direction: column; gap: 5px; }\n");
    html.push_str(".control-group label { font-size: 0.9em; color: #cccccc; }\n");
    html.push_str(".control-group input, .control-group select { padding: 8px; border: 1px solid #555; background-color: #3c3c3c; color: #d4d4d4; border-radius: 3px; }\n");
    html.push_str(".control-group input:focus, .control-group select:focus { outline: none; border-color: #007acc; }\n");
    html.push_str(".btn { padding: 8px 16px; background-color: #007acc; color: white; border: none; border-radius: 3px; cursor: pointer; font-size: 0.9em; }\n");
    html.push_str(".btn:hover { background-color: #005a9e; }\n");
    html.push_str(".btn-danger { background-color: #dc3545; }\n");
    html.push_str(".btn-danger:hover { background-color: #c82333; }\n");
    html.push_str(".log-container { background-color: #1e1e1e; border: 1px solid #3c3c3c; border-radius: 5px; overflow: hidden; max-height: 70vh; overflow-y: auto; }\n");
    html.push_str(".log-entry { padding: 8px 12px; border-bottom: 1px solid #2d2d30; display: flex; align-items: flex-start; gap: 10px; font-size: 0.9em; }\n");
    html.push_str(".log-entry:hover { background-color: #2d2d30; }\n");
    html.push_str(".log-timestamp { color: #608b4e; min-width: 80px; font-size: 0.8em; }\n");
    html.push_str(".log-level { min-width: 60px; font-weight: bold; font-size: 0.8em; }\n");
    html.push_str(".log-level.ERROR { color: #f44747; }\n");
    html.push_str(".log-level.WARN { color: #ffcc02; }\n");
    html.push_str(".log-level.INFO { color: #4ec9b0; }\n");
    html.push_str(".log-level.DEBUG { color: #9cdcfe; }\n");
    html.push_str(".log-message { flex: 1; word-break: break-word; }\n");
    html.push_str(".log-source { color: #808080; font-size: 0.8em; min-width: 150px; }\n");
    html.push_str(".stats { background-color: #2d2d30; padding: 10px; border-radius: 5px; margin-bottom: 15px; display: flex; gap: 20px; flex-wrap: wrap; }\n");
    html.push_str(".stat-item { color: #cccccc; font-size: 0.9em; }\n");
    html.push_str(".stat-value { color: #569cd6; font-weight: bold; }\n");
    html.push_str(".no-logs { text-align: center; padding: 40px; color: #808080; font-style: italic; }\n");
    html.push_str(".refresh-info { text-align: center; color: #808080; font-size: 0.8em; margin-top: 15px; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("<div class=\"container\">\n");

    html.push_str("<h1>📋 Server Logs</h1>\n");

    // Get log entries from both storage and files
    let mut all_entries = if let Ok(storage) = get_log_storage().lock() {
        storage.get_entries(None, None, None) // Get all entries from storage first
    } else {
        Vec::new()
    };

    // Also try to read from log files
    let file_entries = read_log_files();
    all_entries.extend(file_entries);

    // Sort by timestamp (newest first)
    all_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Apply filters to the combined list
    let mut filtered_entries = all_entries;

    // Apply level filter
    if let Some(level) = level_filter {
        if level != "all" {
            filtered_entries.retain(|entry| entry.level.to_lowercase() == level.to_lowercase());
        }
    }

    // Apply text filter
    if let Some(filter_text) = filter {
        if !filter_text.is_empty() {
            let filter_lower = filter_text.to_lowercase();
            filtered_entries.retain(|entry|
                entry.message.to_lowercase().contains(&filter_lower) ||
                entry.source.to_lowercase().contains(&filter_lower)
            );
        }
    }

    // Apply limit
    if let Some(limit) = limit {
        let start = if filtered_entries.len() > limit {
            filtered_entries.len() - limit
        } else {
            0
        };
        filtered_entries = filtered_entries[start..].to_vec();
    }

    // Use filtered entries for display
    let all_entries = filtered_entries;

    // Statistics (calculate from all entries before filtering)
    let mut all_unfiltered_entries = if let Ok(storage) = get_log_storage().lock() {
        storage.get_entries(None, None, None)
    } else {
        Vec::new()
    };
    all_unfiltered_entries.extend(read_log_files());

    let total_entries = all_unfiltered_entries.len();
    let error_count = all_unfiltered_entries.iter().filter(|e| e.level == "ERROR").count();
    let warn_count = all_unfiltered_entries.iter().filter(|e| e.level == "WARN").count();
    let info_count = all_unfiltered_entries.iter().filter(|e| e.level == "INFO").count();

    html.push_str("<div class=\"stats\">\n");
    html.push_str(&format!("<div class=\"stat-item\">Total Logs: <span class=\"stat-value\">{}</span></div>\n", total_entries));
    html.push_str(&format!("<div class=\"stat-item\">Errors: <span class=\"stat-value\" style=\"color: #f44747;\">{}</span></div>\n", error_count));
    html.push_str(&format!("<div class=\"stat-item\">Warnings: <span class=\"stat-value\" style=\"color: #ffcc02;\">{}</span></div>\n", warn_count));
    html.push_str(&format!("<div class=\"stat-item\">Info: <span class=\"stat-value\" style=\"color: #4ec9b0;\">{}</span></div>\n", info_count));
    html.push_str("</div>\n");

    // Controls
    html.push_str("<div class=\"controls\">\n");
    html.push_str("<div class=\"control-group\">\n");
    html.push_str("<label for=\"filter\">Search:</label>\n");
    html.push_str(&format!("<input type=\"text\" id=\"filter\" name=\"filter\" value=\"{}\" placeholder=\"Search logs...\">\n",
        filter.unwrap_or("")));
    html.push_str("</div>\n");

    html.push_str("<div class=\"control-group\">\n");
    html.push_str("<label for=\"level\">Level:</label>\n");
    html.push_str("<select id=\"level\" name=\"level\">\n");
    html.push_str(&format!("<option value=\"all\"{}\">All Levels</option>\n",
        if level_filter == Some("all") || level_filter.is_none() { " selected" } else { "" }));
    html.push_str(&format!("<option value=\"ERROR\"{}\">ERROR</option>\n",
        if level_filter == Some("ERROR") { " selected" } else { "" }));
    html.push_str(&format!("<option value=\"WARN\"{}\">WARN</option>\n",
        if level_filter == Some("WARN") { " selected" } else { "" }));
    html.push_str(&format!("<option value=\"INFO\"{}\">INFO</option>\n",
        if level_filter == Some("INFO") { " selected" } else { "" }));
    html.push_str(&format!("<option value=\"DEBUG\"{}\">DEBUG</option>\n",
        if level_filter == Some("DEBUG") { " selected" } else { "" }));
    html.push_str("</select>\n");
    html.push_str("</div>\n");

    html.push_str("<div class=\"control-group\">\n");
    html.push_str("<label for=\"limit\">Limit:</label>\n");
    html.push_str("<select id=\"limit\" name=\"limit\">\n");
    html.push_str(&format!("<option value=\"100\"{}\">100</option>\n",
        if limit == Some(100) || limit.is_none() { " selected" } else { "" }));
    html.push_str(&format!("<option value=\"500\"{}\">500</option>\n",
        if limit == Some(500) { " selected" } else { "" }));
    html.push_str(&format!("<option value=\"1000\"{}\">1000</option>\n",
        if limit == Some(1000) { " selected" } else { "" }));
    html.push_str(&format!("<option value=\"5000\"{}\">5000</option>\n",
        if limit == Some(5000) { " selected" } else { "" }));
    html.push_str("</select>\n");
    html.push_str("</div>\n");

    html.push_str("<button class=\"btn\" onclick=\"applyFilters()\">Apply Filters</button>\n");
    html.push_str("<button class=\"btn btn-danger\" onclick=\"clearLogs()\">Clear Logs</button>\n");
    html.push_str("</div>\n");

    // Log entries
    html.push_str("<div class=\"log-container\">\n");

    if all_entries.is_empty() {
        html.push_str("<div class=\"no-logs\">No log entries found. Logs will appear here as the server processes requests.</div>\n");
    } else {
        for entry in all_entries {
            let level_class = entry.level.to_uppercase();
            html.push_str(&format!(
                "<div class=\"log-entry\">\n\
                <div class=\"log-timestamp\">{}</div>\n\
                <div class=\"log-level {}\">{}</div>\n\
                <div class=\"log-message\">{}</div>\n\
                <div class=\"log-source\">{}</div>\n\
                </div>\n",
                escape_html_preserve_unicode(&entry.timestamp),
                level_class,
                escape_html_preserve_unicode(&entry.level),
                escape_html_preserve_unicode(&entry.message),
                escape_html_preserve_unicode(&entry.source)
            ));
        }
    }

    html.push_str("</div>\n");

    // JavaScript for filtering
    html.push_str("<script>\n");
    html.push_str("function applyFilters() {\n");
    html.push_str("  console.log('applyFilters called');\n");
    html.push_str("  const filter = document.getElementById('filter').value;\n");
    html.push_str("  const level = document.getElementById('level').value;\n");
    html.push_str("  const limit = document.getElementById('limit').value;\n");
    html.push_str("  \n");
    html.push_str("  console.log('Filter values:', { filter, level, limit });\n");
    html.push_str("  \n");
    html.push_str("  let url = window.location.pathname;\n");
    html.push_str("  const params = new URLSearchParams();\n");
    html.push_str("  if (filter) params.append('filter', filter);\n");
    html.push_str("  if (level !== 'all') params.append('level', level);\n");
    html.push_str("  if (limit !== '100') params.append('limit', limit);\n");
    html.push_str("  \n");
    html.push_str("  if (params.toString()) {\n");
    html.push_str("    url += '?' + params.toString();\n");
    html.push_str("  }\n");
    html.push_str("  \n");
    html.push_str("  console.log('Redirecting to:', url);\n");
    html.push_str("  window.location.href = url;\n");
    html.push_str("}\n");
    html.push_str("\n");
    html.push_str("function clearLogs() {\n");
    html.push_str("  console.log('clearLogs called');\n");
    html.push_str("  if (confirm('Are you sure you want to clear all logs? This action cannot be undone.')) {\n");
    html.push_str("    console.log('User confirmed, sending POST request');\n");
    html.push_str("    fetch(window.location.pathname, {\n");
    html.push_str("      method: 'POST',\n");
    html.push_str("      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },\n");
    html.push_str("      body: 'action=clear'\n");
    html.push_str("    }).then(response => {\n");
    html.push_str("      console.log('Response status:', response.status);\n");
    html.push_str("      if (response.ok) {\n");
    html.push_str("        console.log('Logs cleared successfully, reloading page');\n");
    html.push_str("        window.location.reload();\n");
    html.push_str("      } else {\n");
    html.push_str("        console.error('Error response:', response.status, response.statusText);\n");
    html.push_str("        alert('Error clearing logs. Please try again.');\n");
    html.push_str("      }\n");
    html.push_str("    }).catch(error => {\n");
    html.push_str("      console.error('Fetch error:', error);\n");
    html.push_str("      alert('Error clearing logs. Please try again.');\n");
    html.push_str("    });\n");
    html.push_str("  }\n");
    html.push_str("}\n");
    html.push_str("\n");
    html.push_str("// Add event listeners for better UX\n");
    html.push_str("document.addEventListener('DOMContentLoaded', function() {\n");
    html.push_str("  // Auto-apply filters when Enter is pressed in search box\n");
    html.push_str("  const filterInput = document.getElementById('filter');\n");
    html.push_str("  if (filterInput) {\n");
    html.push_str("    filterInput.addEventListener('keypress', function(e) {\n");
    html.push_str("      if (e.key === 'Enter') {\n");
    html.push_str("        applyFilters();\n");
    html.push_str("      }\n");
    html.push_str("    });\n");
    html.push_str("  }\n");
    html.push_str("  \n");
    html.push_str("  // Auto-apply filters when dropdowns change\n");
    html.push_str("  const levelSelect = document.getElementById('level');\n");
    html.push_str("  const limitSelect = document.getElementById('limit');\n");
    html.push_str("  \n");
    html.push_str("  if (levelSelect) {\n");
    html.push_str("    levelSelect.addEventListener('change', function() {\n");
    html.push_str("      applyFilters();\n");
    html.push_str("    });\n");
    html.push_str("  }\n");
    html.push_str("  \n");
    html.push_str("  if (limitSelect) {\n");
    html.push_str("    limitSelect.addEventListener('change', function() {\n");
    html.push_str("      applyFilters();\n");
    html.push_str("    });\n");
    html.push_str("  }\n");
    html.push_str("});\n");
    html.push_str("</script>\n");

    html.push_str("<div class=\"refresh-info\">\n");
    html.push_str("<p>This page refreshes automatically every 10 seconds</p>\n");
    html.push_str(&format!("<p>Last updated: {}</p>\n", get_current_timestamp()));
    html.push_str("</div>\n");

    html.push_str("</div>\n");
    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

// Main admin handler
pub fn handle_logs_admin_request(
    path: &str,
    method: &str,
    query_string: &str,
    body: &str,
    _headers: &HashMap<String, String>,
    admin_keys: &std::collections::HashMap<String, String>,
) -> Result<String, String> {
    // Check if this looks like a logs admin request
    if !path.starts_with("/logs_") {
        return Err("Not a logs admin request".to_string());
    }

    // Get admin key from memory and validate
    let admin_key = admin_keys.get("logs")
        .ok_or("Logs admin key not found".to_string())?;
    let expected_path = format!("/logs_{}", admin_key);

    if path != expected_path {
        return Err("Invalid admin key".to_string());
    }

    // Handle POST requests (clear logs)
    if method == "POST" {
        if body.contains("action=clear") {
            if let Ok(mut storage) = get_log_storage().lock() {
                storage.entries.clear();
            }
            return Ok("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 22\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\nLogs cleared successfully".to_string());
        }
    }

    // Handle GET requests (display logs panel)
    if method == "GET" {
        // Parse query parameters
        let mut filter = None;
        let mut level_filter = None;
        let mut limit = None;

        if !query_string.is_empty() {
            for param in query_string.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    match key {
                        "filter" => filter = Some(value),
                        "level" => level_filter = Some(value),
                        "limit" => limit = value.parse().ok(),
                        _ => {}
                    }
                }
            }
        }

        let html = generate_logs_panel(admin_key, filter, level_filter, limit);

        return Ok(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
            html
        ));
    }

    Err("Method not allowed".to_string())
}

// Get admin paths
pub fn get_logs_admin_paths() -> Vec<String> {
    vec!["/logs_".to_string()]
}
