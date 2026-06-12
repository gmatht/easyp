//! Windows SChannel TLS implementation via raw SSPI FFI.
//!
//! Provides:
//! - `Credentials` — wraps `AcquireCredentialsHandleW`
//! - `TlsConnection` — pure encrypt/decrypt engine (no I/O)
//! - `TlsStream` — blocking I/O wrapper combining TcpStream + TlsConnection

use std::collections::VecDeque;
use std::ffi::c_void;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::sync::OnceLock;
use std::ffi::OsStr;
use thiserror::Error;

type DWORD = u32;
type LONG = i32;
type ULONG = u32;
type USHORT = u16;
type UCHAR = u8;
type LPCWSTR = *const u16;
type LPWSTR = *mut u16;
type LPVOID = *mut c_void;
type LPCVOID = *const c_void;
type ULONG_PTR = usize;
type SECURITY_STATUS = LONG;

const SEC_E_OK: SECURITY_STATUS = 0x00000000;
const SEC_I_CONTINUE_NEEDED: SECURITY_STATUS = 0x00090312;
const SEC_E_INCOMPLETE_MESSAGE: SECURITY_STATUS = 0x80090318u32 as i32;
const SEC_E_INVALID_TOKEN: SECURITY_STATUS = 0x80090308u32 as i32;
const SEC_I_RENEGOTIATE: SECURITY_STATUS = 0x00090321;

const SECPKG_CRED_OUTBOUND: DWORD = 0x00000002;
const ISC_REQ_ALLOCATE_MEMORY: DWORD = 0x00000100;
const ISC_REQ_CONFIDENTIALITY: DWORD = 0x00000010;
const ISC_REQ_EXTENDED_ERROR: DWORD = 0x00004000;
const ISC_REQ_MANUAL_CRED_VALIDATION: DWORD = 0x00080000;
const ISC_REQ_SEQUENCE_DETECT: DWORD = 0x00000008;
const ISC_REQ_REPLAY_DETECT: DWORD = 0x00000004;
const ISC_REQ_INTEGRITY: DWORD = 0x00000020;
const ISC_REQ_STREAM: DWORD = 0x00008000;
const ISC_RET_ALLOCATED_MEMORY: DWORD = 0x00000100;

const SECBUFFER_VERSION: ULONG = 0;
const SECBUFFER_EMPTY: ULONG = 0;
const SECBUFFER_DATA: ULONG = 1;
const SECBUFFER_TOKEN: ULONG = 2;
const SECBUFFER_EXTRA: ULONG = 5;
const SECBUFFER_STREAM_TRAILER: ULONG = 6;
const SECBUFFER_STREAM_HEADER: ULONG = 7;

const SECPKG_ATTR_SIZES: DWORD = 0x0000000D;

const SCHANNEL_CRED_VERSION: DWORD = 0x00000004;
const SCH_CRED_MANUAL_CRED_VALIDATION: DWORD = 0x00000008;
const SCH_CRED_NO_SYSTEM_MAPPER: DWORD = 0x00000002;

#[repr(C)]
#[derive(Clone, Copy)]
struct SecHandle {
    dw_lower: ULONG_PTR,
    dw_upper: ULONG_PTR,
}

type CredHandle = SecHandle;
type CtxtHandle = SecHandle;

#[repr(C)]
struct TimeStamp {
    _low_part: LONG,
    _high_part: LONG,
}

#[repr(C)]
struct SecBuffer {
    cb_buffer: ULONG,
    buffer_type: ULONG,
    pv_buffer: LPVOID,
}

#[repr(C)]
struct SecBufferDesc {
    ul_version: ULONG,
    c_buffers: ULONG,
    p_buffers: *mut SecBuffer,
}

#[repr(C)]
struct SchannelCred {
    dw_version: DWORD,
    c_creds: DWORD,
    pa_cred: *mut c_void,
    h_root_store: *mut c_void,
    c_mappers: DWORD,
    aph_mappers: *mut c_void,
    c_supported_algs: DWORD,
    palg_supported_algs: *mut c_void,
    grbit_enabled_protocols: DWORD,
    dw_minimum_cipher_strength: DWORD,
    dw_maximum_cipher_strength: DWORD,
    dw_session_lifespan: DWORD,
    dw_flags: DWORD,
    dw_cred_format: DWORD,
}

