// comment.admin.rs - Admin panel for comment moderation
// Handles comment moderation interface and admin panel functionality

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::collections::HashMap;


// Generate a random 32-character alphanumeric string

// Get or create admin key

// Archive processing file with timestamp
fn archive_processing_file() -> Result<(), String> {
    let comments_dir = Path::new("/var/spool/easyp/comments");
    let processing_file = comments_dir.join("processing");
    
    if processing_file.exists() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_secs();
        
        let archived_file = comments_dir.join(format!("processing.{}", timestamp));
        std::fs::rename(&processing_file, &archived_file)
            .map_err(|e| format!("Failed to archive processing file: {}", e))?;
    }
    
    Ok(())
}

// Create live comments directory
fn create_live_comments_dir() -> Result<(), String> {
    let live_dir = Path::new("/var/spool/easyp/comments/live");
    
    if !live_dir.exists() {
        std::fs::create_dir_all(live_dir)
            .map_err(|e| format!("Failed to create live comments directory: {}", e))?;
        
        // Set ownership to www-data
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(live_dir)
                .map_err(|e| format!("Failed to get metadata for live directory: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(live_dir, perms)
                .map_err(|e| format!("Failed to set permissions for live directory: {}", e))?;
        }
    }
    
    Ok(())
}

// Extract return_url from comment and get MD5 hash
fn get_comment_md5(comment: &str) -> Result<String, String> {
    // Parse comment to extract return_url
    // Comment format: "/cgi-bin/comment?return_url=URL&USER=NAME&TEXT=COMMENT"
    // or cleaned format: "return_url=URL&USER=NAME&TEXT=COMMENT"
    
    // First, extract the query string part after "?" if it exists
    let query_part = if let Some((_, query)) = comment.split_once('?') {
        query
    } else {
        // If no '?' found, assume the comment is already in cleaned format
        comment
    };
    
    let params: std::collections::HashMap<&str, &str> = query_part
        .split('&')
        .filter_map(|pair| {
            if let Some((key, value)) = pair.split_once('=') {
                Some((key, value))
            } else {
                None
            }
        })
        .collect();
    
    let return_url = params.get("return_url")
        .ok_or("No return_url found in comment")?;
    
    // URL decode the return_url
    let decoded_url = url_decode(return_url)?;
    
    // Get canonical path (remove query parameters and fragments)
    let canonical_path = if let Some((path, _)) = decoded_url.split_once('?') {
        if let Some((path, _)) = path.split_once('#') {
            path
        } else {
            path
        }
    } else if let Some((path, _)) = decoded_url.split_once('#') {
        path
    } else {
        &decoded_url
    };
    
    // Calculate MD5 hash
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    canonical_path.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Convert to hex string
    Ok(format!("{:x}", hash))
}

// URL decode function
fn url_decode(s: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = s.chars();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex1 = chars.next().ok_or("Invalid URL encoding")?;
            let hex2 = chars.next().ok_or("Invalid URL encoding")?;
            
            let hex_str = format!("{}{}", hex1, hex2);
            let byte = u8::from_str_radix(&hex_str, 16)
                .map_err(|_| "Invalid hex in URL encoding")?;
            
            result.push(byte as char);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    
    Ok(result)
}

// Move comments from 'in' to 'processing' if processing doesn't exist
fn move_comments_to_processing() -> Result<(), String> {
    let comments_dir = Path::new("/var/spool/easyp/comments");
    let in_file = comments_dir.join("in");
    let processing_file = comments_dir.join("processing");
    
    if in_file.exists() {
        let in_content = fs::read_to_string(&in_file)
            .map_err(|e| format!("Failed to read in file: {}", e))?;
        
        if !in_content.trim().is_empty() {
            // Append to processing file
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&processing_file)
                .map_err(|e| format!("Failed to open processing file: {}", e))?;
            
            file.write_all(in_content.as_bytes())
                .map_err(|e| format!("Failed to write to processing file: {}", e))?;
            
            // Clear the in file
            fs::write(&in_file, "")
                .map_err(|e| format!("Failed to clear in file: {}", e))?;
        }
    }
    
    Ok(())
}

// Get comments from processing file
fn get_comments() -> Result<Vec<String>, String> {
    let processing_file = Path::new("/var/spool/easyp/comments/processing");
    
    if !processing_file.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(processing_file)
        .map_err(|e| format!("Failed to read processing file: {}", e))?;
    
    Ok(content.lines().map(|s| s.to_string()).collect())
}

