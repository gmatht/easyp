//! Windows WinHTTP implementation (HTTP/2 + HTTP/3).
//!
//! All functions linked at compile time against `winhttp.dll`.

use std::collections::HashMap;
use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use thiserror::Error;

// ── Type aliases ─────────────────────────────────────────────

type DWORD = u32;
type BOOL = i32;
type LPCWSTR = *const u16;
type LPWSTR = *mut u16;
type LPVOID = *mut c_void;
type LPCVOID = *const c_void;
type HINTERNET = LPVOID;
type LPDWORD = *mut DWORD;

const TRUE: BOOL = 1;
const FALSE: BOOL = 0;
const INVALID_HANDLE_VALUE: HINTERNET = usize::MAX as *mut c_void;

// ── WinHTTP constants ────────────────────────────────────────

const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: DWORD = 0;
const WINHTTP_FLAG_SECURE: DWORD = 0x00800000;
const WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL: DWORD = 139;
const WINHTTP_PROTOCOL_FLAG_HTTP2: DWORD = 0x00000001;
const WINHTTP_PROTOCOL_FLAG_HTTP3: DWORD = 0x00000002;
const WINHTTP_NO_REFERER: LPCWSTR = ptr::null();
const WINHTTP_DEFAULT_ACCEPT_TYPES: *mut LPCWSTR = ptr::null_mut();

const ERROR_INSUFFICIENT_BUFFER: DWORD = 122;
const ERROR_INVALID_PARAMETER: DWORD = 87;

// ── FFI declarations ─────────────────────────────────────────

#[link(name = "winhttp")]
extern "system" {
    fn WinHttpOpen(
        psz_agent_w: LPCWSTR,
        dw_access_type: DWORD,
        psz_proxy_w: LPCWSTR,
        psz_proxy_bypass_w: LPCWSTR,
        dw_flags: DWORD,
    ) -> HINTERNET;

    fn WinHttpCloseHandle(h_internet: HINTERNET) -> BOOL;

    fn WinHttpSetOption(
        h_internet: HINTERNET,
        dw_option: DWORD,
        lp_buffer: LPCVOID,
        dw_buffer_length: DWORD,
    ) -> BOOL;

    fn WinHttpConnect(
        h_session: HINTERNET,
        psz_server_name: LPCWSTR,
        n_server_port: DWORD,
        dw_reserved: DWORD,
    ) -> HINTERNET;

    fn WinHttpOpenRequest(
        h_connect: HINTERNET,
        psz_verb: LPCWSTR,
        psz_object_name: LPCWSTR,
        psz_version: LPCWSTR,
        psz_referrer: LPCWSTR,
        ppwsz_accept_types: *mut LPCWSTR,
        dw_flags: DWORD,
    ) -> HINTERNET;

    fn WinHttpSendRequest(
        h_request: HINTERNET,
        psz_headers: LPCWSTR,
        dw_headers_length: DWORD,
        lp_optional: LPVOID,
        dw_optional_length: DWORD,
        dw_total_length: DWORD,
        dw_context: DWORD,
    ) -> BOOL;

    fn WinHttpReceiveResponse(
        h_request: HINTERNET,
        lp_reserved: LPVOID,
    ) -> BOOL;

    fn WinHttpReadData(
        h_request: HINTERNET,
        lp_buffer: LPVOID,
        dw_number_of_bytes_to_read: DWORD,
        lpdw_number_of_bytes_read: LPDWORD,
    ) -> BOOL;

    fn WinHttpQueryHeaders(
        h_request: HINTERNET,
        dw_info_level: DWORD,
        pwsz_name: LPCWSTR,
        lp_buffer: LPVOID,
        lpdw_buffer_length: LPDWORD,
        lpdw_index: LPDWORD,
    ) -> BOOL;

    // For error handling
    fn GetLastError() -> DWORD;
}

// ── Helpers ──────────────────────────────────────────────────

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn to_wide_opt(s: Option<&str>) -> Vec<u16> {
    match s {
        Some(s) => to_wide(s),
        None => vec![0],
    }
}

