use std::sync::Arc;
use std::net::SocketAddr;

use super::secure_file_server_module::{SecureFileServer, SecurityConfig};
use super::hourly_stats::HourlyStatsCollector;

#[cfg(feature = "http3")]
pub use inner::*;

#[cfg(feature = "http3")]
mod inner {
    use super::*;
    use std::collections::HashMap;
    use std::os::raw::c_void;

    struct QuicConn {
        #[allow(dead_code)]
        ngtcp2_conn: *mut c_void,
        #[allow(dead_code)]
        gnutls_session: *mut c_void,
        #[allow(dead_code)]
        peer: SocketAddr,
    }
    unsafe impl Send for QuicConn {}
    unsafe impl Sync for QuicConn {}

    pub struct Http3Handler {
        socket: async_io::Async<std::net::UdpSocket>,
        #[allow(dead_code)]
        conns: HashMap<Vec<u8>, QuicConn>,
        #[allow(dead_code)]
        file_server: Arc<SecureFileServer>,
        #[allow(dead_code)]
        stats_collector: Arc<HourlyStatsCollector>,
        #[allow(dead_code)]
        security_config: SecurityConfig,
        #[allow(dead_code)]
        _ngtcp2_lib: http_dynamic_loader::h3::quic::Ngtcp2Lib,
        #[allow(dead_code)]
        _nghttp3: http_dynamic_loader::HttpProtocolImpl,
        #[allow(dead_code)]
        _crypto: http_dynamic_loader::h3::crypto::GnutlsCrypto,
    }

    unsafe impl Send for Http3Handler {}

    impl Http3Handler {
        pub async fn new(
            file_server: Arc<SecureFileServer>,
            stats_collector: Arc<HourlyStatsCollector>,
            security_config: SecurityConfig,
            bind_addr: SocketAddr,
            _cert_file: &str,
            _key_file: &str,
        ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            let sock = std::net::UdpSocket::bind(bind_addr)?;
            sock.set_nonblocking(true)?;
            let socket = async_io::Async::new(sock)?;

            let ngtcp2_lib = http_dynamic_loader::h3::quic::Ngtcp2Lib::load()
                .map_err(|e| format!("ngtcp2: {}", e))?;
            let nghttp3 = http_dynamic_loader::h3::http3::Nghttp3::load()
                .map_err(|e| format!("nghttp3: {}", e))?;
            let crypto = http_dynamic_loader::h3::crypto::GnutlsCrypto::load()
                .map_err(|e| format!("crypto: {}", e))?;

            Ok(Http3Handler {
                socket,
                conns: HashMap::new(),
                file_server,
                stats_collector,
                security_config,
                _ngtcp2_lib: ngtcp2_lib,
                _nghttp3: nghttp3,
                _crypto: crypto,
            })
        }

        pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let local = self.socket.local_addr()?;
            println!("HTTP/3 on UDP {}", local);

            let mut buf = [0u8; 65535];
            loop {
                match self.socket.recv_from(&mut buf).await {
                    Ok((n, peer)) => {
                        // QUIC + HTTP/3 processing here
                        println!("H3: {}B from {}", n, peer);
                    }
                    Err(e) => {
                        eprintln!("H3 recv: {}", e);
                        break;
                    }
                }
            }
            Ok(())
        }
    }
}

#[cfg(not(feature = "http3"))]
pub struct Http3Handler;

#[cfg(not(feature = "http3"))]
impl Http3Handler {
    pub async fn new(
        _file_server: Arc<SecureFileServer>,
        _stats_collector: Arc<HourlyStatsCollector>,
        _security_config: SecurityConfig,
        _bind_addr: SocketAddr,
        _cert_file: &str,
        _key_file: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err("HTTP/3 not enabled".into())
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("HTTP/3 not enabled".into())
    }
}
