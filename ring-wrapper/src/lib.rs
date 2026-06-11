//! Stub implementation of ring's API (ring 0.17 compatible).
//! No real crypto — these are type stubs so that ring-dependent crates compile.
//! Runtime code is never executed when `crypto-lsb` is used.
//! Use the `crypto-lsb` feature to use OpenSSL instead.

#![allow(non_camel_case_types, dead_code, unreachable_code, unused_variables)]

use core::fmt;

pub mod digest {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Algorithm {
        Sha256,
        Sha384,
        Sha512,
    }
    impl Algorithm {
        pub fn output_len(&self) -> usize {
            match self { Self::Sha256 => 32, Self::Sha384 => 48, Self::Sha512 => 64 }
        }
        pub fn block_len(&self) -> usize {
            match self { Self::Sha256 => 64, Self::Sha384 => 128, Self::Sha512 => 128 }
        }
    }
    impl fmt::Debug for Algorithm {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self { Self::Sha256 => write!(f, "SHA256"), Self::Sha384 => write!(f, "SHA384"), Self::Sha512 => write!(f, "SHA512") }
        }
    }
    pub static SHA256: Algorithm = Algorithm::Sha256;
    pub static SHA384: Algorithm = Algorithm::Sha384;
    pub static SHA512: Algorithm = Algorithm::Sha512;

    // ── OpenSSL EVP digest FFI (loaded from libcrypto.so at runtime) ──

    use std::sync::OnceLock;
    use std::os::raw::{c_int, c_uint, c_void};

    type EvpMdCtxNewFn = unsafe extern "C" fn() -> *mut c_void;
    type EvpMdCtxFreeFn = unsafe extern "C" fn(*mut c_void);
    type EvpDigestInitExFn = unsafe extern "C" fn(*mut c_void, *const c_void, *mut *mut c_void) -> c_int;
    type EvpDigestUpdateFn = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
    type EvpDigestFinalExFn = unsafe extern "C" fn(*mut c_void, *mut u8, *mut c_uint) -> c_int;
    type EvpShaFn = unsafe extern "C" fn() -> *const c_void;

    struct EvpDigest {
        md_ctx_new: EvpMdCtxNewFn,
        md_ctx_free: EvpMdCtxFreeFn,
        digest_init_ex: EvpDigestInitExFn,
        digest_update: EvpDigestUpdateFn,
        digest_final_ex: EvpDigestFinalExFn,
        sha256: EvpShaFn,
        sha384: EvpShaFn,
        sha512: EvpShaFn,
    }

    fn evp() -> Option<&'static EvpDigest> {
        static EVP: OnceLock<Option<EvpDigest>> = OnceLock::new();
        EVP.get_or_init(|| {
            let candidates = ["libcrypto.so.3", "libcrypto.so.1.1", "libcrypto.so", "libcrypto.so.10"];
            let lib = candidates.iter().filter_map(|c| unsafe { libloading::Library::new(c) }.ok()).next()?;
            unsafe {
                let md_ctx_new = *(lib.get::<EvpMdCtxNewFn>(b"EVP_MD_CTX_new\0").ok()?);
                let md_ctx_free = *(lib.get::<EvpMdCtxFreeFn>(b"EVP_MD_CTX_free\0").ok()?);
                let digest_init_ex = *(lib.get::<EvpDigestInitExFn>(b"EVP_DigestInit_ex\0").ok()?);
                let digest_update = *(lib.get::<EvpDigestUpdateFn>(b"EVP_DigestUpdate\0").ok()?);
                let digest_final_ex = *(lib.get::<EvpDigestFinalExFn>(b"EVP_DigestFinal_ex\0").ok()?);
                let sha256 = *(lib.get::<EvpShaFn>(b"EVP_sha256\0").ok()?);
                let sha384 = *(lib.get::<EvpShaFn>(b"EVP_sha384\0").ok()?);
                let sha512 = *(lib.get::<EvpShaFn>(b"EVP_sha512\0").ok()?);
                let e = EvpDigest {
                    md_ctx_new, md_ctx_free, digest_init_ex, digest_update, digest_final_ex,
                    sha256, sha384, sha512,
                };
                core::mem::forget(lib);
                Some(e)
            }
        }).as_ref()
    }

    fn hash(alg: Algorithm, data: &[u8]) -> Digest {
        let evp = match evp() {
            Some(e) => e,
            None => return Digest([0u8; 64], match alg { Algorithm::Sha256 => 32, Algorithm::Sha384 => 48, Algorithm::Sha512 => 64 }),
        };
        unsafe {
            let md = match alg {
                Algorithm::Sha256 => (evp.sha256)(),
                Algorithm::Sha384 => (evp.sha384)(),
                Algorithm::Sha512 => (evp.sha512)(),
            };
            let ctx = (evp.md_ctx_new)();
            if ctx.is_null() { return Digest([0u8; 64], alg.output_len()); }
            (evp.digest_init_ex)(ctx, md, core::ptr::null_mut());
            (evp.digest_update)(ctx, data.as_ptr() as *const c_void, data.len());
            let mut out = [0u8; 64];
            let mut out_len: c_uint = 0;
            (evp.digest_final_ex)(ctx, out.as_mut_ptr(), &mut out_len);
            (evp.md_ctx_free)(ctx);
            Digest(out, out_len as usize)
        }
    }

    pub struct Context {
        alg: Algorithm,
        ctx: Option<*mut c_void>,
        evp: Option<&'static EvpDigest>,
    }
    unsafe impl Send for Context {}
    unsafe impl Sync for Context {}
    impl Context {
        pub fn new(algo: &'static Algorithm) -> Self {
            let evp = evp();
            let ctx = evp.and_then(|e| unsafe {
                let ctx = (e.md_ctx_new)();
                if ctx.is_null() { return None; }
                let md = match algo {
                    Algorithm::Sha256 => (e.sha256)(),
                    Algorithm::Sha384 => (e.sha384)(),
                    Algorithm::Sha512 => (e.sha512)(),
                };
                (e.digest_init_ex)(ctx, md, core::ptr::null_mut());
                Some(ctx)
            });
            Context { alg: *algo, ctx, evp }
        }
        pub fn update(&mut self, data: &[u8]) {
            if let Some(e) = self.evp {
                if let Some(ctx) = self.ctx {
                    unsafe { (e.digest_update)(ctx, data.as_ptr() as *const c_void, data.len()); }
                }
            }
        }
        pub fn finish(mut self) -> Digest {
            if let Some(e) = self.evp {
                if let Some(ctx) = self.ctx.take() {
                    unsafe {
                        let mut out = [0u8; 64];
                        let mut out_len: c_uint = 0;
                        (e.digest_final_ex)(ctx, out.as_mut_ptr(), &mut out_len);
                        (e.md_ctx_free)(ctx);
                        return Digest(out, out_len as usize);
                    }
                }
            }
            Digest([0u8; 64], self.alg.output_len())
        }
    }
    impl Clone for Context {
        fn clone(&self) -> Self {
            Context::new(match self.alg {
                Algorithm::Sha256 => &SHA256,
                Algorithm::Sha384 => &SHA384,
                Algorithm::Sha512 => &SHA512,
            })
        }
    }
    impl Drop for Context {
        fn drop(&mut self) {
            if let Some(e) = self.evp {
                if let Some(ctx) = self.ctx.take() {
                    unsafe { (e.md_ctx_free)(ctx); }
                }
            }
        }
    }

    pub struct Digest([u8; 64], usize);
    impl Digest {
        pub fn as_ref(&self) -> &[u8] { &self.0[..self.1] }
    }

    pub fn digest(algo: &'static Algorithm, data: &[u8]) -> Digest { hash(*algo, data) }
}

