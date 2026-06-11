//! Runtime-loaded OpenSSL/LibreSSL wrapper that normalises across versions.
use lsb_loader::{LoadedLibrary, LoaderError};
use std::ffi::{CStr, CString};
use std::mem::transmute;
use std::os::raw::{c_char, c_int, c_ulong, c_void};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SslError {
    #[error("loader error: {0}")]
    Loader(#[from] lsb_loader::LoaderError),
    #[error("openssl error: {0} (error queue: {1})")]
    Ssl(i32, String),
    #[error("other: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SslVariant {
    OpenSSL10,
    OpenSSL11,
    OpenSSL30,
    LibreSSL,
}

type SslLibraryInitFn = unsafe extern "C" fn() -> c_int;
type OpenSslInitFn = unsafe extern "C" fn(opts: u64, settings: *const c_void) -> c_int;
type SslCtxNewFn = unsafe extern "C" fn(method: *const c_void) -> *mut c_void;
type SslNewFn = unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void;
type SslSetFdFn = unsafe extern "C" fn(ssl: *mut c_void, fd: c_int) -> c_int;
type SslConnectFn = unsafe extern "C" fn(ssl: *mut c_void) -> c_int;
type SslAcceptFn = unsafe extern "C" fn(ssl: *mut c_void) -> c_int;
type SslReadFn = unsafe extern "C" fn(ssl: *mut c_void, buf: *mut c_void, num: c_int) -> c_int;
type SslWriteFn = unsafe extern "C" fn(ssl: *mut c_void, buf: *const c_void, num: c_int) -> c_int;
type SslShutdownFn = unsafe extern "C" fn(ssl: *mut c_void) -> c_int;
type SslFreeFn = unsafe extern "C" fn(ssl: *mut c_void);
type SslCtxFreeFn = unsafe extern "C" fn(ctx: *mut c_void);
type SslCtxUseCertFileFn = unsafe extern "C" fn(ctx: *mut c_void, file: *const c_char, typ: c_int) -> c_int;
type SslCtxUseKeyFileFn = unsafe extern "C" fn(ctx: *mut c_void, file: *const c_char, typ: c_int) -> c_int;
type SslCtxSetAlpnProtosFn = unsafe extern "C" fn(ctx: *mut c_void, wire: *const u8, len: u16) -> c_int;
type SslSetAlpnProtosFn = unsafe extern "C" fn(ssl: *mut c_void, wire: *const u8, len: u16) -> c_int;
type SslGet0AlpnSelectedFn = unsafe extern "C" fn(ssl: *mut c_void, data: *mut *const u8, len: *mut u16);
type SslCtxSetAlpnSelectCbFn = unsafe extern "C" fn(ctx: *mut c_void, cb: Option<AlpnSelectCb>, arg: *mut c_void);

pub const SSL_TLSEXT_ERR_OK: i32 = 0;
pub const SSL_TLSEXT_ERR_ALERT_FATAL: i32 = 2;
const TLSEXT_NAMETYPE_HOST_NAME: c_int = 0;

type AlpnSelectCb = unsafe extern "C" fn(
    ssl: *mut c_void,
    out: *mut *const u8,
    outlen: *mut u8,
    in_data: *const u8,
    inlen: u32,
    arg: *mut c_void,
) -> i32;
type SslSetAcceptStateFn = unsafe extern "C" fn(ssl: *mut c_void);
pub type ServernameCallbackFn = unsafe extern "C" fn(
    ssl: *mut c_void,
    al: *mut c_int,
    arg: *mut c_void,
) -> c_int;
type SslGetServernameFn = unsafe extern "C" fn(ssl: *mut c_void, typ: c_int) -> *const c_char;
type SslUseCertificateFn = unsafe extern "C" fn(ssl: *mut c_void, x509: *mut c_void) -> c_int;
type SslUsePrivateKeyFn = unsafe extern "C" fn(ssl: *mut c_void, pkey: *mut c_void) -> c_int;
type SslCtxSetServernameCallbackFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    cb: Option<ServernameCallbackFn>,
    arg: *mut c_void,
) -> c_int;
type SslCtxSetCertCallbackFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    cb: Option<CertCallbackFn>,
    arg: *mut c_void,
);
pub type CertCallbackFn = unsafe extern "C" fn(
    ssl: *mut c_void,
    arg: *mut c_void,
) -> c_int;
type SslCheckPrivateKeyFn = unsafe extern "C" fn(ssl: *mut c_void) -> c_int;
type SslGetErrorFn = unsafe extern "C" fn(ssl: *mut c_void, ret: c_int) -> c_int;
type SslSetTlsextHostNameFn = unsafe extern "C" fn(ssl: *mut c_void, name: *const c_char) -> c_int;
type SslSetConnectStateFn = unsafe extern "C" fn(ssl: *mut c_void);
type ErrGetErrorFn = unsafe extern "C" fn() -> c_ulong;
type ErrErrorStringFn = unsafe extern "C" fn(e: c_ulong, buf: *mut c_char, len: c_int) -> *mut c_char;
type Sslv23MethodFn = unsafe extern "C" fn() -> *const c_void;
type TlsMethodFn = unsafe extern "C" fn() -> *const c_void;
type OsslProviderLoadFn = unsafe extern "C" fn(libctx: *mut c_void, name: *const c_char) -> *mut c_void;
type VersionFn = unsafe extern "C" fn(typ: c_int) -> *const c_char;

