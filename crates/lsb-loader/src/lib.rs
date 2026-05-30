//! Generic runtime dynamic loader for system libraries with trust checks.
use libloading::Library;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::os::raw::c_void;
use thiserror::Error;
use libc;
use std::fs;
use std::os::unix::fs::MetadataExt;

#[derive(Error, Debug)]
pub enum LoaderError {
    #[error("dlopen failed: {0}")]
    Dlopen(String),
    #[error("missing symbol: {0}")]
    MissingSymbol(String),
    #[error("trust check failed for {0}")]
    TrustCheckFailed(String),
    #[error("other: {0}")]
    Other(String),
}

pub struct LoadedLibrary {
    lib: Library,
    path: PathBuf,
}

impl LoadedLibrary {
    /// Attempt to load one of the candidate sonames and resolve required symbols.
    /// `required` is a list of symbol names that must be present for the candidate to be accepted.
    pub fn load_from_candidates(
        candidates: &[&str],
        required: &[&str],
    ) -> Result<Self, LoaderError> {
        // Try candidates in order
        for &soname in candidates {
            match unsafe { Library::new(soname) } {
                Ok(lib) => {
                    // try to resolve required symbols
                    let mut missing = Vec::new();
                    for &sym in required {
                        unsafe {
                            let c = CString::new(sym).map_err(|e| LoaderError::Other(format!("bad symbol name: {}", e)))?;
                            let res = lib.get::<*const c_void>(c.as_bytes_with_nul());
                            if res.is_err() {
                                missing.push(sym);
                            }
                        }
                    }
                    if !missing.is_empty() {
                        // drop lib and continue
                        drop(lib);
                        continue;
                    }

                    // Determine resolved path via dladdr on a known symbol
                    let path = match Self::resolved_path_via_dladdr(&lib, required[0]) {
                        Ok(p) => p,
                        Err(_) => PathBuf::from(soname),
                    };

                    // Trust checks (basic): path must be under /lib or /usr/lib or /usr/local/lib
                    if !Self::is_trusted_path(&path) {
                        drop(lib);
                        continue;
                    }

                    return Ok(LoadedLibrary { lib, path });
                }
                Err(e) => {
                    // try next
                    let _ = e;
                    continue;
                }
            }
        }

        Err(LoaderError::Dlopen("no suitable candidate found".into()))
    }

    fn resolved_path_via_dladdr(lib: &Library, sym: &str) -> Result<PathBuf, LoaderError> {
        unsafe {
            let c = CString::new(sym).map_err(|e| LoaderError::Other(format!("bad symbol name: {}", e)))?;
            let s = lib.get::<*const c_void>(c.as_bytes_with_nul()).map_err(|e| LoaderError::Other(format!("symbol lookup failed: {}", e)))?;
            let ptr = *s;
            // dladdr using libc's Dl_info
            let mut info: libc::Dl_info = std::mem::zeroed();
            let rv = libc::dladdr(ptr as *const c_void, &mut info as *mut libc::Dl_info);
            if rv == 0 {
                return Err(LoaderError::Other("dladdr failed".into()));
            }
            if info.dli_fname.is_null() {
                return Err(LoaderError::Other("dladdr returned null fname".into()));
            }
            let cstr = CStr::from_ptr(info.dli_fname);
            let s = cstr.to_string_lossy().into_owned();
            Ok(PathBuf::from(s))
        }
    }