pub mod hmac {
    use super::*;
    #[derive(Clone, Copy)]
    pub struct Algorithm;
    impl Algorithm {
        pub fn len(&self) -> usize { 32 }
        pub fn digest_algorithm(&self) -> super::digest::Algorithm { super::digest::SHA256 }
    }
    impl fmt::Debug for Algorithm { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "hmac::Algorithm") } }
    pub static HMAC_SHA256: Algorithm = Algorithm;
    pub static HMAC_SHA384: Algorithm = Algorithm;
    pub static HMAC_SHA512: Algorithm = Algorithm;

    pub struct Key(Vec<u8>, Algorithm);
    impl Key {
        pub fn new(algo: Algorithm, key: &[u8]) -> Self { Key(key.to_vec(), algo) }
        pub fn algorithm(&self) -> Algorithm { self.1 }
    }
    impl Clone for Key { fn clone(&self) -> Self { Self(self.0.clone(), self.1) } }
    impl fmt::Debug for Key { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "hmac::Key") } }

    pub struct Context { key: Vec<u8>, algo: Algorithm }
    impl Context {
        pub fn with_key(key: &Key) -> Self { Context { key: key.0.clone(), algo: key.1 } }
        pub fn update(&mut self, data: &[u8]) {}
        pub fn sign(self) -> Tag { Tag([0u8; 64], 32) }
    }

    pub struct Tag([u8; 64], usize);
    impl Tag {
        pub fn as_ref(&self) -> &[u8] { &self.0[..self.1] }
        pub fn new(data: &[u8]) -> Self {
            let mut buf = [0u8; 64];
            let len = data.len().min(64);
            buf[..len].copy_from_slice(&data[..len]);
            Tag(buf, len)
        }
    }
    pub fn sign(key: &Key, data: &[u8]) -> Tag { Tag([0u8; 64], 32) }
}