// Accept a comment (move to accept file)
fn accept_comment(comment: &str) -> Result<(), String> {
    // Ensure live comments directory exists
    create_live_comments_dir()?;
    
    // Remove /cgi-bin/comment? prefix from comment
    let clean_comment = if comment.starts_with("/cgi-bin/comment?") {
        &comment[17..] // Remove "/cgi-bin/comment?" (17 characters)
    } else {
        comment
    };
    
    let comments_dir = Path::new("/var/spool/easyp/comments");
    let accept_file = comments_dir.join("accept");
    
    // Append to accept file
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&accept_file)
        .map_err(|e| format!("Failed to open accept file: {}", e))?;
    
    use std::io::Write;
    writeln!(file, "{}", clean_comment)
        .map_err(|e| format!("Failed to write to accept file: {}", e))?;
    
    // Append to live comments directory based on MD5 of return_url
    match get_comment_md5(clean_comment) {
        Ok(md5_hash) => {
            // Write debug messages to a file
            let debug_file = Path::new("/tmp/comment_debug.log");
            let mut debug_handle = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(debug_file)
                .unwrap_or_else(|_| std::fs::File::create("/tmp/comment_debug_fallback.log").unwrap());
            writeln!(debug_handle, "DEBUG: Comment: {}", comment).unwrap_or_default();
            writeln!(debug_handle, "DEBUG: Clean comment: {}", clean_comment).unwrap_or_default();
            writeln!(debug_handle, "DEBUG: MD5 hash: {}", md5_hash).unwrap_or_default();
            
            let live_file = Path::new("/var/spool/easyp/comments/live").join(&md5_hash);
            writeln!(debug_handle, "DEBUG: Live file path: {:?}", live_file).unwrap_or_default();
            
            let mut live_file_handle = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&live_file)
                .map_err(|e| format!("Failed to open live file: {}", e))?;
            
            writeln!(live_file_handle, "{}", clean_comment)
                .map_err(|e| format!("Failed to write to live file: {}", e))?;
            
            writeln!(debug_handle, "DEBUG: Successfully wrote to live file").unwrap_or_default();
        }
        Err(e) => {
            let debug_file = Path::new("/tmp/comment_debug.log");
            let mut debug_handle = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(debug_file)
                .unwrap_or_else(|_| std::fs::File::create("/tmp/comment_debug_fallback.log").unwrap());
            writeln!(debug_handle, "DEBUG: Failed to get MD5 hash: {}", e).unwrap_or_default();
            writeln!(debug_handle, "DEBUG: Comment was: {}", comment).unwrap_or_default();
        }
    }
    
    Ok(())
}

// Reject a comment (move to reject file)
fn reject_comment(comment: &str) -> Result<(), String> {
    // Remove /cgi-bin/comment? prefix from comment
    let clean_comment = if comment.starts_with("/cgi-bin/comment?") {
        &comment[17..] // Remove "/cgi-bin/comment?" (17 characters)
    } else {
        comment
    };
    
    let comments_dir = Path::new("/var/spool/easyp/comments");
    let reject_file = comments_dir.join("reject");
    
    // Append to reject file
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&reject_file)
        .map_err(|e| format!("Failed to open reject file: {}", e))?;
    
    use std::io::Write;
    writeln!(file, "{}", clean_comment)
        .map_err(|e| format!("Failed to write to reject file: {}", e))?;
    
    Ok(())
}

// Remove comment from processing file