pub struct Openssl {
    _libssl: LoadedLibrary,
    #[allow(dead_code)]
    _libcrypto: Option<LoadedLibrary>,
    pub variant: SslVariant,

    open_init_fn: Option<OpenSslInitFn>,
    init_fn: Option<SslLibraryInitFn>,

    ssl_ctx_new: SslCtxNewFn,
    ssl_new: SslNewFn,
    ssl_set_fd: SslSetFdFn,
    ssl_connect: SslConnectFn,
    ssl_accept: SslAcceptFn,
    ssl_read: SslReadFn,
    ssl_write: SslWriteFn,
    ssl_shutdown: SslShutdownFn,
    ssl_free: SslFreeFn,
    ssl_ctx_free: SslCtxFreeFn,
    pub(crate) ssl_ctx_use_cert_file: SslCtxUseCertFileFn,
    pub(crate) ssl_ctx_use_key_file: SslCtxUseKeyFileFn,
    ssl_ctx_set_alpn_protos: Option<SslCtxSetAlpnProtosFn>,
    ssl_set_alpn_protos: Option<SslSetAlpnProtosFn>,
    ssl_get0_alpn_selected: Option<SslGet0AlpnSelectedFn>,
    ssl_ctx_set_alpn_select_cb: Option<SslCtxSetAlpnSelectCbFn>,
    ssl_set_accept_state: Option<SslSetAcceptStateFn>,
    ssl_ctx_set_servername_callback: Option<SslCtxSetServernameCallbackFn>,
    ssl_ctx_set_cert_cb: Option<SslCtxSetCertCallbackFn>,
    ssl_get_servername: Option<SslGetServernameFn>,
    ssl_use_certificate: Option<SslUseCertificateFn>,
    ssl_use_private_key: Option<SslUsePrivateKeyFn>,
    ssl_check_private_key: Option<SslCheckPrivateKeyFn>,
    ssl_set_tlsext_host_name: Option<SslSetTlsextHostNameFn>,
    ssl_set_connect_state: Option<SslSetConnectStateFn>,
    ssl_get_error: SslGetErrorFn,
    err_get_error: Option<ErrGetErrorFn>,
    err_error_string: Option<ErrErrorStringFn>,
    ossl_provider_load: Option<OsslProviderLoadFn>,
}

