// all.admin.rs - Master admin panel that links to all other admin panels
// Provides a central hub for accessing all available admin interfaces

use std::collections::HashMap;

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

// Generate the master admin panel HTML with links to all other admin panels
fn generate_all_admin_panel(admin_keys: &std::collections::HashMap<String, String>) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<title>Easyp Admin Dashboard</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }\n");
    html.push_str(".container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }\n");
    html.push_str("h1 { color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; text-align: center; }\n");
    html.push_str(".admin-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin: 30px 0; }\n");
    html.push_str(".admin-card { background-color: #f8f9fa; padding: 25px; border-radius: 8px; border-left: 4px solid #007bff; text-align: center; transition: transform 0.2s ease, box-shadow 0.2s ease; }\n");
    html.push_str(".admin-card:hover { transform: translateY(-2px); box-shadow: 0 4px 8px rgba(0,0,0,0.15); }\n");
    html.push_str(".admin-card h3 { margin-top: 0; color: #333; font-size: 1.4em; }\n");
    html.push_str(".admin-card p { color: #666; margin: 15px 0; line-height: 1.5; }\n");
    html.push_str(".admin-link { display: inline-block; padding: 12px 24px; background-color: #007bff; color: white; text-decoration: none; border-radius: 4px; font-weight: bold; transition: background-color 0.2s ease; }\n");
    html.push_str(".admin-link:hover { background-color: #0056b3; color: white; text-decoration: none; }\n");
    html.push_str(".refresh-info { text-align: center; color: #666; font-size: 0.9em; margin-top: 30px; padding: 15px; background-color: #f8f9fa; border-radius: 4px; }\n");
    html.push_str(".status-indicator { display: inline-block; width: 12px; height: 12px; border-radius: 50%; background-color: #28a745; margin-right: 8px; }\n");
    html.push_str(".welcome-message { text-align: center; margin-bottom: 30px; color: #555; font-size: 1.1em; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("<div class=\"container\">\n");

    html.push_str("<h1>Easyp Admin Dashboard</h1>\n");
    html.push_str("<div class=\"welcome-message\">\n");
    html.push_str("<p>Welcome to the Easyp administration panel. Select an admin interface below to manage different aspects of your server.</p>\n");
    html.push_str("</div>\n");

    html.push_str("<div class=\"admin-grid\">\n");

    // Dynamically generate links for each admin panel
    for (ext_name, key) in admin_keys {
        if ext_name == "all" {
            continue; // Skip the "all" panel itself
        }

        let title = match ext_name.as_str() {
            "comment" => "Comment Moderation",
            "stats" => "System Statistics",
            "upload" => "File Upload Manager",
            "logs" => "Server Logs",
            "about" => "About",
            _ => ext_name,
        };

        let description = match ext_name.as_str() {
            "comment" => "Manage and moderate user comments. Review, approve, or reject comments submitted through the comment system.",
            "stats" => "Monitor system performance, memory usage, CPU load, disk space, and other server statistics in real-time.",
            "upload" => "Upload, manage, and organize files. View uploaded files, delete unwanted files, and monitor storage usage.",
            "logs" => "View and monitor server logs in real-time. Search, filter, and analyze log messages for debugging and monitoring.",
            "about" => "View server information, version details, and system configuration. Learn about the Easyp server and its capabilities.",
            _ => &format!("Manage {} settings and data.", ext_name),
        };

        html.push_str(&format!(
            "<div class=\"admin-card\">\n<h3>{}</h3>\n<p>{}</p>\n<a href=\"/{}_{}\" class=\"admin-link\">Open {} Panel</a>\n</div>\n",
            html_escape(title),
            html_escape(description),
            html_escape(ext_name),
            html_escape(key),
            html_escape(title)
        ));
    }

    html.push_str("</div>\n");

    html.push_str("<div class=\"refresh-info\">\n");
    html.push_str("<span class=\"status-indicator\"></span>\n");
    html.push_str("<strong>System Status:</strong> All admin panels are operational\n");
    html.push_str("<br>\n");
    html.push_str(&format!("Last updated: {}\n", get_current_time()));
    html.push_str("</div>\n");

    html.push_str("</div>\n");
    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

// Get current time in a simple format
fn get_current_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let total_seconds = now.as_secs();
    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// Main admin handler
pub fn handle_all_admin_request(
    path: &str,
    method: &str,
    _query_string: &str,
    _body: &str,
    _headers: &HashMap<String, String>,
    admin_keys: &std::collections::HashMap<String, String>,
) -> Result<String, String> {
    // Check if this looks like an all admin request
    if !path.starts_with("/all_") {
        return Err("Not an all admin request".to_string());
    }

    // Get admin key from memory and validate
    let admin_key = admin_keys.get("all")
        .ok_or("All admin key not found".to_string())?;
    let expected_path = format!("/all_{}", admin_key);

    if path != expected_path {
        return Err("Invalid admin key".to_string());
    }

    // Handle GET requests (display all admin panel)
    if method == "GET" {
        let html = generate_all_admin_panel(admin_keys);

        return Ok(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
            html
        ));
    }

    Err("Method not allowed".to_string())
}

// Get admin paths
pub fn get_all_admin_paths() -> Vec<String> {
    vec!["/all_".to_string()]
}