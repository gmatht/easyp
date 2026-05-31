//! Off-by-default integration test: build a Debian 6 Squeeze chroot, then exercise the
//! wrapper crates against Squeeze's libssl 0.9.8 and libz.
//!
//! The test is `#[ignore]` by default (no chroot exists).  To enable:
//!   SQUEEZE_CHROOT_READY=1 cargo test -- squeeze   # builds chroot, runs test
//! Once `/tmp/squeeze-chroot/.chroot-ready` exists, subsequent `cargo test` invocations
//! pick it up automatically (no env var needed).
//!
//! Environment variable overrides:
//!   SQUEEZE_CHROOT_DIR   – chroot location (default `/tmp/squeeze-chroot`)

use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    eprintln!("+ {} {}", cmd, args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .env("DEBIAN_FRONTEND", "noninteractive")
        .status()
        .map_err(|e| format!("failed to run `{}`: {}", cmd, e))?;
    if !status.success() {
        return Err(format!("`{}` exited with {:?}", cmd, status.code()));
    }
    Ok(())
}

fn chroot_dir() -> PathBuf {
    std::env::var("SQUEEZE_CHROOT_DIR")
        .unwrap_or_else(|_| "/tmp/squeeze-chroot".into())
        .into()
}

fn marker() -> PathBuf {
    chroot_dir().join(".chroot-ready")
}

fn build_chroot(root: &Path) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("squeeze test requires Linux".into());
    }

    // Pre-requisites
    for bin in &["sudo"] {
        if Command::new("which").arg(bin).output().is_err() {
            return Err(format!("`{}` not found – install it first", bin));
        }
    }

    // Strategy: debootstrap --foreign (stage 1) to get the base environment,
    // then manually extract the library packages we need.  Full debootstrap
    // fails because squeeze's ancient dpkg segfaults on modern kernels.
    if Command::new("which").arg("debootstrap").output().is_ok() {
        eprintln!("Running debootstrap --foreign (stage 1) …");
        let _ = run("sudo", &[
            "debootstrap", "--foreign", "--arch=amd64",
            "squeeze",
            root.to_str().unwrap(),
            "http://archive.debian.org/debian",
        ]);
        // Even if this returns non-zero, the cache dir may still have debs.
    } else {
        eprintln!("debootstrap not available, creating minimal staging dir …");
        run("sudo", &["mkdir", "-p",
            root.join("usr/lib").to_str().unwrap(),
            root.join("var/cache/apt/archives").to_str().unwrap(),
        ])?;
    }

    // Download squeeze library packages directly from the archive.
    eprintln!("Downloading squeeze library packages …");
    let cache = root.join("var/cache/apt/archives");
    let pkgs = [
        ("libssl0.9.8", "http://archive.debian.org/debian/pool/main/o/openssl/libssl0.9.8_0.9.8o-4squeeze14_amd64.deb"),
        ("zlib1g", "http://archive.debian.org/debian/pool/main/z/zlib/zlib1g_1.2.3.4.dfsg-3_amd64.deb"),
    ];
    for (_name, url) in &pkgs {
        let fname = url.rsplit_once('/').map(|(_, f)| f).unwrap_or("pkg.deb");
        let dest = cache.join(fname);
        if !dest.exists() {
            let status = Command::new("sudo")
                .args(["wget", "-q", "-O", dest.to_str().unwrap(), url])
                .status()
                .map_err(|e| format!("wget failed: {}", e))?;
            if !status.success() {
                return Err(format!("failed to download {}", url));
            }
        }
    }

    // Extract the debs into the root.
    eprintln!("Extracting library packages …");
    let root_s = root.to_str().unwrap().to_owned();
    let extract_in = |pattern: &str| -> Result<(), String> {
        let find_out = Command::new("find")
            .args([cache.to_str().unwrap(), "-name", pattern, "-type", "f"])
            .output()
            .map_err(|e| format!("find failed: {}", e))?;
        let debs: Vec<PathBuf> = String::from_utf8_lossy(&find_out.stdout)
            .lines()
            .map(|l| PathBuf::from(l.trim()))
            .filter(|p| p.exists())
            .collect();
        for deb in &debs {
            eprintln!("  extracting {} …", deb.file_name().unwrap().to_string_lossy());
            let st = Command::new("sudo")
                .args(["dpkg-deb", "-x", deb.to_str().unwrap(), &root_s])
                .status()
                .map_err(|e| format!("dpkg-deb failed: {}", e))?;
            if !st.success() {
                return Err(format!("dpkg-deb -x {} failed", deb.display()));
            }
        }
        Ok(())
    };
    extract_in("libssl0.9.8*.deb")?;
    extract_in("zlib1g_*.deb")?;

    // Write the ready marker.
    run("sudo", &["touch", marker().to_str().unwrap()])?;
    run("sudo", &["chmod", "-R", "a+rX", root.to_str().unwrap()])?;

    eprintln!("Squeeze libraries staged at {}", root.display());
    Ok(())
}