impl Openssl {
    pub fn load() -> Result<Self, SslError> {
        let ssl_path = std::env::var("LSBWRAP_LIBSSL_PATH").ok();
        let crypto_path = std::env::var("LSBWRAP_LIBCRYPTO_PATH").ok();

        // Load libcrypto first (if explicitly provided) with RTLD_GLOBAL so that
        // libssl's transitive dependency on libcrypto can be resolved at dlopen time.
        let libcrypto = if let Some(ref path) = crypto_path {
            if !path.starts_with('/') {
                return Err(SslError::Other("LSBWRAP_LIBCRYPTO_PATH must be absolute".into()));
            }
            #[cfg(target_os = "linux")]
            {
                LoadedLibrary::load_explicit_global(path, &["ERR_get_error"]).ok()
            }
            #[cfg(not(target_os = "linux"))]
            {
                LoadedLibrary::load_explicit(path, &["ERR_get_error"]).ok()
            }
        } else {
            let crypto_candidates = [
                "libcrypto.so.3", "libcrypto.so.1.1", "libcrypto.so.1.0.0",
                "libcrypto.so.10", "libcrypto.so",
            ];
            LoadedLibrary::load_from_candidates(&crypto_candidates, &["ERR_get_error"]).ok()
        };

        let required = ["SSL_new", "SSL_connect", "SSL_read", "SSL_write", "SSL_CTX_new"];
        let libssl = if let Some(ref path) = ssl_path {
            if !path.starts_with('/') {
                return Err(SslError::Other("LSBWRAP_LIBSSL_PATH must be absolute".into()));
            }
            LoadedLibrary::load_explicit(path, &required)?
        } else {
            let ssl_candidates = [
                "libssl.so.3", "libssl.so.1.1", "libssl.so.1.0.0",
                "libssl.so.10", "libssl.so",
            ];
            LoadedLibrary::load_from_candidates(&ssl_candidates, &required)?
        };

        let variant = Self::detect_variant(&libssl)?;

        unsafe {
            let ssl_ctx_new: SslCtxNewFn = transmute(libssl.get_symbol_raw("SSL_CTX_new")?);
            let ssl_new: SslNewFn = transmute(libssl.get_symbol_raw("SSL_new")?);
            let ssl_set_fd: SslSetFdFn = transmute(libssl.get_symbol_raw("SSL_set_fd")?);
            let ssl_connect: SslConnectFn = transmute(libssl.get_symbol_raw("SSL_connect")?);
            let ssl_accept: SslAcceptFn = transmute(libssl.get_symbol_raw("SSL_accept")?);
            let ssl_read: SslReadFn = transmute(libssl.get_symbol_raw("SSL_read")?);
            let ssl_write: SslWriteFn = transmute(libssl.get_symbol_raw("SSL_write")?);
            let ssl_shutdown: SslShutdownFn = transmute(libssl.get_symbol_raw("SSL_shutdown")?);
            let ssl_free: SslFreeFn = transmute(libssl.get_symbol_raw("SSL_free")?);
            let ssl_ctx_free: SslCtxFreeFn = transmute(libssl.get_symbol_raw("SSL_CTX_free")?);

            let ssl_ctx_use_cert_file: SslCtxUseCertFileFn =
                match libssl.get_symbol_raw("SSL_CTX_use_certificate_file") {
                    Ok(p) => transmute(p),
                    Err(_) => return Err(SslError::Other("SSL_CTX_use_certificate_file not found".into())),
                };
            let ssl_ctx_use_key_file: SslCtxUseKeyFileFn =
                match libssl.get_symbol_raw("SSL_CTX_use_PrivateKey_file") {
                    Ok(p) => transmute(p),
                    Err(_) => return Err(SslError::Other("SSL_CTX_use_PrivateKey_file not found".into())),
                };
            let ssl_get_error: SslGetErrorFn = transmute(libssl.get_symbol_raw("SSL_get_error")?);
            let ssl_ctx_set_alpn_protos: Option<SslCtxSetAlpnProtosFn> =
                libssl.get_symbol_raw("SSL_CTX_set_alpn_protos").ok().map(|p| transmute(p));
            let ssl_set_alpn_protos: Option<SslSetAlpnProtosFn> =
                libssl.get_symbol_raw("SSL_set_alpn_protos").ok().map(|p| transmute(p));
            let ssl_get0_alpn_selected: Option<SslGet0AlpnSelectedFn> =
                libssl.get_symbol_raw("SSL_get0_alpn_selected").ok().map(|p| transmute(p));
            let ssl_ctx_set_alpn_select_cb: Option<SslCtxSetAlpnSelectCbFn> =
                libssl.get_symbol_raw("SSL_CTX_set_alpn_select_cb").ok().map(|p| transmute(p));
            let ssl_set_accept_state: Option<SslSetAcceptStateFn> =
                libssl.get_symbol_raw("SSL_set_accept_state").ok().map(|p| transmute(p));
            let ssl_ctx_set_servername_callback: Option<SslCtxSetServernameCallbackFn> =
                libssl.get_symbol_raw("SSL_CTX_set_tlsext_servername_callback").ok().map(|p| transmute(p));
            let ssl_ctx_set_cert_cb: Option<SslCtxSetCertCallbackFn> =
                libssl.get_symbol_raw("SSL_CTX_set_cert_cb").ok().map(|p| transmute(p));
            let ssl_get_servername: Option<SslGetServernameFn> =
                libssl.get_symbol_raw("SSL_get_servername").ok().map(|p| transmute(p));
            let ssl_use_certificate: Option<SslUseCertificateFn> =
                libssl.get_symbol_raw("SSL_use_certificate").ok().map(|p| transmute(p));
            let ssl_use_private_key: Option<SslUsePrivateKeyFn> =
                libssl.get_symbol_raw("SSL_use_PrivateKey").ok().map(|p| transmute(p));
            let ssl_check_private_key: Option<SslCheckPrivateKeyFn> =
                libssl.get_symbol_raw("SSL_check_private_key").ok().map(|p| transmute(p));
            let ssl_set_tlsext_host_name: Option<SslSetTlsextHostNameFn> =
                libssl.get_symbol_raw("SSL_set_tlsext_host_name").ok().map(|p| transmute(p));
            let ssl_set_connect_state: Option<SslSetConnectStateFn> =
                libssl.get_symbol_raw("SSL_set_connect_state").ok().map(|p| transmute(p));

            let err_get_error: Option<ErrGetErrorFn> = libcrypto
                .as_ref()
                .and_then(|c| c.get_symbol_raw("ERR_get_error").ok())
                .map(|p| transmute(p));
            let err_error_string: Option<ErrErrorStringFn> = libcrypto
                .as_ref()
                .and_then(|c| c.get_symbol_raw("ERR_error_string").ok())
                .map(|p| transmute(p));
            let ossl_provider_load: Option<OsslProviderLoadFn> = libcrypto
                .as_ref()
                .and_then(|c| c.get_symbol_raw("OSSL_PROVIDER_load").ok())
                .map(|p| transmute(p));

            let (init_fn, open_init_fn) = match variant {
                SslVariant::OpenSSL10 | SslVariant::LibreSSL => {
                    let f: SslLibraryInitFn = transmute(libssl.get_symbol_raw("SSL_library_init")?);
                    (Some(f), None)
                }
                _ => {
                    if let Ok(p) = libssl.get_symbol_raw("OPENSSL_init_ssl") {
                        (None, Some(transmute(p)))
                    } else {
                        let f: SslLibraryInitFn = transmute(libssl.get_symbol_raw("SSL_library_init")?);
                        (Some(f), None)
                    }
                }
            };

            Ok(Openssl {
                _libssl: libssl,
                _libcrypto: libcrypto,
                variant,
                open_init_fn,
                init_fn,
                ssl_ctx_new,
                ssl_new,
                ssl_set_fd,
                ssl_connect,
                ssl_accept,
                ssl_read,
                ssl_write,
                ssl_shutdown,
                ssl_free,
                ssl_ctx_free,
                ssl_ctx_use_cert_file,
                ssl_ctx_use_key_file,
                ssl_ctx_set_alpn_protos,
                ssl_set_alpn_protos,
                ssl_get0_alpn_selected,
                ssl_ctx_set_alpn_select_cb,
                ssl_set_accept_state,
                ssl_ctx_set_servername_callback,
                ssl_ctx_set_cert_cb,
                ssl_get_servername,
                ssl_use_certificate,
                ssl_use_private_key,
                ssl_check_private_key,
                ssl_set_tlsext_host_name,
                ssl_set_connect_state,
                ssl_get_error,
                err_get_error,
                err_error_string,
                ossl_provider_load,
            })
        }
    }

