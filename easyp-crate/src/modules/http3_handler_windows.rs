//! HTTP/3 handler for Windows — uses MsQuic for QUIC transport.

#![cfg(windows)]

use std::sync::Arc;
use std::net::SocketAddr;
use std::ptr;

use crate::secure_file_server_module::{SecureFileServer, SecurityConfig};

use lsb_msquic::*;
use lsb_msquic::{
    QuicApi, QUIC_BUFFER, make_quic_addr_v4,
    QUIC_REGISTRATION_CONFIG, QUIC_EXECUTION_PROFILE_LOW_LATENCY,
    QUIC_LISTENER_EVENT, QUIC_LISTENER_EVENT_NEW_CONNECTION,
    QUIC_STATUS_SUCCESS, HQUIC,
};

const H3_ALPN: &[u8] = b"h3";

fn make_alpn_buffer() -> QUIC_BUFFER {
    QUIC_BUFFER {
        length: H3_ALPN.len() as u32,
        buffer: H3_ALPN.as_ptr() as *mut u8,
    }
}

pub struct Http3Handler {
    quic: Option<QuicApi>,
    registration: HQUIC,
    configuration: HQUIC,
    listener: HQUIC,
}

impl Http3Handler {
    pub async fn new(
        _file_server: Arc<SecureFileServer>,
        _security_config: SecurityConfig,
        bind_addr: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("H3/Windows: initializing MsQuic listener on {}", bind_addr);

        let quic = QuicApi::open()
            .map_err(|e| format!("MsQuic not available: {} — HTTP/3 requires msquic.dll", e))?;

        let reg_config = QUIC_REGISTRATION_CONFIG {
            app_name: "easyp-h3\0".as_ptr() as *const u8,
            execution_profile: QUIC_EXECUTION_PROFILE_LOW_LATENCY,
        };
        let registration = quic.registration_open(Some(&reg_config))
            .map_err(|e| format!("MsQuic RegistrationOpen failed: {:?}", e))?;

        let alpn = [make_alpn_buffer()];
        let configuration = quic.configuration_open(registration, &alpn)
            .map_err(|e| format!("MsQuic ConfigurationOpen failed: {:?}", e))?;

        quic.configuration_load_self_signed(configuration)
            .map_err(|e| format!("MsQuic LoadSelfSignedCert failed: {:?}", e))?;

        let ip = match bind_addr.ip() {
            std::net::IpAddr::V4(v4) => v4.octets(),
            _ => [0, 0, 0, 0],
        };
        let addr = make_quic_addr_v4(ip, bind_addr.port());

        let listener = quic.listener_open(registration, Some(listener_callback), ptr::null_mut())
            .map_err(|e| format!("MsQuic ListenerOpen failed: {:?}", e))?;

        quic.listener_start(listener, &alpn, &addr)
            .map_err(|e| format!("MsQuic ListenerStart failed: {:?}", e))?;

        log::info!("H3/Windows: MsQuic listener started on {}", bind_addr);

        Ok(Http3Handler {
            quic: Some(quic),
            registration,
            configuration,
            listener,
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // MsQuic is event-driven via callbacks on its internal threads.
        // The async task just awaits forever; the callbacks handle everything.
        futures::future::pending::<()>().await;
        Ok(())
    }
}

unsafe impl Send for Http3Handler {}
unsafe impl Sync for Http3Handler {}

impl Drop for Http3Handler {
    fn drop(&mut self) {
        if let Some(ref quic) = self.quic {
            quic.listener_close(self.listener);
            quic.configuration_close(self.configuration);
            quic.registration_close(self.registration);
        }
    }
}

unsafe extern "system" fn listener_callback(
    _listener: HQUIC,
    _context: *mut std::ffi::c_void,
    event: *mut QUIC_LISTENER_EVENT,
) -> i32 {
    let ev = unsafe { &*event };
    match ev.type_ {
        QUIC_LISTENER_EVENT_NEW_CONNECTION => {
            log::info!("H3/Windows: new QUIC connection");
            QUIC_STATUS_SUCCESS
        }
        _ => QUIC_STATUS_SUCCESS,
    }
}
