//! Test zlib (all store versions) + OpenSSL 3.6 (latest).
//!
//! OpenSSL 3.x versions tested in separate binaries to avoid soname conflicts.

mod common;

// ── Version definitions ───────────────────────────────────────────────────

const ZLIB_GLOBS: &[&str] = &[
    "*-zlib-1.2.11",
    "*-zlib-1.3.1",
    "*-zlib-1.3.2",
];

const OPENSSL_GLOB: &str = "*-openssl-3.6.*";

// ── zlib: test all store versions ─────────────────────────────────────────

fn test_single_zlib(glob: &str) -> Option<String> {
    let libz = common::find_zlib_glob(glob)?;
    std::env::set_var("LSBWRAP_LIBZ_PATH", &libz);

    let z = lsb_zlib::Zlib::load().expect("zlib load");
    let v = z.version();

    // Small round-trip
    let input = b"Testing Nix zlib!";
    let compressed = z.compress_vec(input).expect("compress");
    let decompressed = z
        .uncompress_vec(&compressed, input.len() + 64)
        .expect("uncompress");
    assert_eq!(input, &decompressed[..]);

    // Large data (64 KiB)
    let large = vec![0xABu8; 65536];
    let compressed = z.compress_vec(&large).expect("compress large");
    let decompressed = z
        .uncompress_vec(&compressed, large.len() + 65536)
        .expect("uncompress large");
    assert_eq!(large, decompressed);

    // Empty input
    let compressed = z.compress_vec(b"").expect("compress empty");
    let decompressed = z
        .uncompress_vec(&compressed, 64)
        .expect("uncompress empty");
    assert!(decompressed.is_empty());

    drop(z);
    std::env::remove_var("LSBWRAP_LIBZ_PATH");
    Some(v)
}

#[test]
fn test_nix_zlib_all() {
    common::maybe_reexec_under_nix_glibc();
    if !common::nix_available() {
        eprintln!("Nix not available, skipping");
        return;
    }

    eprintln!("Testing zlib versions:");
    let mut found = 0;
    let mut not_found = 0;
    for glob in ZLIB_GLOBS {
        match test_single_zlib(glob) {
            Some(v) => {
                eprintln!("  [{glob}] version: {v}, compress/uncompress OK");
                found += 1;
            }
            None => {
                eprintln!("  [{glob}] not found in store, skipped");
                not_found += 1;
            }
        }
    }
    assert!(found > 0, "at least one zlib version must be found in Nix store");
    if not_found > 0 {
        eprintln!("  ({not_found} versions not found in store)");
    }
}

// ── OpenSSL latest (3.6) ─────────────────────────────────────────────────

#[test]
fn test_nix_openssl_latest() {
    common::maybe_reexec_under_nix_glibc();
    if !common::nix_available() {
        eprintln!("Nix not available, skipping");
        return;
    }

    let (libssl, libcrypto) = common::find_openssl_glob(OPENSSL_GLOB)
        .expect("OpenSSL 3.6 not found in Nix store");

    eprintln!("OpenSSL 3.6:");
    eprintln!("  libssl:   {}", libssl.display());
    eprintln!("  libcrypto: {}", libcrypto.display());

    std::env::set_var("LSBWRAP_LIBSSL_PATH", &libssl);
    std::env::set_var("LSBWRAP_LIBCRYPTO_PATH", &libcrypto);

    let ssl = match lsb_openssl::Openssl::load() {
        Ok(s) => {
            eprintln!("  variant: {:?}, version: {:?}", s.variant, s.version());
            s
        }
        Err(e) if common::is_glibc_mismatch(&e) => {
            eprintln!("  SKIP (glibc mismatch): {e}");
            std::env::remove_var("LSBWRAP_LIBSSL_PATH");
            std::env::remove_var("LSBWRAP_LIBCRYPTO_PATH");
            return;
        }
        Err(e) => panic!("OpenSSL 3.6 load failed: {e}"),
    };

    ssl.init().expect("init");
    let ctx = ssl.ctx_new(true).expect("client ctx");
    drop(ctx);
    let ctx = ssl.ctx_new(false).expect("server ctx");
    drop(ctx);
    eprintln!("  ctx_new OK");

    common::test_openssl_load_cert(&ssl, "OpenSSL 3.6");
    common::test_openssl_handshake(&ssl, "OpenSSL 3.6");

    drop(ssl);
    std::env::remove_var("LSBWRAP_LIBSSL_PATH");
    std::env::remove_var("LSBWRAP_LIBCRYPTO_PATH");
}
