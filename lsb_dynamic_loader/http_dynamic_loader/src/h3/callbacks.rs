//! ngtcp2 callbacks struct and associated C type definitions.
//! Matches libngtcp2 v1.9.x ABI (NGTCP2_CALLBACKS_VERSION = 1).

use std::os::raw::{c_int, c_uint, c_ulong, c_void};

// ── Version constants ─────────────────────────────────────────

pub const NGTCP2_CALLBACKS_VERSION: c_int = 1;
pub const NGTCP2_SETTINGS_VERSION: c_int = 2;
pub const NGTCP2_TRANSPORT_PARAMS_VERSION: c_int = 1;

// ── Type aliases matching ngtcp2.h ─────────────────────────────

pub type Ngtcp2EncryptionLevel = c_int;
pub const NGTCP2_ENCRYPTION_LEVEL_INITIAL: c_int = 0;
pub const NGTCP2_ENCRYPTION_LEVEL_HANDSHAKE: c_int = 1;
pub const NGTCP2_ENCRYPTION_LEVEL_1RTT: c_int = 2;
pub const NGTCP2_ENCRYPTION_LEVEL_0RTT: c_int = 3;

pub const NGTCP2_STATELESS_RESET_TOKENLEN: usize = 16;
pub const NGTCP2_MAX_CIDLEN: usize = 20;

// ── C-compatible structs matching ngtcp2.h ────────────────────

#[repr(C)]
pub struct Ngtcp2Cid {
    pub datalen: usize,
    pub data: [u8; NGTCP2_MAX_CIDLEN],
}

#[repr(C)]
pub struct Ngtcp2CryptoAead {
    pub native_handle: *mut c_void,
    pub max_overhead: usize,
}

#[repr(C)]
pub struct Ngtcp2CryptoAeadCtx {
    pub native_handle: *mut c_void,
}

#[repr(C)]
pub struct Ngtcp2CryptoCipher {
    pub native_handle: *mut c_void,
}

#[repr(C)]
pub struct Ngtcp2CryptoCipherCtx {
    pub native_handle: *mut c_void,
}

// ── Callback function pointer types ────────────────────────────

pub type Ngtcp2ClientInitial = unsafe extern "C" fn(
    *mut c_void, *const Ngtcp2Cid, *mut c_void,
) -> c_int;

pub type Ngtcp2RecvClientInitial = unsafe extern "C" fn(
    *mut c_void, *const Ngtcp2Cid, *mut c_void,
) -> c_int;

pub type Ngtcp2RecvCryptoData = unsafe extern "C" fn(
    *mut c_void, Ngtcp2EncryptionLevel, u64,
    *const u8, usize, *mut c_void,
) -> c_int;