    fn detect_variant(libssl: &LoadedLibrary) -> Result<SslVariant, SslError> {
        unsafe {
            if let Ok(_) = libssl.get_symbol_raw("OSSL_PROVIDER_load") {
                return Ok(SslVariant::OpenSSL30);
            }
            if let Ok(_) = libssl.get_symbol_raw("OPENSSL_init_ssl") {
                return Ok(SslVariant::OpenSSL11);
            }
            if let Ok(p) = libssl.get_symbol_raw("SSLeay_version") {
                let f: VersionFn = transmute(p);
                let ver = CStr::from_ptr(f(0)).to_string_lossy();
                if ver.contains("LibreSSL") || ver.contains("libressl") {
                    return Ok(SslVariant::LibreSSL);
                }
                return Ok(SslVariant::OpenSSL10);
            }
            if let Ok(_) = libssl.get_symbol_raw("SSL_library_init") {
                return Ok(SslVariant::OpenSSL10);
            }
            Err(SslError::Other("cannot detect OpenSSL variant".into()))
        }
    }

    pub fn init(&self) -> Result<(), SslError> {
        unsafe {
            if let Some(open_init) = self.open_init_fn {
                let rc = open_init(0, std::ptr::null());
                if rc != 1 {
                    return Err(SslError::Other("OPENSSL_init_ssl failed".into()));
                }
            } else if let Some(legacy_init) = self.init_fn {
                let rc = legacy_init();
                if rc != 1 {
                    return Err(SslError::Other("SSL_library_init failed".into()));
                }
            }
        }
        Ok(())
    }

    pub fn load_provider(&self, name: &str) -> Result<(), SslError> {
        match self.ossl_provider_load {
            Some(f) => {
                let cname = CString::new(name)
                    .map_err(|_| SslError::Other("bad provider name".into()))?;
                unsafe {
                    let p = f(std::ptr::null_mut(), cname.as_ptr());
                    if p.is_null() {
                        return Err(SslError::Other(format!(
                            "failed to load provider '{}'",
                            name
                        )));
                    }
                }
                Ok(())
            }
            None => Err(SslError::Other("provider API not available".into())),
        }
    }

    pub fn version(&self) -> Option<String> {
        unsafe {
            for sym in &["OpenSSL_version", "SSLeay_version"] {
                if let Ok(p) = self._libssl.get_symbol_raw(sym) {
                    let f: VersionFn = transmute(p);
                    let cstr = CStr::from_ptr(f(0));
                    return Some(cstr.to_string_lossy().into_owned());
                }
            }
            None
        }
    }

    pub fn last_error_string(&self) -> String {
        unsafe {
            if let (Some(get), Some(str_fn)) = (self.err_get_error, self.err_error_string) {
                let e = get();
                if e != 0 {
                    let mut buf = [0i8; 256];
                    str_fn(e, buf.as_mut_ptr(), 256);
                    return CStr::from_ptr(buf.as_ptr())
                        .to_string_lossy()
                        .into_owned();
                }
            }
            "no error".into()
        }
    }

