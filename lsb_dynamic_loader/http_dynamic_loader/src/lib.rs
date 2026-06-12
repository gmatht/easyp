use lsb_loader::LoadedLibrary;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("loader error: {0}")]
    Loader(#[from] lsb_loader::LoaderError),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpProtocol {
    Http2,
    Http3,
}

pub struct HttpProtocolImpl {
    _lib: LoadedLibrary,
    protocol: HttpProtocol,
}

unsafe impl Send for HttpProtocolImpl {}

impl HttpProtocolImpl {
    pub fn protocol(&self) -> HttpProtocol {
        self.protocol
    }
}

#[cfg(feature = "h2")]
pub mod h2;

#[cfg(all(feature = "h3", not(windows)))]
pub mod h3;

/// Generated ngtcp2 FFI bindings (from build.rs + bindgen).
/// Provides correct #[repr(C)] struct layouts for ngtcp2_callbacks,
/// ngtcp2_settings, ngtcp2_transport_params, and related types.
#[cfg(all(feature = "h3", not(windows)))]
#[allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
pub mod ffi {
    include!("ngtcp2_ffi.rs");
}
