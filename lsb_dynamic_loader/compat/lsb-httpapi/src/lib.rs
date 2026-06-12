//! Windows HTTP Server API (HttpApi.dll) bindings.
//!
//! Provides raw FFI to the kernel-mode HTTP stack (http.sys) via `httpapi.dll`.
//! Linked statically to the import library; no runtime dynamic loading needed.

#![cfg(windows)]
#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::ffi::{c_int, c_void};
use std::ptr;
use thiserror::Error;

// ── Windows API for dynamic loading ───────────────────────────

extern "system" {
    fn LoadLibraryW(lp_file_name: *const u16) -> *mut c_void;
    fn FreeLibrary(h_module: *mut c_void) -> i32;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
}

// ── Windows type aliases ──────────────────────────────────────

type DWORD = u32;
type ULONG = u32;
type USHORT = u16;
type UCHAR = u8;
type LPCWSTR = *const u16;
type LPWSTR = *mut u16;
type LPVOID = *mut c_void;
type LPCVOID = *const c_void;
type HANDLE = LPVOID;
type HTTP_OPAQUE_ID = *mut c_void;
pub type HTTP_REQUEST_ID = HTTP_OPAQUE_ID;
pub type HTTP_CONNECTION_ID = HTTP_OPAQUE_ID;
// Per Windows SDK: HTTP_URL_GROUP_ID and HTTP_SERVER_SESSION_ID are ULONGLONG (u64).
// Using u64 instead of *mut c_void ensures correct ABI with the HTTP API functions.
type HTTP_URL_GROUP_ID = u64;
type HTTP_SERVER_SESSION_ID = u64;
type HTTP_REQUEST_QUEUE_ID = HANDLE;

// ── HTTP Server API version ───────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HTTPAPI_VERSION {
    pub http_major: USHORT,
    pub http_minor: USHORT,
}

pub const HTTPAPI_VERSION_2: HTTPAPI_VERSION = HTTPAPI_VERSION { http_major: 2, http_minor: 0 };

// ── Initialize flags ──────────────────────────────────────────

pub const HTTP_INITIALIZE_SERVER: ULONG = 0x00000001;

// ── URL group property IDs ────────────────────────────────────

pub const HttpServerBindingProperty: ULONG = 7;

// ── HTTP verbs ────────────────────────────────────────────────

pub type HTTP_VERB = c_int;
pub const HttpVerbGET: HTTP_VERB = 0;
pub const HttpVerbPOST: HTTP_VERB = 1;
pub const HttpVerbHEAD: HTTP_VERB = 4;
pub const HttpVerbMaximum: HTTP_VERB = 10;

// ── Data chunk types ──────────────────────────────────────────

pub const HttpDataChunkFromMemory: ULONG = 0;

// ── Entity chunk ──────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HTTP_BYTE_RANGE {
    pub starting_offset: u64,
    pub length: u64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HTTP_DATA_CHUNK_FROM_MEMORY {
    pub p_buffer: *const u8,
    pub buffer_length: ULONG,
}

// Largest variant: FromFileHandle (HTTP_BYTE_RANGE + HANDLE = 24 bytes)
#[repr(C)]
#[derive(Copy, Clone)]
struct HTTP_DATA_CHUNK_FILE_HANDLE {
    pub byte_range: HTTP_BYTE_RANGE,
    pub file_handle: HANDLE,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct HTTP_DATA_CHUNK_FRAGMENT_CACHE {
    pub fragment_name_length: USHORT,
    pub p_fragment_name: *const u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
union HTTP_DATA_CHUNK_UNION {
    from_memory: HTTP_DATA_CHUNK_FROM_MEMORY,
    from_file_handle: HTTP_DATA_CHUNK_FILE_HANDLE,
    from_fragment_cache: HTTP_DATA_CHUNK_FRAGMENT_CACHE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HTTP_DATA_CHUNK {
    pub data_chunk_type: ULONG,
    _padding: ULONG,
    u: HTTP_DATA_CHUNK_UNION,
}

// ── HTTP version ──────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HTTP_VERSION {
    pub major: USHORT,
    pub minor: USHORT,
}

pub const HTTP_VERSION_UNKNOWN: HTTP_VERSION = HTTP_VERSION { major: 0, minor: 0 };

// ── Known header enum values ──────────────────────────────────

pub const HttpHeaderContentType: ULONG = 12;
pub const HttpHeaderContentLength: ULONG = 11;
pub const HttpHeaderServer: ULONG = 50;

// ── Known header ──────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HTTP_KNOWN_HEADER {
    pub raw_value_length: USHORT,
    pub p_raw_value: *const UCHAR,
}

// ── Unknown header ────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HTTP_UNKNOWN_HEADER {
    pub name_length: USHORT,
    pub raw_value_length: USHORT,
    pub p_name: *const UCHAR,
    pub p_raw_value: *const UCHAR,
}

// ── Request headers ───────────────────────────────────────────

#[repr(C)]
pub struct HTTP_REQUEST_HEADERS {
    pub known_headers_count: USHORT,
    pub known_headers: [HTTP_KNOWN_HEADER; HttpHeaderRequestMax as usize],
    pub unknown_headers_count: USHORT,
    pub p_unknown_headers: *mut HTTP_UNKNOWN_HEADER,
}

