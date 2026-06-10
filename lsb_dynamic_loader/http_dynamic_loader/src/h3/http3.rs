use crate::{HttpError, HttpProtocol, HttpProtocolImpl};
use lsb_loader::LoadedLibrary;
use std::os::raw::c_int;

type Nghttp3VersionFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;

/// Runtime-loaded libnghttp3 (HTTP/3 framing).
pub struct Nghttp3 {
    #[allow(dead_code)]
    lib: LoadedLibrary,
    #[allow(dead_code)]
    version: Nghttp3VersionFn,
}

impl Nghttp3 {
    pub fn load() -> Result<HttpProtocolImpl, HttpError> {
        let required = &["nghttp3_version"];
        let lib = if let Ok(path) = std::env::var("LSBWRAP_NGHTTP3_PATH") {
            if !path.starts_with('/') {
                return Err(HttpError::Protocol(
                    "LSBWRAP_NGHTTP3_PATH must be absolute".into(),
                ));
            }
            LoadedLibrary::load_explicit(&path, required)?
        } else {
            LoadedLibrary::load_from_candidates(
                &["libnghttp3.so.3", "libnghttp3.so"],
                required,
            )?
        };

        unsafe {
            let version: Nghttp3VersionFn =
                std::mem::transmute(lib.get_symbol_raw("nghttp3_version")?);
            Ok(HttpProtocolImpl {
                protocol: HttpProtocol::Http3,
                _lib: lib,
            })
        }
    }
}
