// upload.admin.rs - Admin panel for file uploads
// Handles file upload interface and admin panel functionality

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

// Create upload directory if it doesn't exist
fn ensure_upload_directory() -> Result<PathBuf, String> {
    let upload_dir = Path::new("/var/www/html/uploads");

    if !upload_dir.exists() {
        fs::create_dir_all(upload_dir)
            .map_err(|e| format!("Failed to create upload directory: {}", e))?;

        // Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(upload_dir)
                .map_err(|e| format!("Failed to get metadata for upload directory: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(upload_dir, perms)
                .map_err(|e| format!("Failed to set permissions for upload directory: {}", e))?;
        }
    }

    Ok(upload_dir.to_path_buf())
}

// Get list of uploaded files
fn get_uploaded_files() -> Result<Vec<UploadedFile>, String> {
    let upload_dir = ensure_upload_directory()?;
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(&upload_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(metadata) = fs::metadata(&path) {
                        files.push(UploadedFile {
                            name: file_name.to_string(),
                            size: metadata.len(),
                            modified: metadata.modified()
                                .map_err(|e| format!("Failed to get modification time: {}", e))?
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_err(|e| format!("Failed to get timestamp: {}", e))?
                                .as_secs(),
                        });
                    }
                }
            }
        }
    }

    // Sort by modification time (newest first)
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(files)
}

// Structure for uploaded file info
#[derive(Debug)]
struct UploadedFile {
    name: String,
    size: u64,
    modified: u64,
}

// Format file size in human readable format
fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

// Format timestamp in human readable format
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};

    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp);
    let system_time = SystemTime::from(datetime);

    // Simple formatting - in production you might want to use a proper date library
    let days_since_epoch = timestamp / 86400;
    let seconds_today = timestamp % 86400;
    let hours = seconds_today / 3600;
    let minutes = (seconds_today % 3600) / 60;
    let seconds = seconds_today % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// Generate upload form HTML
fn generate_upload_form(admin_key: &str) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<title>File Upload Manager</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }\n");
    html.push_str(".container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }\n");
    html.push_str("h1 { color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }\n");
    html.push_str(".upload-section { background-color: #f8f9fa; padding: 20px; border-radius: 4px; margin: 20px 0; }\n");
    html.push_str(".file-list { margin: 20px 0; }\n");
    html.push_str(".file-item { display: flex; justify-content: space-between; align-items: center; padding: 10px; border: 1px solid #dee2e6; margin: 5px 0; border-radius: 4px; background: white; }\n");
    html.push_str(".file-info { flex: 1; }\n");
    html.push_str(".file-name { font-weight: bold; color: #333; }\n");
    html.push_str(".file-meta { color: #666; font-size: 0.9em; margin-top: 5px; }\n");
    html.push_str(".file-actions { display: flex; gap: 10px; }\n");
    html.push_str(".btn { padding: 8px 16px; border: none; border-radius: 4px; cursor: pointer; text-decoration: none; display: inline-block; font-size: 14px; }\n");
    html.push_str(".btn-primary { background-color: #007bff; color: white; }\n");
    html.push_str(".btn-primary:hover { background-color: #0056b3; }\n");
    html.push_str(".btn-danger { background-color: #dc3545; color: white; }\n");
    html.push_str(".btn-danger:hover { background-color: #c82333; }\n");
    html.push_str(".btn-success { background-color: #28a745; color: white; }\n");
    html.push_str(".btn-success:hover { background-color: #218838; }\n");
    html.push_str("input[type=\"file\"] { margin: 10px 0; }\n");
    html.push_str(".upload-form { margin: 20px 0; }\n");
    html.push_str(".empty-state { text-align: center; color: #666; padding: 40px; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("<div class=\"container\">\n");

    html.push_str("<h1>📂 File Upload Manager</h1>\n");

    // Upload form
    html.push_str("<div class=\"upload-section\">\n");
    html.push_str("<h2>Upload New File</h2>\n");
    html.push_str("<form method=\"post\" enctype=\"multipart/form-data\" class=\"upload-form\">\n");
    html.push_str("<input type=\"hidden\" name=\"action\" value=\"upload\">\n");
    html.push_str("<input type=\"file\" name=\"file\" required>\n");
    html.push_str("<br>\n");
    html.push_str("<button type=\"submit\" class=\"btn btn-primary\">Upload File</button>\n");
    html.push_str("</form>\n");
    html.push_str("</div>\n");

    // File list
    html.push_str("<div class=\"file-list\">\n");
    html.push_str("<h2>Uploaded Files</h2>\n");

    match get_uploaded_files() {
        Ok(files) => {
            if files.is_empty() {
                html.push_str("<div class=\"empty-state\">\n");
                html.push_str("<p>No files uploaded yet.</p>\n");
                html.push_str("</div>\n");
            } else {
                for file in files {
                    html.push_str("<div class=\"file-item\">\n");
                    html.push_str("<div class=\"file-info\">\n");
                    html.push_str(&format!("<div class=\"file-name\">{}</div>\n", html_escape(&file.name)));
                    html.push_str(&format!("<div class=\"file-meta\">Size: {} | Modified: {}</div>\n",
                                         format_file_size(file.size), format_timestamp(file.modified)));
                    html.push_str("</div>\n");
                    html.push_str("<div class=\"file-actions\">\n");
                    html.push_str(&format!("<a href=\"/uploads/{}\" class=\"btn btn-success\" target=\"_blank\">View</a>\n",
                                         html_escape(&file.name)));
                    html.push_str(&format!("<a href=\"/upload_{}?action=delete&file={}\" class=\"btn btn-danger\" onclick=\"return confirm('Are you sure you want to delete this file?')\">Delete</a>\n",
                                         admin_key, html_escape(&file.name)));
                    html.push_str("</div>\n");
                    html.push_str("</div>\n");
                }
            }
        }
        Err(e) => {
            html.push_str(&format!("<div class=\"empty-state\">\n"));
            html.push_str(&format!("<p>Error loading files: {}</p>\n", html_escape(&e)));
            html.push_str("</div>\n");
        }
    }

    html.push_str("</div>\n");
    html.push_str("</div>\n");
    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