const HttpHeaderRequestMax: ULONG = 64;

// ── Response headers ──────────────────────────────────────────

#[repr(C)]
pub struct HTTP_RESPONSE_HEADERS {
    pub unknown_header_count: USHORT,
    pub p_unknown_headers: *mut HTTP_UNKNOWN_HEADER,
    pub trailer_count: USHORT,
    pub p_trailers: *mut HTTP_UNKNOWN_HEADER,
    pub known_headers: [HTTP_KNOWN_HEADER; 30],
}

// ── Transport address ─────────────────────────────────────────

#[repr(C)]
pub struct HTTP_TRANSPORT_ADDRESS {
    pub p_remote_address: *mut SOCKADDR_STORAGE,
    pub p_local_address: *mut SOCKADDR_STORAGE,
}

#[repr(C)]
pub struct SOCKADDR_STORAGE {
    pub data: [u8; 128],
}

// ── Cooked URL ────────────────────────────────────────────────

#[repr(C)]
pub struct HTTP_COOKED_URL {
    pub full_url_length: USHORT,
    pub host_length: USHORT,
    pub path_length: USHORT,
    pub query_length: USHORT,
    pub p_full_url: *const USHORT,
    pub p_host: *const USHORT,
    pub p_path: *const USHORT,
    pub p_query: *const USHORT,
}

// ── SSL info ──────────────────────────────────────────────────

#[repr(C)]
pub struct HTTP_SSL_INFO {
    pub server_cert_issuer_size: USHORT,
    pub server_cert_issuer: *mut UCHAR,
    pub server_cert_subject_size: USHORT,
    pub server_cert_subject: *mut UCHAR,
    pub connection_id: *mut c_void,
    pub server_cert_credential_type: ULONG,
    pub client_cert_negotiated: UCHAR,
    pub client_cert_provided: UCHAR,
}

// ── Request V1 ────────────────────────────────────────────────

#[repr(C)]
pub struct HTTP_REQUEST_V1 {
    pub flags: ULONG,
    pub connection_id: HTTP_CONNECTION_ID,
    pub request_id: HTTP_REQUEST_ID,
    pub url_context: HTTP_URL_GROUP_ID,
    pub version: HTTP_VERSION,
    pub verb: HTTP_VERB,
    pub unknown_verb_length: USHORT,
    pub raw_url_length: USHORT,
    pub p_unknown_verb: *const UCHAR,
    pub p_raw_url: *const UCHAR,
    pub cooked_url: HTTP_COOKED_URL,
    pub address: HTTP_TRANSPORT_ADDRESS,
    pub headers: HTTP_REQUEST_HEADERS,
    pub bytes_received: u64,
    pub entity_chunk_count: USHORT,
    pub p_entity_chunks: *mut HTTP_DATA_CHUNK,
    pub raw_connection_id: u64,
    pub p_ssl_info: *mut HTTP_SSL_INFO,
}

// ── Response V1 ───────────────────────────────────────────────

#[repr(C)]
pub struct HTTP_RESPONSE_V1 {
    pub flags: ULONG,
    pub version: HTTP_VERSION,
    pub status_code: USHORT,
    pub reason_length: USHORT,
    pub p_reason: *const UCHAR,
    pub headers: HTTP_RESPONSE_HEADERS,
    pub entity_chunk_count: USHORT,
    pub p_entity_chunks: *mut HTTP_DATA_CHUNK,
}

// ── Binding info ──────────────────────────────────────────────

#[repr(C)]
pub struct HTTP_BINDING_INFO {
    pub flags: HTTP_PROPERTY_FLAGS,
    pub request_queue_handle: HTTP_REQUEST_QUEUE_ID,
}

#[repr(C)]
pub struct HTTP_PROPERTY_FLAGS {
    pub present: ULONG,
}

// ── Service config types ──────────────────────────────────────

pub const HttpServiceConfigSSLSniCertInfo: ULONG = 4;

#[repr(C)]
pub struct HTTP_SERVICE_CONFIG_SSL_SNI_KEY {
    pub host: *mut u16,
}

#[repr(C)]
pub struct HTTP_SERVICE_CONFIG_SSL_PARAM {
    pub cert_hash: [u8; 20],
    pub cert_store_name: *mut u16,
    pub cert_check_group: ULONG,
    pub flags: ULONG,
}

#[repr(C)]
pub struct HTTP_SERVICE_CONFIG_SSL_SNI_SET {
    pub key_desc: HTTP_SERVICE_CONFIG_SSL_SNI_KEY,
    pub param_desc: HTTP_SERVICE_CONFIG_SSL_PARAM,
}

// ── Error ─────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum HttpApiError {
    #[error("HttpApi error code: {0} (0x{0:x})")]
    Win32(ULONG),
    #[error("init failed: {0}")]
    Init(String),
    #[error("url registration failed for {0}: {1}")]
    UrlRegister(String, ULONG),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP version not supported")]
    HttpVersionUnsupported,
    #[error("buffer too small, need {0} bytes")]
    BufferTooSmall(ULONG),
}

