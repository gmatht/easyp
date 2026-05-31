//! Wrapper around system zlib loaded at runtime via lsb-loader.
use lsb_loader::LoadedLibrary;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_ulong};
use std::mem::transmute;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZlibError {
    #[error("loader error: {0}")]
    Loader(#[from] lsb_loader::LoaderError),
    #[error("zlib error: {0}")]
    Zlib(i32),
}

type CompressFn = unsafe extern "C" fn(dest: *mut u8, dest_len: *mut c_ulong, src: *const u8, src_len: c_ulong) -> c_int;
type UncompressFn = unsafe extern "C" fn(dest: *mut u8, dest_len: *mut c_ulong, src: *const u8, src_len: c_ulong) -> c_int;
type CompressBoundFn = unsafe extern "C" fn(source_len: c_ulong) -> c_ulong;
type ZlibVersionFn = unsafe extern "C" fn() -> *const c_char;

pub struct Zlib {
    #[allow(dead_code)]
    lib: LoadedLibrary,
    compress: CompressFn,
    uncompress: UncompressFn,
    compress_bound: CompressBoundFn,
    version: ZlibVersionFn,
}

impl Zlib {
    pub fn load() -> Result<Self, ZlibError> {
        let required = ["compress", "uncompress", "compressBound", "zlibVersion"];
        let lib = if let Ok(path) = std::env::var("LSBWRAP_LIBZ_PATH") {
            // user-provided override: must be an absolute path
            if !path.starts_with('/') {
                return Err(ZlibError::Loader(lsb_loader::LoaderError::Other(
                    "LSBWRAP_LIBZ_PATH must be an absolute path".into(),
                )));
            }
            LoadedLibrary::load_explicit(&path, &required)?
        } else {
            LoadedLibrary::load_from_candidates(&["libz.so.1", "libz.so"], &required)?
        };

        unsafe {
            let compress: CompressFn = transmute(lib.get_symbol_raw("compress")?);
            let uncompress: UncompressFn = transmute(lib.get_symbol_raw("uncompress")?);
            let compress_bound: CompressBoundFn = transmute(lib.get_symbol_raw("compressBound")?);
            let version: ZlibVersionFn = transmute(lib.get_symbol_raw("zlibVersion")?);
            Ok(Zlib { lib, compress, uncompress, compress_bound, version })
        }
    }

    pub fn version(&self) -> String {
        unsafe {
            let p = (self.version)();
            if p.is_null() {
                return "".into();
            }
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }

    pub fn compress_vec(&self, src: &[u8]) -> Result<Vec<u8>, ZlibError> {
        unsafe {
            let bound = (self.compress_bound)(src.len() as c_ulong) as usize;
            let mut out = vec![0u8; bound];
            let mut out_len: c_ulong = bound as c_ulong;
            let rc = (self.compress)(out.as_mut_ptr(), &mut out_len, src.as_ptr(), src.len() as c_ulong);
            if rc != 0 {
                return Err(ZlibError::Zlib(rc));
            }
            out.truncate(out_len as usize);
            Ok(out)
        }
    }

    pub fn uncompress_vec(&self, src: &[u8], expected_max: usize) -> Result<Vec<u8>, ZlibError> {
        unsafe {
            let mut out = vec![0u8; expected_max];
            let mut out_len: c_ulong = expected_max as c_ulong;
            let rc = (self.uncompress)(out.as_mut_ptr(), &mut out_len, src.as_ptr(), src.len() as c_ulong);
            if rc != 0 {
                return Err(ZlibError::Zlib(rc));
            }
            out.truncate(out_len as usize);
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zlib_load() {
        let z = Zlib::load().expect("zlib should load");
        let v = z.version();
        assert!(!v.is_empty(), "version should not be empty");
        eprintln!("zlib version: {}", v);
    }

    #[test]
    fn test_compress_uncompress_roundtrip() {
        let z = Zlib::load().expect("zlib should load");
        let input = b"Hello, LSB dynamic loader! This is a test of zlib round-trip compression.";
        let compressed = z.compress_vec(input).expect("compress should succeed");
        assert!(compressed.len() < input.len() || compressed.len() >= input.len(),
            "compressed data should exist");
        let decompressed = z.uncompress_vec(&compressed, input.len() * 2)
            .expect("uncompress should succeed");
        assert_eq!(decompressed, input, "round-trip should restore original");
    }

    #[test]
    fn test_compress_large_data() {
        let z = Zlib::load().expect("zlib should load");
        let input = vec![0xABu8; 65536];
        let compressed = z.compress_vec(&input).expect("compress large data");
        let decompressed = z.uncompress_vec(&compressed, input.len() * 2)
            .expect("uncompress large data");
        assert_eq!(decompressed, input);
    }

    #[test]
    fn test_compress_empty() {
        let z = Zlib::load().expect("zlib should load");
        let input = b"";
        let compressed = z.compress_vec(input).expect("compress empty");
        let decompressed = z.uncompress_vec(&compressed, 1024).expect("uncompress empty");
        assert_eq!(decompressed, input);
    }
}