#[repr(C)]
#[derive(Clone)]
struct SecPkgContextStreamSizes {
    cb_header: DWORD,
    cb_trailer: DWORD,
    cb_maximum_message: DWORD,
    c_buffers: DWORD,
    cb_block_size: DWORD,
}

#[link(name = "secur32")]
extern "system" {
    fn AcquireCredentialsHandleW(
        psz_principal: LPWSTR,
        psz_package: LPCWSTR,
        f_credential_use: DWORD,
        pv_logon_id: LPVOID,
        p_auth_data: LPVOID,
        p_get_key_fn: LPVOID,
        pv_get_key_argument: LPVOID,
        ph_credential: *mut CredHandle,
        pts_expiry: *mut TimeStamp,
    ) -> SECURITY_STATUS;

    fn FreeCredentialsHandle(ph_credential: *mut CredHandle) -> SECURITY_STATUS;

    fn InitializeSecurityContextW(
        ph_credential: *mut CredHandle,
        ph_context: *mut CtxtHandle,
        psz_target_name: LPWSTR,
        f_context_req: DWORD,
        reserved1: DWORD,
        target_data_rep: DWORD,
        p_input: *mut SecBufferDesc,
        reserved2: DWORD,
        ph_new_context: *mut CtxtHandle,
        p_output: *mut SecBufferDesc,
        pf_context_attr: *mut DWORD,
        pts_expiry: *mut TimeStamp,
    ) -> SECURITY_STATUS;

    fn DeleteSecurityContext(ph_context: *mut CtxtHandle) -> SECURITY_STATUS;

    fn EncryptMessage(
        ph_context: *mut CtxtHandle,
        f_qop: DWORD,
        p_message: *mut SecBufferDesc,
        message_seq_no: DWORD,
    ) -> SECURITY_STATUS;

    fn DecryptMessage(
        ph_context: *mut CtxtHandle,
        p_message: *mut SecBufferDesc,
        message_seq_no: DWORD,
        pf_qop: *mut DWORD,
    ) -> SECURITY_STATUS;

    fn QueryContextAttributesW(
        ph_context: *mut CtxtHandle,
        ul_attribute: DWORD,
        p_buffer: LPVOID,
    ) -> SECURITY_STATUS;

    fn FreeContextBuffer(pv_buffer: LPVOID) -> SECURITY_STATUS;
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn unisp_name() -> &'static [u16] {
    static UNISP: OnceLock<Vec<u16>> = OnceLock::new();
    UNISP.get_or_init(|| to_wide("Microsoft Unified Security Protocol Provider"))
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("SSPI error: 0x{0:08x}")]
    Sspi(SECURITY_STATUS),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("certificate validation failed")]
    CertValidationFailed,
    #[error("renegotiation not supported")]
    Renegotiation,
}

// ── Credentials ──────────────────────────────────────────────

pub struct Credentials {
    handle: CredHandle,
}

impl Credentials {
    pub fn new_client() -> Result<Self, Error> {
        let mut cred = CredHandle { dw_lower: 0, dw_upper: 0 };
        let mut expiry = TimeStamp { _low_part: 0, _high_part: 0 };

        let sch = SchannelCred {
            dw_version: SCHANNEL_CRED_VERSION,
            c_creds: 0,
            pa_cred: ptr::null_mut(),
            h_root_store: ptr::null_mut(),
            c_mappers: 0,
            aph_mappers: ptr::null_mut(),
            c_supported_algs: 0,
            palg_supported_algs: ptr::null_mut(),
            grbit_enabled_protocols: 0,
            dw_minimum_cipher_strength: 0,
            dw_maximum_cipher_strength: 0,
            dw_session_lifespan: 0,
            dw_flags: SCH_CRED_MANUAL_CRED_VALIDATION | SCH_CRED_NO_SYSTEM_MAPPER,
            dw_cred_format: 0,
        };

        let status = unsafe {
            AcquireCredentialsHandleW(
                ptr::null_mut(),
                unisp_name().as_ptr(),
                SECPKG_CRED_OUTBOUND,
                ptr::null_mut(),
                &sch as *const _ as LPVOID,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut cred,
                &mut expiry,
            )
        };

        if status != SEC_E_OK {
            return Err(Error::Sspi(status));
        }
        Ok(Credentials { handle: cred })
    }
}

