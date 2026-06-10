//! X.509 certificate generation using OpenSSL FFI (replaces `rcgen`).

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use lsb_loader::LoadedLibrary;
use crate::{SslError, load_libcrypto};

struct X509Ffi {
    #[allow(dead_code)]
    lib: LoadedLibrary,

    x509_new: unsafe extern "C" fn() -> *mut c_void,
    x509_free: unsafe extern "C" fn(*mut c_void),
    x509_set_version: unsafe extern "C" fn(*mut c_void, c_int) -> c_int,
    x509_get_serial_number: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    asn1_integer_set: unsafe extern "C" fn(*mut c_void, i64) -> c_int,
    x509_gmtime_adj: unsafe extern "C" fn(*mut c_void, i64) -> *mut c_void,
    x509_get_m_not_before: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
    x509_get_m_not_after: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
    x509_set_pubkey: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    x509_name_new: unsafe extern "C" fn() -> *mut c_void,
    x509_name_free: unsafe extern "C" fn(*mut c_void),
    x509_name_add_entry_by_txt: unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *const u8, c_int, c_int, c_int) -> c_int,
    x509_set_subject_name: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    x509_set_issuer_name: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    x509_sign: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void) -> c_int,
    x509v3_ext_conf_nid: unsafe extern "C" fn(*mut c_void, *mut c_void, c_int, *const c_char, *const c_char) -> *mut c_void,
    x509_add_ext: unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> c_int,
    x509_extension_free: unsafe extern "C" fn(*mut c_void),
    evp_sha256: unsafe extern "C" fn() -> *const c_void,
    pem_write_bio_x509: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    pem_write_bio_private_key: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, *mut c_void, c_int, *mut c_void, *mut c_void) -> c_int,
    bio_new_file: unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void,
    bio_new: unsafe extern "C" fn(*const c_void) -> *mut c_void,
    #[allow(dead_code)]
    bio_new_mem_buf: unsafe extern "C" fn(*mut c_void, c_int) -> *mut c_void,
    bio_free: unsafe extern "C" fn(*mut c_void),
    bio_s_mem: unsafe extern "C" fn() -> *const c_void,
    bio_read: unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> c_int,
    bio_write: unsafe extern "C" fn(*mut c_void, *const c_void, c_int) -> c_int,
    i2d_x509: unsafe extern "C" fn(*mut c_void, *mut *mut u8) -> c_int,
    #[allow(dead_code)]
    i2d_x509_bio: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    // On x86_64 SysV ABI, the 7-arg form works for all OpenSSL versions
    // (extra register args are safely ignored by older 2-arg implementations)
    i2d_pkcs8_private_key_bio: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut u8, c_int, *mut c_void, *mut c_void) -> c_int>,
    ec_key_new_by_curve_name: unsafe extern "C" fn(c_int) -> *mut c_void,
    ec_key_free: unsafe extern "C" fn(*mut c_void),
    ec_key_generate_key: unsafe extern "C" fn(*mut c_void) -> c_int,
    evp_pkey_new: unsafe extern "C" fn() -> *mut c_void,
    evp_pkey_free: unsafe extern "C" fn(*mut c_void),
    evp_pkey_set1_ec_key: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    // Use libc::free instead of CRYPTO_free (API changed in OpenSSL 3.5)
    // EVP_DigestSign (for ECDSA signing)
    // EVP_DigestSignInit(ctx, pctx, type, e, pkey) — 5 args on x86_64 SysV
    evp_digest_sign_init: unsafe extern "C" fn(*mut *mut c_void, *mut *mut c_void, *const c_void, *mut c_void, *mut c_void) -> c_int,
    evp_digest_sign: unsafe extern "C" fn(*mut c_void, *mut u8, *mut usize, *const u8, usize) -> c_int,
    evp_md_ctx_free: unsafe extern "C" fn(*mut c_void),
    evp_md_ctx_new: unsafe extern "C" fn() -> *mut c_void,
    evp_digest_init_ex: unsafe extern "C" fn(*mut c_void, *const c_void, *mut *mut c_void) -> c_int,
    evp_digest_update: unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int,
    evp_digest_final_ex: unsafe extern "C" fn(*mut c_void, *mut u8, *mut u32) -> c_int,
    // EC functions for key extraction
    evp_pkey_get1_ec_key: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    ec_key_get0_group: unsafe extern "C" fn(*const c_void) -> *const c_void,
    ec_key_get0_public_key: unsafe extern "C" fn(*const c_void) -> *const c_void,
    ec_point_to_oct: unsafe extern "C" fn(*const c_void, *const c_void, c_int, *mut u8, usize, *mut c_void) -> usize,
    ecdsa_do_sign: unsafe extern "C" fn(*const u8, c_int, *mut c_void) -> *mut c_void,
    ecdsa_sig_get0_r: unsafe extern "C" fn(*const c_void) -> *const c_void,
    ecdsa_sig_get0_s: unsafe extern "C" fn(*const c_void) -> *const c_void,
    ecdsa_sig_free: unsafe extern "C" fn(*mut c_void),
    bn_num_bits: unsafe extern "C" fn(*const c_void) -> c_int,
    bn_bn2binpad: unsafe extern "C" fn(*const c_void, *mut u8, c_int) -> c_int,
    // PEM read
    pem_read_bio_x509: unsafe extern "C" fn(*mut c_void, *mut *mut c_void, *mut c_void, *mut c_void) -> *mut c_void,
    pem_read_bio_private_key: unsafe extern "C" fn(*mut c_void, *mut *mut c_void, *mut c_void, *mut c_void) -> *mut c_void,
    // X509_REQ (CSR)
    d2i_private_key_bio: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> *mut c_void,
    x509_req_new: unsafe extern "C" fn() -> *mut c_void,
    x509_req_free: unsafe extern "C" fn(*mut c_void),
    x509_req_set_version: unsafe extern "C" fn(*mut c_void, c_int) -> c_int,
    x509_req_set_subject_name: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    x509_req_add_extensions: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    x509_req_set_pubkey: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    x509_req_sign: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void) -> c_int,
    i2d_x509_req_bio: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    // OpenSSL 3.x renamed internal stack functions; optional for backward compat
    sk_x509_extension_new_null: Option<unsafe extern "C" fn() -> *mut c_void>,
    sk_x509_extension_push: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int>,
    sk_x509_extension_free: Option<unsafe extern "C" fn(*mut c_void)>,
    // Resolve NIDs dynamically (they differ across OpenSSL installations)
    obj_txt2nid: unsafe extern "C" fn(*const c_char) -> c_int,
}