// ── Function pointer types ────────────────────────────────────

type HttpInitializeFn = unsafe extern "system" fn(HTTPAPI_VERSION, ULONG, LPVOID) -> ULONG;
type HttpTerminateFn = unsafe extern "system" fn(ULONG, LPVOID) -> ULONG;
type HttpCreateServerSessionFn = unsafe extern "system" fn(HTTPAPI_VERSION, *mut HTTP_SERVER_SESSION_ID, ULONG) -> ULONG;
type HttpCloseServerSessionFn = unsafe extern "system" fn(HTTP_SERVER_SESSION_ID) -> ULONG;
type HttpCreateUrlGroupFn = unsafe extern "system" fn(HTTP_SERVER_SESSION_ID, *mut HTTP_URL_GROUP_ID, ULONG) -> ULONG;
type HttpCloseUrlGroupFn = unsafe extern "system" fn(HTTP_URL_GROUP_ID) -> ULONG;
type HttpCreateRequestQueueFn = unsafe extern "system" fn(HTTPAPI_VERSION, LPCWSTR, *mut c_void, ULONG, *mut HTTP_REQUEST_QUEUE_ID) -> ULONG;
type HttpCloseRequestQueueFn = unsafe extern "system" fn(HTTP_REQUEST_QUEUE_ID) -> ULONG;
type HttpSetUrlGroupPropertyFn = unsafe extern "system" fn(HTTP_URL_GROUP_ID, ULONG, LPVOID, ULONG) -> ULONG;
type HttpAddUrlToUrlGroupFn = unsafe extern "system" fn(HTTP_URL_GROUP_ID, LPCWSTR, u64, ULONG) -> ULONG;
type HttpRemoveUrlFromUrlGroupFn = unsafe extern "system" fn(HTTP_URL_GROUP_ID, LPCWSTR, ULONG) -> ULONG;
type HttpReceiveHttpRequestFn = unsafe extern "system" fn(HTTP_REQUEST_QUEUE_ID, HTTP_REQUEST_ID, ULONG, *mut HTTP_REQUEST_V1, ULONG, *mut ULONG, *mut c_void) -> ULONG;
type HttpSendHttpResponseFn = unsafe extern "system" fn(HTTP_REQUEST_QUEUE_ID, HTTP_REQUEST_ID, ULONG, *mut HTTP_RESPONSE_V1, *mut c_void, *mut ULONG, *mut c_void, ULONG, *mut c_void, *mut c_void) -> ULONG;
type HttpSendResponseEntityBodyFn = unsafe extern "system" fn(HTTP_REQUEST_QUEUE_ID, HTTP_REQUEST_ID, ULONG, USHORT, *mut HTTP_DATA_CHUNK, *mut ULONG, *mut c_void, ULONG, *mut c_void, *mut c_void) -> ULONG;
type HttpSetServiceConfigurationFn = unsafe extern "system" fn(HANDLE, ULONG, LPVOID, ULONG, *mut c_void) -> ULONG;
type HttpWaitForDisconnectFn = unsafe extern "system" fn(HTTP_REQUEST_QUEUE_ID, HTTP_CONNECTION_ID, *mut c_void) -> ULONG;

// ── Runtime-loaded API table ──────────────────────────────────

pub struct HttpApiTable {
    http_initialize: Option<HttpInitializeFn>,
    http_terminate: Option<HttpTerminateFn>,
    http_create_server_session: Option<HttpCreateServerSessionFn>,
    http_close_server_session: Option<HttpCloseServerSessionFn>,
    http_create_url_group: Option<HttpCreateUrlGroupFn>,
    http_close_url_group: Option<HttpCloseUrlGroupFn>,
    http_create_request_queue: Option<HttpCreateRequestQueueFn>,
    http_close_request_queue: Option<HttpCloseRequestQueueFn>,
    http_set_url_group_property: Option<HttpSetUrlGroupPropertyFn>,
    http_add_url_to_url_group: Option<HttpAddUrlToUrlGroupFn>,
    http_remove_url_from_url_group: Option<HttpRemoveUrlFromUrlGroupFn>,
    http_receive_http_request: Option<HttpReceiveHttpRequestFn>,
    http_send_http_response: Option<HttpSendHttpResponseFn>,
    http_send_response_entity_body: Option<HttpSendResponseEntityBodyFn>,
    http_set_service_configuration: Option<HttpSetServiceConfigurationFn>,
    http_wait_for_disconnect: Option<HttpWaitForDisconnectFn>,
    _module: *mut c_void,
}

macro_rules! get_fn {
    ($module:expr, $name:literal) => {{
        let name: Vec<u8> = concat!($name, "\0").as_bytes().to_vec();
        let addr = unsafe { GetProcAddress($module as _, name.as_ptr() as *const _) };
        if addr.is_null() {
            return Err(HttpApiError::Init(format!("GetProcAddress({}) failed", $name)));
        }
        Some(unsafe { std::mem::transmute::<*mut c_void, _>(addr as *mut c_void) })
    }};
}