unsafe impl Send for Credentials {}

impl Drop for Credentials {
    fn drop(&mut self) {
        unsafe { FreeCredentialsHandle(&mut self.handle); }
    }
}

// ── TlsConnection (encrypt/decrypt engine, no I/O) ──────────

pub struct TlsConnection {
    context: CtxtHandle,
    stream_sizes: SecPkgContextStreamSizes,
}

unsafe impl Send for TlsConnection {}

impl Drop for TlsConnection {
    fn drop(&mut self) {
        unsafe { DeleteSecurityContext(&mut self.context); }
    }
}

impl TlsConnection {
    /// Encrypt plaintext data into TLS records.
    /// Returns the raw encrypted bytes (header + encrypted + trailer).
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let hdr_len = self.stream_sizes.cb_header as usize;
        let trl_len = self.stream_sizes.cb_trailer as usize;
        let max_msg = self.stream_sizes.cb_maximum_message as usize;

        let mut offset = 0;
        let mut output = Vec::new();

        while offset < data.len() {
            let end = std::cmp::min(offset + max_msg, data.len());
            let chunk = &data[offset..end];
            let data_len = chunk.len();

            let mut hdr = vec![0u8; hdr_len];
            let mut enc = chunk.to_vec();
            let mut trl = vec![0u8; trl_len];

            let mut bufs = [
                SecBuffer { cb_buffer: hdr_len as ULONG, buffer_type: SECBUFFER_STREAM_HEADER, pv_buffer: hdr.as_mut_ptr() as LPVOID },
                SecBuffer { cb_buffer: data_len as ULONG, buffer_type: SECBUFFER_DATA, pv_buffer: enc.as_mut_ptr() as LPVOID },
                SecBuffer { cb_buffer: trl_len as ULONG, buffer_type: SECBUFFER_STREAM_TRAILER, pv_buffer: trl.as_mut_ptr() as LPVOID },
            ];
            let mut desc = SecBufferDesc {
                ul_version: SECBUFFER_VERSION,
                c_buffers: 3,
                p_buffers: bufs.as_mut_ptr(),
            };

            let ptr = &self.context as *const CtxtHandle as *mut CtxtHandle;
            let status = unsafe { EncryptMessage(ptr, 0, &mut desc, 0) };
            if status != SEC_E_OK {
                return Err(Error::Sspi(status));
            }

            output.extend_from_slice(&hdr);
            output.extend_from_slice(&enc);
            output.extend_from_slice(&trl);

            offset = end;
        }

        Ok(output)
    }

    /// Decrypt a complete TLS record.
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut data_buf = data.to_vec();
        let mut bufs = [
            SecBuffer { cb_buffer: 0, buffer_type: SECBUFFER_EMPTY, pv_buffer: ptr::null_mut() },
            SecBuffer { cb_buffer: data_buf.len() as ULONG, buffer_type: SECBUFFER_DATA, pv_buffer: data_buf.as_mut_ptr() as LPVOID },
            SecBuffer { cb_buffer: 0, buffer_type: SECBUFFER_EMPTY, pv_buffer: ptr::null_mut() },
        ];
        let mut desc = SecBufferDesc {
            ul_version: SECBUFFER_VERSION,
            c_buffers: 3,
            p_buffers: bufs.as_mut_ptr(),
        };

        let mut extra_index: DWORD = 0;
        let ptr = &self.context as *const CtxtHandle as *mut CtxtHandle;
        let status = unsafe { DecryptMessage(ptr, &mut desc, 0, &mut extra_index) };

        match status {
            SEC_E_OK => {
                let mut plaintext = Vec::new();
                for b in &bufs {
                    if b.buffer_type == SECBUFFER_DATA && b.cb_buffer > 0 && !b.pv_buffer.is_null() {
                        let slice = unsafe {
                            std::slice::from_raw_parts(b.pv_buffer as *const u8, b.cb_buffer as usize)
                        };
                        plaintext.extend_from_slice(slice);
                    }
                }
                Ok(plaintext)
            }
            SEC_E_INCOMPLETE_MESSAGE => Err(Error::Protocol("incomplete message".into())),
            SEC_I_RENEGOTIATE => Err(Error::Renegotiation),
            _ => Err(Error::Sspi(status)),
        }
    }
}

