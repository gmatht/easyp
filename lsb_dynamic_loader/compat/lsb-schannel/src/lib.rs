//! Native Windows SChannel TLS wrapper via SSPI.
//!
//! Uses compile-time linking to `secur32.dll` and `kernel32.dll` — no `libloading`.
//! On non-Windows platforms, this crate provides only stub types that return errors.

#[cfg(windows)]
mod platform;

#[cfg(windows)]
pub use platform::*;

// ── Non-Windows stubs ────────────────────────────────────────

#[cfg(not(windows))]
mod stub {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("SChannel is only available on Windows")]
        NotSupported,
        #[error("I/O error: {0}")]
        Io(#[from] std::io::Error),
        #[error("protocol error: {0}")]
        Protocol(String),
    }

    pub struct Credentials;
    pub struct TlsConnection;
    pub struct TlsStream;

    impl Credentials {
        pub fn new_client() -> Result<Self, Error> { Err(Error::NotSupported) }
    }

    impl TlsConnection {
        pub fn encrypt(&self, _data: &[u8]) -> Result<Vec<u8>, Error> { Err(Error::NotSupported) }
        pub fn decrypt(&self, _data: &[u8]) -> Result<Vec<u8>, Error> { Err(Error::NotSupported) }
    }

    impl TlsStream {
        pub fn connect(_stream: std::net::TcpStream, _name: &str) -> Result<Self, Error> { Err(Error::NotSupported) }
        pub fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Error> { Err(Error::NotSupported) }
        pub fn write_all(&mut self, _buf: &[u8]) -> Result<(), Error> { Err(Error::NotSupported) }
    }

    pub fn client_handshake(
        _cred: &Credentials,
        _stream: &mut std::net::TcpStream,
        _name: &str,
    ) -> Result<(TlsConnection, Option<Vec<u8>>), Error> { Err(Error::NotSupported) }

    pub fn is_schannel_available() -> bool { false }
    pub fn version() -> &'static str { "none" }
}

#[cfg(not(windows))]
pub use stub::*;