impl HttpApiTable {
    pub fn load() -> Result<Self, HttpApiError> {
        use std::os::windows::ffi::OsStrExt;
        let dll_name: Vec<u16> = std::ffi::OsStr::new("httpapi.dll")
            .encode_wide().chain(Some(0)).collect();
        let module = unsafe { LoadLibraryW(dll_name.as_ptr()) };
        if module.is_null() {
            return Err(HttpApiError::Init("LoadLibraryW(httpapi.dll) failed".into()));
        }
        let module = module as *mut c_void;
        Ok(HttpApiTable {
            http_initialize: get_fn!(module, "HttpInitialize"),
            http_terminate: get_fn!(module, "HttpTerminate"),
            http_create_server_session: get_fn!(module, "HttpCreateServerSession"),
            http_close_server_session: get_fn!(module, "HttpCloseServerSession"),
            http_create_url_group: get_fn!(module, "HttpCreateUrlGroup"),
            http_close_url_group: get_fn!(module, "HttpCloseUrlGroup"),
            http_create_request_queue: get_fn!(module, "HttpCreateRequestQueue"),
            http_close_request_queue: get_fn!(module, "HttpCloseRequestQueue"),
            http_set_url_group_property: get_fn!(module, "HttpSetUrlGroupProperty"),
            http_add_url_to_url_group: get_fn!(module, "HttpAddUrlToUrlGroup"),
            http_remove_url_from_url_group: get_fn!(module, "HttpRemoveUrlFromUrlGroup"),
            http_receive_http_request: get_fn!(module, "HttpReceiveHttpRequest"),
            http_send_http_response: get_fn!(module, "HttpSendHttpResponse"),
            http_send_response_entity_body: get_fn!(module, "HttpSendResponseEntityBody"),
            http_set_service_configuration: get_fn!(module, "HttpSetServiceConfiguration"),
            http_wait_for_disconnect: get_fn!(module, "HttpWaitForDisconnect"),
            _module: module,
        })
    }

    pub fn http_initialize(&self, version: HTTPAPI_VERSION, flags: ULONG, reserved: LPVOID) -> ULONG {
        unsafe { (self.http_initialize.unwrap())(version, flags, reserved) }
    }

    pub fn http_terminate(&self, flags: ULONG, reserved: LPVOID) -> ULONG {
        unsafe { (self.http_terminate.unwrap())(flags, reserved) }
    }

    pub fn http_create_server_session(&self, version: HTTPAPI_VERSION, session: *mut HTTP_SERVER_SESSION_ID, reserved: ULONG) -> ULONG {
        unsafe { (self.http_create_server_session.unwrap())(version, session, reserved) }
    }

    pub fn http_close_server_session(&self, id: HTTP_SERVER_SESSION_ID) -> ULONG {
        unsafe { (self.http_close_server_session.unwrap())(id) }
    }

    pub fn http_create_url_group(&self, session: HTTP_SERVER_SESSION_ID, url_group: *mut HTTP_URL_GROUP_ID, reserved: ULONG) -> ULONG {
        unsafe { (self.http_create_url_group.unwrap())(session, url_group, reserved) }
    }

    pub fn http_close_url_group(&self, id: HTTP_URL_GROUP_ID) -> ULONG {
        unsafe { (self.http_close_url_group.unwrap())(id) }
    }

    pub fn http_create_request_queue(&self, version: HTTPAPI_VERSION, name: LPCWSTR, attrs: *mut c_void, flags: ULONG, queue: *mut HTTP_REQUEST_QUEUE_ID) -> ULONG {
        unsafe { (self.http_create_request_queue.unwrap())(version, name, attrs, flags, queue) }
    }

    pub fn http_close_request_queue(&self, queue: HTTP_REQUEST_QUEUE_ID) -> ULONG {
        unsafe { (self.http_close_request_queue.unwrap())(queue) }
    }

    pub fn http_set_url_group_property(&self, group: HTTP_URL_GROUP_ID, prop: ULONG, info: LPVOID, len: ULONG) -> ULONG {
        unsafe { (self.http_set_url_group_property.unwrap())(group, prop, info, len) }
    }

    pub fn http_add_url_to_url_group(&self, group: HTTP_URL_GROUP_ID, url: LPCWSTR, ctx: u64, reserved: ULONG) -> ULONG {
        unsafe { (self.http_add_url_to_url_group.unwrap())(group, url, ctx, reserved) }
    }

    pub fn http_remove_url_from_url_group(&self, group: HTTP_URL_GROUP_ID, url: LPCWSTR, flags: ULONG) -> ULONG {
        unsafe { (self.http_remove_url_from_url_group.unwrap())(group, url, flags) }
    }

    pub fn http_receive_http_request(&self, queue: HTTP_REQUEST_QUEUE_ID, req_id: HTTP_REQUEST_ID, flags: ULONG, buf: *mut HTTP_REQUEST_V1, len: ULONG, bytes: *mut ULONG, overlapped: *mut c_void) -> ULONG {
        unsafe { (self.http_receive_http_request.unwrap())(queue, req_id, flags, buf, len, bytes, overlapped) }
    }