// ── Client handshake (returns TlsConnection + ALPN) ─────────

/// Perform a blocking TLS client handshake over an existing TcpStream.
/// Returns the TlsConnection (encrypt/decrypt engine) and optionally the
/// negotiated ALPN protocol.
pub fn client_handshake(
    cred: &mut Credentials,
    stream: &mut TcpStream,
    server_name: &str,
) -> Result<(TlsConnection, Option<Vec<u8>>), Error> {
    let hostname_wide = to_wide(server_name);
    let mut context = CtxtHandle { dw_lower: 0, dw_upper: 0 };
    let mut attrs: DWORD = 0;
    let mut expiry = TimeStamp { _low_part: 0, _high_part: 0 };
    let mut first = true;
    let mut server_data = Vec::<u8>::new();

    loop {
        if !first && server_data.is_empty() {
            let mut tmp = [0u8; 8192];
            let n = stream.read(&mut tmp)?;
            if n == 0 {
                return Err(Error::Protocol("connection closed during handshake".into()));
            }
            server_data.extend_from_slice(&tmp[..n]);
        }

        let mut in_bufs = [
            SecBuffer { cb_buffer: 0, buffer_type: SECBUFFER_EMPTY, pv_buffer: ptr::null_mut() },
            SecBuffer { cb_buffer: 0, buffer_type: SECBUFFER_EMPTY, pv_buffer: ptr::null_mut() },
        ];
        let mut in_desc = SecBufferDesc {
            ul_version: SECBUFFER_VERSION,
            c_buffers: 2,
            p_buffers: in_bufs.as_mut_ptr(),
        };

        if !first && !server_data.is_empty() {
            in_bufs[0] = SecBuffer {
                cb_buffer: server_data.len() as ULONG,
                buffer_type: SECBUFFER_TOKEN,
                pv_buffer: server_data.as_ptr() as LPVOID,
            };
        }

        let mut out_buf = SecBuffer {
            cb_buffer: 0,
            buffer_type: SECBUFFER_TOKEN,
            pv_buffer: ptr::null_mut(),
        };
        let mut out_desc = SecBufferDesc {
            ul_version: SECBUFFER_VERSION,
            c_buffers: 1,
            p_buffers: &mut out_buf,
        };

        let status = unsafe {
            InitializeSecurityContextW(
                &mut cred.handle as *mut CredHandle,
                if first { ptr::null_mut() } else { &mut context },
                hostname_wide.as_ptr() as LPWSTR,
                ISC_REQ_ALLOCATE_MEMORY
                    | ISC_REQ_CONFIDENTIALITY
                    | ISC_REQ_EXTENDED_ERROR
                    | ISC_REQ_MANUAL_CRED_VALIDATION
                    | ISC_REQ_SEQUENCE_DETECT
                    | ISC_REQ_REPLAY_DETECT
                    | ISC_REQ_INTEGRITY
                    | ISC_REQ_STREAM,
                0,
                0,
                if first { ptr::null_mut() } else { &mut in_desc },
                0,
                &mut context,
                &mut out_desc,
                &mut attrs,
                &mut expiry,
            )
        };

        first = false;

        if out_buf.cb_buffer > 0 && !out_buf.pv_buffer.is_null() {
            let token = unsafe {
                std::slice::from_raw_parts(out_buf.pv_buffer as *const u8, out_buf.cb_buffer as usize)
            };
            stream.write_all(token)?;
        }

        if (attrs & ISC_RET_ALLOCATED_MEMORY) != 0 && !out_buf.pv_buffer.is_null() {
            unsafe { FreeContextBuffer(out_buf.pv_buffer); }
        }

        match status {
            SEC_E_OK => break,
            SEC_I_CONTINUE_NEEDED => {
                server_data.clear();
                continue;
            }
            SEC_E_INCOMPLETE_MESSAGE | SEC_E_INVALID_TOKEN => continue,
            _ => return Err(Error::Sspi(status)),
        }
    }

    let mut sizes = SecPkgContextStreamSizes {
        cb_header: 0, cb_trailer: 0, cb_maximum_message: 0,
        c_buffers: 0, cb_block_size: 0,
    };
    let sz_status = unsafe {
        QueryContextAttributesW(&mut context, SECPKG_ATTR_SIZES, &mut sizes as *mut _ as LPVOID)
    };
    let stream_sizes = if sz_status == SEC_E_OK { sizes } else {
        SecPkgContextStreamSizes { cb_header: 64, cb_trailer: 64, cb_maximum_message: 16384, c_buffers: 4, cb_block_size: 1 }
    };

    let alpn = None;

    Ok((TlsConnection { context, stream_sizes }, alpn))
}