// Generate success page with stats
fn generate_success_page(accepted_count: usize, rejected_count: usize, admin_key: &str) -> Result<String, String> {
    let mut html = String::new();
    
    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<title>Comments Processed Successfully</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
    html.push_str(".success { background-color: #d4edda; border: 1px solid #c3e6cb; color: #155724; padding: 15px; border-radius: 4px; margin: 20px 0; }\n");
    html.push_str(".stats { background-color: #f8f9fa; border: 1px solid #dee2e6; padding: 15px; border-radius: 4px; margin: 20px 0; }\n");
    html.push_str(".file-content { background-color: #f8f9fa; border: 1px solid #dee2e6; padding: 10px; border-radius: 4px; margin: 10px 0; font-family: monospace; white-space: pre-wrap; max-height: 200px; overflow-y: auto; }\n");
    html.push_str(".btn { padding: 10px 20px; margin: 5px; cursor: pointer; border: none; border-radius: 4px; text-decoration: none; display: inline-block; }\n");
    html.push_str(".btn-primary { background-color: #007bff; color: white; }\n");
    html.push_str(".btn-primary:hover { background-color: #0056b3; }\n");
    html.push_str("h2 { color: #333; border-bottom: 2px solid #007bff; padding-bottom: 5px; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    
    html.push_str("<div class=\"success\">\n");
    html.push_str("<h1>[SUCCESS] Comments Processed Successfully!</h1>\n");
    html.push_str("<p>Your moderation actions have been completed.</p>\n");
    html.push_str("</div>\n");
    
    html.push_str("<div class=\"stats\">\n");
    html.push_str("<h2>[STATS] Session Statistics</h2>\n");
    html.push_str(&format!("<p><strong>Comments Accepted:</strong> {}</p>\n", accepted_count));
    html.push_str(&format!("<p><strong>Comments Rejected:</strong> {}</p>\n", rejected_count));
    html.push_str(&format!("<p><strong>Total Processed:</strong> {}</p>\n", accepted_count + rejected_count));
    html.push_str("</div>\n");
    
    // Show last 10 lines of accept file
    html.push_str("<h2>[ACCEPTED] Recent Accepted Comments</h2>\n");
    match get_last_lines("/var/spool/easyp/comments/accept", 10) {
        Ok(lines) => {
            if lines.is_empty() {
                html.push_str("<div class=\"file-content\">No accepted comments yet.</div>\n");
            } else {
                html.push_str("<div class=\"file-content\">");
                for line in lines {
                    html.push_str(&html_escape(&line));
                    html.push_str("\n");
                }
                html.push_str("</div>\n");
            }
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"file-content\">Error reading accept file: {}</div>\n", e));
        }
    }
    
    // Show last 10 lines of reject file
    html.push_str("<h2>[REJECTED] Recent Rejected Comments</h2>\n");
    match get_last_lines("/var/spool/easyp/comments/reject", 10) {
        Ok(lines) => {
            if lines.is_empty() {
                html.push_str("<div class=\"file-content\">No rejected comments yet.</div>\n");
            } else {
                html.push_str("<div class=\"file-content\">");
                for line in lines {
                    html.push_str(&html_escape(&line));
                    html.push_str("\n");
                }
                html.push_str("</div>\n");
            }
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"file-content\">Error reading reject file: {}</div>\n", e));
        }
    }
    
    
    html.push_str("</body>\n");
    html.push_str("</html>\n");
    
    Ok(html)
}

// Get last N lines from a file
fn get_last_lines(file_path: &str, n: usize) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path, e))?;
    
    let lines: Vec<&str> = content.lines().collect();
    let start = if lines.len() > n { lines.len() - n } else { 0 };
    
    Ok(lines[start..].iter().map(|s| s.to_string()).collect())
}

// Generate admin panel HTML
fn generate_admin_panel(comments: &[String], admin_key: &str) -> String {
    let mut html = String::new();
    
    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<title>Comment Moderation Panel</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
    html.push_str(".comment { border: 1px solid #ccc; margin: 10px 0; padding: 10px; }\n");
    html.push_str(".comment-content { margin: 10px 0; }\n");
    html.push_str("input[type=\"checkbox\"] { margin-right: 5px; }\n");
    html.push_str("label { margin-right: 15px; cursor: pointer; }\n");
    html.push_str(".btn { padding: 10px 20px; margin: 5px; cursor: pointer; border: none; border-radius: 4px; }\n");
    html.push_str(".submit { background-color: #2196F3; color: white; font-size: 16px; }\n");
    html.push_str(".submit:hover { background-color: #1976D2; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("<h1>Comment Moderation Panel</h1>\n");
    
    if comments.is_empty() {
        html.push_str("<p>No comments to moderate.</p>\n");
    } else {
        html.push_str("<form method=\"post\">\n");
        for (i, comment) in comments.iter().enumerate() {
            html.push_str(&format!("<div class=\"comment\">\n"));
            html.push_str(&format!("<input type=\"checkbox\" name=\"accept\" value=\"{}\" id=\"accept_{}\">\n", i, i));
            html.push_str(&format!("<label for=\"accept_{}\">Accept</label>\n", i));
            
            // Parse and display comment with proper URL decoding
            let decoded_comment = if let Some(parsed) = parse_comment_for_admin(comment) {
                format!("<strong>User:</strong> {}<br><strong>Comment:</strong> {}", 
                       html_escape(&parsed.name), html_escape(&parsed.text))
            } else {
                html_escape(comment)
            };
            
            html.push_str(&format!("<div class=\"comment-content\">{}</div>\n", decoded_comment));
            html.push_str("</div>\n");
        }
        html.push_str("<div style=\"margin-top: 20px;\">\n");
        html.push_str("<button type=\"submit\" class=\"btn submit\">Process Selected Comments</button>\n");
        html.push_str("</div>\n");
        html.push_str("</form>\n");
    }
    
    
    html.push_str("</body>\n");
    html.push_str("</html>\n");
    
    html
}

// Parse comment for admin display
fn parse_comment_for_admin(comment: &str) -> Option<ParsedComment> {
    // Comment format: "return_url=URL&USER=NAME&TEXT=COMMENT" (prefix already removed)
    
    let params: std::collections::HashMap<&str, &str> = comment
        .split('&')
        .filter_map(|pair| {
            if let Some((key, value)) = pair.split_once('=') {
                Some((key, value))
            } else {
                None
            }
        })
        .collect();
    
    // URL decode the USER and TEXT parameters
    let name = url_decode(params.get("USER")?).ok()?.to_string();
    let text = url_decode(params.get("TEXT")?).ok()?.to_string();
    
    Some(ParsedComment { name, text })
}

// Structure for parsed comment data
struct ParsedComment {
    name: String,
    text: String,
}

// HTML escape function
fn html_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#x27;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

// Parse query string
fn parse_query(query: &str) -> HashMap<String, Vec<String>> {
    let mut params = HashMap::new();
    
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            params.entry(key.to_string())
                .or_insert_with(Vec::new)
                .push(value.to_string());
        }
    }
    
    params
}