use std::sync::OnceLock;

static X509_FFI: OnceLock<Result<X509Ffi, &'static str>> = OnceLock::new();

impl X509Ffi {
    fn load() -> Result<&'static Self, &'static str> {
        let result: &Result<X509Ffi, &str> = X509_FFI.get_or_init(|| {
            Self::try_load()
        });
        match result {
            Ok(ffi) => Ok(ffi),
            Err(e) => Err(e),
        }
    }

    fn try_load() -> Result<Self, &'static str> {
        unsafe {
            let lib = load_libcrypto().map_err(|_| "failed to load libcrypto")?;
            macro_rules! s { ($n:literal) => {
                lib.get_symbol_raw($n).map_err(|_| concat!("sym: ", $n))?
            }; }

            let p_x509_new = s!("X509_new");
            let p_x509_free = s!("X509_free");
            let p_x509_set_version = s!("X509_set_version");
            let p_x509_get_serial_number = s!("X509_get_serialNumber");
            let p_asn1_integer_set = s!("ASN1_INTEGER_set");
            let p_x509_gmtime_adj = s!("X509_gmtime_adj");
            // X509_getm_notBefore/After (OpenSSL 1.1+) with X509_get_notBefore/After fallback (1.0.x)
            let p_x509_get_m_not_before = lib.get_symbol_raw("X509_getm_notBefore")
                .or_else(|_| lib.get_symbol_raw("X509_get_notBefore")).ok();
            let p_x509_get_m_not_after = lib.get_symbol_raw("X509_getm_notAfter")
                .or_else(|_| lib.get_symbol_raw("X509_get_notAfter")).ok();
            let p_x509_set_pubkey = s!("X509_set_pubkey");
            let p_x509_name_new = s!("X509_NAME_new");
            let p_x509_name_free = s!("X509_NAME_free");
            let p_x509_name_add_entry_by_txt = s!("X509_NAME_add_entry_by_txt");
            let p_x509_set_subject_name = s!("X509_set_subject_name");
            let p_x509_set_issuer_name = s!("X509_set_issuer_name");
            let p_x509_sign = s!("X509_sign");
            let p_x509v3_ext_conf_nid = s!("X509V3_EXT_conf_nid");
            let p_x509_add_ext = s!("X509_add_ext");
            let p_x509_extension_free = s!("X509_EXTENSION_free");
            let p_evp_sha256 = s!("EVP_sha256");
            let p_pem_write_bio_x509 = s!("PEM_write_bio_X509");
            let p_pem_write_bio_private_key = s!("PEM_write_bio_PrivateKey");
            let p_bio_new_file = s!("BIO_new_file");
            let p_bio_new = s!("BIO_new");
            let p_bio_new_mem_buf = s!("BIO_new_mem_buf");
            let p_bio_free = s!("BIO_free");
            let p_bio_s_mem = s!("BIO_s_mem");
            let p_bio_read = s!("BIO_read");
            let p_bio_write = s!("BIO_write");
            let p_i2d_x509 = s!("i2d_X509");
            let p_i2d_x509_bio = s!("i2d_X509_bio");
            // On x86_64 SysV ABI, the 7-arg form works for all OpenSSL versions
            let p_i2d_pkcs8_private_key_bio = lib.get_symbol_raw("i2d_PKCS8PrivateKey_bio").ok()
                .map(|p| unsafe { std::mem::transmute::<_, unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut u8, c_int, *mut c_void, *mut c_void) -> c_int>(p) });
            let p_ec_key_new_by_curve_name = s!("EC_KEY_new_by_curve_name");
            let p_ec_key_free = s!("EC_KEY_free");
            let p_ec_key_generate_key = s!("EC_KEY_generate_key");
            let p_evp_pkey_new = s!("EVP_PKEY_new");
            let p_evp_pkey_free = s!("EVP_PKEY_free");
            let p_evp_pkey_set1_ec_key = s!("EVP_PKEY_set1_EC_KEY");
            let p_evp_digest_sign_init = s!("EVP_DigestSignInit");
            let p_evp_digest_sign = s!("EVP_DigestSign");
            let p_evp_md_ctx_free = s!("EVP_MD_CTX_free");
            let p_evp_md_ctx_new = s!("EVP_MD_CTX_new");
            let p_evp_digest_init_ex = s!("EVP_DigestInit_ex");
            let p_evp_digest_update = s!("EVP_DigestUpdate");
            let p_evp_digest_final_ex = s!("EVP_DigestFinal_ex");
            let p_evp_pkey_get1_ec_key = s!("EVP_PKEY_get1_EC_KEY");
            let p_ec_key_get0_group = s!("EC_KEY_get0_group");
            let p_ec_key_get0_public_key = s!("EC_KEY_get0_public_key");
            let p_ec_point_to_oct = s!("EC_POINT_point2oct");
            let p_ecdsa_do_sign = s!("ECDSA_do_sign");
            let p_ecdsa_sig_get0_r = s!("ECDSA_SIG_get0_r");
            let p_ecdsa_sig_get0_s = s!("ECDSA_SIG_get0_s");
            let p_ecdsa_sig_free = s!("ECDSA_SIG_free");
            let p_bn_num_bits = s!("BN_num_bits");
            let p_bn_bn2binpad = s!("BN_bn2binpad");
            let p_pem_read_bio_x509 = s!("PEM_read_bio_X509");
            let p_pem_read_bio_private_key = s!("PEM_read_bio_PrivateKey");
            let p_d2i_private_key_bio = s!("d2i_PrivateKey_bio");
            let p_x509_req_new = s!("X509_REQ_new");
            let p_x509_req_free = s!("X509_REQ_free");
            let p_x509_req_set_version = s!("X509_REQ_set_version");
            let p_x509_req_set_subject_name = s!("X509_REQ_set_subject_name");
            let p_x509_req_add_extensions = s!("X509_REQ_add_extensions");
            let p_x509_req_set_pubkey = s!("X509_REQ_set_pubkey");
            let p_x509_req_sign = s!("X509_REQ_sign");
            let p_i2d_x509_req_bio = s!("i2d_X509_REQ_bio");
            // Try OpenSSL 1.1 names first, then 3.x internal names
            let p_obj_txt2nid = s!("OBJ_txt2nid");
            let p_sk_x509_extension_new_null = lib.get_symbol_raw("sk_X509_EXTENSION_new_null")
                .or_else(|_| lib.get_symbol_raw("OPENSSL_sk_new_null"));
            let p_sk_x509_extension_push = lib.get_symbol_raw("sk_X509_EXTENSION_push")
                .or_else(|_| lib.get_symbol_raw("OPENSSL_sk_push"));
            let p_sk_x509_extension_free = lib.get_symbol_raw("sk_X509_EXTENSION_free")
                .or_else(|_| lib.get_symbol_raw("OPENSSL_sk_free"));