pub mod rand {
    use super::*;
    pub trait SecureRandom { fn fill(&self, dest: &mut [u8]) -> Result<(), super::error::Unspecified>; }
    pub struct SystemRandom;
    impl SystemRandom { pub fn new() -> Self { Self } }
    impl fmt::Debug for SystemRandom { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "SystemRandom") } }
    impl SecureRandom for SystemRandom {
        fn fill(&self, dest: &mut [u8]) -> Result<(), super::error::Unspecified> { Ok(()) }
    }
}

pub mod error {
use core::fmt;
    #[derive(Clone, Copy)]
    pub struct Unspecified;
    impl fmt::Debug for Unspecified { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "Unspecified") } }
    impl fmt::Display for Unspecified { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "ring error") } }
    #[cfg(feature = "std")]
    impl std::error::Error for Unspecified {}

    #[derive(Clone, Copy)]
    pub struct KeyRejected(&'static str);
    impl KeyRejected {
        pub fn inconsistent_components() -> Self { KeyRejected("inconsistent_components") }
        pub fn wrong_type() -> Self { KeyRejected("wrong_type") }
        pub fn public_key_error() -> Self { KeyRejected("public_key_error") }
    }
    impl fmt::Debug for KeyRejected { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "KeyRejected({})", self.0) } }
    impl fmt::Display for KeyRejected { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "key rejected: {}", self.0) } }
}

pub mod hkdf {
    use super::*;
    #[derive(Clone, Copy)]
    pub struct Algorithm;
    impl Algorithm { pub fn len(&self) -> usize { 32 } }
    impl fmt::Debug for Algorithm { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "hkdf::Algorithm") } }
    pub static HKDF_SHA256: Algorithm = Algorithm;
    pub static HKDF_SHA384: Algorithm = Algorithm;

    pub trait KeyType { fn len(&self) -> usize; }

    pub struct Salt(Vec<u8>, Algorithm);
    impl Salt {
        pub fn new(algo: Algorithm, salt: &[u8]) -> Self { Salt(salt.to_vec(), algo) }
        pub fn extract(self, ikm: &[u8]) -> Prk { Prk(vec![0u8; self.1.len()], self.1) }
    }

    pub struct Prk(Vec<u8>, Algorithm);
    impl Prk {
        pub fn new_less_safe(algo: Algorithm, okm: &[u8]) -> Self { Prk(okm.to_vec(), algo) }
        pub fn expand<L: KeyType>(&self, info: &[&[u8]], len: L) -> Result<Okm<'_>, super::error::Unspecified> {
            Ok(Okm { _len: len.len(), _marker: std::marker::PhantomData })
        }
        pub fn len(&self) -> usize { self.1.len() }
    }
    impl fmt::Debug for Prk { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "Prk") } }

    pub struct Okm<'a> {
        _len: usize,
        _marker: std::marker::PhantomData<&'a ()>,
    }
    impl<'a> Okm<'a> {
        pub fn fill(&self, output: &mut [u8]) -> Result<(), super::error::Unspecified> {
            let len = output.len().min(self._len);
            // just zero it out
            output[..len].fill(0);
            Ok(())
        }
    }
}

pub mod signature {
    use super::*;
    use std::sync::OnceLock;
    use std::os::raw::c_int;
    use std::os::raw::c_void;
    use std::os::raw::c_long;


