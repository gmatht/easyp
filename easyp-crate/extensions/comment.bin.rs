// comment.bin.rs - CGI-like comment handler
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

// Import CgiEnv and url_decode from the main crate
use crate::cgi_env::{CgiEnv, url_decode};

pub fn calculate_md5(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// Count pending comments awaiting moderation
fn count_pending_comments() -> usize {
    let comments_dir = Path::new("/var/spool/easyp/comments");
    let in_file = comments_dir.join("in");

    if !in_file.exists() {
        return 0;
    }

    match fs::read_to_string(&in_file) {
        Ok(content) => {
            if content.trim().is_empty() {
                0
            } else {
                content.lines().filter(|line| !line.trim().is_empty()).count()
            }
        }
        Err(_) => 0
    }
}

/// Main CGI function for comment handling
pub fn cgi_main(env: &CgiEnv) -> Result<String, String> {
    // Parse query parameters
    let params = env.parse_query();

    // Get return URL from query parameters
    let return_url = params.get("return_url")
        .map(|s| url_decode(s))
        .unwrap_or_else(|| "/".to_string());

    // Get USER and TEXT parameters
    let user = params.get("USER")
        .map(|s| url_decode(s))
        .unwrap_or_else(|| "Anonymous".to_string());
    let text = params.get("TEXT")
        .map(|s| url_decode(s))
        .unwrap_or_else(|| "".to_string());

    // Validate required parameters
    if user.trim().is_empty() || text.trim().is_empty() {
        return Ok(format!(
            "HTTP/1.1 400 Bad Request\r\n\
            Content-Type: text/html\r\n\r\n\
            <!DOCTYPE html>\n\
            <html>\n\
            <head><title>Missing Parameters</title></head>\n\
            <body>\n\
            <h1>Missing Required Parameters</h1>\n\
            <p>Both USER and TEXT parameters are required.</p>\n\
            <p><a href=\"{}\">Go back</a></p>\n\
            </body>\n\
            </html>",
            return_url
        ));
    }

    // Create the comment entry in the expected format
    let comment_entry = format!(
        "return_url={}&USER={}&TEXT={}\n",
        return_url, user, text
    );
    // Limit text to 10K characters
    if comment_entry.len() > 10000 {
        return Ok(format!(
            "HTTP/1.1 400 Bad Request\r\n\
            Content-Type: text/html\r\n\r\n\
            <!DOCTYPE html>\n\
            <html>\n\
            <head><title>Comment Too Long</title></head>\n\
            <body>\n\
            <h1>Comment Too Long</h1>\n\
            <p>The comment is too long and was not submitted.</p>\n\
            <p><a href=\"{}\">Go back</a></p>\n\
            </body>\n\
            </html>",
            return_url
        ));
    }

    // Ensure the comments directory exists
    let comments_dir = Path::new("/var/spool/easyp/comments");
    if !comments_dir.exists() {
        std::fs::create_dir_all(comments_dir)
            .map_err(|e| format!("Failed to create comments directory: {}", e))?;
    }

    // Check for duplicate comment using MD5 hashes
    let comments_file = comments_dir.join("in");
    static EXISTING_HASHES: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    let new_comment_hash = calculate_md5(&comment_entry);

    if EXISTING_HASHES.lock().unwrap().contains(&new_comment_hash) {
        return Ok(format!(
            "HTTP/1.1 409 Conflict\r\n\
            Content-Type: text/html\r\n\r\n\
            <!DOCTYPE html>\n\
            <html>\n\
            <head><title>Duplicate Comment</title></head>\n\
            <body>\n\
            <h1>Duplicate Comment Detected</h1>\n\
            <p>This comment is identical to a recent one and was not submitted.</p>\n\
            <p><a href=\"{}\">Go back</a></p>\n\
            </body>\n\
            </html>",
            return_url
        ));
    }


    // Append to the comments file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&comments_file)
        .map_err(|e| format!("Failed to open comments file: {}", e))?;

    file.write_all(comment_entry.as_bytes())
        .map_err(|e| format!("Failed to write comment: {}", e))?;

    EXISTING_HASHES.lock().unwrap().insert(new_comment_hash);
    // Count pending comments for display
    let pending_count = count_pending_comments();

    // Return success response
    Ok(format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: text/html\r\n\r\n\
        <!DOCTYPE html>\n\
        <html>\n\
        <head>\n\
        <title>Comment Submitted</title>\n\
        <style>\n\
        body {{ font-family: Arial, sans-serif; margin: 40px; background-color: #f8f9fa; }}\n\
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}\n\
        .success {{ background-color: #d4edda; border: 1px solid #c3e6cb; color: #155724; padding: 20px; border-radius: 6px; margin: 20px 0; }}\n\
        .info {{ background-color: #e7f3ff; border: 1px solid #b8daff; color: #004085; padding: 15px; border-radius: 6px; margin: 20px 0; }}\n\
        .btn {{ display: inline-block; padding: 12px 24px; background-color: #007bff; color: white; text-decoration: none; border-radius: 4px; margin-top: 15px; }}\n\
        .btn:hover {{ background-color: #0056b3; }}\n\
        h1 {{ color: #28a745; margin-bottom: 20px; }}\n\
        .count {{ font-weight: bold; color: #dc3545; }}\n\
        </style>\n\
        </head>\n\
        <body>\n\
        <div class=\"container\">\n\
        <div class=\"success\">\n\
        <h1>✓ Comment Submitted Successfully</h1>\n\
        <p>Your comment has been recorded and is now awaiting moderation.</p>\n\
        </div>\n\
        <div class=\"info\">\n\
        <p><strong>What happens next?</strong></p>\n\
        <ul>\n\
        <li>Your comment will be reviewed by a moderator</li>\n\
        <li>Once approved, it will appear on the page</li>\n\
        <li>You'll receive an email notification when it's published</li>\n\
        </ul>\n\
        <p>There <span class=\"count\">{}</span> comment{} currently awaiting moderation.</p>\n\
        </div>\n\
        <a href=\"{}\" class=\"btn\">← Go back to the page</a>\n\
        </div>\n\
        </body>\n\
        </html>",
        pending_count,
        if pending_count == 1 { "" } else { "s" },
        return_url
    ))
}

/// Handler function that can be called from the main server
pub fn handle_comment_request(
    method: &str,
    uri: &str,
    host: &str,
    query_string: &str,
    headers: &HashMap<String, String>,
) -> Result<String, String> {
    let env = CgiEnv::from_request(method, uri, host, query_string, headers);
    cgi_main(&env)
}

