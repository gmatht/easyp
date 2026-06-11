use futures::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(feature = "http2")]
pub use inner::*;

#[cfg(feature = "http2")]
mod inner {
    use super::*;
    use http_dynamic_loader::HttpError;
    use std::path::Path;

    pub struct Http2Handler {
        doc_root: String,
        extension_registry: Option<std::sync::Arc<std::sync::Mutex<crate::ExtensionRegistry>>>,
    }

    impl Http2Handler {
        pub fn try_load() -> Result<Self, HttpError> {
            Ok(Self { doc_root: String::new(), extension_registry: None })
        }

        pub fn set_doc_root<S: Into<String>>(&mut self, path: S) {
            self.doc_root = path.into();
        }

        pub fn set_extension_registry(&mut self, registry: std::sync::Arc<std::sync::Mutex<crate::ExtensionRegistry>>) {
            self.extension_registry = Some(registry);
        }

        /// HTTP/2 over libssl (used when `crypto-lsb` feature is active)
        pub async fn upgrade_connection_tls(
            &mut self,
            tls_stream: &mut crate::openssl_stream::TlsStream,
            _server_name: &str,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.h2_impl(tls_stream).await
        }

        /// Shared HTTP/2 implementation over any AsyncRead+AsyncWrite
        async fn h2_impl(
            &mut self,
            tls_stream: &mut (impl futures::io::AsyncRead + futures::io::AsyncWrite + Unpin),
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut session = http_dynamic_loader::h2::Session::new()?;

            // Send server connection preface (SETTINGS)
            let preface = session.send_preface()?;
            if !preface.is_empty() {
                tls_stream.write_all(&preface).await?;
                tls_stream.flush().await?;
            }

            // Read client data incrementally
            let mut buf = [0u8; 65536];
            loop {
                let n = tls_stream.read(&mut buf).await?;
                if n == 0 { break; }
                session.feed_frame_data(&buf[..n])?;
                if session.has_request() { break; }
            }

            // Get path and serve file
            let path = session.get_path();
            eprintln!("H2: path='{}'", path);
            let doc_root = if self.doc_root.is_empty() {
                Path::new(".")
            } else {
                Path::new(&self.doc_root)
            };
            let file_path = doc_root.join(path.trim_start_matches('/'));
            let body = match std::fs::read(&file_path) {
                Ok(data) => data,
                Err(_) if file_path.is_dir() || path == "/" => {
                    // Try index.html for directory paths
                    match std::fs::read(file_path.join("index.html")) {
                        Ok(data) => data,
                        Err(_) => {
                            "<html><body><h1>404 Not Found</h1></body></html>".to_string().into_bytes()
                        }
                    }
                }
                Err(_) => {
                    "<html><body><h1>404 Not Found</h1></body></html>".to_string().into_bytes()
                }
            };

            // Apply #EXTEND: processing for HTML responses via unblock to avoid
            // blocking the async executor with std::sync::Mutex and disk I/O
            let path = path;
            let body = if is_html_path(&path) {
                let body_str = String::from_utf8_lossy(&body);
                if body_str.contains("#EXTEND:") {
                    if let Some(ref registry) = self.extension_registry {
                        let reg = registry.clone();
                        let p = path.clone();
                        let b = body_str.to_string();
                        let processed = smol::unblock(move || {
                            reg.lock().unwrap().process_html(&b, &p)
                        }).await;
                        processed.into_bytes()
                    } else {
                        body.to_vec()
                    }
                } else {
                    body.to_vec()
                }
            } else {
                body.to_vec()
            };

            // Submit response – this returns all output frames combined.
            // For large bodies this can produce many MB of serialized frames.
            let output = session.submit_body_with_type(&body, &content_type_for_path(&path))?;

            // Write output in chunks to avoid huge TLS records
            for chunk in output.chunks(65536) {
                tls_stream.write_all(chunk).await?;
                tls_stream.flush().await?;
            }

            println!("HTTP/2 served {} bytes for '{}'", body.len(), path);

            smol::Timer::after(std::time::Duration::from_millis(10)).await;
            let _ = tls_stream.close().await;
            Ok(())
        }
    }

    fn is_html_path(path: &str) -> bool {
        path.ends_with(".html") || path.ends_with(".htm")
    }

    fn content_type_for_path(path: &str) -> String {
        if path.ends_with(".html") || path.ends_with(".htm") {
            "text/html".to_string()
        } else if path.ends_with(".css") {
            "text/css".to_string()
        } else if path.ends_with(".js") {
            "application/javascript".to_string()
        } else if path.ends_with(".png") {
            "image/png".to_string()
        } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            "image/jpeg".to_string()
        } else if path.ends_with(".gif") {
            "image/gif".to_string()
        } else if path.ends_with(".svg") {
            "image/svg+xml".to_string()
        } else if path.ends_with(".ico") {
            "image/x-icon".to_string()
        } else if path.ends_with(".json") {
            "application/json".to_string()
        } else if path.ends_with(".txt") {
            "text/plain".to_string()
        } else if path.ends_with(".pdf") {
            "application/pdf".to_string()
        } else {
            "application/octet-stream".to_string()
        }
    }
}

#[cfg(not(feature = "http2"))]
pub struct Http2Handler;

#[cfg(not(feature = "http2"))]
impl Http2Handler {
    pub fn try_load() -> Result<Self, HttpErrorStub> {
        Err(HttpErrorStub)
    }

    pub fn set_doc_root<S: Into<String>>(&mut self, _path: S) {}
}

#[cfg(not(feature = "http2"))]
#[derive(Debug)]
pub struct HttpErrorStub;

#[cfg(not(feature = "http2"))]
impl std::fmt::Display for HttpErrorStub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP/2 not available")
    }
}

#[cfg(not(feature = "http2"))]
impl std::error::Error for HttpErrorStub {}
