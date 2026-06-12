//! Modules for the easyp HTTPS server
//!
//! This module contains various components for handling HTTP requests,
//! file serving, security, and protocol support.

pub mod connection_policy;
pub mod extension_traits;
pub mod file_cache;
pub mod file_handler;
pub mod http_response;
pub mod http_version;
pub mod secure_file_server_module;

#[cfg(feature = "http2")]
pub mod http2_handler;

#[cfg(feature = "http3")]
pub mod http3_handler;
#[cfg(feature = "http3")]
pub mod http3_monitor;

#[cfg(all(windows, feature = "http-sys"))]
pub mod http_sys_handler;
