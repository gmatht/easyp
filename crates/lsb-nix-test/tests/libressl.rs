//! Test all LibreSSL versions from the Nix store.
//!
//! Unlike OpenSSL 3.x, different LibreSSL versions have unique sonames
//! (libssl.so.59 vs 60, libcrypto.so.55 vs 56), so they can be tested
//! safely in the same process.

mod common;

const LIBRESSL_GLOBS: &[&str] = &[
    "*-libressl-4.1.*",
    "*-libressl-4.2.*",
];

fn test_single_libressl(glob: &str) -> Option<String> {
    let (libssl, libcrypto) = common::find_libressl_glob(glob)?;

    std::env::set_var("LSBWRAP_LIBSSL_PATH", &libssl);
    std::env::set_var("LSBWRAP_LIBCRYPTO_PATH", &libcrypto);

    let ssl = match lsb_openssl::Openssl::load() {
        Ok(s) => s,
        Err(e) if common::is_glibc_mismatch(&e) => {
            std::env::remove_var("LSBWRAP_LIBSSL_PATH");
            std::env::remove_var("LSBWRAP_LIBCRYPTO_PATH");
            return None;
        }
        Err(e) => panic!("LibreSSL ({glob}) load failed: {e}"),
    };

    let ver = ssl.version().unwrap_or_default().to_string();
    ssl.init().expect("init");
    let ctx = ssl.ctx_new(true).expect("client ctx");
    drop(ctx);

    assert!(
        ver.contains("LibreSSL") || ver.contains("libressl"),
        "expected LibreSSL, got: {ver}"
    );

    common::test_openssl_load_cert(&ssl, &format!("LibreSSL ({glob})"));
    common::test_openssl_handshake(&ssl, &format!("LibreSSL ({glob})"));

    drop(ssl);
    std::env::remove_var("LSBWRAP_LIBSSL_PATH");
    std::env::remove_var("LSBWRAP_LIBCRYPTO_PATH");
    Some(ver)
}

#[test]
fn test_nix_libressl_all() {
    common::maybe_reexec_under_nix_glibc();
    if !common::nix_available() {
        eprintln!("Nix not available, skipping");
        return;
    }

    eprintln!("Testing LibreSSL versions:");
    let mut found = 0;
    let mut skipped = 0;
    for glob in LIBRESSL_GLOBS {
        match test_single_libressl(glob) {
            Some(ver) => {
                eprintln!("  [{glob}] version: {ver}, OK");
                found += 1;
            }
            None => {
                skipped += 1;
            }
        }
    }
    assert!(found > 0, "at least one LibreSSL version must be found");
    if skipped > 0 {
        eprintln!("  ({skipped} versions not found or skipped)");
    }
}