            Ok(X509Ffi {
                lib,
                x509_new: std::mem::transmute(p_x509_new), x509_free: std::mem::transmute(p_x509_free),
                x509_set_version: std::mem::transmute(p_x509_set_version),
                x509_get_serial_number: std::mem::transmute(p_x509_get_serial_number),
                asn1_integer_set: std::mem::transmute(p_asn1_integer_set),
                x509_gmtime_adj: std::mem::transmute(p_x509_gmtime_adj),
                x509_get_m_not_before: p_x509_get_m_not_before.map(|p| unsafe { std::mem::transmute(p) }),
                x509_get_m_not_after: p_x509_get_m_not_after.map(|p| unsafe { std::mem::transmute(p) }),
                x509_set_pubkey: std::mem::transmute(p_x509_set_pubkey),
                x509_name_new: std::mem::transmute(p_x509_name_new), x509_name_free: std::mem::transmute(p_x509_name_free),
                x509_name_add_entry_by_txt: std::mem::transmute(p_x509_name_add_entry_by_txt),
                x509_set_subject_name: std::mem::transmute(p_x509_set_subject_name),
                x509_set_issuer_name: std::mem::transmute(p_x509_set_issuer_name),
                x509_sign: std::mem::transmute(p_x509_sign),
                x509v3_ext_conf_nid: std::mem::transmute(p_x509v3_ext_conf_nid),
                x509_add_ext: std::mem::transmute(p_x509_add_ext),
                x509_extension_free: std::mem::transmute(p_x509_extension_free),
                evp_sha256: std::mem::transmute(p_evp_sha256),
                pem_write_bio_x509: std::mem::transmute(p_pem_write_bio_x509),
                pem_write_bio_private_key: std::mem::transmute(p_pem_write_bio_private_key),
                bio_new_file: std::mem::transmute(p_bio_new_file),
                bio_new: std::mem::transmute(p_bio_new),
                bio_new_mem_buf: std::mem::transmute(p_bio_new_mem_buf),
                bio_free: std::mem::transmute(p_bio_free),
                bio_s_mem: std::mem::transmute(p_bio_s_mem),
                bio_read: std::mem::transmute(p_bio_read),
                bio_write: std::mem::transmute(p_bio_write),
                i2d_x509: std::mem::transmute(p_i2d_x509),
                i2d_x509_bio: std::mem::transmute(p_i2d_x509_bio),
                i2d_pkcs8_private_key_bio: p_i2d_pkcs8_private_key_bio,
                ec_key_new_by_curve_name: std::mem::transmute(p_ec_key_new_by_curve_name),
                ec_key_free: std::mem::transmute(p_ec_key_free),
                ec_key_generate_key: std::mem::transmute(p_ec_key_generate_key),
                evp_pkey_new: std::mem::transmute(p_evp_pkey_new), evp_pkey_free: std::mem::transmute(p_evp_pkey_free),
                evp_pkey_set1_ec_key: std::mem::transmute(p_evp_pkey_set1_ec_key),
                evp_digest_sign_init: std::mem::transmute(p_evp_digest_sign_init),
                evp_digest_sign: std::mem::transmute(p_evp_digest_sign),
                evp_md_ctx_free: std::mem::transmute(p_evp_md_ctx_free),
                evp_md_ctx_new: std::mem::transmute(p_evp_md_ctx_new),
                evp_digest_init_ex: std::mem::transmute(p_evp_digest_init_ex),
                evp_digest_update: std::mem::transmute(p_evp_digest_update),
                evp_digest_final_ex: std::mem::transmute(p_evp_digest_final_ex),
                evp_pkey_get1_ec_key: std::mem::transmute(p_evp_pkey_get1_ec_key),
                ec_key_get0_group: std::mem::transmute(p_ec_key_get0_group),
                ec_key_get0_public_key: std::mem::transmute(p_ec_key_get0_public_key),
                ec_point_to_oct: std::mem::transmute(p_ec_point_to_oct),
                ecdsa_do_sign: std::mem::transmute(p_ecdsa_do_sign),
                ecdsa_sig_get0_r: std::mem::transmute(p_ecdsa_sig_get0_r),
                ecdsa_sig_get0_s: std::mem::transmute(p_ecdsa_sig_get0_s),
                ecdsa_sig_free: std::mem::transmute(p_ecdsa_sig_free),
                bn_num_bits: std::mem::transmute(p_bn_num_bits),
                bn_bn2binpad: std::mem::transmute(p_bn_bn2binpad),
                pem_read_bio_x509: std::mem::transmute(p_pem_read_bio_x509),
                pem_read_bio_private_key: std::mem::transmute(p_pem_read_bio_private_key),
                d2i_private_key_bio: std::mem::transmute(p_d2i_private_key_bio),
                x509_req_new: std::mem::transmute(p_x509_req_new),
                x509_req_free: std::mem::transmute(p_x509_req_free),
                x509_req_set_version: std::mem::transmute(p_x509_req_set_version),
                x509_req_set_subject_name: std::mem::transmute(p_x509_req_set_subject_name),
                x509_req_add_extensions: std::mem::transmute(p_x509_req_add_extensions),
                x509_req_set_pubkey: std::mem::transmute(p_x509_req_set_pubkey),
                x509_req_sign: std::mem::transmute(p_x509_req_sign),
                i2d_x509_req_bio: std::mem::transmute(p_i2d_x509_req_bio),
                sk_x509_extension_new_null: p_sk_x509_extension_new_null.ok().map(|p| unsafe { std::mem::transmute(p) }),
                sk_x509_extension_push: p_sk_x509_extension_push.ok().map(|p| unsafe { std::mem::transmute(p) }),
                sk_x509_extension_free: p_sk_x509_extension_free.ok().map(|p| unsafe { std::mem::transmute(p) }),
                obj_txt2nid: std::mem::transmute(p_obj_txt2nid),
            })
        }
    }
}