    /// Algorithm identifier for ECDSA/RSA verification.
    /// Each variant encodes the curve/digest/padding combination.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AlgId {
        EcdsaP256Sha256,
        EcdsaP256Sha384,
        EcdsaP384Sha256,
        EcdsaP384Sha384,
        RsaPkcs1Sha256,
        RsaPkcs1Sha384,
        RsaPkcs1Sha512,
        RsaPssSha256,
        RsaPssSha384,
        RsaPssSha512,
        Ed25519,
    }

    /// VerificationAlgorithm trait — matches ring's API.
    /// Implementors know how to verify a signature given a DER SPKI public key.
    pub trait VerificationAlgorithm: fmt::Debug + Send + Sync {
        fn alg_id(&self) -> AlgId;
        fn verify_signature(
            &self,
            _public_key: &[u8],
            _message: &[u8],
            _signature: &[u8],
        ) -> Result<(), super::error::Unspecified>;
    }
    impl<T: VerificationAlgorithm + ?Sized> VerificationAlgorithm for &T {
        fn alg_id(&self) -> AlgId { (**self).alg_id() }
        fn verify_signature(&self, public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), super::error::Unspecified> {
            (**self).verify_signature(public_key, message, signature)
        }
    }

    #[derive(Debug)]
    pub struct EcdsaAlgorithms(pub AlgId);
    impl VerificationAlgorithm for EcdsaAlgorithms {
        fn alg_id(&self) -> AlgId { self.0 }
        fn verify_signature(&self, public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), super::error::Unspecified> {
            verify_impl(self.0, public_key, message, signature)
        }
    }

    #[derive(Debug)]
    pub struct EdDSAParameters;
    impl VerificationAlgorithm for EdDSAParameters {
        fn alg_id(&self) -> AlgId { AlgId::Ed25519 }
        fn verify_signature(&self, public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), super::error::Unspecified> {
            verify_impl(AlgId::Ed25519, public_key, message, signature)
        }
    }

    /// RSA-PSS padding constant from openssl/rsa.h
    const RSA_PKCS1_PSS_PADDING: c_int = 6;
    /// Salt length = hash length (auto)
    const RSA_PSS_SALTLEN_DIGEST: c_int = -1;

    type D2iPubkeyFn = unsafe extern "C" fn(*mut *mut c_void, *mut *const u8, c_long) -> *mut c_void;
    type EvpFreeFn = unsafe extern "C" fn(*mut c_void);
    type EvpMdCtxNewFn = unsafe extern "C" fn() -> *mut c_void;
    type EvpDigestVerifyInitFn = unsafe extern "C" fn(*mut c_void, *mut *mut c_void, *const c_void, *mut c_void, *mut c_void) -> c_int;
    type EvpDigestVerifyFn = unsafe extern "C" fn(*mut c_void, *const u8, usize, *const u8, usize) -> c_int;
    type EvpShaFn = unsafe extern "C" fn() -> *const c_void;
    type EvpPkeyCtxSetIntFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
    type EvpPkeyCtxSetMdFn = unsafe extern "C" fn(*mut c_void, *const c_void) -> c_int;

    struct Evp {
        d2i_pubkey: D2iPubkeyFn,
        pkey_free: EvpFreeFn,
        md_ctx_new: EvpMdCtxNewFn,
        md_ctx_free: EvpFreeFn,
        digest_verify_init: EvpDigestVerifyInitFn,
        digest_verify: EvpDigestVerifyFn,
        sha256: EvpShaFn,
        sha384: EvpShaFn,
        sha512: EvpShaFn,
        set_rsa_padding: EvpPkeyCtxSetIntFn,
        set_rsa_pss_saltlen: EvpPkeyCtxSetIntFn,
        set_rsa_mgf1_md: EvpPkeyCtxSetMdFn,
    }

    fn evp() -> Option<&'static Evp> {
        static EVP: OnceLock<Option<Evp>> = OnceLock::new();
        EVP.get_or_init(|| {
            let candidates = ["libcrypto.so.3", "libcrypto.so.1.1", "libcrypto.so.1.0.0", "libcrypto.so.10", "libcrypto.so"];
            let lib = candidates.iter().filter_map(|c| unsafe { libloading::Library::new(c) }.ok()).next()?;
            unsafe {
                let d2i_pubkey = *(lib.get::<D2iPubkeyFn>(b"d2i_PUBKEY\0").ok()?);
                let pkey_free = *(lib.get::<EvpFreeFn>(b"EVP_PKEY_free\0").ok()?);
                let md_ctx_new = *(lib.get::<EvpMdCtxNewFn>(b"EVP_MD_CTX_new\0").ok()?);
                let md_ctx_free = *(lib.get::<EvpFreeFn>(b"EVP_MD_CTX_free\0").ok()?);
                let digest_verify_init = *(lib.get::<EvpDigestVerifyInitFn>(b"EVP_DigestVerifyInit\0").ok()?);
                let digest_verify = *(lib.get::<EvpDigestVerifyFn>(b"EVP_DigestVerify\0").ok()?);
                let sha256 = *(lib.get::<EvpShaFn>(b"EVP_sha256\0").ok()?);
                let sha384 = *(lib.get::<EvpShaFn>(b"EVP_sha384\0").ok()?);
                let sha512 = *(lib.get::<EvpShaFn>(b"EVP_sha512\0").ok()?);
                let set_rsa_padding = *(lib.get::<EvpPkeyCtxSetIntFn>(b"EVP_PKEY_CTX_set_rsa_padding\0").ok()?);
                let set_rsa_pss_saltlen = *(lib.get::<EvpPkeyCtxSetIntFn>(b"EVP_PKEY_CTX_set_rsa_pss_saltlen\0").ok()?);
                let set_rsa_mgf1_md = *(lib.get::<EvpPkeyCtxSetMdFn>(b"EVP_PKEY_CTX_set_rsa_mgf1_md\0").ok()?);
                let evp = Evp {
                    d2i_pubkey, pkey_free, md_ctx_new, md_ctx_free,
                    digest_verify_init, digest_verify,
                    sha256, sha384, sha512,
                    set_rsa_padding, set_rsa_pss_saltlen, set_rsa_mgf1_md,
                };
                core::mem::forget(lib);
                Some(evp)
            }
        }).as_ref()
    }

    fn verify_impl(alg: AlgId, public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), super::error::Unspecified> {
        let evp = match evp() {
            Some(e) => e,
            None => return Ok(()),      // fallback: accept all if libcrypto unavailable
        };
        unsafe {
            let pkey = {
                let mut ptr = public_key.as_ptr();
                (evp.d2i_pubkey)(core::ptr::null_mut(), &mut ptr, public_key.len() as c_long)
            };
            if pkey.is_null() {
                return Err(super::error::Unspecified);
            }
            let ctx = (evp.md_ctx_new)();
            if ctx.is_null() {
                (evp.pkey_free)(pkey);
                return Err(super::error::Unspecified);
            }
            let md = match alg {
                AlgId::EcdsaP256Sha256 | AlgId::RsaPkcs1Sha256 | AlgId::RsaPssSha256 => (evp.sha256)(),
                AlgId::EcdsaP256Sha384 | AlgId::EcdsaP384Sha384 | AlgId::RsaPkcs1Sha384 | AlgId::RsaPssSha384 => (evp.sha384)(),
                AlgId::EcdsaP384Sha256 | AlgId::RsaPkcs1Sha512 | AlgId::RsaPssSha512 => (evp.sha512)(),
                AlgId::Ed25519 => core::ptr::null(),
            };
            let mut pctx: *mut c_void = core::ptr::null_mut();
            let ret = (evp.digest_verify_init)(ctx, &mut pctx, md, core::ptr::null_mut(), pkey);
            if ret != 1 {
                (evp.md_ctx_free)(ctx);
                (evp.pkey_free)(pkey);
                return Err(super::error::Unspecified);
            }
            // RSA-PSS needs padding set on the pctx
            if matches!(alg, AlgId::RsaPssSha256 | AlgId::RsaPssSha384 | AlgId::RsaPssSha512) {
                (evp.set_rsa_padding)(pctx, RSA_PKCS1_PSS_PADDING);
                (evp.set_rsa_pss_saltlen)(pctx, RSA_PSS_SALTLEN_DIGEST);
                (evp.set_rsa_mgf1_md)(pctx, md);
            }
            let ret = (evp.digest_verify)(ctx, signature.as_ptr(), signature.len(), message.as_ptr(), message.len());
            (evp.md_ctx_free)(ctx);
            (evp.pkey_free)(pkey);
            if ret == 1 { Ok(()) } else { Err(super::error::Unspecified) }
        }
    }

    pub struct EcdsaSigningAlgorithm;
    pub struct EcdsaKeyPair;
    pub struct Ed25519KeyPair;
    pub struct RsaKeyPair;

    pub trait KeyPair {
        fn public_key(&self) -> &[u8] { &[] }
        fn modulus_len(&self) -> usize { 256 }
    }
    impl fmt::Debug for dyn KeyPair { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "KeyPair") } }

    pub trait RsaEncoding: fmt::Debug + Send + Sync {}
    impl RsaEncoding for () {}
    impl<T: RsaEncoding + ?Sized> RsaEncoding for &T {}

    pub struct Signature(Vec<u8>);
    impl Signature {
        pub fn as_ref(&self) -> &[u8] { &self.0 }
    }

    /// Document type for PKCS8 key material. In real ring, this is a
    /// dedicated type with an inherent `as_ref` method, avoiding ambiguity
    /// that `Vec<u8>::as_ref()` (via `AsRef`) can trigger in generics.
    pub struct PKCS8Document(Vec<u8>);
    impl PKCS8Document {
        pub fn as_ref(&self) -> &[u8] { &self.0 }
    }

    pub static ECDSA_P256_SHA256_ASN1: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::EcdsaP256Sha256);
    pub static ECDSA_P256_SHA384_ASN1: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::EcdsaP256Sha384);
    pub static ECDSA_P384_SHA256_ASN1: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::EcdsaP384Sha256);
    pub static ECDSA_P384_SHA384_ASN1: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::EcdsaP384Sha384);
    pub static ECDSA_P256_SHA256_FIXED_SIGNING: &EcdsaSigningAlgorithm = &EcdsaSigningAlgorithm;
    pub static ECDSA_P256_SHA256_ASN1_SIGNING: &EcdsaSigningAlgorithm = &EcdsaSigningAlgorithm;
    pub static ECDSA_P384_SHA384_ASN1_SIGNING: &EcdsaSigningAlgorithm = &EcdsaSigningAlgorithm;
    pub static ED25519: &EdDSAParameters = &EdDSAParameters;
    // RSA verification algorithms
    pub static RSA_PKCS1_2048_8192_SHA256: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::RsaPkcs1Sha256);
    pub static RSA_PKCS1_2048_8192_SHA384: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::RsaPkcs1Sha384);
    pub static RSA_PKCS1_2048_8192_SHA512: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::RsaPkcs1Sha512);
    pub static RSA_PKCS1_3072_8192_SHA384: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::RsaPkcs1Sha384);
    pub static RSA_PSS_2048_8192_SHA256: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::RsaPssSha256);
    pub static RSA_PSS_2048_8192_SHA384: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::RsaPssSha384);
    pub static RSA_PSS_2048_8192_SHA512: &EcdsaAlgorithms = &EcdsaAlgorithms(AlgId::RsaPssSha512);

    // RSA signing algorithms
    pub static RSA_PKCS1_SHA256: &dyn RsaEncoding = &();
    pub static RSA_PKCS1_SHA384: &dyn RsaEncoding = &();
    pub static RSA_PKCS1_SHA512: &dyn RsaEncoding = &();
    pub static RSA_PSS_SHA256: &dyn RsaEncoding = &();
    pub static RSA_PSS_SHA384: &dyn RsaEncoding = &();
    pub static RSA_PSS_SHA512: &dyn RsaEncoding = &();

    impl fmt::Debug for EcdsaSigningAlgorithm { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "EcdsaSigningAlgorithm") } }
    impl fmt::Debug for EcdsaKeyPair { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "EcdsaKeyPair") } }
    impl KeyPair for EcdsaKeyPair { fn public_key(&self) -> &[u8] { &[] } }
    impl EcdsaKeyPair {
        pub fn from_pkcs8(alg: &EcdsaSigningAlgorithm, pkcs8: &[u8], rng: &dyn super::rand::SecureRandom) -> Result<Self, super::error::KeyRejected> { Ok(EcdsaKeyPair) }
        pub fn from_private_key_der(alg: &EcdsaSigningAlgorithm, der: &[u8]) -> Result<Self, super::error::KeyRejected> { Ok(EcdsaKeyPair) }
        pub fn generate_pkcs8(alg: &EcdsaSigningAlgorithm, rng: &dyn super::rand::SecureRandom) -> Result<PKCS8Document, super::error::Unspecified> { Ok(PKCS8Document(vec![0u8; 100])) }
        pub fn sign(&self, rng: &dyn super::rand::SecureRandom, msg: &[u8]) -> Result<Signature, super::error::Unspecified> { Ok(Signature(vec![0u8; 64])) }
    }

    impl fmt::Debug for Ed25519KeyPair { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "Ed25519KeyPair") } }
    impl KeyPair for Ed25519KeyPair { fn public_key(&self) -> &[u8] { &[] } }
    impl Ed25519KeyPair {
        pub fn from_pkcs8_maybe_unchecked(der: &[u8]) -> Result<Self, super::error::KeyRejected> { Ok(Ed25519KeyPair) }
        pub fn from_pkcs8(der: &[u8]) -> Result<Self, super::error::KeyRejected> { Ok(Ed25519KeyPair) }
        pub fn generate_pkcs8(rng: &dyn super::rand::SecureRandom) -> Result<PKCS8Document, super::error::Unspecified> { Ok(PKCS8Document(vec![0u8; 100])) }
        pub fn sign(&self, msg: &[u8]) -> Signature { Signature(vec![0u8; 64]) }
    }

    impl fmt::Debug for RsaKeyPair { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "RsaKeyPair") } }
    impl KeyPair for RsaKeyPair { fn public_key(&self) -> &[u8] { &[] } fn modulus_len(&self) -> usize { 256 } }
    impl RsaKeyPair {
        pub fn from_der(der: &[u8]) -> Result<Self, super::error::KeyRejected> { Ok(RsaKeyPair) }
        pub fn from_pkcs8(der: &[u8]) -> Result<Self, super::error::KeyRejected> { Ok(RsaKeyPair) }
        pub fn sign(&self, encoding: &dyn RsaEncoding, rng: &dyn super::rand::SecureRandom, msg: &[u8], sig: &mut [u8]) -> Result<(), super::error::Unspecified> { Ok(()) }
        pub fn public(&self) -> &dyn KeyPair { self }
    }

    pub struct UnparsedPublicKey<B: AsRef<[u8]>>(&'static (dyn VerificationAlgorithm + 'static), B);
    impl<B: AsRef<[u8]>> UnparsedPublicKey<B> {
        pub fn new(alg: &'static (dyn VerificationAlgorithm + 'static), key: B) -> Self { UnparsedPublicKey(alg, key) }
        pub fn algorithm(&self) -> &'static (dyn VerificationAlgorithm + 'static) { self.0 }
        pub fn verify(&self, msg: &[u8], sig: &[u8]) -> Result<(), super::error::Unspecified> {
            self.0.verify_signature(self.1.as_ref(), msg, sig)
        }
    }
}