    fn resolve_method(&self, is_client: bool) -> Result<*const c_void, SslError> {
        unsafe {
            let method_name = if is_client { "TLS_client_method" } else { "TLS_server_method" };
            if let Ok(p) = self._libssl.get_symbol_raw(method_name) {
                let f: TlsMethodFn = transmute(p);
                return Ok(f() as *const c_void);
            }
            if let Ok(p) = self._libssl.get_symbol_raw("TLS_method") {
                let f: TlsMethodFn = transmute(p);
                return Ok(f() as *const c_void);
            }
            if is_client {
                if let Ok(p) = self._libssl.get_symbol_raw("SSLv23_client_method") {
                    let f: Sslv23MethodFn = transmute(p);
                    return Ok(f() as *const c_void);
                }
            }
            if let Ok(p) = self._libssl.get_symbol_raw("SSLv23_method") {
                let f: Sslv23MethodFn = transmute(p);
                return Ok(f() as *const c_void);
            }
            Err(SslError::Other("no TLS method found".into()))
        }
    }

    pub fn ctx_new(&self, is_client: bool) -> Result<SslCtx, SslError> {
        let method = self.resolve_method(is_client)?;
        let ctx = unsafe { (self.ssl_ctx_new)(method) };
        if ctx.is_null() {
            return Err(SslError::Other("SSL_CTX_new returned null".into()));
        }
        Ok(SslCtx {
            ctx,
            ssl_ctx_free: self.ssl_ctx_free,
            ssl_ctx_use_cert_file: self.ssl_ctx_use_cert_file,
            ssl_ctx_use_key_file: self.ssl_ctx_use_key_file,
            ssl_ctx_set_alpn_protos: self.ssl_ctx_set_alpn_protos,
            ssl_ctx_set_alpn_select_cb: self.ssl_ctx_set_alpn_select_cb,
            ssl_ctx_set_servername_callback: self.ssl_ctx_set_servername_callback,
            ssl_ctx_set_cert_cb: self.ssl_ctx_set_cert_cb,
        })
    }

    pub fn ctx_set_servername_callback(
        &self,
        ctx: &SslCtx,
        cb: ServernameCallbackFn,
        arg: *mut c_void,
    ) -> Result<(), SslError> {
        match self.ssl_ctx_set_servername_callback {
            Some(f) => {
                unsafe { f(ctx.as_ptr(), Some(cb), arg); }
                Ok(())
            }
            None => Err(SslError::Other("SNI callback not available in this OpenSSL version".into())),
        }
    }

    pub fn ssl_get_servername(&self, ssl: *mut c_void) -> Option<String> {
        self.ssl_get_servername.and_then(|f| {
            unsafe {
                let p = f(ssl, TLSEXT_NAMETYPE_HOST_NAME);
                if p.is_null() { None }
                else { Some(CStr::from_ptr(p).to_string_lossy().into_owned()) }
            }
        })
    }

    pub fn ssl_use_certificate(&self, ssl: *mut c_void, x509: *mut c_void) -> Result<(), SslError> {
        match self.ssl_use_certificate {
            Some(f) => {
                let rc = unsafe { f(ssl, x509) };
                if rc != 1 { Err(SslError::Other("SSL_use_certificate failed".into())) }
                else { Ok(()) }
            }
            None => Err(SslError::Other("SSL_use_certificate not available".into())),
        }
    }

    pub fn ssl_use_private_key(&self, ssl: *mut c_void, pkey: *mut c_void) -> Result<(), SslError> {
        match self.ssl_use_private_key {
            Some(f) => {
                let rc = unsafe { f(ssl, pkey) };
                if rc != 1 { Err(SslError::Other("SSL_use_PrivateKey failed".into())) }
                else { Ok(()) }
            }
            None => Err(SslError::Other("SSL_use_PrivateKey not available".into())),
        }
    }

    pub fn ssl_check_private_key(&self, ssl: *mut c_void) -> Result<(), SslError> {
        match self.ssl_check_private_key {
            Some(f) => {
                let rc = unsafe { f(ssl) };
                if rc != 1 { Err(SslError::Other("SSL_check_private_key failed".into())) }
                else { Ok(()) }
            }
            None => Err(SslError::Other("SSL_check_private_key not available".into())),
        }
    }

    pub fn ssl_new_from_fd(&self, ctx: &SslCtx, fd: std::os::unix::io::RawFd) -> Result<SslConn, SslError> {
        unsafe {
            let ssl = (self.ssl_new)(ctx.as_ptr());
            if ssl.is_null() {
                return Err(SslError::Other("SSL_new returned null".into()));
            }
            let rc = (self.ssl_set_fd)(ssl, fd);
            if rc == 0 {
                let e = self.last_error_string();
                (self.ssl_free)(ssl);
                return Err(SslError::Other(format!("SSL_set_fd failed: {}", e)));
            }
            Ok(SslConn {
                ssl,
                ssl_free: self.ssl_free,
                ssl_connect: self.ssl_connect,
                ssl_accept: self.ssl_accept,
                ssl_read: self.ssl_read,
                ssl_write: self.ssl_write,
                ssl_shutdown: self.ssl_shutdown,
                ssl_get_error: self.ssl_get_error,
                ssl_set_alpn_protos: self.ssl_set_alpn_protos,
                ssl_get0_alpn_selected: self.ssl_get0_alpn_selected,
                ssl_set_accept_state: self.ssl_set_accept_state,
                ssl_set_tlsext_host_name: self.ssl_set_tlsext_host_name,
                ssl_set_connect_state: self.ssl_set_connect_state,
                err_get_error: self.err_get_error,
                err_error_string: self.err_error_string,
            })
        }
    }
}

