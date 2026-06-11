use crate::HttpError;
use lsb_loader::LoadedLibrary;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use super::callbacks::*;

#[repr(C)]
pub struct GnutlsDatum {
    pub data: *mut u8,
    pub size: c_uint,
}

pub struct GnutlsCrypto {
    pub gnutls_lib: LoadedLibrary,
    pub crypto_lib: LoadedLibrary,
    // ngtcp2 crypto callbacks (from libngtcp2_crypto_gnutls)
    // Types are already Option<fnptr> from bindgen
    pub recv_client_initial_cb: Ngtcp2RecvClientInitial,
    pub recv_crypto_data_cb: Ngtcp2RecvCryptoData,
    pub encrypt_cb: Ngtcp2Encrypt,
    pub decrypt_cb: Ngtcp2Decrypt,
    pub hp_mask_cb: Ngtcp2HpMask,
    pub update_key_cb: Ngtcp2UpdateKey,
    pub delete_crypto_aead_ctx_cb: Ngtcp2DeleteCryptoAeadCtx,
    pub delete_crypto_cipher_ctx_cb: Ngtcp2DeleteCryptoCipherCtx,
    pub get_path_challenge_data_cb: Ngtcp2GetPathChallengeData,
    pub version_negotiation_cb: Ngtcp2VersionNegotiation,
    // GnuTLS session management
    pub gnutls_init: unsafe extern "C" fn(*mut *mut c_void, u32) -> c_int,
    pub gnutls_session_ticket_key_generate: unsafe extern "C" fn(*mut GnutlsDatum) -> c_int,
    pub gnutls_session_ticket_enable_server: unsafe extern "C" fn(*mut c_void, *const GnutlsDatum) -> c_int,
    pub gnutls_session_ticket_key: GnutlsDatum,
    pub gnutls_anti_replay_init: unsafe extern "C" fn(*mut *mut c_void) -> c_int,
    pub gnutls_anti_replay_enable: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub gnutls_record_set_max_early_data_size: unsafe extern "C" fn(*mut c_void, usize) -> c_int,
    pub anti_replay: *mut c_void,
    pub gnutls_deinit: unsafe extern "C" fn(*mut c_void),
    pub gnutls_credentials_set: unsafe extern "C" fn(*mut c_void, u32, *mut c_void) -> c_int,
    pub gnutls_certificate_allocate_credentials: unsafe extern "C" fn(*mut *mut c_void) -> c_int,
    pub gnutls_certificate_free_credentials: unsafe extern "C" fn(*mut c_void),
    pub gnutls_certificate_set_x509_key_file: unsafe extern "C" fn(*mut c_void, *const u8, *const u8, u32) -> c_int,
    pub gnutls_certificate_set_x509_system_trust: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub gnutls_handshake: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub gnutls_session_set_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub gnutls_session_get_ptr: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    pub gnutls_transport_set_int2: unsafe extern "C" fn(*mut c_void, i32, i32),
    pub gnutls_record_recv_seq: unsafe extern "C" fn(*mut c_void, *mut u8, usize, *mut u8) -> isize,
    pub gnutls_record_send: unsafe extern "C" fn(*mut c_void, *const u8, usize) -> isize,
    pub gnutls_error_is_fatal: unsafe extern "C" fn(c_int) -> c_int,
    pub gnutls_perror: unsafe extern "C" fn(c_int),
    pub gnutls_alert_send: unsafe extern "C" fn(*mut c_void, u8, u8) -> c_int,
    pub read_write_crypto_data: unsafe extern "C" fn(*mut c_void, u32, *const u8, usize) -> c_int,
    pub configure_server_session: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub crypto_ctx_tls: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    pub gnutls_alpn_set_protocols: unsafe extern "C" fn(*mut c_void, *const GnutlsDatum, c_uint, c_uint) -> c_int,
    pub gnutls_handshake_set_private_extensions: unsafe extern "C" fn(*mut c_void, c_int) -> (),
    pub gnutls_handshake_write: unsafe extern "C" fn(*mut c_void, u32, *const u8, usize) -> c_int,
    pub gnutls_alert_send_appropriate: unsafe extern "C" fn(*mut c_void, c_int) -> c_int,
    pub from_ngtcp2_level: unsafe extern "C" fn(u32) -> u32,
    pub gnutls_strerror: unsafe extern "C" fn(c_int) -> *const c_char,
    pub gnutls_priority_set_direct: unsafe extern "C" fn(*mut c_void, *const c_char, *mut *const c_char) -> c_int,
    pub gnutls_handshake_set_hook_function: unsafe extern "C" fn(*mut c_void, c_uint, c_uint, Option<unsafe extern "C" fn(*mut c_void, c_uint, c_uint, c_uint, *const GnutlsDatum) -> c_int>),
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
                "ngtcp2_crypto_gnutls_configure_server_session",
                "ngtcp2_crypto_ctx_tls",
                "ngtcp2_crypto_gnutls_from_ngtcp2_level",
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
                "gnutls_certificate_set_x509_key_file",
                "gnutls_certificate_set_x509_system_trust",
                "gnutls_handshake",
                "gnutls_handshake_write",
                "gnutls_alert_send_appropriate",
                "gnutls_session_set_ptr",
                "gnutls_session_get_ptr",
                "gnutls_transport_set_int2",
                "gnutls_record_recv_seq",
                "gnutls_record_send",
                "gnutls_error_is_fatal",
                "gnutls_perror",
                "gnutls_alert_send",
                "gnutls_alpn_set_protocols",
                "gnutls_handshake_set_private_extensions",
                "gnutls_session_ticket_enable_server",
                "gnutls_session_ticket_key_generate",
                "gnutls_anti_replay_init",
                "gnutls_anti_replay_enable",
                "gnutls_record_set_max_early_data_size",
                "gnutls_handshake_set_hook_function",
            ],
        )?;

        unsafe {
            let recv_client_initial_cb: Ngtcp2RecvClientInitial =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_recv_client_initial_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let recv_crypto_data_cb: Ngtcp2RecvCryptoData =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_recv_crypto_data_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let encrypt_cb: Ngtcp2Encrypt =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_encrypt_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let decrypt_cb: Ngtcp2Decrypt =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_decrypt_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let hp_mask_cb: Ngtcp2HpMask =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_hp_mask_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let update_key_cb: Ngtcp2UpdateKey =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_update_key_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let delete_crypto_aead_ctx_cb: Ngtcp2DeleteCryptoAeadCtx =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_delete_crypto_aead_ctx_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let delete_crypto_cipher_ctx_cb: Ngtcp2DeleteCryptoCipherCtx =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_delete_crypto_cipher_ctx_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let get_path_challenge_data_cb: Ngtcp2GetPathChallengeData =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_get_path_challenge_data_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let version_negotiation_cb: Ngtcp2VersionNegotiation =
                crypto_lib.get_symbol_raw("ngtcp2_crypto_version_negotiation_cb").ok().map(|p| std::mem::transmute(p)).flatten();
            let read_write_crypto_data =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, u32, *const u8, usize) -> c_int>(
                    crypto_lib.get_symbol_raw("ngtcp2_crypto_read_write_crypto_data")?);
            let configure_server_session =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void) -> c_int>(
                    crypto_lib.get_symbol_raw("ngtcp2_crypto_gnutls_configure_server_session")?);
            let crypto_ctx_tls =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void>(
                    crypto_lib.get_symbol_raw("ngtcp2_crypto_ctx_tls")?);
            let from_ngtcp2_level =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(u32) -> u32>(
                    crypto_lib.get_symbol_raw("ngtcp2_crypto_gnutls_from_ngtcp2_level")?);

            let gnutls_init =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut *mut c_void, u32) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_init")?);
            let gnutls_deinit =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void)>(
                    gnutls_lib.get_symbol_raw("gnutls_deinit")?);
            let gnutls_credentials_set =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, u32, *mut c_void) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_credentials_set")?);
            let gnutls_certificate_allocate_credentials =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut *mut c_void) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_certificate_allocate_credentials")?);
            let gnutls_certificate_free_credentials =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void)>(
                    gnutls_lib.get_symbol_raw("gnutls_certificate_free_credentials")?);
            let gnutls_certificate_set_x509_key_file =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *const u8, *const u8, u32) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_certificate_set_x509_key_file")?);
            let gnutls_certificate_set_x509_system_trust =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_certificate_set_x509_system_trust")?);
            let gnutls_handshake =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_handshake")?);
            let gnutls_session_set_ptr =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *mut c_void)>(
                    gnutls_lib.get_symbol_raw("gnutls_session_set_ptr")?);
            let gnutls_session_get_ptr =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void) -> *mut c_void>(
                    gnutls_lib.get_symbol_raw("gnutls_session_get_ptr")?);
            let gnutls_transport_set_int2 =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, i32, i32)>(
                    gnutls_lib.get_symbol_raw("gnutls_transport_set_int2")?);
            let gnutls_record_recv_seq =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *mut u8, usize, *mut u8) -> isize>(
                    gnutls_lib.get_symbol_raw("gnutls_record_recv_seq")?);
            let gnutls_record_send =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *const u8, usize) -> isize>(
                    gnutls_lib.get_symbol_raw("gnutls_record_send")?);
            let gnutls_error_is_fatal =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(c_int) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_error_is_fatal")?);
            let gnutls_perror =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(c_int)>(
                    gnutls_lib.get_symbol_raw("gnutls_perror")?);
            let gnutls_alert_send =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, u8, u8) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_alert_send")?);
            let gnutls_alpn_set_protocols =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *const GnutlsDatum, c_uint, c_uint) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_alpn_set_protocols")?);
            let gnutls_handshake_set_private_extensions =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, c_int)>(
                    gnutls_lib.get_symbol_raw("gnutls_handshake_set_private_extensions")?);
            let gnutls_handshake_write =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, u32, *const u8, usize) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_handshake_write")?);
            let gnutls_alert_send_appropriate =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, c_int) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_alert_send_appropriate")?);
            let gnutls_strerror =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(c_int) -> *const c_char>(
                    gnutls_lib.get_symbol_raw("gnutls_strerror")?);
            let gnutls_priority_set_direct =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *const c_char, *mut *const c_char) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_priority_set_direct")?);
            let gnutls_handshake_set_hook_function =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, c_uint, c_uint, Option<unsafe extern "C" fn(*mut c_void, c_uint, c_uint, c_uint, *const GnutlsDatum) -> c_int>)>(
                    gnutls_lib.get_symbol_raw("gnutls_handshake_set_hook_function")?);
            let gnutls_session_ticket_key_generate =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut GnutlsDatum) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_session_ticket_key_generate")?);
            let gnutls_session_ticket_enable_server =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *const GnutlsDatum) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_session_ticket_enable_server")?);
            let gnutls_anti_replay_init =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut *mut c_void) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_anti_replay_init")?);
            let gnutls_anti_replay_enable =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, *mut c_void)>(
                    gnutls_lib.get_symbol_raw("gnutls_anti_replay_enable")?);
            let gnutls_record_set_max_early_data_size =
                std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void, usize) -> c_int>(
                    gnutls_lib.get_symbol_raw("gnutls_record_set_max_early_data_size")?);

            let mut result = GnutlsCrypto {
                gnutls_lib,
                crypto_lib,
                recv_client_initial_cb,
                recv_crypto_data_cb,
                encrypt_cb,
                decrypt_cb,
                hp_mask_cb,
                update_key_cb,
                delete_crypto_aead_ctx_cb,
                delete_crypto_cipher_ctx_cb,
                get_path_challenge_data_cb,
                version_negotiation_cb,
                gnutls_init,
                gnutls_deinit,
                gnutls_credentials_set,
                gnutls_certificate_allocate_credentials,
                gnutls_certificate_free_credentials,
                gnutls_certificate_set_x509_key_file,
                gnutls_certificate_set_x509_system_trust,
                gnutls_handshake,
                gnutls_session_set_ptr,
                gnutls_session_get_ptr,
                gnutls_transport_set_int2,
                gnutls_record_recv_seq,
                gnutls_record_send,
                gnutls_error_is_fatal,
                gnutls_perror,
                gnutls_alert_send,
                read_write_crypto_data,
                configure_server_session,
                crypto_ctx_tls,
                gnutls_alpn_set_protocols,
                gnutls_handshake_set_private_extensions,
                gnutls_handshake_write,
                gnutls_alert_send_appropriate,
                from_ngtcp2_level,
                gnutls_strerror,
                gnutls_priority_set_direct,
                gnutls_handshake_set_hook_function,
                gnutls_session_ticket_key_generate,
                gnutls_session_ticket_enable_server,
                gnutls_session_ticket_key: GnutlsDatum {
                    data: std::ptr::null_mut(),
                    size: 0,
                },
                gnutls_anti_replay_init,
                gnutls_anti_replay_enable,
                gnutls_record_set_max_early_data_size,
                anti_replay: std::ptr::null_mut(),
            };

            let r = (result.gnutls_session_ticket_key_generate)(&mut result.gnutls_session_ticket_key);
            if r != 0 {
                return Err(HttpError::Protocol(format!("gnutls_session_ticket_key_generate: {}", r)));
            }

            let r = (result.gnutls_anti_replay_init)(&mut result.anti_replay);
            if r != 0 {
                return Err(HttpError::Protocol(format!("gnutls_anti_replay_init: {}", r)));
            }

            Ok(result)
        }
    }
}
