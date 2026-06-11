//! ngtcp2 and nghttp3 type definitions.
//!
//! All `#[repr(C)]` structs are re-exported from bindgen-generated bindings
//! in `crate::ffi` to guarantee ABI compatibility with libngtcp2.
//!
//! Type aliases (e.g. `Ngtcp2Callbacks`, `Ngtcp2Cid`) preserve the camelCase
//! naming used throughout the http3 handler code.

use std::os::raw::{c_int, c_void};
use crate::ffi;

// ── Re-export bindgen-generated structs with camelCase aliases ─

pub use ffi::ngtcp2_callbacks as Ngtcp2Callbacks;
pub use ffi::ngtcp2_cid as Ngtcp2Cid;
pub use ffi::ngtcp2_vec as Ngtcp2Vec;
pub use ffi::ngtcp2_path as Ngtcp2Path;
pub use ffi::ngtcp2_addr as Ngtcp2Addr;
pub use ffi::ngtcp2_pkt_hd as Ngtcp2PktHd;
pub use ffi::ngtcp2_settings as Ngtcp2Settings;
pub use ffi::ngtcp2_transport_params as Ngtcp2TransportParams;
pub use ffi::ngtcp2_mem as Ngtcp2Mem;
pub use ffi::ngtcp2_crypto_aead as Ngtcp2CryptoAead;
pub use ffi::ngtcp2_crypto_aead_ctx as Ngtcp2CryptoAeadCtx;
pub use ffi::ngtcp2_crypto_cipher as Ngtcp2CryptoCipher;
pub use ffi::ngtcp2_crypto_cipher_ctx as Ngtcp2CryptoCipherCtx;
pub use ffi::ngtcp2_crypto_ctx as Ngtcp2CryptoCtx;
pub use ffi::ngtcp2_rand_ctx as Ngtcp2RandCtx;
pub use ffi::ngtcp2_preferred_addr as Ngtcp2PreferredAddr;
pub use ffi::ngtcp2_version_info as Ngtcp2VersionInfo;
pub use ffi::ngtcp2_version_cid as Ngtcp2VersionCid;
pub use ffi::ngtcp2_connection_close_error as Ngtcp2ConnectionCloseError;
pub use ffi::ngtcp2_conn as Ngtcp2Conn;
pub use ffi::ngtcp2_sockaddr as Ngtcp2Sockaddr;
pub use ffi::ngtcp2_socklen as Ngtcp2Socklen;

