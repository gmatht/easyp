// Functions in this module are used by different test binaries; each binary
// compiles its own copy, so not every function is used in every binary.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

const SENTINEL: &str = "LSBWRAP_NIX_GLIBC";

// ── Availability ──────────────────────────────────────────────────────────

pub fn nix_available() -> bool {
    if !std::path::Path::new("/nix/store").is_dir() {
        return false;
    }
    Command::new("which")
        .arg("nix-build")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── Store glob helpers ───────────────────────────────────────────────────

/// Resolve a store glob pattern to the latest matching path.
/// Pattern examples: "*-openssl-3.0.*", "*-zlib-*"
pub fn resolve_store_latest(glob: &str) -> Option<PathBuf> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ls -d /nix/store/{glob}/lib/libssl.so.* /nix/store/{glob}/lib/libz.so.1 2>/dev/null | sort -V | tail -1"
        ))
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

/// Resolve a store glob pattern returning the store directory.
pub fn resolve_store_dir(glob: &str) -> Option<PathBuf> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("ls -d /nix/store/{glob}/ 2>/dev/null | sort -V | tail -1"))
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s.trim_end_matches('/')))
    }
}

/// Resolve all matching store paths for a glob, sorted by version.
pub fn resolve_store_all(glob: &str) -> Vec<PathBuf> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ls -d /nix/store/{glob}/ 2>/dev/null | sort -V"
        ))
        .output()
        .ok();
    match out {
        Some(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| PathBuf::from(l.trim_end_matches('/')))
            .collect(),
        None => vec![],
    }
}

/// Build a nixpkgs attribute and return its store directory.
pub fn build_nix_attr(attr: &str) -> Option<PathBuf> {
    let out = Command::new("nix-build")
        .args([
            "--no-out-link",
            "-E",
            &format!("with import <nixpkgs> {{}}; {}.out", attr),
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(dir))
    }
}

// ── OpenSSL path resolution ───────────────────────────────────────────────

pub fn find_openssl_paths(lib_dir: &PathBuf) -> Option<(PathBuf, PathBuf)> {
    let lib = lib_dir.join("lib");
    let ssl_glob = format!("{}", lib.join("libssl.so.*").display());
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("ls {ssl_glob} 2>/dev/null | sort -V | tail -1"))
        .output()
        .ok()?;
    let ssl = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if ssl.is_empty() {
        return None;
    }
    let ssl_path = PathBuf::from(&ssl);
    let crypto = ssl_path
        .parent()?
        .join(ssl_path.file_name()?.to_string_lossy().replace("libssl", "libcrypto"));
    if !crypto.exists() {
        return None;
    }
    Some((ssl_path, crypto))
}

/// Find OpenSSL paths from a store glob like "*-openssl-3.0.*".
pub fn find_openssl_glob(glob: &str) -> Option<(PathBuf, PathBuf)> {
    let dir = resolve_store_dir(glob)?;
    find_openssl_paths(&dir)
}

/// Find LibreSSL paths from a store directory.
pub fn find_libressl_paths(lib_dir: &PathBuf) -> Option<(PathBuf, PathBuf)> {
    // LibreSSL has libssl.so as a symlink; resolve it to the real soname.
    fn resolve_soname(dir: &PathBuf, name: &str) -> Option<PathBuf> {
        let link = dir.join("lib").join(name);
        if !link.exists() {
            return None;
        }
        let real = std::fs::read_link(&link).unwrap_or_else(|_| link.clone());
        let real = if real.is_relative() {
            link.parent()?.join(&real)
        } else {
            real
        };
        if real.exists() {
            Some(real)
        } else {
            None
        }
    }
    let ssl = resolve_soname(lib_dir, "libssl.so")?;
    let crypto = resolve_soname(lib_dir, "libcrypto.so")?;
    Some((ssl, crypto))
}

pub fn find_libressl_glob(glob: &str) -> Option<(PathBuf, PathBuf)> {
    let dir = resolve_store_dir(glob)?;
    find_libressl_paths(&dir)
}

/// Find zlib path from a store directory.
pub fn find_zlib_path(lib_dir: &PathBuf) -> Option<PathBuf> {
    let z = lib_dir.join("lib/libz.so.1");
    if z.exists() {
        Some(z)
    } else {
        None
    }
}

pub fn find_zlib_glob(glob: &str) -> Option<PathBuf> {
    let dir = resolve_store_dir(glob)?;
    find_zlib_path(&dir)
}

// ── glibc re-exec ─────────────────────────────────────────────────────────

