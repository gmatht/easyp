//! Test OpenSSL 3.0 from the Nix store.
//!
//! Requires a separate test binary because different 3.x versions share
//! soname libssl.so.3 / libcrypto.so.3 and conflict in the same process.

mod common;

#[test]
fn test_nix_openssl_30() {
    common::maybe_reexec_under_nix_glibc();
    if !common::nix_available() {
        eprintln!("Nix not available, skipping");
        return;
    }

    let (libssl, libcrypto) = match common::find_openssl_glob("*-openssl-3.0.*") {
        Some(p) => p,
        None => {
            eprintln!("OpenSSL 3.0 not found in Nix store, skipping");
            return;
        }
    };

    eprintln!("OpenSSL 3.0:");
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
        Err(e) => panic!("OpenSSL 3.0 load failed: {e}"),
    };

    ssl.init().expect("init");
    let ctx = ssl.ctx_new(true).expect("client ctx");
    drop(ctx);
    let ctx = ssl.ctx_new(false).expect("server ctx");
    drop(ctx);
    eprintln!("  ctx_new OK");

    common::test_openssl_load_cert(&ssl, "OpenSSL 3.0");
    common::test_openssl_handshake(&ssl, "OpenSSL 3.0");

    drop(ssl);
    std::env::remove_var("LSBWRAP_LIBSSL_PATH");
    std::env::remove_var("LSBWRAP_LIBCRYPTO_PATH");
}