unsafe impl Send for Openssl {}

pub struct SslCtx {
    ctx: *mut c_void,
    ssl_ctx_free: SslCtxFreeFn,
    ssl_ctx_use_cert_file: SslCtxUseCertFileFn,
    ssl_ctx_use_key_file: SslCtxUseKeyFileFn,
    ssl_ctx_set_alpn_protos: Option<SslCtxSetAlpnProtosFn>,
    ssl_ctx_set_alpn_select_cb: Option<SslCtxSetAlpnSelectCbFn>,
    ssl_ctx_set_servername_callback: Option<SslCtxSetServernameCallbackFn>,
    ssl_ctx_set_cert_cb: Option<SslCtxSetCertCallbackFn>,
}

unsafe impl Send for SslCtx {}

impl SslCtx {
    pub fn as_ptr(&self) -> *mut c_void {
        self.ctx
    }

    pub fn load_cert_file(&self, path: &str) -> Result<(), SslError> {
        let cpath = CString::new(path).map_err(|_| SslError::Other("bad path".into()))?;
        unsafe {
            let rc = (self.ssl_ctx_use_cert_file)(self.ctx, cpath.as_ptr(), 1);
            if rc != 1 {
                return Err(SslError::Other("failed to load cert".into()));
            }
        }
        Ok(())
    }

    pub fn load_key_file(&self, path: &str) -> Result<(), SslError> {
        let cpath = CString::new(path).map_err(|_| SslError::Other("bad path".into()))?;
        unsafe {
            let rc = (self.ssl_ctx_use_key_file)(self.ctx, cpath.as_ptr(), 1);
            if rc != 1 {
                return Err(SslError::Other("failed to load key".into()));
            }
        }
        Ok(())
    }

    pub fn set_alpn_protocols(&self, protocols: &[&[u8]]) -> Result<(), SslError> {
        match self.ssl_ctx_set_alpn_protos {
            Some(f) => {
                // Build wire format: each protocol is length-byte prefixed
                let mut wire = Vec::new();
                for p in protocols {
                    wire.push(p.len() as u8);
                    wire.extend_from_slice(p);
                }
                unsafe {
                    let rc = f(self.ctx, wire.as_ptr(), wire.len() as u16);
                    if rc != 0 {
                        return Err(SslError::Other("SSL_CTX_set_alpn_protos failed".into()));
                    }
                }
                Ok(())
            }
            None => Err(SslError::Other("ALPN not available in this OpenSSL version".into())),
        }
    }

    /// Register ALPN protocols for the server via select callback.
    /// `wire` must have layout: [total_len: u32] [wire_format_protocols...]
    pub fn set_alpn_select_callback(&self, wire: &'static [u8]) -> Result<(), SslError> {
        match self.ssl_ctx_set_alpn_select_cb {
            Some(f) => {
                unsafe {
                    f(self.ctx, Some(alpn_select_cb_static), wire.as_ptr() as *mut c_void);
                }
                Ok(())
            }
            None => Err(SslError::Other("ALPN select callback not available".into())),
        }
    }
}

impl SslCtx {
    /// Register a per-connection SNI callback.
    /// The callback fires during `SSL_accept` with the SSL pointer and user arg.
    pub fn set_servername_callback(
        &self,
        cb: ServernameCallbackFn,
        arg: *mut c_void,
    ) -> Result<(), SslError> {
        match self.ssl_ctx_set_servername_callback {
            Some(f) => {
                unsafe { f(self.ctx, Some(cb), arg); }
                Ok(())
            }
            None => Err(SslError::Other("SNI callback not available".into())),
        }
    }

    /// Fallback: use SSL_CTX_set_cert_cb (OpenSSL 3.x).
    /// The caller must provide a `CertCallbackFn` that receives `arg` directly.
    pub fn set_cert_cb(
        &self,
        cb: CertCallbackFn,
        arg: *mut c_void,
    ) -> Result<(), SslError> {
        match self.ssl_ctx_set_cert_cb {
            Some(f) => {
                unsafe { f(self.ctx, Some(cb), arg); }
                Ok(())
            }
            None => Err(SslError::Other("SNI callback not available".into())),
        }
    }
}

