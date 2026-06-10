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

#[cfg(feature = "h3")]
pub mod h3;