    pub fn http_send_http_response(&self, queue: HTTP_REQUEST_QUEUE_ID, req_id: HTTP_REQUEST_ID, flags: ULONG, resp: *mut HTTP_RESPONSE_V1, cache: *mut c_void, sent: *mut ULONG, reserved1: *mut c_void, reserved2: ULONG, overlapped: *mut c_void, log: *mut c_void) -> ULONG {
        unsafe { (self.http_send_http_response.unwrap())(queue, req_id, flags, resp, cache, sent, reserved1, reserved2, overlapped, log) }
    }

    pub fn http_send_response_entity_body(&self, queue: HTTP_REQUEST_QUEUE_ID, req_id: HTTP_REQUEST_ID, flags: ULONG, chunks: USHORT, chunks_ptr: *mut HTTP_DATA_CHUNK, sent: *mut ULONG, reserved1: *mut c_void, reserved2: ULONG, overlapped: *mut c_void, log: *mut c_void) -> ULONG {
        unsafe { (self.http_send_response_entity_body.unwrap())(queue, req_id, flags, chunks, chunks_ptr, sent, reserved1, reserved2, overlapped, log) }
    }

    pub fn http_set_service_configuration(&self, handle: HANDLE, config_id: ULONG, info: LPVOID, len: ULONG, overlapped: *mut c_void) -> ULONG {
        unsafe { (self.http_set_service_configuration.unwrap())(handle, config_id, info, len, overlapped) }
    }

    pub fn http_wait_for_disconnect(&self, queue: HTTP_REQUEST_QUEUE_ID, conn_id: HTTP_CONNECTION_ID, overlapped: *mut c_void) -> ULONG {
        unsafe { (self.http_wait_for_disconnect.unwrap())(queue, conn_id, overlapped) }
    }
}

impl Drop for HttpApiTable {
    fn drop(&mut self) {
        if !self._module.is_null() {
            unsafe { FreeLibrary(self._module as _); }
        }
    }
}

// ── Error code constants ──────────────────────────────────────

pub const NO_ERROR: ULONG = 0;
pub const ERROR_MORE_DATA: ULONG = 234;
pub const ERROR_IO_PENDING: ULONG = 997;
pub const ERROR_CONNECTION_INVALID: ULONG = 1229;

// ── High-level wrapper ────────────────────────────────────────

pub struct HttpApiServer {
    api: std::sync::Arc<HttpApiTable>,
    server_session_id: HTTP_SERVER_SESSION_ID,
    url_group_id: HTTP_URL_GROUP_ID,
    queue: HTTP_REQUEST_QUEUE_ID,
}

impl HttpApiServer {
    pub fn new(api: std::sync::Arc<HttpApiTable>) -> Result<Self, HttpApiError> {
        let ver = HTTPAPI_VERSION_2;
        let rc = api.http_initialize(ver, HTTP_INITIALIZE_SERVER, ptr::null_mut());
        eprintln!("DEBUG HttpApiServer::new: HttpInitialize = {}", rc);
        if rc != NO_ERROR {
            return Err(HttpApiError::Init(format!("HttpInitialize failed: {}", rc)));
        }

        let mut server_session_id: HTTP_SERVER_SESSION_ID = 0;
        let rc = api.http_create_server_session(ver, &mut server_session_id, 0);
        eprintln!("DEBUG HttpApiServer::new: HttpCreateServerSession = {} id={:#x}", rc, server_session_id);
        if rc != NO_ERROR {
            api.http_terminate(HTTP_INITIALIZE_SERVER, ptr::null_mut());
            return Err(HttpApiError::Init(format!("HttpCreateServerSession failed: {}", rc)));
        }

        let mut url_group_id: HTTP_URL_GROUP_ID = 0;
        let rc = api.http_create_url_group(server_session_id, &mut url_group_id, 0);
        eprintln!("DEBUG HttpApiServer::new: HttpCreateUrlGroup = {} id={:#x}", rc, url_group_id);
        if rc != NO_ERROR {
            api.http_close_server_session(server_session_id);
            api.http_terminate(HTTP_INITIALIZE_SERVER, ptr::null_mut());
            return Err(HttpApiError::Init(format!("HttpCreateUrlGroup failed: {}", rc)));
        }

        let mut queue: HTTP_REQUEST_QUEUE_ID = ptr::null_mut();
        let rc = api.http_create_request_queue(ver, ptr::null(), ptr::null_mut(), 0, &mut queue);
        if rc != NO_ERROR {
            api.http_close_url_group(url_group_id);
            api.http_close_server_session(server_session_id);
            api.http_terminate(HTTP_INITIALIZE_SERVER, ptr::null_mut());
            return Err(HttpApiError::Init(format!("HttpCreateRequestQueue failed: {}", rc)));
        }

        eprintln!("DEBUG HttpApiServer::new: all init ok sid={:#x} gid={:#x} queue={:p}",
            server_session_id, url_group_id, queue);

        Ok(HttpApiServer {
            api,
            server_session_id,
            url_group_id,
            queue,
        })
    }

