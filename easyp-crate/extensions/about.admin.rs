use std::collections::HashMap;

/// Generate the about admin panel HTML
pub fn generate_about_admin_panel(_admin_keys: &std::collections::HashMap<String, String>) -> String {
    let mut html = String::new();

    // Get system information
    let version = env!("CARGO_PKG_VERSION");
    let binary_path = std::env::current_exe().unwrap_or_else(|_| "unknown".into());
    let binary_checksum = get_binary_checksum(&binary_path);
    let build_time = get_build_time();
    let rust_version = get_rust_version();
    let target_arch = std::env::consts::ARCH;
    let target_os = std::env::consts::OS;
    let target_family = std::env::consts::FAMILY;

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html lang=\"en\">\n");
    html.push_str("<head>\n");
    html.push_str("    <meta charset=\"UTF-8\">\n");
    html.push_str("    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("    <title>About - Easyp Admin</title>\n");
    html.push_str("    <style>\n");
    html.push_str("        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }\n");
    html.push_str("        .container { max-width: 1200px; margin: 0 auto; background: white; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); overflow: hidden; }\n");
    html.push_str("        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; text-align: center; }\n");
    html.push_str("        .header h1 { margin: 0; font-size: 2.5em; font-weight: 300; }\n");
    html.push_str("        .header p { margin: 10px 0 0 0; opacity: 0.9; font-size: 1.1em; }\n");
    html.push_str("        .content { padding: 30px; }\n");
    html.push_str("        .section { margin-bottom: 30px; }\n");
    html.push_str("        .section h2 { color: #333; border-bottom: 2px solid #667eea; padding-bottom: 10px; margin-bottom: 20px; font-size: 1.5em; }\n");
    html.push_str("        .info-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; }\n");
    html.push_str("        .info-card { background: #f8f9fa; border: 1px solid #e9ecef; border-radius: 6px; padding: 20px; }\n");
    html.push_str("        .info-card h3 { margin: 0 0 15px 0; color: #495057; font-size: 1.1em; }\n");
    html.push_str("        .info-item { margin-bottom: 10px; }\n");
    html.push_str("        .info-label { font-weight: 600; color: #6c757d; display: inline-block; width: 120px; }\n");
    html.push_str("        .info-value { color: #212529; font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace; word-break: break-all; }\n");
    html.push_str("        .checksum { background: #e9ecef; padding: 8px; border-radius: 4px; font-family: monospace; font-size: 0.9em; word-break: break-all; }\n");
    html.push_str("        .status-indicator { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 8px; }\n");
    html.push_str("        .status-online { background: #28a745; }\n");
    html.push_str("    </style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("    <div class=\"container\">\n");
    html.push_str("        <div class=\"header\">\n");
    html.push_str("            <h1>About Easyp</h1>\n");
    html.push_str("            <p>On-demand HTTPS server with ACME certificate management</p>\n");
    html.push_str("        </div>\n");
    html.push_str("        <div class=\"content\">\n");

    // Version Information
    html.push_str("            <div class=\"section\">\n");
    html.push_str("                <h2>Version Information</h2>\n");
    html.push_str("                <div class=\"info-grid\">\n");
    html.push_str("                    <div class=\"info-card\">\n");
    html.push_str("                        <h3>Application</h3>\n");
    html.push_str("                        <div class=\"info-item\">\n");
    html.push_str("                            <span class=\"info-label\">Version:</span>\n");
    html.push_str("                            <span class=\"info-value\">");
    html.push_str(version);
    html.push_str("</span>\n");
    html.push_str("                        </div>\n");
    html.push_str("                        <div class=\"info-item\">\n");
    html.push_str("                            <span class=\"info-label\">Build Time:</span>\n");
    html.push_str("                            <span class=\"info-value\">");
    html.push_str(&build_time);
    html.push_str("</span>\n");
    html.push_str("                        </div>\n");
    html.push_str("                        <div class=\"info-item\">\n");
    html.push_str("                            <span class=\"info-label\">Status:</span>\n");
    html.push_str("                            <span class=\"status-indicator status-online\"></span>\n");
    html.push_str("                            <span class=\"info-value\">Online</span>\n");
    html.push_str("                        </div>\n");
    html.push_str("                    </div>\n");
    html.push_str("                    <div class=\"info-card\">\n");
    html.push_str("                        <h3>Runtime Environment</h3>\n");
    html.push_str("                        <div class=\"info-item\">\n");
    html.push_str("                            <span class=\"info-label\">Rust Version:</span>\n");
    html.push_str("                            <span class=\"info-value\">");
    html.push_str(&rust_version);
    html.push_str("</span>\n");
    html.push_str("                        </div>\n");
    html.push_str("                        <div class=\"info-item\">\n");
    html.push_str("                            <span class=\"info-label\">Target OS:</span>\n");
    html.push_str("                            <span class=\"info-value\">");
    html.push_str(target_os);
    html.push_str("</span>\n");
    html.push_str("                        </div>\n");
    html.push_str("                        <div class=\"info-item\">\n");
    html.push_str("                            <span class=\"info-label\">Target Arch:</span>\n");
    html.push_str("                            <span class=\"info-value\">");
    html.push_str(target_arch);
    html.push_str("</span>\n");
    html.push_str("                        </div>\n");
    html.push_str("                        <div class=\"info-item\">\n");
    html.push_str("                            <span class=\"info-label\">Target Family:</span>\n");
    html.push_str("                            <span class=\"info-value\">");
    html.push_str(target_family);
    html.push_str("</span>\n");
    html.push_str("                        </div>\n");
    html.push_str("                    </div>\n");
    html.push_str("                </div>\n");
    html.push_str("            </div>\n");

    // Binary Information
    html.push_str("            <div class=\"section\">\n");
    html.push_str("                <h2>Binary Information</h2>\n");
    html.push_str("                <div class=\"info-card\">\n");
    html.push_str("                    <div class=\"info-item\">\n");
    html.push_str("                        <span class=\"info-label\">Path:</span>\n");
    html.push_str("                        <span class=\"info-value\">");
    html.push_str(&binary_path.to_string_lossy());
    html.push_str("</span>\n");
    html.push_str("                    </div>\n");
    html.push_str("                    <div class=\"info-item\">\n");
    html.push_str("                        <span class=\"info-label\">SHA256:</span>\n");
    html.push_str("                        <div class=\"checksum\">");
    html.push_str(&binary_checksum);
    html.push_str("</div>\n");
    html.push_str("                    </div>\n");
    html.push_str("                </div>\n");
    html.push_str("            </div>\n");

    // Command Line Options
    html.push_str("            <div class=\"section\">\n");
    html.push_str("                <h2>Command Line Options</h2>\n");
    html.push_str("                <div class=\"info-card\">\n");
    html.push_str("                    <pre style=\"background: #f8f9fa; padding: 15px; border-radius: 4px; overflow-x: auto; font-size: 0.9em; line-height: 1.4;\">\n");
    html.push_str("USAGE:\n");
    html.push_str("    easyp [OPTIONS] [DOMAINS]...\n\n");
    html.push_str("ARGS:\n");
    html.push_str("    [DOMAINS]...    Optional domains to serve (e.g., example.com, *.example.com)\n");
    html.push_str("                   If not specified, domains will be discovered on-demand from certificate requests\n\n");
    html.push_str("OPTIONS:\n");
    html.push_str("    -p, --port <PORT>                    Port to listen on (legacy, use --https-port instead) [default: 443]\n");
    html.push_str("        --http-port <PORT>               HTTP port [default: 80]\n");
    html.push_str("        --https-port <PORT>              HTTPS port [default: 443]\n");
    html.push_str("        --email <EMAIL>                  Email for ACME certificate registration\n");
    html.push_str("        --staging                         Use Let's Encrypt staging environment\n");
    html.push_str("        --over-9000                       Add 9000 to default port numbers (HTTP: 9080, HTTPS: 9443)\n");
    html.push_str("        --test-client <CLIENT>            Test client binary to run when server is ready\n");
    html.push_str("        --test-root <ROOT>                Test root directory for integration tests [default: test_root]\n");
    html.push_str("        --root <ROOT>                     Document root directory [default: /var/www/html]\n");
    html.push_str("        --allowed-ips <IPS>               Allowed IP addresses for on-demand certificate requests (comma-separated)\n");
    html.push_str("        --cache-dir <DIR>                 Cache directory for ACME certificates [default: /var/lib/easyp/certs]\n");
    html.push_str("    -v, --verbose                        Enable verbose logging\n");
    html.push_str("        --test-mode                       Enable test mode with self-signed certificates\n");
    html.push_str("        --restore-backup                  Restore ACME certificates from backup\n");
    html.push_str("        --bogus-domain <DOMAIN>           Use a bogus domain for testing\n");
    html.push_str("        --acme-directory <URL>            ACME directory URL [default: https://acme-v02.api.letsencrypt.org/directory]\n");
    html.push_str("        --acme-email <EMAIL>              Email for ACME certificate registration (legacy)\n");
    html.push_str("        --challenge-type <TYPE>           ACME challenge type [default: http01]\n");
    html.push_str("        --admin-urls                      Display admin panel URLs and exit\n");
    html.push_str("    -h, --help                           Print help information\n");
    html.push_str("                    </pre>\n");
    html.push_str("                </div>\n");
    html.push_str("            </div>\n");

    // Footer
    html.push_str("            <div class=\"section\">\n");
    html.push_str("                <p style=\"text-align: center; color: #6c757d; font-size: 0.9em;\">\n");
    html.push_str("                    Easyp - On-demand HTTPS server with ACME certificate management\n");
    html.push_str("                </p>\n");
    html.push_str("            </div>\n");

    html.push_str("        </div>\n");
    html.push_str("    </div>\n");
    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

/// Handle about admin panel requests
pub fn handle_about_admin_request(
    _path: &str,
    _method: &str,
    _query: &str,
    _body: &str,
    _headers: &HashMap<String, String>,
    admin_keys: &std::collections::HashMap<String, String>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let html = generate_about_admin_panel(admin_keys);

    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-cache, no-store, must-revalidate\r\n\
         Pragma: no-cache\r\n\
         Expires: 0\r\n\
         \r\n\
         {}",
        html.len(),
        html
    );

    Ok(response)
}

/// Get binary checksum using MD5
fn get_binary_checksum(path: &std::path::Path) -> String {
    use std::fs::File;
    use std::io::Read;

    if let Ok(mut file) = File::open(path) {
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_ok() {
            let digest = md5::compute(&buffer);
            return format!("{:x}", digest);
        }
    }
    "Unable to calculate checksum".to_string()
}

/// Get build time from environment variable
fn get_build_time() -> String {
    std::env::var("BUILD_TIME")
        .unwrap_or_else(|_| "Unknown".to_string())
}

/// Get Rust version
fn get_rust_version() -> String {
    std::env::var("RUSTC_VERSION")
        .unwrap_or_else(|_| "Unknown".to_string())
}

/// Get all admin panel paths for the about extension
pub fn get_about_admin_paths() -> Vec<String> {
    vec!["about".to_string()]
}
