// Minimal C helper for GnuTLS QUIC session creation.
// Rust can't activate GnuTLS QUIC callbacks properly; C can.
// This function creates and configures a GnuTLS session exactly
// like the working gtlsserver does.
#include <gnutls/gnutls.h>
#include <ngtcp2/ngtcp2.h>
#include <ngtcp2/ngtcp2_crypto_gnutls.h>

// Called from Rust.  Sets up a GnuTLS server session in QUIC mode.
// Returns 0 on success, -1 on error.
// The session pointer is written to *out_session.
__attribute__((constructor)) static void ensure_global_init() {
    // C++ binaries get this implicitly via static initializers.
    // Rust doesn't — we must call it explicitly so GnuTLS's QUIC
    // callback infrastructure is properly initialized.
    (void)gnutls_global_init();
}

int h3_create_session(
    void *cred,
    const gnutls_datum_t *ticket_key,
    void *anti_replay,
    void *conn,
    gnutls_session_t *out_session
) {
    gnutls_session_t session;
    int r;

    r = gnutls_init(&session, GNUTLS_SERVER | GNUTLS_ENABLE_EARLY_DATA |
                    GNUTLS_NO_AUTO_SEND_TICKET | GNUTLS_NO_END_OF_EARLY_DATA);
    if (r != 0) return -1;

    const char *priority = "%DISABLE_TLS13_COMPAT_MODE:"
        "NORMAL:-VERS-ALL:+VERS-TLS1.3:-CIPHER-ALL:+AES-128-GCM:+AES-256-GCM:"
        "+CHACHA20-POLY1305:+AES-128-CCM:"
        "-GROUP-ALL:+GROUP-X25519:+GROUP-SECP256R1:+GROUP-SECP384R1:"
        "+GROUP-SECP521R1";
    gnutls_priority_set_direct(session, priority, NULL);

    gnutls_session_ticket_enable_server(session, ticket_key);

    // ALPN validation hook (like working example)
    gnutls_handshake_set_hook_function(session, GNUTLS_HANDSHAKE_CLIENT_HELLO,
                                       GNUTLS_HOOK_POST, NULL);

    // CRITICAL: this registers the static callbacks inside libngtcp2_crypto_gnutls
    r = ngtcp2_crypto_gnutls_configure_server_session(session);
    if (r != 0) { gnutls_deinit(session); return -1; }

    gnutls_anti_replay_enable(session, anti_replay);
    gnutls_record_set_max_early_data_size(session, 0xffffffff);

    // Session pointer: used by callbacks to find the ngtcp2 connection
    gnutls_session_set_ptr(session, conn);

    gnutls_credentials_set(session, GNUTLS_CRD_CERTIFICATE, cred);

    gnutls_datum_t alpn = { (unsigned char*)"h3", 2 };
    gnutls_alpn_set_protocols(session, &alpn, 1,
                              GNUTLS_ALPN_MANDATORY | GNUTLS_ALPN_SERVER_PRECEDENCE);

    *out_session = session;
    return 0;
}