    /// Bind the URL group to the request queue.
    /// Must be called AFTER adding URLs to the URL group.
    pub fn register_url(&self, url: &str) -> Result<(), HttpApiError> {
        let fn_ptr = self.api.http_add_url_to_url_group.unwrap();
        let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let rc = unsafe { (fn_ptr)(self.url_group_id, wide.as_ptr(), 0_u64, 0_u32) };
        eprintln!("DEBUG register_url: HttpAddUrlToUrlGroup({}) = {} (group={:#x})",
            url, rc, self.url_group_id);
        if rc != NO_ERROR {
            return Err(HttpApiError::UrlRegister(url.to_string(), rc));
        }
        Ok(())
    }

    /// Quick test that HttpAddUrlToUrlGroup works from HttpApiServer.
    /// Only active when running as admin (required for URL registration).
    pub fn test_url_registration(&self, url: &str) -> Result<(), HttpApiError> {
        let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let fn_ptr = self.api.http_add_url_to_url_group.unwrap();
        let fn_addr = fn_ptr as *const std::ffi::c_void;
        eprintln!("DEBUG test_url_reg: fn_ptr={:p} group={:#x} url={:?} wide={:p}",
            fn_addr, self.url_group_id, url, wide.as_ptr());
        let rc = unsafe { (fn_ptr)(self.url_group_id, wide.as_ptr(), 0_u64, 0_u32) };
        eprintln!("DEBUG test_url_reg: rc={}", rc);
        if rc != NO_ERROR {
            return Err(HttpApiError::UrlRegister(url.to_string(), rc));
        }
        Ok(())
    }

    /// Expose the URL group ID for diagnostic purposes.
    pub fn url_group_id(&self) -> u64 {
        self.url_group_id
    }

    pub fn bind_queue(&self) -> Result<(), HttpApiError> {
        eprintln!("DEBUG bind_queue: url_group_id={}", self.url_group_id);
        let binding_info = HTTP_BINDING_INFO {
            flags: HTTP_PROPERTY_FLAGS { present: 1 },
            request_queue_handle: self.queue,
        };
        let rc = self.api.http_set_url_group_property(
            self.url_group_id,
            HttpServerBindingProperty,
            &binding_info as *const _ as LPVOID,
            std::mem::size_of::<HTTP_BINDING_INFO>() as ULONG,
        );
        eprintln!("DEBUG bind_queue: returned {}", rc);
        if rc != NO_ERROR {
            return Err(HttpApiError::Init(format!(
                "HttpSetUrlGroupProperty failed: {}",
                rc
            )));
        }
        Ok(())
    }

    pub fn receive_request(
        &self,
    ) -> Result<(HTTP_REQUEST_ID, Box<HTTP_REQUEST_V1>), HttpApiError> {
        let request_size = std::mem::size_of::<HTTP_REQUEST_V1>();
        let mut buf = Box::new(unsafe { std::mem::zeroed::<HTTP_REQUEST_V1>() });
        let mut bytes_received: ULONG = 0;
        let rc = self.api.http_receive_http_request(
            self.queue,
            ptr::null_mut(),
            0,
            buf.as_mut() as *mut HTTP_REQUEST_V1,
            request_size as ULONG,
            &mut bytes_received,
            ptr::null_mut(),
        );
        match rc {
            NO_ERROR => Ok((buf.request_id, buf)),
            ERROR_MORE_DATA => {
                let req_id_retry = buf.request_id;
                let sz = bytes_received;
                let mut bigger = vec![0u8; sz as usize].into_boxed_slice();
                let ptr = bigger.as_mut_ptr() as *mut HTTP_REQUEST_V1;
                let mut bytes2: ULONG = 0;
                let rc2 = self.api.http_receive_http_request(
                    self.queue,
                    req_id_retry,
                    0,
                    ptr,
                    sz,
                    &mut bytes2,
                    ptr::null_mut(),
                );
                if rc2 == NO_ERROR {
                    let req = unsafe { Box::from_raw(ptr) };
                    std::mem::forget(bigger);
                    Ok((req.request_id, req))
                } else {
                    Err(HttpApiError::Win32(rc2))
                }
            }
            _ => Err(HttpApiError::Win32(rc)),
        }
    }

    pub fn send_response(
        &self,
        request_id: HTTP_REQUEST_ID,
        status_code: USHORT,
        body: &[u8],
        content_type: &str,
    ) -> Result<(), HttpApiError> {
        let ct_bytes: Vec<UCHAR> = content_type.bytes().collect();
        let cl_str = body.len().to_string();
        let cl_bytes: Vec<UCHAR> = cl_str.bytes().collect();
        let mut response: HTTP_RESPONSE_V1 =
            unsafe { std::mem::zeroed::<HTTP_RESPONSE_V1>() };
        response.version = HTTP_VERSION { major: 1, minor: 1 };
        response.status_code = status_code;
        response.p_reason = b"OK\0".as_ptr() as *const UCHAR;

        let data_chunk_ref = HTTP_DATA_CHUNK_FROM_MEMORY {
            p_buffer: body.as_ptr(),
            buffer_length: body.len() as ULONG,
        };
        let data_chunk = HTTP_DATA_CHUNK {
            data_chunk_type: HttpDataChunkFromMemory,
            _padding: 0,
            u: HTTP_DATA_CHUNK_UNION { from_memory: data_chunk_ref },
        };

        if !body.is_empty() {
            response.entity_chunk_count = 1;
            response.p_entity_chunks = &data_chunk as *const _ as *mut HTTP_DATA_CHUNK;
        }

        let mut bytes_sent: ULONG = 0;
        let flags: ULONG = 0;
        let rc = self.api.http_send_http_response(
            self.queue,
            request_id,
            flags,
            &mut response,
            ptr::null_mut(),
            &mut bytes_sent,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        );
        if rc != NO_ERROR && rc != ERROR_IO_PENDING {
            return Err(HttpApiError::Win32(rc));
        }

        Ok(())
    }