    fn is_trusted_path(p: &Path) -> bool {
        if let Ok(abs) = p.canonicalize() {
            let s = abs.to_string_lossy();
            if s.starts_with("/lib") || s.starts_with("/usr/lib") || s.starts_with("/usr/local/lib") {
                // ensure file is regular and not world-writable
                if let Ok(meta) = fs::metadata(&abs) {
                    let mode = meta.mode();
                    // skip if world-writable
                    if mode & 0o002 != 0 {
                        return false;
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load from an explicit path (env-var override). Performs trust checks but allows
    /// non-whitelisted directories since the user explicitly requested this path.
    pub fn load_explicit(path: &str, required: &[&str]) -> Result<Self, LoaderError> {
        let p = Path::new(path);
        if !p.is_absolute() {
            return Err(LoaderError::Other("explicit path must be absolute".into()));
        }
        // Basic file sanity: must be a regular, non-world-writable file
        if let Ok(meta) = fs::metadata(p) {
            if !meta.is_file() {
                return Err(LoaderError::Other(format!("not a regular file: {}", path)));
            }
            if meta.mode() & 0o002 != 0 {
                return Err(LoaderError::Other(format!("world-writable: {}", path)));
            }
        } else {
            return Err(LoaderError::Other(format!("cannot stat: {}", path)));
        }
        match unsafe { Library::new(path) } {
            Ok(lib) => {
                let mut missing = Vec::new();
                for &sym in required {
                    unsafe {
                        let c = CString::new(sym)
                            .map_err(|e| LoaderError::Other(format!("bad symbol name: {}", e)))?;
                        let res = lib.get::<*const c_void>(c.as_bytes_with_nul());
                        if res.is_err() {
                            missing.push(sym);
                        }
                    }
                }
                if !missing.is_empty() {
                    drop(lib);
                    return Err(LoaderError::MissingSymbol(format!(
                        "required symbols missing in {}: {:?}",
                        path, missing
                    )));
                }
                let resolved = Self::resolved_path_via_dladdr(&lib, required[0])
                    .unwrap_or_else(|_| PathBuf::from(path));
                Ok(LoadedLibrary { lib, path: resolved })
            }
            Err(e) => Err(LoaderError::Dlopen(format!("{}: {}", path, e))),
        }
    }

    /// Load from an explicit path with RTLD_GLOBAL so symbols are visible to transitive deps.
    #[cfg(target_os = "linux")]
    pub fn load_explicit_global(path: &str, required: &[&str]) -> Result<Self, LoaderError> {
        use libloading::os::unix::Library as UnixLibrary;
        let p = Path::new(path);
        if !p.is_absolute() {
            return Err(LoaderError::Other("explicit path must be absolute".into()));
        }
        if let Ok(meta) = fs::metadata(p) {
            if !meta.is_file() {
                return Err(LoaderError::Other(format!("not a regular file: {}", path)));
            }
            if meta.mode() & 0o002 != 0 {
                return Err(LoaderError::Other(format!("world-writable: {}", path)));
            }
        } else {
            return Err(LoaderError::Other(format!("cannot stat: {}", path)));
        }
        match unsafe { UnixLibrary::open(Some(path), libc::RTLD_NOW | libc::RTLD_GLOBAL) } {
            Ok(unix_lib) => {
                let mut missing = Vec::new();
                for &sym in required {
                    unsafe {
                        let c = CString::new(sym)
                            .map_err(|e| LoaderError::Other(format!("bad symbol name: {}", e)))?;
                        let res = unix_lib.get::<*const c_void>(c.as_bytes_with_nul());
                        if res.is_err() {
                            missing.push(sym);
                        }
                    }
                }
                if !missing.is_empty() {
                    drop(unix_lib);
                    return Err(LoaderError::MissingSymbol(format!(
                        "required symbols missing in {}: {:?}",
                        path, missing
                    )));
                }
                let lib: Library = unix_lib.into();
                let resolved = Self::resolved_path_via_dladdr(&lib, required[0])
                    .unwrap_or_else(|_| PathBuf::from(path));
                Ok(LoadedLibrary { lib, path: resolved })
            }
            Err(e) => Err(LoaderError::Dlopen(format!("{}: {}", path, e))),
        }
    }

    /// Obtain a raw symbol pointer. Caller must ensure lifetime of library.
    pub unsafe fn get_symbol_raw(&self, name: &str) -> Result<*const c_void, LoaderError> {
        let c = CString::new(name).map_err(|e| LoaderError::Other(format!("bad symbol name: {}", e)))?;
        let sym = self.lib.get::<*const c_void>(c.as_bytes_with_nul()).map_err(|e| LoaderError::MissingSymbol(format!("{}: {}", name, e)))?;
        Ok(*sym)
    }
}