fn find_nix_glibc_ld_so() -> Option<PathBuf> {
    let out = Command::new("sh")
        .arg("-c")
        .arg("ls -d /nix/store/*-glibc-*/lib/ld-linux-x86-64.so.2 2>/dev/null | sort -V | tail -1")
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

fn parse_glibc_ver(name: &str) -> Option<(u32, u32)> {
    let rest = name.split("glibc-").nth(1)?;
    let mut parts = rest.splitn(2, '.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts
        .next()?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((major, minor))
}

fn nix_glibc_newer_than_system() -> Option<PathBuf> {
    let ld_so = find_nix_glibc_ld_so()?;
    let nix_glibc_dir = ld_so.parent()?.parent()?;
    let nix_name = nix_glibc_dir.file_name()?.to_string_lossy();
    let nix_ver = parse_glibc_ver(&nix_name)?;

    let sys_out = Command::new("ldd").arg("--version").output().ok()?;
    let sys_str = String::from_utf8_lossy(&sys_out.stdout);
    let sys_ver_str = sys_str.lines().next()?;
    let sys_ver = sys_ver_str.rsplit_once(' ')?;
    let sys_ver_str = sys_ver.1.trim();
    let mut parts = sys_ver_str.splitn(2, '.');
    let sys_major: u32 = parts.next()?.parse().ok()?;
    let sys_minor: u32 = parts
        .next()?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;

    if (nix_ver.0 > sys_major) || (nix_ver.0 == sys_major && nix_ver.1 > sys_minor) {
        eprintln!(
            "Nix glibc {}.{} > system glibc {}.{} — will re-exec under Nix ld.so",
            nix_ver.0, nix_ver.1, sys_major, sys_minor,
        );
        Some(ld_so)
    } else {
        None
    }
}

pub fn maybe_reexec_under_nix_glibc() {
    if std::env::var(SENTINEL).is_ok() {
        return;
    }
    static REEXEC: std::sync::Once = std::sync::Once::new();
    REEXEC.call_once(|| {
        let ld_so = match nix_glibc_newer_than_system() {
            Some(l) => l,
            None => return,
        };
        let glibc_lib = ld_so.parent().expect("ld.so has parent");

        // Collect all Nix lib directories for re-exec library path.
        let mut lib_dirs = vec![glibc_lib.to_string_lossy().to_string()];

        // Add known store paths for all libraries that might be tested.
        for glob in &[
            "*-zlib-*",
            "*-openssl-*",
            "*-libressl-*",
        ] {
            for dir in resolve_store_all(glob) {
                let lib = dir.join("lib");
                if lib.exists() {
                    lib_dirs.push(lib.to_string_lossy().to_string());
                }
            }
        }

        lib_dirs.push("/lib".into());
        lib_dirs.push("/lib64".into());
        lib_dirs.push("/usr/lib".into());
        lib_dirs.push("/usr/lib/x86_64-linux-gnu".into());

        // Remove duplicates while preserving order.
        let mut seen = std::collections::HashSet::new();
        lib_dirs.retain(|d| seen.insert(d.clone()));

        let lib_path = lib_dirs.join(":");
        let exe = std::env::current_exe().expect("get exe path");
        let args: Vec<String> = std::env::args().collect();

        eprintln!("Re-exec'ing under Nix ld.so: {}", ld_so.display());
        let status = Command::new(&ld_so)
            .arg("--library-path")
            .arg(&lib_path)
            .arg(&exe)
            .env(SENTINEL, "1")
            .args(&args[1..])
            .status()
            .expect("re-exec under Nix glibc");

        std::process::exit(status.code().unwrap_or(1));
    });
}

// ── Glibc mismatch detection ──────────────────────────────────────────────

pub fn is_glibc_mismatch<T: std::fmt::Debug>(e: &T) -> bool {
    let msg = format!("{:?}", e);
    msg.contains("GLIBC") || msg.contains("version")
}

// ── OpenSSL integration test helpers ──────────────────────────────────────

const CERT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/resources/cert.pem"
);
const KEY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/resources/key.pem"
);

/// Load cert.pem and key.pem into a server context.
pub fn test_openssl_load_cert(ssl: &lsb_openssl::Openssl, label: &str) {
    let ctx = ssl.ctx_new(false).expect("server ctx");
    ctx.load_cert_file(CERT).expect("load cert");
    ctx.load_key_file(KEY).expect("load key");
    eprintln!("  {label}: cert/key loaded OK");
}

/// Full TLS handshake over a Unix socket pair.
pub fn test_openssl_handshake(ssl: &lsb_openssl::Openssl, label: &str) {
    use std::os::unix::io::AsRawFd;
    use std::os::unix::net::UnixStream;

    let (client_stream, server_stream) = UnixStream::pair().expect("socketpair");
    let (cfd, sfd) = (client_stream.as_raw_fd(), server_stream.as_raw_fd());

    let server_ctx = ssl.ctx_new(false).expect("server ctx");
    server_ctx.load_cert_file(CERT).expect("load cert");
    server_ctx.load_key_file(KEY).expect("load key");

    let client_ctx = ssl.ctx_new(true).expect("client ctx");

    let client_conn = ssl
        .ssl_new_from_fd(&client_ctx, cfd)
        .expect("client conn");
    let server_conn = ssl
        .ssl_new_from_fd(&server_ctx, sfd)
        .expect("server conn");

    let client_handle = std::thread::spawn(move || {
        client_conn.connect()?;
        let n = client_conn.write(b"hello from client")?;
        assert_eq!(n, 17);
        let mut buf = vec![0u8; 1024];
        let n = client_conn.read(&mut buf)?;
        assert_eq!(&buf[..n], b"pong");
        client_conn.shutdown()?;
        Ok::<_, lsb_openssl::SslError>(())
    });

    let server_handle = std::thread::spawn(move || {
        server_conn.accept()?;
        let mut buf = vec![0u8; 1024];
        let n = server_conn.read(&mut buf)?;
        assert_eq!(&buf[..n], b"hello from client");
        let n = server_conn.write(b"pong")?;
        assert_eq!(n, 4);
        server_conn.shutdown()?;
        Ok::<_, lsb_openssl::SslError>(())
    });

    client_handle
        .join()
        .expect("client panicked")
        .expect("client handshake failed");
    server_handle
        .join()
        .expect("server panicked")
        .expect("server handshake failed");
    eprintln!("  {label}: TLS handshake OK");
}