impl Drop for SslCtx {
    fn drop(&mut self) {
        unsafe { (self.ssl_ctx_free)(self.ctx); }
    }
}

pub struct SslConn {
    ssl: *mut c_void,
    ssl_free: SslFreeFn,
    ssl_connect: SslConnectFn,
    ssl_accept: SslAcceptFn,
    ssl_read: SslReadFn,
    ssl_write: SslWriteFn,
    ssl_shutdown: SslShutdownFn,
    ssl_get_error: SslGetErrorFn,
    ssl_set_alpn_protos: Option<SslSetAlpnProtosFn>,
    ssl_get0_alpn_selected: Option<SslGet0AlpnSelectedFn>,
    ssl_set_accept_state: Option<SslSetAcceptStateFn>,
    ssl_set_tlsext_host_name: Option<SslSetTlsextHostNameFn>,
    ssl_set_connect_state: Option<SslSetConnectStateFn>,
    err_get_error: Option<ErrGetErrorFn>,
    err_error_string: Option<ErrErrorStringFn>,
}

unsafe impl Send for SslConn {}

impl SslConn {
    pub fn as_ptr(&self) -> *mut c_void {
        self.ssl
    }

    pub fn connect(&self) -> Result<(), SslError> {
        unsafe {
            let rc = (self.ssl_connect)(self.ssl);
            if rc != 1 {
                let err = (self.ssl_get_error)(self.ssl, rc);
                return Err(SslError::Ssl(err, self.get_error_string()));
            }
        }
        Ok(())
    }