/// Generate an EC P-256 key pair and return PKCS#8 DER bytes.
pub fn create_ec_p256_key() -> Result<Vec<u8>, SslError> {
    create_ec_key("prime256v1")
}

/// Generate an EC P-384 key pair and return PKCS#8 DER bytes.
pub fn create_ec_p384_key() -> Result<Vec<u8>, SslError> {
    create_ec_key("secp384r1")
}

fn create_ec_key(curve_name: &str) -> Result<Vec<u8>, SslError> {
    let ffi = X509Ffi::load().map_err(|e| SslError::Other(e.into()))?;
    unsafe {
        let c_name = CString::new(curve_name).unwrap();
        let nid = (ffi.obj_txt2nid)(c_name.as_ptr());
        if nid == 0 { return Err(SslError::Other("unknown curve".into())); }
        let ec_key = (ffi.ec_key_new_by_curve_name)(nid);
        if ec_key.is_null() { return Err(SslError::Other("EC_KEY_new".into())); }
        (ffi.ec_key_generate_key)(ec_key);
        let pkey = (ffi.evp_pkey_new)();
        (ffi.evp_pkey_set1_ec_key)(pkey, ec_key);
        (ffi.ec_key_free)(ec_key);

        // DER encode as PKCS8
        let bio_s_mem = (ffi.bio_s_mem)();
        let bio = (ffi.bio_new)(bio_s_mem);
        if let Some(pkcs8) = ffi.i2d_pkcs8_private_key_bio {
            pkcs8(bio, pkey, std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut());
        }
        let mut buf = vec![0u8; 4096];
        let n = (ffi.bio_read)(bio, buf.as_mut_ptr() as *mut c_void, 4096);
        (ffi.bio_free)(bio);
        (ffi.evp_pkey_free)(pkey);
        buf.truncate(if n > 0 { n as usize } else { 0 });
        Ok(buf)
    }
}

