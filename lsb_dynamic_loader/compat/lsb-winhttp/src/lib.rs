//! Native Windows WinHTTP wrapper supporting HTTP/2 and HTTP/3.
//!
//! Uses compile-time linking to `winhttp.dll` — no `libloading`.
//! On non-Windows, this crate provides only stub types that return errors.

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
        #[error("WinHTTP is only available on Windows")]
        NotSupported,
    }

    pub struct Session;
    pub struct Response;

    impl Session {
        pub fn new() -> Result<Self, Error> { Err(Error::NotSupported) }
        pub fn enable_http2(&self) -> Result<(), Error> { Err(Error::NotSupported) }
        pub fn enable_http3(&self) -> Result<(), Error> { Err(Error::NotSupported) }
        pub fn get(&self, _url: &str) -> Result<Response, Error> { Err(Error::NotSupported) }
    }

    impl Response {
        pub fn status(&self) -> u16 { 0 }
        pub fn headers(&self) -> &std::collections::HashMap<String, String> { panic!("no impl") }
        pub fn body(&self) -> &[u8] { &[] }
    }

    pub fn is_winhttp_available() -> bool { false }
    pub fn version() -> &'static str { "none" }
}

#[cfg(not(windows))]
pub use stub::*;