    /// Return the server session ID for use with HttpSetServiceConfiguration
    pub fn server_session_id(&self) -> HTTP_SERVER_SESSION_ID {
        self.server_session_id
    }

    /// Bind an SSL certificate to the server for a specific SNI hostname.
    ///
    /// `host` — SNI hostname (e.g. "example.com") or empty string for default.
    /// `thumbprint` — 20-byte SHA-1 hash of the certificate.
    /// `store_name` — certificate store name, e.g. "MY".
    pub fn bind_ssl_cert_sni(
        &self,
        host: &str,
        thumbprint: &[u8; 20],
        store_name: &str,
    ) -> Result<(), HttpApiError> {
        let host_wide: Vec<u16> = host.encode_utf16().chain(std::iter::once(0)).collect();
        let store_wide: Vec<u16> = store_name.encode_utf16().chain(std::iter::once(0)).collect();

        let param = HTTP_SERVICE_CONFIG_SSL_PARAM {
            cert_hash: *thumbprint,
            cert_store_name: store_wide.as_ptr() as *mut u16,
            cert_check_group: 0,
            flags: 0,
        };

        let key = HTTP_SERVICE_CONFIG_SSL_SNI_KEY {
            host: host_wide.as_ptr() as *mut u16,
        };

        let set = HTTP_SERVICE_CONFIG_SSL_SNI_SET {
            key_desc: key,
            param_desc: param,
        };

        let rc = self.api.http_set_service_configuration(
            self.server_session_id as HANDLE,
            HttpServiceConfigSSLSniCertInfo,
            &set as *const _ as LPVOID,
            std::mem::size_of::<HTTP_SERVICE_CONFIG_SSL_SNI_SET>() as ULONG,
            ptr::null_mut(),
        );
        if rc != NO_ERROR && rc != ERROR_IO_PENDING {
            return Err(HttpApiError::Win32(rc));
        }
        Ok(())
    }

    pub fn table(&self) -> &HttpApiTable {
        &self.api
    }
}

impl Drop for HttpApiServer {
    fn drop(&mut self) {
        self.api.http_close_request_queue(self.queue);
        self.api.http_close_url_group(self.url_group_id);
        self.api.http_close_server_session(self.server_session_id);
        self.api.http_terminate(HTTP_INITIALIZE_SERVER, ptr::null_mut());
    }
}

// Force Send for HttpApiTable even though it holds a raw module handle.
// The handle is only used/dropped from this thread. HttpApiServer auto-derives
// Send because all its fields (Arc<HttpApiTable>, *mut c_void handles) are Send.
unsafe impl Send for HttpApiTable {}
unsafe impl Sync for HttpApiTable {}