/// Generate a PKCS#10 CSR for the given private key (PKCS8 DER) and domains.
/// Uses OpenSSL's X509_REQ API.
pub fn generate_csr(pkcs8_der: &[u8], domains: &[&str]) -> Result<Vec<u8>, SslError> {
    let ffi = X509Ffi::load().map_err(|e| SslError::Other(e.into()))?;
    unsafe { gen_csr_inner(&ffi, pkcs8_der, domains) }
}

unsafe fn gen_csr_inner(f: &X509Ffi, pkcs8_der: &[u8], domains: &[&str]) -> Result<Vec<u8>, SslError> {
    let mbstring_asc: c_int = 0x1001;

    // Load private key from PKCS8 DER via memory BIO
    let bio_s_mem = (f.bio_s_mem)();
    let bio = (f.bio_new)(bio_s_mem);
    (f.bio_write)(bio, pkcs8_der.as_ptr() as *const c_void, pkcs8_der.len() as c_int);
    let pkey = (f.d2i_private_key_bio)(bio, std::ptr::null_mut());
    (f.bio_free)(bio);
    if pkey.is_null() { return Err(SslError::Other("d2i_PrivateKey_bio failed".into())); }

    // Create X509_REQ
    let req = (f.x509_req_new)();
    if req.is_null() { (f.evp_pkey_free)(pkey); return Err(SslError::Other("X509_REQ_new".into())); }

    // Version 0 = PKCS#10 v1.0
    (f.x509_req_set_version)(req, 0);

    // Subject name (use first domain as CN)
    let name = (f.x509_name_new)();
    let c_domain = CString::new(domains[0]).unwrap();
    (f.x509_name_add_entry_by_txt)(name, c"CN".as_ptr(), mbstring_asc,
        c_domain.as_ptr() as *const u8, c_domain.as_bytes().len() as i32, -1, 0);
    (f.x509_req_set_subject_name)(req, name);
    (f.x509_name_free)(name);

    // Set public key
    (f.x509_req_set_pubkey)(req, pkey);

    // Add SAN extension (OpenSSL 3.x renamed stack APIs; handle missing gracefully)
    let ext_sk = f.sk_x509_extension_new_null.map(|new_null| {
        let sk = new_null();
        for domain in domains {
            let san_str = CString::new(format!("DNS:{}", domain)).unwrap();
            let ext = (f.x509v3_ext_conf_nid)(std::ptr::null_mut(), std::ptr::null_mut(),
                0x55 /* NID_subject_alt_name */, san_str.as_ptr(), std::ptr::null());
            if !ext.is_null() {
                if let Some(push) = f.sk_x509_extension_push {
                    push(sk, ext);
                }
            }
        }
        sk
    });
    if let Some(sk) = ext_sk {
        (f.x509_req_add_extensions)(req, sk);
        if let Some(free) = f.sk_x509_extension_free {
            free(sk);
        }
    }

    // Sign
    if (f.x509_req_sign)(req, pkey, (f.evp_sha256)()) == 0 {
        (f.x509_req_free)(req); (f.evp_pkey_free)(pkey);
        return Err(SslError::Other("X509_REQ_sign".into()));
    }

    // DER encode
    let bio_out = (f.bio_new)(bio_s_mem);
    (f.i2d_x509_req_bio)(bio_out, req);
    let mut buf = vec![0u8; 4096];
    let n = (f.bio_read)(bio_out, buf.as_mut_ptr() as *mut c_void, 4096);
    (f.bio_free)(bio_out);
    (f.x509_req_free)(req);
    (f.evp_pkey_free)(pkey);
    buf.truncate(if n > 0 { n as usize } else { 0 });

    if buf.is_empty() { return Err(SslError::Other("CSR DER empty".into())); }
    Ok(buf)
}