/// Find `.so` files inside the chroot whose filename contains `pattern`. Returns
/// only regular files or symlinks (not directories).
fn find_libs(root: &Path, pattern: &str) -> Result<Vec<PathBuf>, String> {
    let out = Command::new("find")
        .args([
            root.join("lib").to_str().unwrap(),
            root.join("usr/lib").to_str().unwrap(),
            "-name", &format!("*{}*", pattern),
            "(",
            "-type", "f",
            "-o",
            "-type", "l",
            ")",
        ])
        .output()
        .map_err(|e| format!("find failed: {}", e))?;
    if !out.status.success() {
        return Err(format!("find exited with {:?}", out.status.code()));
    }
    let paths: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| PathBuf::from(l.trim()))
        .filter(|p| p.to_string_lossy().contains(pattern))
        .collect();
    Ok(paths)
}

/// Ensure an env-var override points to a real library inside the chroot.
fn ensure_libs(root: &Path, var: &str, name_hint: &str) -> Result<PathBuf, String> {
    // If the caller already set the env var, honour it.
    if let Ok(v) = std::env::var(var) {
        let p = PathBuf::from(&v);
        if p.exists() {
            return Ok(p);
        }
    }

    let candidates = find_libs(root, name_hint)?;
    // Pick the longest filename → usually the most specific (full version)
    let best = candidates
        .into_iter()
        .max_by_key(|p| p.to_string_lossy().len())
        .ok_or_else(|| format!("no library matching '{}' found in chroot ({})", name_hint, root.display()))?;
    std::env::set_var(var, best.to_str().unwrap());
    Ok(best)
}

// ---------------------------------------------------------------------------
// The actual test
// ---------------------------------------------------------------------------

