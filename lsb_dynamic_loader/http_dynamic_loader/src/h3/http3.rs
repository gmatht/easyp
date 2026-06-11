use crate::{HttpError, HttpProtocol, HttpProtocolImpl};
use lsb_loader::LoadedLibrary;
use std::os::raw::c_int;

type Nghttp3VersionFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;

// ── nghttp3 callback types ────────────────────────────────────

pub type Nghttp3DeferredConsume = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, usize, *mut std::os::raw::c_void,
);
pub type Nghttp3BeginHeaders = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, *mut std::os::raw::c_void,
) -> c_int;
pub type Nghttp3RecvHeader = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, *const u8, usize, *const u8, usize,
    *mut std::os::raw::c_void,
) -> c_int;
pub type Nghttp3EndHeaders = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, *mut std::os::raw::c_void,
) -> c_int;
pub type Nghttp3RecvData = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, *const u8, usize, *mut std::os::raw::c_void,
) -> c_int;
pub type Nghttp3EndStream = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, *mut std::os::raw::c_void,
) -> c_int;
pub type Nghttp3StopSending = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, u64, *mut std::os::raw::c_void,
) -> c_int;
pub type Nghttp3ResetStream = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, u64, *mut std::os::raw::c_void,
) -> c_int;
pub type Nghttp3SendData = unsafe extern "C" fn(
    *mut std::os::raw::c_void, i64, *mut std::os::raw::c_void,
) -> c_int;

#[repr(C)]
pub struct Nghttp3Callbacks {
    pub deferred_consume: Option<Nghttp3DeferredConsume>,
    pub begin_headers: Option<Nghttp3BeginHeaders>,
    pub recv_header: Option<Nghttp3RecvHeader>,
    pub end_headers: Option<Nghttp3EndHeaders>,
    pub recv_data: Option<Nghttp3RecvData>,
    pub end_stream: Option<Nghttp3EndStream>,
    pub stop_sending: Option<Nghttp3StopSending>,
    pub reset_stream: Option<Nghttp3ResetStream>,
    pub send_data: Option<Nghttp3SendData>,
    pub error: Option<unsafe extern "C" fn(*mut std::os::raw::c_void, i64, c_int, *mut std::os::raw::c_void)>,
}

/// Runtime-loaded libnghttp3 (HTTP/3 framing).
pub struct Nghttp3 {
    pub lib: LoadedLibrary,
    pub conn_server_new: unsafe extern "C" fn(
        *mut *mut std::os::raw::c_void,
        c_int, *const std::os::raw::c_void,
        c_int, *const std::os::raw::c_void,
        *const std::os::raw::c_void, *mut std::os::raw::c_void,
    ) -> c_int,
    pub conn_del: unsafe extern "C" fn(*mut std::os::raw::c_void),
    pub conn_read_stream: unsafe extern "C" fn(
        *mut std::os::raw::c_void, i64, *const u8, usize, c_int,
    ) -> c_int,
    pub conn_writev_stream: unsafe extern "C" fn(
        *mut std::os::raw::c_void, *mut i64, c_int, *mut *const std::os::raw::c_void, *mut usize, c_int,
        *mut std::os::raw::c_void,
    ) -> c_int,
    pub conn_submit_response: unsafe extern "C" fn(
        *mut std::os::raw::c_void, i64,
        *const std::os::raw::c_void, usize,
        *const std::os::raw::c_void, usize,
        *mut std::os::raw::c_void,
    ) -> c_int,
    pub settings_default: unsafe extern "C" fn(c_int, *mut std::os::raw::c_void),
    pub conn_set_stream_user_data: unsafe extern "C" fn(
        *mut std::os::raw::c_void, i64, *mut std::os::raw::c_void,
    ),
    version: Nghttp3VersionFn,
}

impl Nghttp3 {
    pub fn load() -> Result<Self, HttpError> {
        let required = &[
            "nghttp3_version",
            "nghttp3_conn_server_new_versioned",
            "nghttp3_conn_del",
            "nghttp3_conn_read_stream",
            "nghttp3_conn_writev_stream",
            "nghttp3_conn_submit_response",
            "nghttp3_settings_default_versioned",
            "nghttp3_conn_set_stream_user_data",
        ];
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
            let conn_server_new = std::mem::transmute(
                lib.get_symbol_raw("nghttp3_conn_server_new_versioned")?);
            let conn_del = std::mem::transmute(
                lib.get_symbol_raw("nghttp3_conn_del")?);
            let conn_read_stream = std::mem::transmute(
                lib.get_symbol_raw("nghttp3_conn_read_stream")?);
            let conn_writev_stream = std::mem::transmute(
                lib.get_symbol_raw("nghttp3_conn_writev_stream")?);
            let conn_submit_response = std::mem::transmute(
                lib.get_symbol_raw("nghttp3_conn_submit_response")?);
            let settings_default = std::mem::transmute(
                lib.get_symbol_raw("nghttp3_settings_default_versioned")?);
            let conn_set_stream_user_data = std::mem::transmute(
                lib.get_symbol_raw("nghttp3_conn_set_stream_user_data")?);
            Ok(Nghttp3 {
                lib: lib,
                version,
                conn_server_new,
                conn_del,
                conn_read_stream,
                conn_writev_stream,
                conn_submit_response,
                settings_default,
                conn_set_stream_user_data,
            })
        }
    }
}