/// ECDSA sign data with a P-256 key loaded from PKCS8 DER.
/// Returns the raw ECDSA signature (r||s, 64 bytes).
pub fn ecdsa_sign_p256(pkcs8_der: &[u8], data: &[u8]) -> Result<Vec<u8>, SslError> {
    let ffi = X509Ffi::load().map_err(|e| SslError::Other(e.into()))?;
    unsafe {
        let bio_s_mem = (ffi.bio_s_mem)();
        let bio = (ffi.bio_new)(bio_s_mem);
        (ffi.bio_write)(bio, pkcs8_der.as_ptr() as *const c_void, pkcs8_der.len() as c_int);
        let pkey = (ffi.d2i_private_key_bio)(bio, std::ptr::null_mut());
        (ffi.bio_free)(bio);
        if pkey.is_null() { return Err(SslError::Other("d2i failed for ecdsa_sign".into())); }

        // Hash the data with SHA-256
        let hash = sha256(data)?;
        let digest = std::slice::from_raw_parts(hash.as_ptr(), hash.len());

        // Extract EC_KEY from EVP_PKEY
        let ec_key = (ffi.evp_pkey_get1_ec_key)(pkey);
        if ec_key.is_null() { (ffi.evp_pkey_free)(pkey); return Err(SslError::Other("no EC key".into())); }

        // Use ECDSA_do_sign which returns BIGNUMs directly (no DER parsing needed)
        let sig = (ffi.ecdsa_do_sign)(digest.as_ptr(), digest.len() as c_int, ec_key);
        if sig.is_null() {
            (ffi.ec_key_free)(ec_key);
            (ffi.evp_pkey_free)(pkey);
            return Err(SslError::Other("ECDSA_do_sign failed".into()));
        }
        let r_bn = (ffi.ecdsa_sig_get0_r)(sig);
        let s_bn = (ffi.ecdsa_sig_get0_s)(sig);
        // BN_bn2binpad zero-pads to the full tolen (32 bytes each for P-256)
        let mut raw = vec![0u8; 64];
        (ffi.bn_bn2binpad)(r_bn, raw.as_mut_ptr(), 32);
        (ffi.bn_bn2binpad)(s_bn, raw.as_mut_ptr().add(32), 32);
        (ffi.ecdsa_sig_free)(sig);
        (ffi.ec_key_free)(ec_key);
        (ffi.evp_pkey_free)(pkey);
        Ok(raw)
    }
}

/// Extract the uncompressed public key (65 bytes: 0x04 || x || y) from a P-256 PKCS8 key.
pub fn ec_public_key_bytes(pkcs8_der: &[u8]) -> Result<Vec<u8>, SslError> {
    let ffi = X509Ffi::load().map_err(|e| SslError::Other(e.into()))?;
    unsafe {
        let bio = (ffi.bio_new)((ffi.bio_s_mem)());
        (ffi.bio_write)(bio, pkcs8_der.as_ptr() as *const c_void, pkcs8_der.len() as c_int);
        let pkey = (ffi.d2i_private_key_bio)(bio, std::ptr::null_mut());
        (ffi.bio_free)(bio);
        if pkey.is_null() { return Err(SslError::Other("d2i failed".into())); }

        let ec_key = (ffi.evp_pkey_get1_ec_key)(pkey);
        if ec_key.is_null() { (ffi.evp_pkey_free)(pkey); return Err(SslError::Other("no EC key".into())); }

        let grp = (ffi.ec_key_get0_group)(ec_key);
        let pt = (ffi.ec_key_get0_public_key)(ec_key);
        let mut buf = vec![0u8; 128];
        let n = (ffi.ec_point_to_oct)(grp, pt, 4 /* POINT_CONVERSION_UNCOMPRESSED */, buf.as_mut_ptr(), buf.len(), std::ptr::null_mut());
        (ffi.ec_key_free)(ec_key);
        (ffi.evp_pkey_free)(pkey);
        buf.truncate(n);
        Ok(buf)
    }
}

