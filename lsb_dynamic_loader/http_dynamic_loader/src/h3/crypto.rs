use crate::HttpError;
use lsb_loader::LoadedLibrary;
use std::os::raw::c_int;

pub struct GnutlsCrypto {
    pub gnutls_lib: LoadedLibrary,
    pub crypto_lib: LoadedLibrary,
}

unsafe impl Send for GnutlsCrypto {}

impl GnutlsCrypto {
    pub fn load() -> Result<Self, HttpError> {
        let crypto_lib = LoadedLibrary::load_from_candidates(
            &["libngtcp2_crypto_gnutls.so.2", "libngtcp2_crypto_gnutls.so"],
            &[
                "ngtcp2_crypto_gnutls_configure_server_session",
                "ngtcp2_crypto_recv_client_initial_cb",
                "ngtcp2_crypto_recv_crypto_data_cb",
                "ngtcp2_crypto_encrypt_cb",
                "ngtcp2_crypto_decrypt_cb",
                "ngtcp2_crypto_hp_mask_cb",
                "ngtcp2_crypto_update_key_cb",
                "ngtcp2_crypto_delete_crypto_aead_ctx_cb",
                "ngtcp2_crypto_delete_crypto_cipher_ctx_cb",
                "ngtcp2_crypto_get_path_challenge_data_cb",
                "ngtcp2_crypto_version_negotiation_cb",
                "ngtcp2_crypto_read_write_crypto_data",
            ],
        )?;

        let gnutls_lib = LoadedLibrary::load_from_candidates(
            &["libgnutls.so.30", "libgnutls.so"],
            &[
                "gnutls_init",
                "gnutls_deinit",
                "gnutls_credentials_set",
                "gnutls_certificate_allocate_credentials",
                "gnutls_certificate_free_credentials",
                "gnutls_certificate_set_x509_key_file2",
                "gnutls_handshake",
                "gnutls_session_set_ptr",
                "gnutls_session_get_ptr",
                "gnutls_transport_set_int2",
                "gnutls_record_recv_seq",
                "gnutls_record_send",
                "gnutls_error_is_fatal",
                "gnutls_perror",
                "gnutls_alert_send",
            ],
        )?;

        Ok(GnutlsCrypto { crypto_lib, gnutls_lib })
    }

    pub fn get_cb<T>(&self, name: &[u8]) -> Option<T> {
        self.crypto_lib.get_symbol_raw(name).ok()
            .map(|p| unsafe { std::mem::transmute::<_, T>(p) })
    }
}