// Generate success page after upload
fn generate_upload_success(filename: &str, admin_key: &str) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<title>Upload Successful</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; text-align: center; }\n");
    html.push_str(".success { background-color: #d4edda; border: 1px solid #c3e6cb; color: #155724; padding: 20px; border-radius: 4px; margin: 20px 0; }\n");
    html.push_str(".btn { padding: 10px 20px; margin: 10px; border: none; border-radius: 4px; cursor: pointer; text-decoration: none; display: inline-block; }\n");
    html.push_str(".btn-primary { background-color: #007bff; color: white; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");

    html.push_str("<div class=\"success\">\n");
    html.push_str("<h1>✅ Upload Successful!</h1>\n");
    html.push_str(&format!("<p>File <strong>{}</strong> has been uploaded successfully.</p>\n", html_escape(filename)));
    html.push_str("</div>\n");

    html.push_str(&format!("<a href=\"/upload_{}\" class=\"btn btn-primary\">Back to Upload Manager</a>\n", admin_key));

    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

// Generate delete success page
fn generate_delete_success(filename: &str, admin_key: &str) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<title>File Deleted</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; text-align: center; }\n");
    html.push_str(".success { background-color: #d4edda; border: 1px solid #c3e6cb; color: #155724; padding: 20px; border-radius: 4px; margin: 20px 0; }\n");
    html.push_str(".btn { padding: 10px 20px; margin: 10px; border: none; border-radius: 4px; cursor: pointer; text-decoration: none; display: inline-block; }\n");
    html.push_str(".btn-primary { background-color: #007bff; color: white; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");

    html.push_str("<div class=\"success\">\n");
    html.push_str("<h1>🗑️ File Deleted</h1>\n");
    html.push_str(&format!("<p>File <strong>{}</strong> has been deleted successfully.</p>\n", html_escape(filename)));
    html.push_str("</div>\n");

    html.push_str(&format!("<a href=\"/upload_{}\" class=\"btn btn-primary\">Back to Upload Manager</a>\n", admin_key));

    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

