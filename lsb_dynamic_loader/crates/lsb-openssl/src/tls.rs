//! Cross-platform TLS connector and stream.
//!
//! Provides a unified API that:
//! - On Unix/Linux: uses the existing `lsb_openssl` (OpenSSL via dlopen)
//! - On Windows: uses `lsb-schannel` (SChannel via compile-time link)
//!
//! # Usage (synchronous, blocking)
//! ```rust,ignore
//! use lsb_openssl::tls::TlsConnector;
//!
//! let connector = TlsConnector::new(true)?;
//! let tls = connector.connect(tcp_stream, "example.org")?;
//! // tls implements Read + Write
//! ```

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TlsError {
    #[error("need more data to read (would block)")]
    WantRead,
    #[error("need to write pending data (would block)")]
    WantWrite,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TLS protocol error: {0}")]
    Protocol(String),
    #[error("connection closed")]
    Closed,
    #[error("feature not available on this platform")]
    NotSupported,
}

// ── Platform dispatch ────────────────────────────────────────

#[cfg(unix)]
mod platform {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use super::TlsError;

    pub struct TlsConnector {
        pub(crate) openssl: &'static crate::Openssl,
        pub(crate) alpn: Option<Vec<Vec<u8>>>,
        pub(crate) is_client: bool,
        pub(crate) cert_path: Option<String>,
        pub(crate) key_path: Option<String>,
    }

    impl TlsConnector {
        pub fn new(is_client: bool) -> Result<Self, TlsError> {
            let openssl = crate::Openssl::global()
                .map_err(|e| TlsError::Protocol(format!("Failed to load OpenSSL: {e}")))?;
            openssl.init()
                .map_err(|e| TlsError::Protocol(format!("OpenSSL init failed: {e}")))?;
            Ok(TlsConnector {
                openssl,
                alpn: None,
                is_client,
                cert_path: None,
                key_path: None,
            })
        }

        pub fn set_alpn(&mut self, protocols: &[&[u8]]) {
            self.alpn = Some(protocols.iter().map(|p| p.to_vec()).collect());
        }

        pub fn set_certificate(&mut self, cert_pem: &str, key_pem: &str) {
            // Write to temp files for OpenSSL's file-based API
            let cert_path = format!("/tmp/lsb_tls_cert_{}.pem", std::process::id());
            let key_path = format!("/tmp/lsb_tls_key_{}.pem", std::process::id());
            let _ = std::fs::write(&cert_path, cert_pem);
            let _ = std::fs::write(&key_path, key_pem);
            self.cert_path = Some(cert_path);
            self.key_path = Some(key_path);
        }

        pub fn connect(&self, stream: TcpStream, hostname: &str) -> Result<TlsStream, TlsError> {
            use std::os::unix::io::AsRawFd;
            let fd = stream.as_raw_fd();
            // Set non-blocking so the TLS handshake completes via internal BIO,
            // but we do it in a blocking fashion by making the fd blocking temporarily
            // Actually, the caller is responsible for blocking mode.
            // For the synchronous case, we assume the stream is in blocking mode.
            // For async, the wrapper handles this.

            let ctx = self.openssl.ctx_new(self.is_client)
                .map_err(|e| TlsError::Protocol(format!("ctx_new: {e}")))?;

            if let Some(ref alpn) = self.alpn {
                let protos: Vec<&[u8]> = alpn.iter().map(|p| p.as_slice()).collect();
                ctx.set_alpn_protocols(&protos)
                    .map_err(|e| TlsError::Protocol(format!("set_alpn: {e}")))?;
            }

            let ssl = self.openssl.ssl_new_from_fd(&ctx, fd)
                .map_err(|e| TlsError::Protocol(format!("ssl_new: {e}")))?;

            if self.is_client {
                ssl.set_hostname(hostname)
                    .map_err(|e| TlsError::Protocol(format!("set_hostname: {e}")))?;
            }

            if self.is_client {
                ssl.connect()
                    .map_err(|e| TlsError::Protocol(format!("connect handshake: {e}")))?;
            } else {
                ssl.accept()
                    .map_err(|e| TlsError::Protocol(format!("accept handshake: {e}")))?;
            }

            Ok(TlsStream {
                ssl: Box::new(ssl),
                stream,
            })
        }