/// SHA-256 hash using OpenSSL.
pub fn sha256(data: &[u8]) -> Result<[u8; 32], SslError> {
    let ffi = X509Ffi::load().map_err(|e| SslError::Other(e.into()))?;
    unsafe {
        let md = (ffi.evp_sha256)();
        let ctx = (ffi.evp_md_ctx_new)();
        if ctx.is_null() { return Err(SslError::Other("EVP_MD_CTX_new failed".into())); }
        if (ffi.evp_digest_init_ex)(ctx, md, std::ptr::null_mut()) != 1 {
            (ffi.evp_md_ctx_free)(ctx);
            return Err(SslError::Other("EVP_DigestInit_ex failed".into()));
        }
        if (ffi.evp_digest_update)(ctx, data.as_ptr() as *const _, data.len()) != 1 {
            (ffi.evp_md_ctx_free)(ctx);
            return Err(SslError::Other("EVP_DigestUpdate failed".into()));
        }
        let mut out = [0u8; 64];
        let mut len: u32 = 0;
        if (ffi.evp_digest_final_ex)(ctx, out.as_mut_ptr(), &mut len) != 1 {
            (ffi.evp_md_ctx_free)(ctx);
            return Err(SslError::Other("EVP_DigestFinal_ex failed".into()));
        }
        (ffi.evp_md_ctx_free)(ctx);
        let mut result = [0u8; 32];
        result.copy_from_slice(&out[..32]);
        Ok(result)
    }
}

/// Generate a self-signed X.509 certificate and EC P-256 key pair.
/// Writes PEM files to `cert_path` / `key_path`.
/// Returns (cert_der_vec, key_der_vec) bytes for CertifiedKey construction.
pub fn generate_self_signed(
    domain: &str,
    cert_path: &str,
    key_path: &str,
) -> Result<(Vec<u8>, Vec<u8>), SslError> {
    let ffi = X509Ffi::load().map_err(|e| SslError::Other(e.into()))?;
    unsafe { gen_inner(&ffi, domain, cert_path, key_path) }
}