// ── Bindgen generates callback types as `Option<fnptr>`. ──────
// We re-export them directly.  The `Option` wrapper means None = NULL fnptr,
// which is what ngtcp2 expects for optional callbacks.
pub type Ngtcp2ClientInitial = ffi::ngtcp2_client_initial;
pub type Ngtcp2RecvClientInitial = ffi::ngtcp2_recv_client_initial;
pub type Ngtcp2RecvCryptoData = ffi::ngtcp2_recv_crypto_data;
pub type Ngtcp2HandshakeCompleted = ffi::ngtcp2_handshake_completed;
pub type Ngtcp2RecvVersionNegotiation = ffi::ngtcp2_recv_version_negotiation;
pub type Ngtcp2Encrypt = ffi::ngtcp2_encrypt;
pub type Ngtcp2Decrypt = ffi::ngtcp2_decrypt;
pub type Ngtcp2HpMask = ffi::ngtcp2_hp_mask;
pub type Ngtcp2RecvStreamData = ffi::ngtcp2_recv_stream_data;
pub type Ngtcp2AckedStreamDataOffset = ffi::ngtcp2_acked_stream_data_offset;
pub type Ngtcp2StreamOpen = ffi::ngtcp2_stream_open;
pub type Ngtcp2StreamClose = ffi::ngtcp2_stream_close;
pub type Ngtcp2RecvStatelessReset = ffi::ngtcp2_recv_stateless_reset;
pub type Ngtcp2RecvRetry = ffi::ngtcp2_recv_retry;
pub type Ngtcp2ExtendMaxStreams = ffi::ngtcp2_extend_max_streams;
pub type Ngtcp2Rand = ffi::ngtcp2_rand;
pub type Ngtcp2GetNewConnectionId = ffi::ngtcp2_get_new_connection_id;
pub type Ngtcp2RemoveConnectionId = ffi::ngtcp2_remove_connection_id;
pub type Ngtcp2UpdateKey = ffi::ngtcp2_update_key;
pub type Ngtcp2PathValidation = ffi::ngtcp2_path_validation;
pub type Ngtcp2SelectPreferredAddr = ffi::ngtcp2_select_preferred_addr;
pub type Ngtcp2StreamReset = ffi::ngtcp2_stream_reset;
pub type Ngtcp2ExtendMaxStreamData = ffi::ngtcp2_extend_max_stream_data;
pub type Ngtcp2ConnectionIdStatus = ffi::ngtcp2_connection_id_status;
pub type Ngtcp2HandshakeConfirmed = ffi::ngtcp2_handshake_confirmed;
pub type Ngtcp2RecvNewToken = ffi::ngtcp2_recv_new_token;
pub type Ngtcp2DeleteCryptoAeadCtx = ffi::ngtcp2_delete_crypto_aead_ctx;
pub type Ngtcp2DeleteCryptoCipherCtx = ffi::ngtcp2_delete_crypto_cipher_ctx;
pub type Ngtcp2RecvDatagram = ffi::ngtcp2_recv_datagram;
pub type Ngtcp2AckDatagram = ffi::ngtcp2_ack_datagram;
pub type Ngtcp2LostDatagram = ffi::ngtcp2_lost_datagram;
pub type Ngtcp2GetPathChallengeData = ffi::ngtcp2_get_path_challenge_data;
pub type Ngtcp2StreamStopSending = ffi::ngtcp2_stream_stop_sending;
pub type Ngtcp2VersionNegotiation = ffi::ngtcp2_version_negotiation;
pub type Ngtcp2RecvKey = ffi::ngtcp2_recv_key;
pub type Ngtcp2TlsEarlyDataRejected = ffi::ngtcp2_early_data_rejected;
pub use ffi::ngtcp2_early_data_rejected as Ngtcp2EarlyDataRejected;

// ── Compile-time size verification against C structs ──────────

#[cfg(feature = "h3")]
const _: () = {
    let _ = std::mem::transmute::<Ngtcp2Callbacks, [u8; 320]>;
    let _ = std::mem::transmute::<Ngtcp2Cid, [u8; 32]>;
    let _ = std::mem::transmute::<Ngtcp2Settings, [u8; 184]>;
    let _ = std::mem::transmute::<Ngtcp2TransportParams, [u8; 344]>;
};

// ── Version constants matching ngtcp2.h ──────────────────────

pub const NGTCP2_CALLBACKS_VERSION: c_int = 1;
pub const NGTCP2_SETTINGS_VERSION: c_int = 1;
pub const NGTCP2_TRANSPORT_PARAMS_VERSION: c_int = 1;

pub const NGTCP2_MAX_CIDLEN: usize = 20;
pub const NGTCP2_STATELESS_RESET_TOKENLEN: usize = 16;

pub type Ngtcp2EncryptionLevel = c_int;
pub const NGTCP2_ENCRYPTION_LEVEL_INITIAL: c_int = 0;
pub const NGTCP2_ENCRYPTION_LEVEL_HANDSHAKE: c_int = 1;
pub const NGTCP2_ENCRYPTION_LEVEL_1RTT: c_int = 2;
pub const NGTCP2_ENCRYPTION_LEVEL_0RTT: c_int = 3;

pub const SSL_TLSEXT_ERR_OK: i32 = 0;
pub const SSL_TLSEXT_ERR_ALERT_FATAL: i32 = 2;

// ── Connection reference for crypto callbacks ─────────────────

#[repr(C)]
pub struct Ngtcp2CryptoConnRef {
    pub get_conn: Option<unsafe extern "C" fn(*mut Ngtcp2CryptoConnRef) -> *mut c_void>,
    pub user_data: *mut c_void,
}
