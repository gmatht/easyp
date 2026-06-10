use std::os::raw::{c_int, c_void};
use lsb_loader::LoadedLibrary;

/// State for a single HTTP/2 session. Created per-connection.
pub struct Session {
    api: &'static Nghttp2Api,
    session: *mut c_void,
    cbs: *mut c_void,
    inner: *mut SessionInner,
    data_prd: *mut Nghttp2DataPrd,
    body_box: *mut BodyState,
}

struct SessionInner {
    stream_id: i32,
    responded: bool,
    request_path: String,
}

impl Session {
    pub fn new() -> Result<Self, String> {
        let api = Nghttp2Api::load().map_err(|e| format!("{}", e))?;
        let api: &'static Nghttp2Api = Box::leak(Box::new(api));

        let inner = Box::into_raw(Box::new(SessionInner {
            stream_id: 0,
            responded: false,
            request_path: String::new(),
        }));

        unsafe {
            let mut cbs: *mut c_void = std::ptr::null_mut();
            let rc = (api.callbacks_new)(&mut cbs);
            if rc != 0 {
                let _ = Box::from_raw(inner);
                return Err("callbacks_new failed".into());
            }
            (api.set_on_begin_headers)(cbs, Some(on_begin_headers_cb));
            (api.set_on_header)(cbs, Some(on_header_cb));

            let mut session: *mut c_void = std::ptr::null_mut();
            let rc = (api.session_server_new)(&mut session, cbs, inner as *mut c_void);
            if rc != 0 {
                (api.callbacks_del)(cbs);
                let _ = Box::from_raw(inner);
                return Err("session_server_new failed".into());
            }

            Ok(Session { api, session, cbs, inner, data_prd: std::ptr::null_mut(), body_box: std::ptr::null_mut() })
        }
    }

    pub fn send_preface(&mut self) -> Result<Vec<u8>, String> {
        unsafe {
            let rc = (self.api.submit_settings)(self.session, 0, std::ptr::null(), 0);
            if rc != 0 {
                return Err(format!("submit_settings failed: {}", rc));
            }
        }
        self.collect_output()
    }

    /// Feed incoming frame data to nghttp2. This processes headers
    /// and captures the request path. Call `get_path()` after this.
    pub fn feed_frame_data(&mut self, data: &[u8]) -> Result<(), String> {
        let rc = unsafe { (self.api.mem_recv)(self.session, data.as_ptr(), data.len()) };
        if rc < 0 {
            return Err(format!("mem_recv failed: {}", rc));
        }
        Ok(())
    }

    /// Submit the HTTP/2 response with the given body bytes.
    /// Must be called after feed_frame_data when a request is ready.
    /// Submit the HTTP/2 response with body. Returns serialized output frames.
    /// For large bodies, the output can be many MB (all DATA frames combined).
    pub fn submit_body(&mut self, body: &[u8]) -> Result<Vec<u8>, String> {
        let sid = unsafe { (*self.inner).stream_id };
        if sid <= 0 {
            return Ok(self.collect_output()?);
        }
        unsafe { (*self.inner).responded = true; }

        // Free any previous data provider state
        if !self.data_prd.is_null() {
            unsafe { let _ = Box::from_raw(self.data_prd); }
            self.data_prd = std::ptr::null_mut();
        }
        if !self.body_box.is_null() {
            unsafe { let _ = Box::from_raw(self.body_box); }
            self.body_box = std::ptr::null_mut();
        }

        let out = if body.is_empty() {
            let nva = [
                nv(b":status", b"200"),
                nv(b"content-type", b"text/plain"),
            ];
            unsafe {
                (self.api.submit_response)(self.session, sid, nva.as_ptr(), 2, std::ptr::null());
            }
            self.collect_output()?
        } else {
            let content_len = body.len().to_string();
            let nva = [
                nv(b":status", b"200"),
                nv(b"content-type", b"application/octet-stream"),
                nv(b"content-length", content_len.as_bytes()),
            ];

            let body_box = Box::into_raw(Box::new(BodyState {
                data: body.to_vec(),
                offset: 0,
            }));

            let data_prd = Box::into_raw(Box::new(Nghttp2DataPrd {
                source: Nghttp2DataSource { ptr: body_box as *mut c_void },
                read_callback: Some(data_read_cb),
            }));

            self.body_box = body_box;
            self.data_prd = data_prd;

            unsafe {
                (self.api.submit_response)(self.session, sid, nva.as_ptr(), 3, data_prd as *const _ as *const c_void);
            }
            self.collect_output()?
        };

        Ok(out)
    }