unsafe fn gen_inner(
    f: &X509Ffi,
    domain: &str,
    cert_path: &str,
    key_path: &str,
) -> Result<(Vec<u8>, Vec<u8>), SslError> {
    let nid_subject_alt_name: c_int = 0x55;
    let mbstring_asc: c_int = 0x1001;

    // ── EC key pair (resolve curve NID dynamically — varies across OpenSSL builds) ──
    let nid_prime256v1 = (f.obj_txt2nid)(c"prime256v1".as_ptr());
    if nid_prime256v1 == 0 { return Err(SslError::Other("OBJ_txt2nid(prime256v1) failed".into())); }
    let ec_key = (f.ec_key_new_by_curve_name)(nid_prime256v1);
    if ec_key.is_null() { return Err(SslError::Other("EC_KEY_new".into())); }
    (f.ec_key_generate_key)(ec_key);

    let pkey = (f.evp_pkey_new)();
    (f.evp_pkey_set1_ec_key)(pkey, ec_key);

    // ── X.509 certificate ──
    let x509 = (f.x509_new)();
    if x509.is_null() { (f.evp_pkey_free)(pkey); (f.ec_key_free)(ec_key);
        return Err(SslError::Other("X509_new".into())); }

    (f.x509_set_version)(x509, 2); // X509v3
    let serial = (f.x509_get_serial_number)(x509);
    (f.asn1_integer_set)(serial, 1);
    if let Some(gmfb) = f.x509_get_m_not_before { (f.x509_gmtime_adj)(gmfb(x509), 0); }
    if let Some(gmfa) = f.x509_get_m_not_after { (f.x509_gmtime_adj)(gmfa(x509), 365 * 24 * 3600); }
    (f.x509_set_pubkey)(x509, pkey);

    // ── Distinguished Name ──
    let name = (f.x509_name_new)();
    let c_domain = CString::new(domain).map_err(|_| SslError::Other("bad domain".into()))?;
    (f.x509_name_add_entry_by_txt)(name, c"CN".as_ptr(), mbstring_asc,
        c_domain.as_ptr() as *const u8, c_domain.as_bytes().len() as i32, -1, 0);
    (f.x509_set_subject_name)(x509, name);
    (f.x509_set_issuer_name)(x509, name);

    // ── Extensions ──
    let add_ext = |nid: c_int, value: &CStr| {
        let ext = (f.x509v3_ext_conf_nid)(std::ptr::null_mut(), std::ptr::null_mut(), nid, value.as_ptr(), std::ptr::null());
        if !ext.is_null() { (f.x509_add_ext)(x509, ext, -1); (f.x509_extension_free)(ext); }
    };
    let san_str = CString::new(format!("DNS:{}, DNS:localhost, IP:127.0.0.1, IP:::1", domain)).unwrap();
    add_ext(nid_subject_alt_name, &san_str);
    add_ext(0x57, c"critical,digitalSignature,keyEncipherment"); // keyUsage
    add_ext(0x26, c"serverAuth"); // extendedKeyUsage

    // ── Self-sign ──
    if (f.x509_sign)(x509, pkey, (f.evp_sha256)()) == 0 {
        (f.x509_name_free)(name); (f.x509_free)(x509); (f.evp_pkey_free)(pkey); (f.ec_key_free)(ec_key);
        return Err(SslError::Other("X509_sign".into()));
    }

    // ── DER encode certificate (via BIO so OpenSSL manages memory) ──
    let bio_s_mem = (f.bio_s_mem)();
    let cert_bio = (f.bio_new)(bio_s_mem);
    (f.i2d_x509_bio)(cert_bio, x509);
    let mut cert_der = vec![0u8; 4096];
    let n = (f.bio_read)(cert_bio, cert_der.as_mut_ptr() as *mut c_void, 4096);
    (f.bio_free)(cert_bio);
    cert_der.truncate(if n > 0 { n as usize } else { 0 });

    // ── DER encode private key (unencrypted PKCS#8) ──
    let bio = (f.bio_new)(bio_s_mem);
    if let Some(pkcs8) = f.i2d_pkcs8_private_key_bio {
        pkcs8(bio, pkey, std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut());
    }
    let mut key_der = vec![0u8; 4096];
    let n = (f.bio_read)(bio, key_der.as_mut_ptr() as *mut c_void, 4096);
    (f.bio_free)(bio);
    key_der.truncate(if n > 0 { n as usize } else { 0 });

    // ── Write PEM files ──
    let c_cert = CString::new(cert_path).unwrap();
    let bio = (f.bio_new_file)(c_cert.as_ptr(), c"w".as_ptr());
    if !bio.is_null() { (f.pem_write_bio_x509)(bio, x509); (f.bio_free)(bio); }
    let c_key = CString::new(key_path).unwrap();
    let bio = (f.bio_new_file)(c_key.as_ptr(), c"w".as_ptr());
    if !bio.is_null() {
        (f.pem_write_bio_private_key)(bio, pkey, std::ptr::null(), std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut());
        (f.bio_free)(bio);
    }

    // ── Cleanup ──
    (f.x509_name_free)(name);
    (f.x509_free)(x509);
    (f.evp_pkey_free)(pkey);
    (f.ec_key_free)(ec_key);

    if cert_der.is_empty() || key_der.is_empty() {
        return Err(SslError::Other("DER encoding empty".into()));
    }

    Ok((cert_der, key_der))
}

/// Parse a PEM-encoded certificate string into a raw `*mut X509` pointer.
/// The caller must free the returned pointer with `X509_free`.
/// Returns null on error.
pub fn pem_read_certificate(pem: &str) -> Result<*mut c_void, SslError> {
    let ffi = X509Ffi::load().map_err(|e| SslError::Other(e.into()))?;
    unsafe {
        let bio = (ffi.bio_new)((ffi.bio_s_mem)());
        if bio.is_null() { return Err(SslError::Other("BIO_new".into())); }
        (ffi.bio_write)(bio, pem.as_ptr() as *const c_void, pem.len() as c_int);
        let x509 = (ffi.pem_read_bio_x509)(bio, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        (ffi.bio_free)(bio);
        if x509.is_null() { Err(SslError::Other("PEM_read_bio_X509".into())) }
        else { Ok(x509) }
    }
}

/// Parse a PEM-encoded private key string into a raw `*mut EVP_PKEY` pointer.
/// The caller must free the returned pointer with `EVP_PKEY_free`.
/// Returns null on error.
pub fn pem_read_private_key(pem: &str) -> Result<*mut c_void, SslError> {
    let ffi = X509Ffi::load().map_err(|e| SslError::Other(e.into()))?;
    unsafe {
        let bio = (ffi.bio_new)((ffi.bio_s_mem)());
        if bio.is_null() { return Err(SslError::Other("BIO_new".into())); }
        (ffi.bio_write)(bio, pem.as_ptr() as *const c_void, pem.len() as c_int);
        let pkey = (ffi.pem_read_bio_private_key)(bio, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        (ffi.bio_free)(bio);
        if pkey.is_null() { Err(SslError::Other("PEM_read_bio_PrivateKey".into())) }
        else { Ok(pkey) }
    }
}