    pub fn accept(&self) -> Result<(), SslError> {
        unsafe {
            let rc = (self.ssl_accept)(self.ssl);
            if rc != 1 {
                let err = (self.ssl_get_error)(self.ssl, rc);
                return Err(SslError::Ssl(err, self.get_error_string()));
            }
        }
        Ok(())
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, SslError> {
        unsafe {
            let rc = (self.ssl_read)(self.ssl, buf.as_mut_ptr() as *mut c_void, buf.len() as c_int);
            if rc <= 0 {
                let err = (self.ssl_get_error)(self.ssl, rc);
                return Err(SslError::Ssl(err, self.get_error_string()));
            }
            Ok(rc as usize)
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, SslError> {
        unsafe {
            let rc = (self.ssl_write)(self.ssl, buf.as_ptr() as *const c_void, buf.len() as c_int);
            if rc <= 0 {
                let err = (self.ssl_get_error)(self.ssl, rc);
                return Err(SslError::Ssl(err, self.get_error_string()));
            }
            Ok(rc as usize)
        }
    }

    pub fn shutdown(&self) -> Result<(), SslError> {
        unsafe {
            let rc = (self.ssl_shutdown)(self.ssl);
            if rc < 0 {
                let err = (self.ssl_get_error)(self.ssl, rc);
                return Err(SslError::Ssl(err, self.get_error_string()));
            }
        }
        Ok(())
    }

    pub fn get_alpn_selected(&self) -> Option<Vec<u8>> {
        self.ssl_get0_alpn_selected.and_then(|f| {
            unsafe {
                let mut data: *const u8 = std::ptr::null();
                let mut len: u16 = 0;
                f(self.ssl, &mut data, &mut len);
                if len > 0 && !data.is_null() {
                    Some(std::slice::from_raw_parts(data, len as usize).to_vec())
                } else {
                    None
                }
            }
        })
    }

    pub fn set_alpn_protocols(&self, protocols: &[&[u8]]) -> Result<(), SslError> {
        match self.ssl_set_alpn_protos {
            Some(f) => {
                let mut wire = Vec::new();
                for p in protocols {
                    wire.push(p.len() as u8);
                    wire.extend_from_slice(p);
                }
                unsafe {
                    let rc = f(self.ssl, wire.as_ptr(), wire.len() as u16);
                    if rc != 0 {
                        return Err(SslError::Other("SSL_set_alpn_protos failed".into()));
                    }
                }
                Ok(())
            }
            None => Err(SslError::Other("SSL_set_alpn_protos not available".into())),
        }
    }

    pub fn set_accept_state(&self) -> Result<(), SslError> {
        match self.ssl_set_accept_state {
            Some(f) => { unsafe { f(self.ssl); } Ok(()) }
            None => Err(SslError::Other("SSL_set_accept_state not available".into())),
        }
    }

    /// Set the SNI hostname for client-mode connections.
    pub fn set_hostname(&self, hostname: &str) -> Result<(), SslError> {
        match self.ssl_set_tlsext_host_name {
            Some(f) => {
                let cname = CString::new(hostname)
                    .map_err(|_| SslError::Other("hostname contains null byte".into()))?;
                unsafe {
                    let rc = f(self.ssl, cname.as_ptr());
                    if rc != 1 {
                        return Err(SslError::Other("SSL_set_tlsext_host_name failed".into()));
                    }
                }
                Ok(())
            }
            None => Err(SslError::Other("SSL_set_tlsext_host_name not available".into())),
        }
    }

    /// Set connect state for client-mode connections.
    /// Normally not needed if the SSL object was created from a client-method CTX.
    pub fn set_connect_state(&self) -> Result<(), SslError> {
        match self.ssl_set_connect_state {
            Some(f) => { unsafe { f(self.ssl); } Ok(()) }
            None => Err(SslError::Other("SSL_set_connect_state not available".into())),
        }
    }

    fn get_error_string(&self) -> String {
        unsafe {
            if let (Some(get), Some(str_fn)) = (self.err_get_error, self.err_error_string) {
                let e = get();
                if e != 0 {
                    let mut buf = [0i8; 256];
                    str_fn(e, buf.as_mut_ptr(), 256);
                    return CStr::from_ptr(buf.as_ptr())
                        .to_string_lossy()
                        .into_owned();
                }
            }
            String::new()
        }
    }
}

impl Drop for SslConn {
    fn drop(&mut self) {
        unsafe { (self.ssl_free)(self.ssl); }
    }
}

// ── ALPN server callback ─────────────────────────────────────

// ALPN protocol array stored as a static with the total length prepended.
// Layout: [total_len: u32] [wire_format_protocols...]
// The callback receives arg pointing to the full array; we read total_len first.

unsafe extern "C" fn alpn_select_cb_static(
    _ssl: *mut c_void,
    out: *mut *const u8,
    outlen: *mut u8,
    in_data: *const u8,
    inlen: u32,
    arg: *mut c_void,
) -> i32 {
    // The first 4 bytes at arg are the total length of the wire-format protocol list
    let total_len = *(arg as *const u32);
    let server_bytes = std::slice::from_raw_parts(arg as *const u8, 4 + total_len as usize);
    let server_body = &server_bytes[4..];
    let client_protos = std::slice::from_raw_parts(in_data, inlen as usize);

    let mut so = 0;
    while so < server_body.len() {
        let splen = server_body[so] as usize;
        if so + 1 + splen > server_body.len() { break; }
        let sp = &server_body[so + 1..so + 1 + splen];
        let mut co = 0;
        while co < client_protos.len() {
            let cplen = client_protos[co] as usize;
            if co + 1 + cplen <= client_protos.len() && &client_protos[co + 1..co + 1 + cplen] == sp {
                *out = server_body.as_ptr().add(so + 1);
                *outlen = splen as u8;
                return SSL_TLSEXT_ERR_OK;
            }
            co += 1 + cplen;
        }
        so += 1 + splen;
    }
    SSL_TLSEXT_ERR_OK
}

/// X.509 certificate generation via OpenSSL (replaces rcgen).
pub mod certs;

/// Cross-platform TLS connector and stream (OpenSSL on Unix, SChannel on Windows).
pub mod tls;

// Public helper: load libcrypto independently (used by certs module).
pub fn load_libcrypto() -> Result<LoadedLibrary, LoaderError> {
    let crypto_candidates = [
        "libcrypto.so.3", "libcrypto.so.1.1", "libcrypto.so.1.0.0",
        "libcrypto.so.10", "libcrypto.so",
    ];
    LoadedLibrary::load_from_candidates(&crypto_candidates, &["ERR_get_error"])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ensure_loaded() -> Openssl {
        let ssl = Openssl::load().expect("OpenSSL should load");
        ssl.init().expect("OpenSSL init should succeed");
        ssl
    }

    #[test]
    fn test_load_and_detect_variant() {
        let ssl = Openssl::load().expect("OpenSSL should load");
        eprintln!("detected variant: {:?}", ssl.variant);
        if let Some(v) = ssl.version() {
            eprintln!("version: {}", v);
        }
        // Ensure variant is one of the valid ones
        match ssl.variant {
            SslVariant::OpenSSL30 | SslVariant::OpenSSL11 | SslVariant::OpenSSL10 | SslVariant::LibreSSL => {}
        }
    }

    #[test]
    fn test_init() {
        let ssl = ensure_loaded();
        // init already called in ensure_loaded, just verify no panic
        assert!(ssl.version().is_some() || ssl.version().is_none());
    }

    #[test]
    fn test_ctx_new_client() {
        let ssl = ensure_loaded();
        let ctx = ssl.ctx_new(true).expect("client ctx should create");
        let _ = ctx; // Drop test
    }

    #[test]
    fn test_ctx_new_server() {
        let ssl = ensure_loaded();
        let ctx = ssl.ctx_new(false).expect("server ctx should create");
        let _ = ctx;
    }

    #[test]
    fn test_client_server_ctx_load_cert() {
        let ssl = ensure_loaded();
        let cert_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/resources/cert.pem"
        );
        let key_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/resources/key.pem"
        );
        let ctx = ssl.ctx_new(false).expect("server ctx");
        ctx.load_cert_file(cert_path).expect("should load cert");
        ctx.load_key_file(key_path).expect("should load key");
    }
}