        pub fn accept(&self, stream: TcpStream) -> Result<TlsStream, TlsError> {
            self.connect(stream, "localhost") // hostname ignored on server
        }
    }

    pub struct TlsStream {
        ssl: Box<crate::SslConn>,
        #[allow(dead_code)]
        stream: TcpStream,
    }

    unsafe impl Send for TlsStream {}

    impl TlsStream {
        pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, TlsError> {
            match self.ssl.read(buf) {
                Ok(n) => Ok(n),
                Err(crate::SslError::Ssl(code, _)) => {
                    let want_read = code == 2;  // SSL_ERROR_WANT_READ
                    let want_write = code == 3; // SSL_ERROR_WANT_WRITE
                    if want_read { Err(TlsError::WantRead) }
                    else if want_write { Err(TlsError::WantWrite) }
                    else { Err(TlsError::Protocol(format!("SSL read error code {code}"))) }
                }
                Err(e) => Err(TlsError::Protocol(e.to_string())),
            }
        }

        pub fn write(&mut self, buf: &[u8]) -> Result<usize, TlsError> {
            match self.ssl.write(buf) {
                Ok(n) => Ok(n),
                Err(crate::SslError::Ssl(code, _)) => {
                    let want_read = code == 2;
                    let want_write = code == 3;
                    if want_read { Err(TlsError::WantRead) }
                    else if want_write { Err(TlsError::WantWrite) }
                    else { Err(TlsError::Protocol(format!("SSL write error code {code}"))) }
                }
                Err(e) => Err(TlsError::Protocol(e.to_string())),
            }
        }

        pub fn shutdown(&mut self) -> Result<(), TlsError> {
            self.ssl.shutdown()
                .map_err(|e| TlsError::Protocol(e.to_string()))
        }

        pub fn alpn_selected(&self) -> Option<Vec<u8>> {
            self.ssl.get_alpn_selected()
        }
    }

