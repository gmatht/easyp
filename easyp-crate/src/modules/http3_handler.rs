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
    use std::os::raw::{c_int, c_void};

    // ── QUIC connection state ──────────────────────────────────

    struct QuicConn {
        conn: *mut c_void,
        peer: SocketAddr,
    }

    unsafe impl Send for QuicConn {}
    unsafe impl Sync for QuicConn {}

    pub struct Http3Handler {
        socket: async_io::Async<std::net::UdpSocket>,
        conns: HashMap<Vec<u8>, QuicConn>,
        file_server: Arc<SecureFileServer>,
        stats_collector: Arc<HourlyStatsCollector>,
        security_config: SecurityConfig,
        ngtcp2: std::sync::Arc<http_dynamic_loader::h3::quic::Ngtcp2Lib>,
        _nghttp3: http_dynamic_loader::HttpProtocolImpl,
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

            let ngtcp2 = http_dynamic_loader::h3::quic::Ngtcp2Lib::load()
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
                ngtcp2: Arc::new(ngtcp2),
                _nghttp3: nghttp3,
                _crypto: crypto,
            })
        }

        pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let local = self.socket.get_ref().local_addr()?;
            println!("HTTP/3 on UDP {}", local);

            let mut buf = [0u8; 65535];
            loop {
                match self.socket.recv_from(&mut buf).await {
                    Ok((n, peer)) => {
                        if let Err(e) = self.handle_packet(&buf[..n], peer) {
                            eprintln!("H3 pkt err: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("H3 recv: {}", e);
                        break;
                    }
                }
            }
            Ok(())
        }

        fn handle_packet(&self, data: &[u8], peer: SocketAddr) -> Result<(), String> {
            let key = data[..data.len().min(20)].to_vec();
            if self.conns.contains_key(&key) {
                if let Some(qc) = self.conns.get(&key) {
                    let path = sockaddr_storage(peer);
                    let ecn = 0;
                    let ret = unsafe {
                        (self.ngtcp2.conn_read_pkt)(
                            qc.conn,
                            &path as *const _ as *const c_void,
                            0,
                            std::ptr::null(),
                            data.as_ptr(), data.len(), 0u64)
                    };
                    if ret != 0 {
                        return Err(format!("conn_read_pkt: {}", ret));
                    }
                    self.flush_conn(&key)?;
                }
            } else {
                let conn = self.create_server_conn(data, peer)?;
                println!("H3: new QUIC conn from {}", peer);
                // Process the initial packet
                if let Some(qc) = self.conns.get(&key) {
                    let ret = unsafe {
                        (self.ngtcp2.conn_read_pkt)(
                            qc.conn,
                            std::ptr::null(), 0, std::ptr::null(),
                            data.as_ptr(), data.len(), 0u64)
                    };
                    if ret != 0 {
                        return Err(format!("conn_read_pkt initial: {}", ret));
                    }
                    self.flush_conn(&key)?;
                }
            }
            Ok(())
        }

        fn create_server_conn(&self, data: &[u8], peer: SocketAddr)
            -> Result<*mut c_void, String>
        {
            use http_dynamic_loader::h3::callbacks::*;
            let mut conn: *mut c_void = std::ptr::null_mut();
            let mem = unsafe { (self.ngtcp2.mem_default)() };

            let ret = unsafe {
                (self.ngtcp2.conn_server_new)(
                    &mut conn,
                    std::ptr::null(), std::ptr::null(),
                    std::ptr::null(),
                    0x00000001,  // QUIC v1
                    0, std::ptr::null(),  // no callbacks
                    0, std::ptr::null(),  // default settings
                    0, std::ptr::null(),  // default params
                    mem,
                    std::ptr::null_mut(),
                )
            };
            if ret != 0 {
                return Err(format!("conn_server_new: {}", ret));
            }
            if conn.is_null() {
                return Err("conn_server_new returned null".into());
            }
            self.conns.insert(data[..data.len().min(20)].to_vec(), QuicConn { conn, peer });
            Ok(conn)
        }

        fn flush_conn(&self, key: &[u8]) -> Result<(), String> {
            let qc = match self.conns.get(key) {
                Some(c) => c,
                None => return Ok(()),
            };
            let mut out = [0u8; 1500];
            loop {
                let written = unsafe {
                    (self.ngtcp2.conn_write_pkt)(
                        qc.conn,
                        std::ptr::null_mut(), 0,
                        std::ptr::null_mut(),
                        out.as_mut_ptr(), out.len(), 0u64)
                };
                if written <= 0 { break; }
                let _ = self.socket.get_ref().send_to(&out[..written as usize], qc.peer);
            }
            Ok(())
        }
    }
}

/// Convert a SocketAddr to a sockaddr_storage struct for ngtcp2.
fn sockaddr_storage(addr: SocketAddr) -> [u8; 28] {
    let mut buf = [0u8; 28];
    match addr {
        SocketAddr::V4(v4) => {
            buf[0..2].copy_from_slice(&[2, 0]); // AF_INET
            buf[2..4].copy_from_slice(&v4.port().to_be_bytes());
            buf[4..8].copy_from_slice(&v4.ip().octets());
        }
        SocketAddr::V6(v6) => {
            buf[0..2].copy_from_slice(&[10, 0]); // AF_INET6
            buf[2..4].copy_from_slice(&v6.port().to_be_bytes());
            buf[8..24].copy_from_slice(&v6.ip().octets());
        }
    }
    buf
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
