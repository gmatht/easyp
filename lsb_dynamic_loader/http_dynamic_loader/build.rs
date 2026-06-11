fn main() {
    // Only generate ngtcp2 bindings when the h3 feature is enabled
    #[cfg(feature = "h3")]
    {
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=/usr/include/ngtcp2/ngtcp2.h");

        let bindings = bindgen::Builder::default()
            .header("/usr/include/ngtcp2/ngtcp2.h")
            .allowlist_type("ngtcp2_callbacks")
            .allowlist_type("ngtcp2_settings")
            .allowlist_type("ngtcp2_transport_params")
            .allowlist_type("ngtcp2_cid")
            .allowlist_type("ngtcp2_vec")
            .allowlist_type("ngtcp2_path")
            .allowlist_type("ngtcp2_addr")
            .allowlist_type("ngtcp2_pkt_hd")
            .allowlist_type("ngtcp2_crypto_aead")
            .allowlist_type("ngtcp2_crypto_aead_ctx")
            .allowlist_type("ngtcp2_crypto_cipher")
            .allowlist_type("ngtcp2_crypto_cipher_ctx")
            .allowlist_type("ngtcp2_crypto_ctx")
            .allowlist_type("ngtcp2_preferred_addr")
            .allowlist_type("ngtcp2_transport_params_type")
            .allowlist_type("ngtcp2_rand_ctx")
            .allowlist_type("ngtcp2_mem")
            .allowlist_type("ngtcp2_connection_close_error")
            .allowlist_type("ngtcp2_connection_close_error_code_type")
            .allowlist_type("ngtcp2_version_cid")
            .allowlist_type("sockaddr_in")
            .allowlist_type("sockaddr_in6")
            .allowlist_type("sockaddr")
            .allowlist_type("sockaddr_storage")
            .allowlist_var("NGTCP2_MAX_CIDLEN")
            .derive_default(true)
            .layout_tests(true)
            .generate()
            .expect("Unable to generate ngtcp2 bindings");

        bindings
            .write_to_file("src/ngtcp2_ffi.rs")
            .expect("Couldn't write ngtcp2 bindings");
    }
}