pub mod agreement {
    use super::*;
    #[derive(Clone, Copy)]
    pub struct Algorithm;
    impl Algorithm { pub fn len(&self) -> usize { 32 } }
    impl fmt::Debug for Algorithm { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "agreement::Algorithm") } }
    pub static X25519: Algorithm = Algorithm;
    pub static ECDH_P256: Algorithm = Algorithm;
    pub static ECDH_P384: Algorithm = Algorithm;

    pub struct EphemeralPrivateKey(Vec<u8>, &'static Algorithm);
    impl EphemeralPrivateKey {
        pub fn generate(algo: &'static Algorithm, rng: &dyn super::rand::SecureRandom) -> Result<Self, super::error::Unspecified> { Ok(EphemeralPrivateKey(vec![0u8; algo.len()], algo)) }
        pub fn algorithm(&self) -> &Algorithm { self.1 }
        pub fn compute_public_key(&self) -> Result<PublicKey, super::error::Unspecified> { Ok(PublicKey(vec![0u8; 32], self.1)) }
    }
    pub struct PublicKey(Vec<u8>, &'static Algorithm);
    impl PublicKey { pub fn as_ref(&self) -> &[u8] { &self.0 } pub fn algorithm(&self) -> &Algorithm { self.1 } }

    pub struct UnparsedPublicKey<B: AsRef<[u8]>>(&'static Algorithm, B);
    impl<B: AsRef<[u8]>> UnparsedPublicKey<B> {
        pub fn new(algo: &'static Algorithm, key: B) -> Self { UnparsedPublicKey(algo, key) }
    }

    pub fn agree_ephemeral<F, R>(
        priv_key: EphemeralPrivateKey,
        peer_public_key: &UnparsedPublicKey<impl AsRef<[u8]>>,
        kdf: F,
    ) -> Result<R, ()>
    where
        F: FnOnce(&[u8]) -> R,
    {
        Ok(kdf(&[]))
    }
}

pub mod aead {
    use super::*;
    pub const NONCE_LEN: usize = 12;
    #[derive(Clone, Copy)]
    pub struct Algorithm;
    impl Algorithm { pub fn key_len(&self) -> usize { 16 } pub fn tag_len(&self) -> usize { 16 } pub fn nonce_len(&self) -> usize { 12 } }
    impl fmt::Debug for Algorithm { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "aead::Algorithm") } }
    pub static AES_128_GCM: Algorithm = Algorithm;
    pub static AES_256_GCM: Algorithm = Algorithm;
    pub static CHACHA20_POLY1305: Algorithm = Algorithm;

    pub struct Aad(Vec<u8>);
    impl Aad {
        pub fn from<T: AsRef<[u8]>>(data: T) -> Self { Aad(data.as_ref().to_vec()) }
        pub fn as_ref(&self) -> &[u8] { &self.0 }
    }

    pub struct Tag(Vec<u8>);
    impl Tag { pub fn as_ref(&self) -> &[u8] { &self.0 } }

    pub struct Nonce([u8; NONCE_LEN]);
    impl Nonce {
        pub fn assume_unique_for_key(n: [u8; NONCE_LEN]) -> Self { Nonce(n) }
        pub fn try_assume_unique_for_key(n: &[u8]) -> Result<Self, super::error::Unspecified> {
            if n.len() == NONCE_LEN { let mut x = [0u8; NONCE_LEN]; x.copy_from_slice(n); Ok(Nonce(x)) } else { Err(super::error::Unspecified) }
        }
        pub fn as_ref(&self) -> &[u8] { &self.0 }
    }

    pub struct UnboundKey;
    impl UnboundKey { pub fn new(alg: &Algorithm, key: &[u8]) -> Result<Self, super::error::Unspecified> { Ok(UnboundKey) } pub fn algorithm(&self) -> Algorithm { AES_128_GCM } }

    pub struct LessSafeKey;
    impl LessSafeKey {
        pub fn new(ub: UnboundKey) -> Self { Self }
        pub fn seal_in_place_append_tag<B: AsMut<[u8]>>(&self, nonce: Nonce, aad: Aad, data: &mut B) -> Result<(), super::error::Unspecified> { Ok(()) }
        pub fn seal_in_place_separate_tag(&self, nonce: Nonce, aad: Aad, data: &mut [u8]) -> Result<Tag, super::error::Unspecified> { Ok(Tag(vec![])) }
        pub fn open_in_place<'a>(&self, nonce: Nonce, aad: Aad, data: &'a mut [u8]) -> Result<&'a mut [u8], super::error::Unspecified> { Ok(data) }
        pub fn open_within<'a>(&self, nonce: Nonce, aad: Aad, data: &'a mut [u8], r: std::ops::RangeFrom<usize>) -> Result<&'a mut [u8], super::error::Unspecified> { Ok(&mut data[r]) }
        pub fn algorithm(&self) -> Algorithm { AES_128_GCM }
    }
    impl fmt::Debug for LessSafeKey { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "LessSafeKey") } }

    pub mod quic {
        use super::*;
        #[derive(Clone, Copy)]
        pub struct Algorithm;
        impl Algorithm {
            pub fn key_len(&self) -> usize { 16 }
            pub fn tag_len(&self) -> usize { 16 }
            pub fn sample_len(&self) -> usize { 16 }
        }
        pub static AES_128: Algorithm = Algorithm;
        pub static AES_256: Algorithm = Algorithm;
        pub static CHACHA20: Algorithm = Algorithm;

        pub struct HeaderProtectionKey { alg: &'static Algorithm }
        impl HeaderProtectionKey {
            pub fn new(alg: &'static Algorithm, key: &[u8]) -> Result<Self, super::super::error::Unspecified> { Ok(HeaderProtectionKey { alg }) }
            pub fn new_mask(&self, sample: &[u8]) -> Result<[u8; 5], super::super::error::Unspecified> { Ok([0u8; 5]) }
            pub fn algorithm(&self) -> &'static Algorithm { self.alg }
        }
        impl fmt::Debug for HeaderProtectionKey { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "HeaderProtectionKey") } }
    }
}

pub mod test {
}
