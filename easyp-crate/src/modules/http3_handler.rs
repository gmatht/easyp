// HTTP/3 handler — subprocess approach with embedded gtlsserver.
// C helper (h3_gnutls.c) is compiled but not yet used — available for native porting.
use std::sync::Arc;
use std::net::SocketAddr;
use std::process::{Command, Child, Stdio};
use std::io::{BufRead, BufReader};

use super::secure_file_server_module::{SecureFileServer, SecurityConfig};
use super::hourly_stats::HourlyStatsCollector;

include!(concat!(env!("OUT_DIR"), "/gtlsserver_embedded.rs"));

#[cfg(feature = "http3")]
pub use inner::*;

#[cfg(feature = "http3")]
mod inner {
    use super::*;

    pub struct Http3Handler {
        child: Option<Child>,
    }

    unsafe impl Send for Http3Handler {}
    unsafe impl Sync for Http3Handler {}

    fn extract_and_prepare_libs() -> Result<String, String> {
        if GTLSSERVER.is_empty() {
            return Err("not embedded".into());
        }
        let tmp = std::env::temp_dir().join("easyp-h3");
        let _ = std::fs::create_dir_all(&tmp);
        let bin = tmp.join("gtlsserver");
        if !bin.exists() {
            std::fs::write(&bin, GTLSSERVER).map_err(|e| format!("write gtlsserver: {}", e))?;
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).ok();
            }
            // Extract required shared libraries alongside the binary
            for (data, name) in &[(LIBNGTCP2, "libngtcp2.so.9"),
                                 (LIBNGTCP2_CRYPTO, "libngtcp2_crypto_gnutls.so.2"),
                                 (LIBGNUTLS, "libgnutls.so.30")] {
                if !data.is_empty() {
                    let _ = std::fs::write(tmp.join(name), data);
                }
            }
        }
        // Set LD_LIBRARY_PATH so the subprocess finds our .so files first
        let libpath = tmp.to_string_lossy().to_string();
        let prev = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let new_path = if prev.is_empty() { libpath } else { format!("{}:{}", libpath, prev) };
        std::env::set_var("LD_LIBRARY_PATH", &new_path);
        Ok(bin.to_string_lossy().to_string())
    }

    impl Http3Handler {
        pub async fn new(
            _file_server: Arc<SecureFileServer>,
            _stats_collector: Arc<HourlyStatsCollector>,
            _security_config: SecurityConfig,
            bind_addr: SocketAddr,
            cert_file: &str,
            key_file: &str,
        ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            let exe = match extract_and_prepare_libs() {
                Ok(e) => e,
                Err(_) => {
                    // Fallback: search known paths (for development environments)
                    let mut found = None;
                    for p in &["/tmp/ngtcp2_build/examples/gtlsserver", "/usr/local/bin/gtlsserver",
                                 "/usr/bin/gtlsserver", "/opt/gtlsserver"] {
                        if std::path::Path::new(p).exists() { found = Some(p.to_string()); break; }
                    }
                    match found {
                        Some(e) => e,
                        None => {
                            eprintln!("H3: gtlsserver not available — HTTP/3 disabled");
                            return Ok(Http3Handler { child: None });
                        }
                    }
                }
            };
            match Command::new(&exe)
                .arg("-q")
                .arg(bind_addr.ip().to_string())
                .arg(bind_addr.port().to_string())
                .arg(key_file).arg(cert_file)
                .stdout(Stdio::null()).stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => Ok(Http3Handler { child: Some(child) }),
                Err(e) => {
                    eprintln!("H3 init: spawn failed ({}) — HTTP/3 disabled", e);
                    Ok(Http3Handler { child: None })
                }
            }
        }

        pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            if let Some(ref mut child) = self.child {
                let stderr = match child.stderr.take() {
                    Some(s) => s,
                    None => return Ok(()),
                };
                std::thread::spawn(move || {
                    for line in BufReader::new(stderr).lines().flatten() {
                        eprintln!("H3: {}", line);
                    }
                });
                match child.wait() {
                    Ok(status) => eprintln!("H3: exited with {}", status),
                    Err(e) => eprintln!("H3: wait error: {}", e),
                }
            }
            Ok(())
        }
    }

    impl Drop for Http3Handler {
        fn drop(&mut self) {
            if let Some(ref mut child) = self.child {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    pub type GnutlsCrypto = ();
    pub type GnutlsDatum = ();
}