#[cfg_attr(not(squeeze_chroot_exists), ignore)]
#[test]
fn test_debian_squeeze_libs() {
    let root = chroot_dir();
    let m = marker();

    if !m.exists() {
        // Build script may have cached the cfg from a previous build.
        // If the chroot was removed, rebuild by setting SQUEEZE_CHROOT_READY=1.
        if std::env::var("SQUEEZE_CHROOT_READY").is_err() {
            panic!(
                "Squeeze chroot not found at {}.\n\
                 To build it:   SQUEEZE_CHROOT_READY=1 cargo test -- squeeze\n\
                 To clean cfg:  cargo clean -p lsb-squeeze-test",
                root.display()
            );
        }
        eprintln!("Building squeeze chroot at {} …", root.display());
        build_chroot(&root).expect("building squeeze chroot failed");
    }

    // Locate libraries inside the chroot.
    eprintln!("Locating squeeze libraries in {} …", root.display());
    let libz = ensure_libs(&root, "LSBWRAP_LIBZ_PATH", "libz.so").unwrap();
    let libssl = ensure_libs(&root, "LSBWRAP_LIBSSL_PATH", "libssl.so").unwrap();
    let libcrypto = ensure_libs(&root, "LSBWRAP_LIBCRYPTO_PATH", "libcrypto.so").unwrap();

    eprintln!("  LSBWRAP_LIBZ_PATH    = {}", libz.display());
    eprintln!("  LSBWRAP_LIBSSL_PATH  = {}", libssl.display());
    eprintln!("  LSBWRAP_LIBCRYPTO_PATH = {}", libcrypto.display());

    // ---- zlib ------------------------------------------------------------
    eprintln!("--- Testing zlib from Squeeze ---");
    {
        let z = lsb_zlib::Zlib::load().expect("load squeeze zlib");
        let ver = z.version();
        eprintln!("zlib version: {}", ver);
        assert!(!ver.is_empty(), "zlib version should be non-empty");

        // Small round-trip
        let input = b"Squeeze compression test.";
        let compressed = z.compress_vec(input).expect("compress");
        let decompressed = z.uncompress_vec(&compressed, 4096).expect("uncompress");
        assert_eq!(&decompressed, input, "zlib round-trip");
        eprintln!("  small round-trip OK");

        // Large data (64 KiB)
        let large = vec![0xABu8; 65536];
        let compressed = z.compress_vec(&large).expect("compress large");
        let decompressed = z
            .uncompress_vec(&compressed, large.len() + 65536)
            .expect("uncompress large");
        assert_eq!(large, decompressed, "zlib large round-trip");
        eprintln!("  large (64 KiB) round-trip OK");

        // Empty input
        let compressed = z.compress_vec(b"").expect("compress empty");
        let decompressed = z.uncompress_vec(&compressed, 64).expect("uncompress empty");
        assert!(decompressed.is_empty(), "zlib empty round-trip");
        eprintln!("  empty input OK");
    }

    // ---- OpenSSL ---------------------------------------------------------
    eprintln!("--- Testing OpenSSL from Squeeze ---");
    {
        let ssl = lsb_openssl::Openssl::load().expect("load squeeze openssl");
        eprintln!("  detected variant: {:?}", ssl.variant);
        if let Some(v) = ssl.version() {
            eprintln!("  version string: {}", v);
        }

        // Squeeze ships OpenSSL 0.9.8, so variant must be OpenSSL10 or LibreSSL.
        match ssl.variant {
            lsb_openssl::SslVariant::OpenSSL10 | lsb_openssl::SslVariant::LibreSSL => {}
            other => panic!(
                "expected OpenSSL 0.9.8 (OpenSSL10) on Squeeze, got {:?}",
                other
            ),
        }

        ssl.init().expect("init");

        // Create client & server contexts – this exercises the method
        // selection fallback (SSLv23_{client,method}).
        let _client_ctx = ssl.ctx_new(true).expect("client ctx");
        let _server_ctx = ssl.ctx_new(false).expect("server ctx");
        eprintln!("  client & server contexts created successfully");

        // Load cert/key into a server context.
        let cert = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/resources/cert.pem"
        );
        let key = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/resources/key.pem"
        );
        let cert_ctx = ssl.ctx_new(false).expect("cert ctx");
        cert_ctx.load_cert_file(cert).expect("load cert");
        cert_ctx.load_key_file(key).expect("load key");
        eprintln!("  cert/key loaded OK");

        // Full TLS handshake over a Unix socket pair.
        {
            use std::os::unix::io::AsRawFd;
            use std::os::unix::net::UnixStream;

            let (client_stream, server_stream) =
                UnixStream::pair().expect("socketpair");
            let (cfd, sfd) = (client_stream.as_raw_fd(), server_stream.as_raw_fd());

            let server_ctx = ssl.ctx_new(false).expect("hs server ctx");
            server_ctx.load_cert_file(cert).expect("hs load cert");
            server_ctx.load_key_file(key).expect("hs load key");

            let client_ctx = ssl.ctx_new(true).expect("hs client ctx");

            let client_conn = ssl
                .ssl_new_from_fd(&client_ctx, cfd)
                .expect("client conn");
            let server_conn = ssl
                .ssl_new_from_fd(&server_ctx, sfd)
                .expect("server conn");

            let ch = std::thread::spawn(move || {
                client_conn.connect()?;
                let n = client_conn.write(b"hello from client")?;
                assert_eq!(n, 17);
                let mut buf = vec![0u8; 1024];
                let n = client_conn.read(&mut buf)?;
                assert_eq!(&buf[..n], b"pong");
                client_conn.shutdown()?;
                Ok::<_, lsb_openssl::SslError>(())
            });

            let sh = std::thread::spawn(move || {
                server_conn.accept()?;
                let mut buf = vec![0u8; 1024];
                let n = server_conn.read(&mut buf)?;
                assert_eq!(&buf[..n], b"hello from client");
                let n = server_conn.write(b"pong")?;
                assert_eq!(n, 4);
                server_conn.shutdown()?;
                Ok::<_, lsb_openssl::SslError>(())
            });

            ch.join()
                .expect("client panicked")
                .expect("client handshake failed");
            sh.join()
                .expect("server panicked")
                .expect("server handshake failed");
            eprintln!("  TLS handshake OK");
        }
    }

    eprintln!(
        "--- All Squeeze library tests passed ({} / {} / {}) ---",
        libz.file_name().unwrap().to_string_lossy(),
        libssl.file_name().unwrap().to_string_lossy(),
        libcrypto.file_name().unwrap().to_string_lossy(),
    );
}