fn wide_to_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}

// ── Error ────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum Error {
    #[error("WinHTTP error: {0}")]
    WinHttp(DWORD),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("feature unsupported on this Windows version")]
    Unsupported,
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
}

fn last_error() -> Error {
    Error::WinHttp(unsafe { GetLastError() })
}

// ── Response ─────────────────────────────────────────────────

pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

// ── Session ──────────────────────────────────────────────────

pub struct Session {
    h_session: HINTERNET,
    h2_enabled: bool,
    h3_enabled: bool,
}

impl Session {
    pub fn new() -> Result<Self, Error> {
        let agent = to_wide("lsb-winhttp");
        let h = unsafe {
            WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            )
        };
        if h.is_null() || h == INVALID_HANDLE_VALUE as HINTERNET {
            return Err(last_error());
        }
        Ok(Session { h_session: h, h2_enabled: false, h3_enabled: false })
    }

    /// Enable HTTP/2. Returns `Err(Unsupported)` on old Windows builds.
    pub fn enable_http2(&mut self) -> Result<(), Error> {
        if self.h2_enabled {
            return Ok(());
        }
        let flags = WINHTTP_PROTOCOL_FLAG_HTTP2;
        let ok = unsafe {
            WinHttpSetOption(
                self.h_session,
                WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL,
                &flags as *const _ as LPCVOID,
                std::mem::size_of::<DWORD>() as DWORD,
            )
        };
        if ok == FALSE {
            let err = unsafe { GetLastError() };
            if err == ERROR_INVALID_PARAMETER {
                return Err(Error::Unsupported);
            }
            return Err(Error::WinHttp(err));
        }
        self.h2_enabled = true;
        Ok(())
    }

    /// Enable HTTP/3 via WinHTTP. Requires Windows 11+.
    /// Returns `Err(Unsupported)` on older builds.
    pub fn enable_http3(&mut self) -> Result<(), Error> {
        if self.h3_enabled {
            return Ok(());
        }
        let flags = WINHTTP_PROTOCOL_FLAG_HTTP3;
        let ok = unsafe {
            WinHttpSetOption(
                self.h_session,
                WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL,
                &flags as *const _ as LPCVOID,
                std::mem::size_of::<DWORD>() as DWORD,
            )
        };
        if ok == FALSE {
            let err = unsafe { GetLastError() };
            if err == ERROR_INVALID_PARAMETER {
                return Err(Error::Unsupported);
            }
            return Err(Error::WinHttp(err));
        }
        self.h3_enabled = true;
        Ok(())
    }

    /// Enable both HTTP/2 and HTTP/3.
    pub fn enable_http2_and_3(&mut self) -> Result<(), Error> {
        let flags = WINHTTP_PROTOCOL_FLAG_HTTP2 | WINHTTP_PROTOCOL_FLAG_HTTP3;
        let ok = unsafe {
            WinHttpSetOption(
                self.h_session,
                WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL,
                &flags as *const _ as LPCVOID,
                std::mem::size_of::<DWORD>() as DWORD,
            )
        };
        if ok == FALSE {
            let err = unsafe { GetLastError() };
            if err == ERROR_INVALID_PARAMETER {
                // Try just HTTP/2
                self.enable_http2()
            } else {
                return Err(Error::WinHttp(err));
            }
        } else {
            self.h2_enabled = true;
            self.h3_enabled = true;
            Ok(())
        }
    }

    /// Perform a GET request.
    pub fn get(&self, url: &str) -> Result<Response, Error> {
        let (host, port, path) = parse_url(url)?;

        let host_wide = to_wide(host);
        let path_wide = to_wide(path);
        let verb_wide = to_wide("GET");

        let h_connect = unsafe {
            WinHttpConnect(
                self.h_session,
                host_wide.as_ptr(),
                port as DWORD,
                0,
            )
        };
        if h_connect.is_null() {
            return Err(last_error());
        }

        let h_request = unsafe {
            WinHttpOpenRequest(
                h_connect,
                verb_wide.as_ptr(),
                path_wide.as_ptr(),
                ptr::null(),
                WINHTTP_NO_REFERER,
                WINHTTP_DEFAULT_ACCEPT_TYPES,
                WINHTTP_FLAG_SECURE,
            )
        };
        if h_request.is_null() {
            let err = last_error();
            unsafe { WinHttpCloseHandle(h_connect); }
            return Err(err);
        }

        let ok = unsafe {
            WinHttpSendRequest(
                h_request,
                ptr::null(),
                0,
                ptr::null_mut(),
                0,
                0,
                0,
            )
        };
        if ok == FALSE {
            let err = last_error();
            unsafe { WinHttpCloseHandle(h_request); WinHttpCloseHandle(h_connect); }
            return Err(err);
        }

        let ok = unsafe { WinHttpReceiveResponse(h_request, ptr::null_mut()) };
        if ok == FALSE {
            let err = last_error();
            unsafe { WinHttpCloseHandle(h_request); WinHttpCloseHandle(h_connect); }
            return Err(err);
        }

        // Read status code
        let mut status_code: DWORD = 0;
        let mut buf_len: DWORD = std::mem::size_of::<DWORD>() as DWORD;
        unsafe {
            WinHttpQueryHeaders(
                h_request,
                19, // WINHTTP_QUERY_STATUS_CODE
                ptr::null(),
                &mut status_code as *mut _ as LPVOID,
                &mut buf_len,
                ptr::null_mut(),
            );
        }

        // Read response headers
        let mut headers_buf_len: DWORD = 0;
        unsafe {
            WinHttpQueryHeaders(
                h_request,
                22, // WINHTTP_QUERY_RAW_HEADERS_CRLF
                ptr::null(),
                ptr::null_mut(),
                &mut headers_buf_len,
                ptr::null_mut(),
            );
        }
        let mut headers_wide: Vec<u16> = Vec::with_capacity((headers_buf_len / 2) as usize);
        let mut actual_headers_len = headers_buf_len;
        if headers_buf_len > 0 {
            headers_wide.resize((headers_buf_len / 2) as usize, 0);
            unsafe {
                WinHttpQueryHeaders(
                    h_request,
                    22,
                    ptr::null(),
                    headers_wide.as_mut_ptr() as LPVOID,
                    &mut actual_headers_len,
                    ptr::null_mut(),
                );
            }
        }
        let headers_str = wide_to_string(&headers_wide);
        let headers = parse_headers(&headers_str);

        // Read response body
        let mut body = Vec::new();
        let mut tmp = [0u8; 8192];
        loop {
            let mut bytes_read: DWORD = 0;
            let ok = unsafe {
                WinHttpReadData(
                    h_request,
                    tmp.as_mut_ptr() as LPVOID,
                    tmp.len() as DWORD,
                    &mut bytes_read,
                )
            };
            if ok == FALSE || bytes_read == 0 {
                break;
            }
            body.extend_from_slice(&tmp[..bytes_read as usize]);
        }

        unsafe { WinHttpCloseHandle(h_request); WinHttpCloseHandle(h_connect); }

        Ok(Response {
            status: status_code as u16,
            headers,
            body,
        })
    }
}

unsafe impl Send for Session {}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { WinHttpCloseHandle(self.h_session); }
    }
}

// ── URL parser ───────────────────────────────────────────────

fn parse_url(url: &str) -> Result<(&str, u16, &str), Error> {
    let rest = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| Error::InvalidUrl("must start with http:// or https://".into()))?;

    let (host_port, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(i) => (
            &host_port[..i],
            host_port[i+1..].parse::<u16>().unwrap_or(443),
        ),
        None => (host_port, 443),
    };

    Ok((host, port, path))
}

// ── Header parser ────────────────────────────────────────────

fn parse_headers(raw: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in raw.lines() {
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }
    headers
}

// ── Utility ──────────────────────────────────────────────────

pub fn is_winhttp_available() -> bool {
    true
}

pub fn version() -> &'static str {
    "WinHTTP (Windows native)"
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let s = Session::new();
        assert!(s.is_ok(), "session: {:?}", s.err());
    }
}