pub type Ngtcp2HandshakeCompleted = unsafe extern "C" fn(
    *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2RecvVersionNegotiation = unsafe extern "C" fn(
    *mut c_void, u32, *const u8, usize, *mut c_void,
) -> c_int;

pub type Ngtcp2Encrypt = unsafe extern "C" fn(
    *mut u8, *const Ngtcp2CryptoAead, *const Ngtcp2CryptoAeadCtx,
    *const u8, usize, *const u8, usize, *const u8,
) -> c_int;

pub type Ngtcp2Decrypt = unsafe extern "C" fn(
    *mut u8, *const Ngtcp2CryptoAead, *const Ngtcp2CryptoAeadCtx,
    *const u8, usize, *const u8, usize, *const u8,
) -> c_int;

pub type Ngtcp2HpMask = unsafe extern "C" fn(
    *mut u8, *const Ngtcp2CryptoCipher, *const Ngtcp2CryptoCipherCtx,
    *const u8,
) -> c_int;

pub type Ngtcp2RecvStreamData = unsafe extern "C" fn(
    *mut c_void, u32, i64, *const u8, usize, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2AckedStreamDataOffset = unsafe extern "C" fn(
    *mut c_void, i64, u64, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2StreamOpen = unsafe extern "C" fn(
    *mut c_void, i64, *mut c_void,
) -> c_int;

pub type Ngtcp2StreamClose = unsafe extern "C" fn(
    *mut c_void, u32, i64, u64, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2RecvStatelessReset = unsafe extern "C" fn(
    *mut c_void, *const u8, *mut c_void,
) -> c_int;

pub type Ngtcp2RecvRetry = unsafe extern "C" fn(
    *mut c_void, *const Ngtcp2Cid, *const Ngtcp2Cid, *mut c_void,
) -> c_int;

pub type Ngtcp2ExtendMaxStreams = unsafe extern "C" fn(
    *mut c_void, u64, *mut c_void,
) -> c_int;

pub type Ngtcp2Rand = unsafe extern "C" fn(
    *mut u8, usize, *const c_void,
);

pub type Ngtcp2GetNewConnectionId = unsafe extern "C" fn(
    *mut c_void, *mut Ngtcp2Cid, *mut u8, usize, *mut c_void,
) -> c_int;

pub type Ngtcp2RemoveConnectionId = unsafe extern "C" fn(
    *mut c_void, *const Ngtcp2Cid, *mut c_void,
) -> c_int;

pub type Ngtcp2UpdateKey = unsafe extern "C" fn(
    *mut c_void, *mut u8, *mut u8,
    *mut Ngtcp2CryptoAeadCtx, *mut u8,
    *mut Ngtcp2CryptoAeadCtx, *mut u8,
    *const u8, *const u8, usize, *mut c_void,
) -> c_int;

pub type Ngtcp2PathValidation = unsafe extern "C" fn(
    *mut c_void, *const c_void, c_int, *mut c_void,
) -> c_int;

pub type Ngtcp2SelectPreferredAddr = unsafe extern "C" fn(
    *mut c_void, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2StreamReset = unsafe extern "C" fn(
    *mut c_void, i64, u64, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2ExtendMaxStreamData = unsafe extern "C" fn(
    *mut c_void, i64, u64, u64, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2ConnectionIdStatus = unsafe extern "C" fn(
    *mut c_void, c_int, *const Ngtcp2Cid, *mut c_void,
) -> c_int;

pub type Ngtcp2HandshakeConfirmed = unsafe extern "C" fn(
    *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2RecvNewToken = unsafe extern "C" fn(
    *mut c_void, *const u8, usize, *mut c_void,
) -> c_int;

pub type Ngtcp2DeleteCryptoAeadCtx = unsafe extern "C" fn(
    *mut c_void, *mut Ngtcp2CryptoAeadCtx, *mut c_void,
);

pub type Ngtcp2DeleteCryptoCipherCtx = unsafe extern "C" fn(
    *mut c_void, *mut Ngtcp2CryptoCipherCtx, *mut c_void,
);

pub type Ngtcp2RecvDatagram = unsafe extern "C" fn(
    *mut c_void, *const u8, usize, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2AckDatagram = unsafe extern "C" fn(
    *mut c_void, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2LostDatagram = unsafe extern "C" fn(
    *mut c_void, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2GetPathChallengeData = unsafe extern "C" fn(
    *mut c_void, *mut u8, *mut c_void,
) -> c_int;

pub type Ngtcp2StreamStopSending = unsafe extern "C" fn(
    *mut c_void, i64, u64, *mut c_void, *mut c_void,
) -> c_int;

pub type Ngtcp2VersionNegotiation = unsafe extern "C" fn(
    *mut c_void, u32, *const u8, usize, *mut c_void,
) -> c_int;

pub type Ngtcp2RecvKey = unsafe extern "C" fn(
    *mut c_void, Ngtcp2EncryptionLevel, *mut c_void,
) -> c_int;

pub type Ngtcp2TlsEarlyDataRejected = unsafe extern "C" fn(
    *mut c_void, *mut c_void,
) -> c_int;

// ── The full callbacks struct (36 fields, NGTCP2_CALLBACKS_V1) ─

#[repr(C)]
pub struct Ngtcp2Callbacks {
    pub client_initial: Option<Ngtcp2ClientInitial>,
    pub recv_client_initial: Option<Ngtcp2RecvClientInitial>,
    pub recv_crypto_data: Option<Ngtcp2RecvCryptoData>,
    pub handshake_completed: Option<Ngtcp2HandshakeCompleted>,
    pub recv_version_negotiation: Option<Ngtcp2RecvVersionNegotiation>,
    pub encrypt: Option<Ngtcp2Encrypt>,
    pub decrypt: Option<Ngtcp2Decrypt>,
    pub hp_mask: Option<Ngtcp2HpMask>,
    pub recv_stream_data: Option<Ngtcp2RecvStreamData>,
    pub acked_stream_data_offset: Option<Ngtcp2AckedStreamDataOffset>,
    pub stream_open: Option<Ngtcp2StreamOpen>,
    pub stream_close: Option<Ngtcp2StreamClose>,
    pub recv_stateless_reset: Option<Ngtcp2RecvStatelessReset>,
    pub recv_retry: Option<Ngtcp2RecvRetry>,
    pub extend_max_local_streams_bidi: Option<Ngtcp2ExtendMaxStreams>,
    pub extend_max_local_streams_uni: Option<Ngtcp2ExtendMaxStreams>,
    pub rand: Option<Ngtcp2Rand>,
    pub get_new_connection_id: Option<Ngtcp2GetNewConnectionId>,
    pub remove_connection_id: Option<Ngtcp2RemoveConnectionId>,
    pub update_key: Option<Ngtcp2UpdateKey>,
    pub path_validation: Option<Ngtcp2PathValidation>,
    pub select_preferred_addr: Option<Ngtcp2SelectPreferredAddr>,
    pub stream_reset: Option<Ngtcp2StreamReset>,
    pub extend_max_remote_streams_bidi: Option<Ngtcp2ExtendMaxStreams>,
    pub extend_max_remote_streams_uni: Option<Ngtcp2ExtendMaxStreams>,
    pub extend_max_stream_data: Option<Ngtcp2ExtendMaxStreamData>,
    pub dcid_status: Option<Ngtcp2ConnectionIdStatus>,
    pub handshake_confirmed: Option<Ngtcp2HandshakeConfirmed>,
    pub recv_new_token: Option<Ngtcp2RecvNewToken>,
    pub delete_crypto_aead_ctx: Option<Ngtcp2DeleteCryptoAeadCtx>,
    pub delete_crypto_cipher_ctx: Option<Ngtcp2DeleteCryptoCipherCtx>,
    pub recv_datagram: Option<Ngtcp2RecvDatagram>,
    pub ack_datagram: Option<Ngtcp2AckDatagram>,
    pub lost_datagram: Option<Ngtcp2LostDatagram>,
    pub get_path_challenge_data: Option<Ngtcp2GetPathChallengeData>,
    pub stream_stop_sending: Option<Ngtcp2StreamStopSending>,
    pub version_negotiation: Option<Ngtcp2VersionNegotiation>,
    pub recv_rx_key: Option<Ngtcp2RecvKey>,
    pub recv_tx_key: Option<Ngtcp2RecvKey>,
    pub tls_early_data_rejected: Option<Ngtcp2TlsEarlyDataRejected>,
}
