//! Test OpenSSL 1.1 from the Nix store.
//!
//! Requires a separate test binary because OpenSSL 1.1 has a different
//! soname (libssl.so.1.1) and must not be loaded alongside 3.x versions.

mod common;

#[test]
fn test_nix_openssl_11() {
    common::maybe_reexec_under_nix_glibc();
    if !common::nix_available() {
        eprintln!("Nix not available, skipping");
        return;
    }

    let (libssl, libcrypto) = match common::find_openssl_glob("*-openssl-1.1.*") {
        Some(p) => p,
        None => {
            eprintln!("OpenSSL 1.1 not found in Nix store, skipping");
            return;
        }
    };

    eprintln!("OpenSSL 1.1:");
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
        Err(e) => panic!("OpenSSL 1.1 load failed: {e}"),
    };

    ssl.init().expect("init");
    let ctx = ssl.ctx_new(true).expect("client ctx");
    drop(ctx);
    let ctx = ssl.ctx_new(false).expect("server ctx");
    drop(ctx);
    eprintln!("  ctx_new OK");

    common::test_openssl_load_cert(&ssl, "OpenSSL 1.1");
    common::test_openssl_handshake(&ssl, "OpenSSL 1.1");

    drop(ssl);
    std::env::remove_var("LSBWRAP_LIBSSL_PATH");
    std::env::remove_var("LSBWRAP_LIBCRYPTO_PATH");
}