// Parse multipart form data (simplified)
fn parse_multipart_data(body: &str, boundary: &str) -> Result<HashMap<String, String>, String> {
    let mut data = HashMap::new();

    // Find the file part
    let file_start = body.find(&format!("name=\"file\""));
    if let Some(start) = file_start {
        // Find the filename
        if let Some(filename_start) = body[start..].find("filename=\"") {
            let filename_start = start + filename_start + 10;
            if let Some(filename_end) = body[filename_start..].find("\"") {
                let filename = &body[filename_start..filename_start + filename_end];
                data.insert("filename".to_string(), filename.to_string());
            }
        }

        // Find the file content (simplified - in production you'd want proper multipart parsing)
        if let Some(content_start) = body[start..].find("\r\n\r\n") {
            let content_start = start + content_start + 4;
            if let Some(content_end) = body[content_start..].find(&format!("--{}", boundary)) {
                let content = &body[content_start..content_start + content_end];
                data.insert("content".to_string(), content.to_string());
            }
        }
    }

    Ok(data)
}

// Save uploaded file
fn save_uploaded_file(filename: &str, content: &str) -> Result<(), String> {
    let upload_dir = ensure_upload_directory()?;
    let file_path = upload_dir.join(filename);

    // Basic security check - prevent directory traversal
    if filename.contains("..") || filename.contains("/") || filename.contains("\\") {
        return Err("Invalid filename".to_string());
    }

    // Check file size (10MB limit)
    if content.len() > 10 * 1024 * 1024 {
        return Err("File too large (max 10MB)".to_string());
    }

    fs::write(&file_path, content)
        .map_err(|e| format!("Failed to save file: {}", e))?;

    Ok(())
}

// Delete uploaded file
fn delete_uploaded_file(filename: &str) -> Result<(), String> {
    let upload_dir = ensure_upload_directory()?;
    let file_path = upload_dir.join(filename);

    // Basic security check
    if filename.contains("..") || filename.contains("/") || filename.contains("\\") {
        return Err("Invalid filename".to_string());
    }

    if file_path.exists() {
        fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;
    }

    Ok(())
}

// Parse query string
fn parse_query(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            params.insert(key.to_string(), value.to_string());
        }
    }

    params
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

// Main admin handler
pub fn handle_upload_admin_request(
    path: &str,
    method: &str,
    query_string: &str,
    body: &str,
    headers: &HashMap<String, String>,
    admin_keys: &std::collections::HashMap<String, String>,
) -> Result<String, String> {
    // Check if this looks like an upload admin request
    if !path.starts_with("/upload_") {
        return Err("Not an upload admin request".to_string());
    }

    // Get admin key from memory and validate
    let admin_key = admin_keys.get("upload")
        .ok_or("Upload admin key not found".to_string())?;
    let expected_path = format!("/upload_{}", admin_key);

    if path != expected_path {
        return Err("Invalid admin key".to_string());
    }

    // Handle POST requests (file upload)
    if method == "POST" {
        // Parse multipart data
        let content_type = headers.get("content-type")
            .ok_or("Content-Type header missing".to_string())?;

        if !content_type.starts_with("multipart/form-data") {
            return Err("Invalid content type".to_string());
        }

        let boundary = if let Some(boundary_start) = content_type.find("boundary=") {
            &content_type[boundary_start + 9..]
        } else {
            return Err("Boundary not found".to_string());
        };

        let data = parse_multipart_data(body, boundary)?;
        let filename = data.get("filename")
            .ok_or("No filename provided".to_string())?;
        let content = data.get("content")
            .ok_or("No file content provided".to_string())?;

        // Save the file
        save_uploaded_file(filename, content)?;

        // Generate success page
        let success_html = generate_upload_success(filename, admin_key);
        return Ok(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
            success_html
        ));
    }

    // Handle GET requests (display upload manager or handle delete)
    if method == "GET" {
        let params = parse_query(query_string);

        // Handle delete action
        if params.get("action") == Some(&"delete".to_string()) {
            if let Some(filename) = params.get("file") {
                delete_uploaded_file(filename)?;
                let success_html = generate_delete_success(filename, admin_key);
                return Ok(format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
                    success_html
                ));
            }
        }

        // Display upload manager
        let html = generate_upload_form(admin_key);
        return Ok(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
            html
        ));
    }

    Err("Method not allowed".to_string())
}

// Get admin paths
pub fn get_upload_admin_paths() -> Vec<String> {
    vec!["/upload_".to_string()]
}