unsafe impl Send for HttpApiServer {}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn httpapi_version_size() {
        assert_eq!(std::mem::size_of::<HTTPAPI_VERSION>(), 4);
    }

    #[test]
    fn http_request_v1_size() {
        // HTTP_REQUEST_V1 should be reasonably sized (not zero, not absurd)
        let sz = std::mem::size_of::<HTTP_REQUEST_V1>();
        assert!(sz > 100);
        assert!(sz < 2000);
    }

    #[test]
    fn error_message_format() {
        let err = HttpApiError::Win32(5);
        let msg = format!("{}", err);
        assert!(msg.contains("5"));
    }

    /// Test basic HTTP API initialization and URL registration.
    /// Uses GetProcAddress (function pointer) approach, same as the main library.
    #[test]
    fn test_httpapi_url_registration() {
        use std::os::windows::ffi::OsStrExt;

        let dll_name: Vec<u16> = std::ffi::OsStr::new("httpapi.dll")
            .encode_wide().chain(Some(0)).collect();
        let module = unsafe { LoadLibraryW(dll_name.as_ptr()) };
        assert!(!module.is_null(), "LoadLibraryW failed: {}", std::io::Error::last_os_error());

        let module = module as *mut c_void;

        macro_rules! get_fn {
            ($name:literal) => {{
                let name: Vec<u8> = concat!($name, "\0").as_bytes().to_vec();
                let addr = unsafe { GetProcAddress(module as _, name.as_ptr() as *const _) };
                assert!(!addr.is_null(), "GetProcAddress({}) failed: {}", $name, std::io::Error::last_os_error());
                unsafe { std::mem::transmute::<*mut c_void, _>(addr as *mut c_void) }
            }};
        }

        type HttpInitializeFn = unsafe extern "system" fn(HTTPAPI_VERSION, ULONG, LPVOID) -> ULONG;
        type HttpCreateServerSessionFn = unsafe extern "system" fn(HTTPAPI_VERSION, *mut HTTP_SERVER_SESSION_ID, ULONG) -> ULONG;
        type HttpCreateUrlGroupFn = unsafe extern "system" fn(HTTP_SERVER_SESSION_ID, *mut HTTP_URL_GROUP_ID, ULONG) -> ULONG;
        type HttpCreateRequestQueueFn = unsafe extern "system" fn(HTTPAPI_VERSION, LPCWSTR, LPVOID, ULONG, *mut HTTP_REQUEST_QUEUE_ID) -> ULONG;
        type HttpAddUrlToUrlGroupFn = unsafe extern "system" fn(HTTP_URL_GROUP_ID, LPCWSTR, u64, ULONG) -> ULONG;
        type HttpSetUrlGroupPropertyFn = unsafe extern "system" fn(HTTP_URL_GROUP_ID, ULONG, LPVOID, ULONG) -> ULONG;
        type HttpCloseRequestQueueFn = unsafe extern "system" fn(HTTP_REQUEST_QUEUE_ID) -> ULONG;
        type HttpCloseUrlGroupFn = unsafe extern "system" fn(HTTP_URL_GROUP_ID) -> ULONG;
        type HttpCloseServerSessionFn = unsafe extern "system" fn(HTTP_SERVER_SESSION_ID) -> ULONG;
        type HttpTerminateFn = unsafe extern "system" fn(ULONG, LPVOID) -> ULONG;

        let http_initialize: HttpInitializeFn = get_fn!("HttpInitialize");
        let http_create_server_session: HttpCreateServerSessionFn = get_fn!("HttpCreateServerSession");
        let http_create_url_group: HttpCreateUrlGroupFn = get_fn!("HttpCreateUrlGroup");
        let http_create_request_queue: HttpCreateRequestQueueFn = get_fn!("HttpCreateRequestQueue");
        let http_add_url_to_url_group: HttpAddUrlToUrlGroupFn = get_fn!("HttpAddUrlToUrlGroup");
        let http_set_url_group_property: HttpSetUrlGroupPropertyFn = get_fn!("HttpSetUrlGroupProperty");
        let http_close_request_queue: HttpCloseRequestQueueFn = get_fn!("HttpCloseRequestQueue");
        let http_close_url_group: HttpCloseUrlGroupFn = get_fn!("HttpCloseUrlGroup");
        let http_close_server_session: HttpCloseServerSessionFn = get_fn!("HttpCloseServerSession");
        let http_terminate: HttpTerminateFn = get_fn!("HttpTerminate");

        unsafe {
            let ver = HTTPAPI_VERSION_2;
            let mut rc: ULONG;

            rc = http_initialize(ver, HTTP_INITIALIZE_SERVER, ptr::null_mut());
            eprintln!("HttpInitialize: {}", rc);
            assert_eq!(rc, NO_ERROR);

            let mut sid: HTTP_SERVER_SESSION_ID = 0;
            rc = http_create_server_session(ver, &mut sid, 0);
            eprintln!("HttpCreateServerSession: {} sid={:#x}", rc, sid);
            assert_eq!(rc, NO_ERROR);

            let mut gid: HTTP_URL_GROUP_ID = 0;
            rc = http_create_url_group(sid, &mut gid, 0);
            eprintln!("HttpCreateUrlGroup: {} gid={:#x}", rc, gid);
            assert_eq!(rc, NO_ERROR);

            let mut queue: HTTP_REQUEST_QUEUE_ID = ptr::null_mut();
            rc = http_create_request_queue(ver, ptr::null(), ptr::null_mut(), 0, &mut queue);
            eprintln!("HttpCreateRequestQueue: {} queue={:p}", rc, queue);
            assert_eq!(rc, NO_ERROR);

            // Register URL with u64 for UrlContext
            let url = "http://+:9996/";
            let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
            // Use u64 for the context parameter (HTTP_URL_CONTEXT = ULONGLONG = u64)
            rc = http_add_url_to_url_group(gid, wide.as_ptr(), 0_u64, 0);
            eprintln!("HttpAddUrlToUrlGroup (u64): {}", rc);

            // Bind
            let bi = HTTP_BINDING_INFO {
                flags: HTTP_PROPERTY_FLAGS { present: 1 },
                request_queue_handle: queue,
            };
            rc = http_set_url_group_property(gid, HttpServerBindingProperty, &bi as *const _ as LPVOID, std::mem::size_of::<HTTP_BINDING_INFO>() as ULONG);
            eprintln!("HttpSetUrlGroupProperty: {}", rc);

            // Cleanup
            http_close_request_queue(queue);
            http_close_url_group(gid);
            http_close_server_session(sid);
            http_terminate(HTTP_INITIALIZE_SERVER, ptr::null_mut());
            eprintln!("=== Test cleanup done ===");
        }
    }
}
