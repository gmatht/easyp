fn main() {
    println!("cargo::rustc-check-cfg=cfg(squeeze_chroot_exists)");

    let marker = std::path::Path::new("/tmp/squeeze-chroot/.chroot-ready");

    if std::env::var("SQUEEZE_CHROOT_READY").is_ok() {
        println!("cargo:rustc-cfg=squeeze_chroot_exists");
    } else if marker.exists() {
        println!("cargo:rustc-cfg=squeeze_chroot_exists");
    }

    // Always watch the env var so the user can toggle with
    //   SQUEEZE_CHROOT_READY=1 cargo test -- squeeze
    // even after a previous build without the env var.
    println!("cargo:rerun-if-env-changed=SQUEEZE_CHROOT_READY");
}