    pub fn get_path(&self) -> String {
        unsafe { (*self.inner).request_path.clone() }
    }

    pub fn has_request(&self) -> bool {
        unsafe { (*self.inner).stream_id > 0 }
    }

    pub fn is_done(&self) -> bool {
        unsafe { (*self.inner).responded }
    }

    fn collect_output(&mut self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        for _ in 0..65536 {
            let mut data_ptr: *const u8 = std::ptr::null();
            let written = unsafe { (self.api.mem_send)(self.session, &mut data_ptr) };
            if written <= 0 {
                break;
            }
            let slice = unsafe { std::slice::from_raw_parts(data_ptr, written as usize) };
            out.extend_from_slice(slice);
        }
        Ok(out)
    }
}

unsafe impl Send for Session {}
unsafe impl Send for Nghttp2Api {}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            (self.api.session_del)(self.session);
            (self.api.callbacks_del)(self.cbs);
            if !self.data_prd.is_null() { let _ = Box::from_raw(self.data_prd); }
            if !self.body_box.is_null() { let _ = Box::from_raw(self.body_box); }
            let _ = Box::from_raw(self.inner);
        }
    }
}

// ── Data provider types (mirrors nghttp2 C API) ──────────────

#[repr(C)]
union Nghttp2DataSource {
    fd: i32,
    ptr: *mut c_void,
}

#[repr(C)]
struct Nghttp2DataPrd {
    source: Nghttp2DataSource,
    read_callback: Option<DataReadCb>,
}

type DataReadCb = unsafe extern "C" fn(
    *mut c_void, i32, *mut u8, usize, *mut u32, *mut Nghttp2DataSource, *mut c_void,
) -> isize;

struct BodyState {
    data: Vec<u8>,
    offset: usize,
}

unsafe extern "C" fn data_read_cb(
    _session: *mut c_void,
    _stream_id: i32,
    buf: *mut u8,
    length: usize,
    data_flags: *mut u32,
    source: *mut Nghttp2DataSource,
    _user_data: *mut c_void,
) -> isize {
    let state = &mut *((*source).ptr as *mut BodyState);
    let remaining = state.data.len() - state.offset;
    if remaining == 0 {
        *data_flags = 1; // NGHTTP2_DATA_FLAG_EOF
        return 0;
    }
    let to_copy = std::cmp::min(remaining, length);
    std::ptr::copy_nonoverlapping(state.data.as_ptr().add(state.offset), buf, to_copy);
    state.offset += to_copy;
    to_copy as isize
}

// ── Callback types and FFI ────────────────────────────────────

type BeginHdrsCb = unsafe extern "C" fn(*mut c_void, *const c_void, *mut c_void) -> c_int;
type OnHeaderCb = unsafe extern "C" fn(*mut c_void, *const c_void, *const u8, usize, *const u8, usize, u8, *mut c_void) -> c_int;

#[repr(C)]
struct Nghttp2FrameHd {
    _length: usize,
    stream_id: i32,
    _frame_type: u8,
    _flags: u8,
    _reserved: u8,
}

unsafe extern "C" fn on_begin_headers_cb(
    _session: *mut c_void,
    frame: *const c_void,
    user_data: *mut c_void,
) -> c_int {
    let hd = &*(frame as *const Nghttp2FrameHd);
    let inner = &mut *(user_data as *mut SessionInner);
    inner.stream_id = hd.stream_id;
    0
}

unsafe extern "C" fn on_header_cb(
    _session: *mut c_void,
    _frame: *const c_void,
    name: *const u8,
    namelen: usize,
    value: *const u8,
    valuelen: usize,
    _flags: u8,
    user_data: *mut c_void,
) -> c_int {
    let name_slice = std::slice::from_raw_parts(name, namelen);
    let value_slice = std::slice::from_raw_parts(value, valuelen);
    if name_slice == b":path" {
        let inner = &mut *(user_data as *mut SessionInner);
        if let Ok(s) = std::str::from_utf8(value_slice) {
            inner.request_path = s.to_string();
        }
    }
    0
}

