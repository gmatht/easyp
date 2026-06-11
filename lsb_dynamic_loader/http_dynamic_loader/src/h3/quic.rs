use std::os::raw::{c_char, c_int, c_void};
use lsb_loader::LoadedLibrary;

// ── Core ngtcp2 function pointers ─────────────────────────────

pub struct Ngtcp2Lib {
    pub lib: LoadedLibrary,
    pub accept: unsafe extern "C" fn(*mut c_void, *const u8, usize) -> c_int,
    pub conn_server_new: unsafe extern "C" fn(
        *mut *mut c_void, *const c_void, *const c_void,
        *const c_void, u32,
        c_int, *const c_void,
        c_int, *const c_void,
        c_int, *const c_void,
        *const c_void, *mut c_void,
    ) -> c_int,
    pub conn_del: unsafe extern "C" fn(*mut c_void),
    pub conn_read_pkt: unsafe extern "C" fn(
        *mut c_void, *const c_void, c_int, *const c_void,
        *const u8, usize, u64,
    ) -> c_int,
    pub conn_write_pkt: unsafe extern "C" fn(
        *mut c_void, *mut c_void, c_int, *mut c_void,
        *mut u8, usize, u64,
    ) -> isize,
    pub submit_crypto_data: unsafe extern "C" fn(
        *mut c_void, c_int, *const u8, usize,
    ) -> c_int,
    pub handshake_completed: unsafe extern "C" fn(*mut c_void),
    pub get_handshake_completed: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub settings_default: unsafe extern "C" fn(c_int, *mut c_void),
    pub transport_params_default: unsafe extern "C" fn(c_int, *mut c_void),
    pub mem_default: unsafe extern "C" fn() -> *const c_void,
    pub cid_init: unsafe extern "C" fn(*mut c_void, *const u8, usize),
    pub pkt_decode_version_cid: unsafe extern "C" fn(
        *mut c_void, *const u8, usize, usize,
    ) -> c_int,
    pub addr_init: unsafe extern "C" fn(
        *mut c_void, *const c_void, u32,
    ) -> *mut c_void,
    pub get_tls_native_handle: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    pub set_tls_native_handle: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub set_initial_crypto_ctx: unsafe extern "C" fn(*mut c_void, *const c_void),
    pub strerror: unsafe extern "C" fn(c_int) -> *const c_char,
}

unsafe extern "C" fn ngtcp2_strerror_stub(_: c_int) -> *const c_char {
    std::ptr::null()
}

impl Ngtcp2Lib {
    pub fn load() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let lib = LoadedLibrary::load_from_candidates(
            &["libngtcp2.so.9", "libngtcp2.so"],
            &[
                "ngtcp2_accept",
                "ngtcp2_conn_server_new_versioned",
                "ngtcp2_conn_del",
                "ngtcp2_conn_read_pkt_versioned",
                "ngtcp2_conn_write_pkt_versioned",
                "ngtcp2_conn_submit_crypto_data",
                "ngtcp2_conn_handshake_completed",
                "ngtcp2_conn_get_handshake_completed",
                "ngtcp2_settings_default_versioned",
                "ngtcp2_transport_params_default_versioned",
                "ngtcp2_mem_default",
                "ngtcp2_cid_init",
                "ngtcp2_pkt_decode_version_cid",
                "ngtcp2_addr_init",
                "ngtcp2_conn_get_tls_native_handle",
                "ngtcp2_conn_set_tls_native_handle",
                "ngtcp2_conn_set_initial_crypto_ctx",
            ],
        )?;
        unsafe {
            let accept = std::mem::transmute(lib.get_symbol_raw("ngtcp2_accept")?);
            let conn_server_new = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_server_new_versioned")?);
            let conn_del = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_del")?);
            let conn_read_pkt = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_read_pkt_versioned")?);
            let conn_write_pkt = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_write_pkt_versioned")?);
            let submit_crypto_data = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_submit_crypto_data")?);
            let handshake_completed = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_handshake_completed")?);
            let get_handshake_completed = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_get_handshake_completed")?);
            let settings_default = std::mem::transmute(lib.get_symbol_raw("ngtcp2_settings_default_versioned")?);
            let transport_params_default = std::mem::transmute(lib.get_symbol_raw("ngtcp2_transport_params_default_versioned")?);
            let mem_default = std::mem::transmute(lib.get_symbol_raw("ngtcp2_mem_default")?);
            let cid_init = std::mem::transmute(lib.get_symbol_raw("ngtcp2_cid_init")?);
            let pkt_decode_version_cid = std::mem::transmute(lib.get_symbol_raw("ngtcp2_pkt_decode_version_cid")?);
            let addr_init = std::mem::transmute(lib.get_symbol_raw("ngtcp2_addr_init")?);
            let get_tls_native_handle = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_get_tls_native_handle")?);
            let set_tls_native_handle = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_set_tls_native_handle")?);
            let set_initial_crypto_ctx = std::mem::transmute(lib.get_symbol_raw("ngtcp2_conn_set_initial_crypto_ctx")?);
            let strerror_ptr = lib.get_symbol_raw("ngtcp2_strerror").ok();
            let strerror: unsafe extern "C" fn(c_int) -> *const c_char = match strerror_ptr {
                Some(p) => unsafe { std::mem::transmute(p) },
                None => ngtcp2_strerror_stub,
            };
            Ok(Ngtcp2Lib { lib, accept, conn_server_new, conn_del, conn_read_pkt, conn_write_pkt,
                submit_crypto_data, handshake_completed, get_handshake_completed,
                settings_default, transport_params_default, mem_default, cid_init, pkt_decode_version_cid, addr_init,
                get_tls_native_handle, set_tls_native_handle, set_initial_crypto_ctx, strerror })
        }
    }
}

unsafe impl Send for Ngtcp2Lib {}

/// Backward-compatible alias used by http3_handler.
pub use self::Ngtcp2Lib as Ngtcp2;