    impl Read for TlsStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            loop {
                match self.ssl.read(buf) {
                    Ok(n) => return Ok(n),
                    Err(crate::SslError::Ssl(code, _)) if code == 2 || code == 3 => {
                        // WANT_READ or WANT_WRITE — spin if blocking, return WouldBlock if non-blocking
                        // For blocking calls, we should never get these unless the fd is non-blocking.
                        // Map to WouldBlock for compatibility.
                        return Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "would block"));
                    }
                    Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
                }
            }
        }
    }

    impl Write for TlsStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            loop {
                match self.ssl.write(buf) {
                    Ok(n) => return Ok(n),
                    Err(crate::SslError::Ssl(code, _)) if code == 2 || code == 3 => {
                        return Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "would block"));
                    }
                    Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
                }
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::collections::VecDeque;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use super::TlsError;

    pub struct TlsConnector {
        pub(crate) alpn: Option<Vec<Vec<u8>>>,
        pub(crate) is_client: bool,
    }

    impl TlsConnector {
        pub fn new(is_client: bool) -> Result<Self, TlsError> {
            Ok(TlsConnector { alpn: None, is_client })
        }

        pub fn set_alpn(&mut self, protocols: &[&[u8]]) {
            self.alpn = Some(protocols.iter().map(|p| p.to_vec()).collect());
        }

        pub fn set_certificate(&mut self, _cert_pem: &str, _key_pem: &str) {}

        pub fn connect(&self, stream: TcpStream, hostname: &str) -> Result<TlsStream, TlsError> {
            let cred = lsb_schannel::Credentials::new_client()
                .map_err(|e| TlsError::Protocol(format!("schannel cred: {e}")))?;

            let mut stream = stream;
            let (conn, _alpn) = lsb_schannel::client_handshake(&cred, &mut stream, hostname)
                .map_err(|e| TlsError::Protocol(format!("schannel handshake: {e}")))?;

            Ok(TlsStream {
                stream,
                conn: Box::new(conn),
                read_buf: Vec::new(),
                decrypted: VecDeque::new(),
                write_pending: None,
            })
        }

        pub fn accept(&self, _stream: TcpStream) -> Result<TlsStream, TlsError> {
            Err(TlsError::NotSupported)
        }
    }

    pub struct TlsStream {
        stream: TcpStream,
        conn: Box<lsb_schannel::TlsConnection>,
        read_buf: Vec<u8>,
        decrypted: VecDeque<u8>,
        write_pending: Option<Vec<u8>>,
    }

    unsafe impl Send for TlsStream {}

    impl TlsStream {
        pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, TlsError> {
            if !self.decrypted.is_empty() {
                let n = std::cmp::min(buf.len(), self.decrypted.len());
                for i in 0..n { buf[i] = self.decrypted.pop_front().unwrap(); }
                return Ok(n);
            }

            loop {
                if self.read_buf.len() >= 5 {
                    let rec_len = u16::from_be_bytes([self.read_buf[3], self.read_buf[4]]) as usize;
                    let total = 5 + rec_len;
                    if self.read_buf.len() >= total {
                        let record: Vec<u8> = self.read_buf.drain(..total).collect();
                        match self.conn.decrypt(&record) {
                            Ok(plaintext) => {
                                self.decrypted.extend(plaintext);
                                let n = std::cmp::min(buf.len(), self.decrypted.len());
                                for i in 0..n { buf[i] = self.decrypted.pop_front().unwrap(); }
                                return Ok(n);
                            }
                            Err(lsb_schannel::Error::Protocol(_)) => continue,
                            Err(e) => return Err(TlsError::Protocol(format!("decrypt: {e}"))),
                        }
                    }
                }

                // Need more data from socket — try non-blocking read
                self.stream.set_nonblocking(true)
                    .map_err(|e| TlsError::Io(e))?;
                let mut tmp = [0u8; 8192];
                match self.stream.read(&mut tmp) {
                    Ok(0) => return Err(TlsError::Closed),
                    Ok(n) => {
                        self.read_buf.extend_from_slice(&tmp[..n]);
                        // Switch back to blocking for the caller's convenience
                        let _ = self.stream.set_nonblocking(false);
                        continue;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        let _ = self.stream.set_nonblocking(false);
                        return Err(TlsError::WantRead);
                    }
                    Err(e) => {
                        let _ = self.stream.set_nonblocking(false);
                        return Err(TlsError::Io(e));
                    }
                }
            }
        }

        pub fn write(&mut self, buf: &[u8]) -> Result<usize, TlsError> {
            let encrypted = self.conn.encrypt(buf)
                .map_err(|e| TlsError::Protocol(format!("encrypt: {e}")))?;

            self.stream.set_nonblocking(true)
                .map_err(|e| TlsError::Io(e))?;

            let mut written = 0;
            let remaining = &encrypted[written..];
            match self.stream.write(remaining) {
                Ok(n) => written += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    self.write_pending = Some(encrypted);
                    let _ = self.stream.set_nonblocking(false);
                    return Err(TlsError::WantWrite);
                }
                Err(e) => {
                    let _ = self.stream.set_nonblocking(false);
                    return Err(TlsError::Io(e));
                }
            }

            let _ = self.stream.set_nonblocking(false);
            Ok(buf.len())
        }

        pub fn shutdown(&mut self) -> Result<(), TlsError> {
            Ok(())
        }

        pub fn alpn_selected(&self) -> Option<Vec<u8>> {
            None
        }
    }

    impl Read for TlsStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.read(buf) {
                Ok(n) => Ok(n),
                Err(TlsError::WantRead) => {
                    // Block until data available
                    self.stream.set_nonblocking(false).ok();
                    let mut tmp = [0u8; 8192];
                    match self.stream.read(&mut tmp) {
                        Ok(0) => Ok(0),
                        Ok(n) => {
                            self.read_buf.extend_from_slice(&tmp[..n]);
                            self.read(buf).or_else(|_| Ok(0))
                        }
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            }
        }
    }

    impl Write for TlsStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.write(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.stream.flush()
        }
    }
}

// ── Public re-exports ────────────────────────────────────────

pub use platform::{TlsConnector, TlsStream};
