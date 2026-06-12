//! Windows http.sys kernel-mode HTTP server handler.
//!
//! Replaces the TCP+OpenSSL listener on Windows with the kernel-mode
//! HTTP Server API (http.sys). http.sys handles:
//! - TCP connection management
//! - TLS handshake & termination (with configured certificates)
//! - HTTP/1.x + HTTP/2 protocol parsing
//! - Request queuing
//!
//! This module only:
//! - Receives pre-parsed HTTP requests from the kernel queue
//! - Dispatches to the easyp business logic (ACME, file server, extensions)
//! - Sends responses back via the kernel API

#![cfg(windows)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::secure_file_server_module::SecureFileServer;

use lsb_httpapi::*;
use lsb_httpapi::HttpVerbGET;
use lsb_httpapi::HTTP_REQUEST_ID;

// ── Helper: extract UTF-16 string from cooked URL ─────────────

fn wide_slice_to_string(ptr: *const u16, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

// ── MIME type mapping ─────────────────────────────────────────

fn mime_for_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

// ── Path resolution (safe) ────────────────────────────────────

fn resolve_path_safe(doc_root: &PathBuf, path: &str) -> PathBuf {
    let clean = path.trim_start_matches('/');
    let mut result = doc_root.clone();
    for component in clean.split('/') {
        if component == ".." || component == "." || component.is_empty() {
            continue;
        }
        result.push(component);
    }
    result
}

// ── HttpSysServer ─────────────────────────────────────────────

pub struct HttpSysServer {
    api: HttpApiServer,
    secure_file_server: SecureFileServer,
    http_challenges: Option<Arc<Mutex<BTreeMap<String, String>>>>,
    #[cfg(feature = "acme")]
    acme_client: Option<crate::AcmeClientType>,
    extension_registry: Option<Arc<Mutex<crate::ExtensionRegistry>>>,
}

impl HttpSysServer {
    pub fn new(
        secure_file_server: SecureFileServer,
        http_challenges: Option<Arc<Mutex<BTreeMap<String, String>>>>,
        #[cfg(feature = "acme")] acme_client: Option<crate::AcmeClientType>,
        extension_registry: Option<Arc<Mutex<crate::ExtensionRegistry>>>,
    ) -> Result<Self, HttpApiError> {
        let api_table = std::sync::Arc::new(HttpApiTable::load()?);
        let api = HttpApiServer::new(api_table)?;
        Ok(HttpSysServer {
            api,
            secure_file_server,
            http_challenges,
            #[cfg(feature = "acme")]
            acme_client,
            extension_registry,
        })
    }

    /// Register URL prefixes with http.sys
    pub fn register_urls(&self, http_port: u16, https_port: u16) -> Result<(), HttpApiError> {
        // IMPORTANT: URLs must be added to the URL group BEFORE binding it to the request queue.
        // Per MSDN sample: HttpAddUrlToUrlGroup → HttpSetUrlGroupProperty (HttpServerBindingProperty).
        let http_url = format!("http://+:{}/", http_port);
        self.api.register_url(&http_url)?;
        log::info!("Registered http.sys URL: {}", http_url);

        let https_url = format!("https://+:{}/", https_port);
        self.api.register_url(&https_url)?;
        log::info!("Registered http.sys URL: {}", https_url);

        // Now bind the URL group to the request queue (must be after adding URLs)
        self.api.bind_queue()?;
        log::info!("Bound URL group to request queue");

        Ok(())
    }

    /// Main request processing loop
    pub fn run(&self) -> Result<(), HttpApiError> {
        log::info!("Starting http.sys request processing loop");
        loop {
            let (req_id, request) = match self.api.receive_request() {
                Ok(r) => r,
                Err(HttpApiError::Win32(ERROR_CONNECTION_INVALID)) => {
                    continue;
                }
                Err(e) => {
                    log::error!("http.sys receive error: {}", e);
                    continue;
                }
            };

            match self.handle_request(req_id, &request) {
                Ok(()) => {}
                Err(HttpApiError::Win32(ERROR_CONNECTION_INVALID)) => {}
                Err(e) => {
                    log::error!("http.sys request handler error: {}", e);
                }
            }
        }
    }

    fn handle_request(
        &self,
        req_id: HTTP_REQUEST_ID,
        request: &HTTP_REQUEST_V1,
    ) -> Result<(), HttpApiError> {
        let path = if request.cooked_url.p_path.is_null() {
            "/".to_string()
        } else {
            wide_slice_to_string(
                request.cooked_url.p_path,
                request.cooked_url.path_length as usize / 2,
            )
        };

        let method = request.verb;

        // ── ACME HTTP-01 challenge handling ─────────────────
        if method == HttpVerbGET && path.starts_with("/.well-known/acme-challenge/") {
            if let Some(ref challenges) = self.http_challenges {
                let token = path.trim_start_matches("/.well-known/acme-challenge/");
                let key_auth = {
                    let map = challenges.lock().unwrap();
                    map.get(token).cloned()
                };
                if let Some(ka) = key_auth {
                    return self.api.send_response(
                        req_id,
                        200,
                        ka.as_bytes(),
                        "application/octet-stream",
                    );
                }
            }
        }

        // ── Extension / CGI request handling ────────────────
        if path.starts_with("/cgi-bin/") {
            return self.handle_extension_request(req_id, &path, method);
        }

        #[cfg(feature = "extensions")]
        {
            let is_admin = self.extension_registry.as_ref().map_or(false, |reg| {
                reg.lock().unwrap().is_admin_path(&path)
            });
            if is_admin {
                return self.handle_extension_request(req_id, &path, method);
            }
        }

        // ── File serving ────────────────────────────────────
        self.serve_file(req_id, &path)
    }

    fn handle_extension_request(
        &self,
        req_id: HTTP_REQUEST_ID,
        path: &str,
        _method: i32,
    ) -> Result<(), HttpApiError> {
        match self.extension_registry {
            Some(ref reg) => {
                let query = path.split('?').nth(1).unwrap_or("");
                let clean_path = path.split('?').next().unwrap_or(path);

                let resp = reg.lock().unwrap().handle_bin_request(
                    clean_path,
                    "GET",
                    query,
                    &std::collections::HashMap::new(),
                );

                match resp {
                    Ok(body) => self.api.send_response(
                        req_id,
                        200,
                        body.as_bytes(),
                        "text/html; charset=utf-8",
                    ),
                    Err(e) => {
                        let msg = format!("Extension error: {}", e);
                        self.api.send_response(
                            req_id,
                            500,
                            msg.as_bytes(),
                            "text/plain; charset=utf-8",
                        )
                    }
                }
            }
            None => self.api.send_response(
                req_id,
                404,
                b"Not Found",
                "text/plain; charset=utf-8",
            ),
        }
    }

    fn serve_file(&self, req_id: HTTP_REQUEST_ID, path: &str) -> Result<(), HttpApiError> {
        let doc_root = self.secure_file_server.config().document_root.clone();
        let file_path = resolve_path_safe(&doc_root, path);

        let body = match std::fs::read(&file_path) {
            Ok(data) => data,
            Err(_) if file_path.is_dir() || path == "/" => {
                let index = file_path.join("index.html");
                match std::fs::read(&index) {
                    Ok(data) => data,
                    Err(_) => {
                        return self.api.send_response(
                            req_id,
                            404,
                            format!("<html><body><h1>404 Not Found</h1><p>{}</p></body></html>", path).as_bytes(),
                            "text/html; charset=utf-8",
                        );
                    }
                }
            }
            Err(_) => {
                return self.api.send_response(
                    req_id,
                    404,
                    format!("<html><body><h1>404 Not Found</h1><p>{}</p></body></html>", path).as_bytes(),
                    "text/html; charset=utf-8",
                );
            }
        };

        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let content_type = mime_for_extension(ext);

        // Apply #EXTEND: processing for HTML
        let final_body = if ext == "html" || ext == "htm" {
            let body_str = String::from_utf8_lossy(&body);
            if body_str.contains("#EXTEND:") {
                if let Some(ref reg) = self.extension_registry {
                    let processed = reg.lock().unwrap().process_html(&body_str, path);
                    processed.into_bytes()
                } else {
                    body
                }
            } else {
                body
            }
        } else {
            body
        };

        self.api.send_response(req_id, 200, &final_body, content_type)
    }

        /// Bind a TLS certificate for an SNI hostname to http.sys.
    /// `thumbprint` is the SHA-1 hash of the cert (20 bytes), `host` is the SNI name
    /// or empty string for default.
    pub fn bind_ssl_cert(
        &self,
        host: &str,
        thumbprint: &[u8; 20],
    ) -> Result<(), HttpApiError> {
        self.api.bind_ssl_cert_sni(host, thumbprint, "MY")
    }

    /// Set up a self-signed certificate for http.sys.
    /// Generates one via PowerShell if none exists, then binds it as the default SNI cert.
    pub fn setup_self_signed_cert(&self, host: &str) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Setting up self-signed certificate for http.sys: {}", host);

        // Call PowerShell to create a self-signed cert if one doesn't exist
        let ps_script = format!(
            r#"
$hostname = '{host}'
$existing = Get-ChildItem "Cert:\LocalMachine\My" | Where-Object {{ $_.DnsNameList -contains $hostname }} | Select-Object -First 1
if (-not $existing) {{
    $cert = New-SelfSignedCertificate -DnsName $hostname -CertStoreLocation "Cert:\LocalMachine\My" -NotAfter (Get-Date).AddYears(10)
    $cert.Thumbprint
}} else {{
    $existing.Thumbprint
}}
"#
        );

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!("PowerShell cert generation failed: {}", stderr);
            return Err(format!("PowerShell cert generation failed: {}", stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let thumbprint_hex = stdout.trim();
        log::info!("Self-signed cert thumbprint: {}", thumbprint_hex);

        let thumbprint = parse_thumbprint(thumbprint_hex)
            .ok_or_else(|| format!("Failed to parse thumbprint: {}", thumbprint_hex))?;

        self.bind_ssl_cert(host, &thumbprint)?;
        log::info!("Self-signed cert bound to http.sys for host '{}'", host);

        Ok(())
    }
}

/// Parse a 40-character hex thumbprint string into a 20-byte array
fn parse_thumbprint(hex: &str) -> Option<[u8; 20]> {
    let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() != 40 {
        return None;
    }
    let mut result = [0u8; 20];
    for i in 0..20 {
        let byte_str = &clean[i * 2..i * 2 + 2];
        result[i] = u8::from_str_radix(byte_str, 16).ok()?;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_thumbprint_valid() {
        let hex = "abcdef1234567890abcdef1234567890abcdef12";
        let result = parse_thumbprint(hex);
        assert!(result.is_some());
        let tp = result.unwrap();
        assert_eq!(tp.len(), 20);
        assert_eq!(tp[0], 0xab);
        assert_eq!(tp[19], 0x12);
    }

    #[test]
    fn parse_thumbprint_with_spaces() {
        let hex = "ab cd ef 12 34 56 78 90 ab cd ef 12 34 56 78 90 ab cd ef 12";
        let result = parse_thumbprint(hex);
        assert!(result.is_some());
    }

    #[test]
    fn parse_thumbprint_too_short() {
        assert!(parse_thumbprint("abcd").is_none());
    }

    #[test]
    fn parse_thumbprint_empty() {
        assert!(parse_thumbprint("").is_none());
    }

    #[test]
    fn mime_for_extension_mappings() {
        assert_eq!(mime_for_extension("html"), "text/html; charset=utf-8");
        assert_eq!(mime_for_extension("css"), "text/css; charset=utf-8");
        assert_eq!(mime_for_extension("js"), "application/javascript");
        assert_eq!(mime_for_extension("png"), "image/png");
        assert_eq!(mime_for_extension("unknown_ext"), "application/octet-stream");
        assert_eq!(mime_for_extension(""), "application/octet-stream");
    }

    #[test]
    fn resolve_path_safe_traversal() {
        let root = PathBuf::from(r"C:\www");
        let result = resolve_path_safe(&root, "/../../../windows/system32");
        assert_eq!(result, PathBuf::from(r"C:\www\windows\system32"));
    }
}