// ── TlsStream (blocking I/O convenience) ─────────────────────

pub struct TlsStream {
    stream: TcpStream,
    conn: TlsConnection,
    read_buf: Vec<u8>,
    decrypted: VecDeque<u8>,
}

impl TlsStream {
    pub fn connect(stream: TcpStream, server_name: &str) -> Result<Self, Error> {
        let mut cred = Credentials::new_client()?;
        let mut stream = stream;
        let (conn, _alpn) = client_handshake(&mut cred, &mut stream, server_name)?;
        Ok(TlsStream { stream, conn, read_buf: Vec::new(), decrypted: VecDeque::new() })
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if !self.decrypted.is_empty() {
            let n = std::cmp::min(buf.len(), self.decrypted.len());
            for i in 0..n { buf[i] = self.decrypted.pop_front().unwrap(); }
            return Ok(n);
        }

        loop {
            if self.read_buf.len() >= 5 {
                let rec_len = u16::from_be_bytes([self.read_buf[3], self.read_buf[4]]) as usize;
                let total = 5 + rec_len;
                if self.read_buf.len() >= total {
                    let record: Vec<u8> = self.read_buf.drain(..total).collect();
                    match self.conn.decrypt(&record) {
                        Ok(plaintext) => {
                            self.decrypted.extend(plaintext);
                            let n = std::cmp::min(buf.len(), self.decrypted.len());
                            for i in 0..n { buf[i] = self.decrypted.pop_front().unwrap(); }
                            return Ok(n);
                        }
                        Err(Error::Protocol(_)) => continue,
                        Err(e) => return Err(e),
                    }
                }
            }

            let mut tmp = [0u8; 8192];
            let n = self.stream.read(&mut tmp)?;
            if n == 0 { return Ok(0); }
            self.read_buf.extend_from_slice(&tmp[..n]);
        }
    }

    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> {
        let encrypted = self.conn.encrypt(buf)?;
        self.stream.write_all(&encrypted).map_err(Error::Io)
    }

    /// Non-blocking read: tries to decrypt buffered data only.
    /// Returns WantRead if more data is needed from the socket.
    fn try_read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if !self.decrypted.is_empty() {
            let n = std::cmp::min(buf.len(), self.decrypted.len());
            for i in 0..n { buf[i] = self.decrypted.pop_front().unwrap(); }
            return Ok(n);
        }

        if self.read_buf.len() < 5 {
            return Err(Error::Protocol("incomplete message".into()));
        }

        let rec_len = u16::from_be_bytes([self.read_buf[3], self.read_buf[4]]) as usize;
        let total = 5 + rec_len;

        if self.read_buf.len() < total {
            return Err(Error::Protocol("incomplete message".into()));
        }

        let record: Vec<u8> = self.read_buf.drain(..total).collect();
        let plaintext = self.conn.decrypt(&record)?;
        self.decrypted.extend(plaintext);

        let n = std::cmp::min(buf.len(), self.decrypted.len());
        for i in 0..n { buf[i] = self.decrypted.pop_front().unwrap(); }
        Ok(n)
    }

    pub fn feed_data(&mut self, data: &[u8]) {
        self.read_buf.extend_from_slice(data);
    }
}

unsafe impl Send for TlsStream {}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_all(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

pub fn is_schannel_available() -> bool { true }
pub fn version() -> &'static str { "SChannel (Windows SSPI)" }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cred_new_client() {
        let cred = Credentials::new_client();
        assert!(cred.is_ok(), "client credentials: {:?}", cred.err());
    }
}