// Main admin handler
pub fn handle_comment_admin_request(
    path: &str,
    method: &str,
    _query_string: &str,
    body: &str,
    _headers: &HashMap<String, String>,
    admin_keys: &std::collections::HashMap<String, String>,
) -> Result<String, String> {
    // Check if this looks like a comment admin request
    if !path.starts_with("/comment_") {
        return Err("Not a comment admin request".to_string());
    }
    
    // Get admin key from memory and validate
    println!("DEBUG: Admin keys map: {:?}", admin_keys);
    println!("DEBUG: Looking for 'comment' key in admin_keys");
    let admin_key = admin_keys.get("comment")
        .ok_or("Comment admin key not found".to_string())?;
    println!("DEBUG: Found admin key: {}", admin_key);
    let expected_path = format!("/comment_{}", admin_key);
    
    if path != expected_path {
        return Err("Invalid admin key".to_string());
    }
    
    // Handle POST requests (batch moderation actions)
    if method == "POST" {
        let params = parse_query(body);
        let comments = get_comments()?;
        
        // Get selected accept indices
        let accept_indices: Vec<usize> = params.get("accept")
            .map(|values| values.iter()
                .filter_map(|s| s.parse::<usize>().ok())
                .filter(|&i| i < comments.len())
                .collect())
            .unwrap_or_default();
        
        // Process accepted comments
        for &index in &accept_indices {
            let comment = &comments[index];
            accept_comment(comment)?;
        }
        
        // Process rejected comments (all others)
        for (index, comment) in comments.iter().enumerate() {
            if !accept_indices.contains(&index) {
                reject_comment(comment)?;
            }
        }
        
        // Archive the processing file with timestamp
        if !comments.is_empty() {
            archive_processing_file()?;
        }
        
        // Generate success page with stats
        let success_html = generate_success_page(accept_indices.len(), comments.len() - accept_indices.len(), admin_key)?;
        return Ok(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
            success_html
        ));
    }
    
    // Handle GET requests (display admin panel)
    if method == "GET" {
        // Move comments from 'in' to 'processing' only when viewing the moderation input page
        if let Err(e) = move_comments_to_processing() {
            return Ok(format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h1>Admin Panel</h1><p>Error moving comments: {}</p>",
                e
            ));
        }
        
        let comments = match get_comments() {
            Ok(c) => c,
            Err(e) => {
                return Ok(format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h1>Admin Panel</h1><p>Error getting comments: {}</p>",
                    e
                ));
            }
        };
        
        let html = generate_admin_panel(&comments, admin_key);
        
        return Ok(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
            html
        ));
    }
    
    Err("Method not allowed".to_string())
}

// Get admin paths - always return a pattern that can be checked dynamically
pub fn get_comment_admin_paths() -> Vec<String> {
    // Return patterns that match comment admin requests and done pages
    vec!["/comment_".to_string()]
}