fn nv(name: &'static [u8], value: &[u8]) -> Nghttp2Nv {
    Nghttp2Nv {
        name: name.as_ptr(),
        value: value.as_ptr(),
        namelen: name.len(),
        valuelen: value.len(),
        flags: 0,
    }
}

#[repr(C)]
struct Nghttp2Nv {
    name: *const u8,
    value: *const u8,
    namelen: usize,
    valuelen: usize,
    flags: u8,
}

#[repr(C)]
struct Nghttp2SettingsEntry {
    settings_id: i32,
    value: u32,
}

struct Nghttp2Api {
    lib: LoadedLibrary,
    callbacks_new: unsafe extern "C" fn(*mut *mut c_void) -> c_int,
    callbacks_del: unsafe extern "C" fn(*mut c_void),
    set_on_begin_headers: unsafe extern "C" fn(*mut c_void, Option<BeginHdrsCb>),
    set_on_header: unsafe extern "C" fn(*mut c_void, Option<OnHeaderCb>),
    session_server_new: unsafe extern "C" fn(*mut *mut c_void, *const c_void, *mut c_void) -> c_int,
    session_del: unsafe extern "C" fn(*mut c_void) -> c_int,
    mem_recv: unsafe extern "C" fn(*mut c_void, *const u8, usize) -> isize,
    mem_send: unsafe extern "C" fn(*mut c_void, *mut *const u8) -> isize,
    submit_response: unsafe extern "C" fn(*mut c_void, c_int, *const Nghttp2Nv, usize, *const c_void) -> c_int,
    submit_settings: unsafe extern "C" fn(*mut c_void, u8, *const Nghttp2SettingsEntry, usize) -> c_int,
}

impl Nghttp2Api {
    fn load() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let required = [
            "nghttp2_session_callbacks_new",
            "nghttp2_session_callbacks_del",
            "nghttp2_session_callbacks_set_on_begin_headers_callback",
            "nghttp2_session_callbacks_set_on_header_callback",
            "nghttp2_session_server_new",
            "nghttp2_session_del",
            "nghttp2_session_mem_recv",
            "nghttp2_session_mem_send",
            "nghttp2_submit_response",
            "nghttp2_submit_settings",
        ];
        let lib = match std::env::var("LSBWRAP_NGHTTP2_PATH") {
            Ok(path) => {
                if !path.starts_with('/') {
                    return Err("LSBWRAP_NGHTTP2_PATH must be absolute".into());
                }
                LoadedLibrary::load_explicit(&path, &required)?
            }
            Err(_) => LoadedLibrary::load_from_candidates(
                &["libnghttp2.so.14", "libnghttp2.so"],
                &required,
            )?,
        };

        unsafe {
            let callbacks_new = std::mem::transmute(lib.get_symbol_raw("nghttp2_session_callbacks_new")?);
            let callbacks_del = std::mem::transmute(lib.get_symbol_raw("nghttp2_session_callbacks_del")?);
            let set_on_begin_headers = std::mem::transmute(
                lib.get_symbol_raw("nghttp2_session_callbacks_set_on_begin_headers_callback")?,
            );
            let set_on_header = std::mem::transmute(
                lib.get_symbol_raw("nghttp2_session_callbacks_set_on_header_callback")?,
            );
            let session_server_new = std::mem::transmute(lib.get_symbol_raw("nghttp2_session_server_new")?);
            let session_del = std::mem::transmute(lib.get_symbol_raw("nghttp2_session_del")?);
            let mem_recv = std::mem::transmute(lib.get_symbol_raw("nghttp2_session_mem_recv")?);
            let mem_send = std::mem::transmute(lib.get_symbol_raw("nghttp2_session_mem_send")?);
            let submit_response = std::mem::transmute(lib.get_symbol_raw("nghttp2_submit_response")?);
            let submit_settings = std::mem::transmute(lib.get_symbol_raw("nghttp2_submit_settings")?);

            Ok(Nghttp2Api {
                lib, callbacks_new, callbacks_del,                 set_on_begin_headers, set_on_header,
                session_server_new, session_del, mem_recv, mem_send, submit_response, submit_settings,
            })
        }
    }
}
