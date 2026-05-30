//! HTTP/3 Handler Module
//!
//! This module provides HTTP/3 over QUIC connection handling using h3 and quinn.
//! It integrates with the existing SecureFileServer for file serving and supports
//! all existing features including CGI, extensions, and ACME certificates.

#[cfg(feature = "http3")]
use std::sync::{Arc, Mutex};
use std::net::{IpAddr, SocketAddr};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use quinn::{Endpoint, ServerConfig as QuinnServerConfig, Incoming, Connection};
use h3::server::{RequestStream, Connection as H3Connection};
use h3_quinn::OpenStreams;
use rustls::ServerConfig as RustlsServerConfig;

// Import existing modules
use super::secure_file_server_module::{SecureFileServer, SecurityConfig};
use super::hourly_stats::HourlyStatsCollector;
use super::http_response::HttpResponse;
use super::file_handler::extract_domain_from_host_header;
use super::cgi_env::CgiEnvironment;

/// HTTP/3 Handler for managing QUIC connections and HTTP/3 requests
#[cfg(feature = "http3")]
pub struct Http3Handler {
    endpoint: Endpoint,
    file_server: Arc<SecureFileServer>,
    stats_collector: Arc<Mutex<HourlyStatsCollector>>,
    security_config: SecurityConfig,
}

/// HTTP/3 connection state for tracking individual client connections
#[cfg(feature = "http3")]
struct Http3ConnectionState {
    connection: Connection,
    h3_connection: H3Connection<OpenStreams, bytes::Bytes>,
    client_addr: SocketAddr,
    domain: Option<String>,
}

#[cfg(feature = "http3")]
impl Http3Handler {
    /// Create a new HTTP/3 handler
    pub fn new(
        server_config: Arc<RustlsServerConfig>,
        file_server: Arc<SecureFileServer>,
        stats_collector: Arc<Mutex<HourlyStatsCollector>>,
        security_config: SecurityConfig,
        bind_addr: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Convert rustls ServerConfig to quinn ServerConfig
        let quinn_config = QuinnServerConfig::with_crypto(server_config.crypto_provider().clone());

        // Create QUIC endpoint
        let endpoint = Endpoint::server(quinn_config, bind_addr)?;

        Ok(Self {
            endpoint,
            file_server,
            stats_collector,
            security_config,
        })
    }

    /// Start accepting HTTP/3 connections
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔍 Starting HTTP/3 server on UDP port {}", self.endpoint.local_addr()?.port());

        let incoming = self.endpoint.accept();
        self.handle_incoming_connections(incoming).await
    }

    /// Handle incoming QUIC connections
    async fn handle_incoming_connections(&self, mut incoming: Incoming) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        while let Some(connection) = incoming.next().await {
            let connection = connection.await?;
            let client_addr = connection.remote_address();

            println!("🔍 New HTTP/3 connection from {}", client_addr);

            // Clone shared resources for this connection
            let file_server = Arc::clone(&self.file_server);
            let stats_collector = Arc::clone(&self.stats_collector);
            let security_config = self.security_config.clone();

            // Spawn task to handle this connection
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(connection, file_server, stats_collector, security_config, client_addr).await {
                    eprintln!("🔍 Error handling HTTP/3 connection from {}: {}", client_addr, e);
                }
            });
        }

        Ok(())
    }

    /// Handle a single HTTP/3 connection
    async fn handle_connection(
        connection: Connection,
        file_server: Arc<SecureFileServer>,
        stats_collector: Arc<Mutex<HourlyStatsCollector>>,
        security_config: SecurityConfig,
        client_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create HTTP/3 connection from QUIC connection
        let h3_connection = h3_quinn::Connection::new(connection);

        // Process HTTP/3 requests
        while let Some((request, stream)) = h3_connection.accept().await? {
            Self::handle_request(request, stream, &file_server, &stats_collector, &security_config, client_addr).await?;
        }

        Ok(())
    }

    /// Handle a single HTTP/3 request
    async fn handle_request(
        request: h3::Request<()>,
        mut stream: RequestStream<bytes::Bytes, h3::server::OpenStreams>,
        file_server: &SecureFileServer,
        stats_collector: &Mutex<HourlyStatsCollector>,
        security_config: &SecurityConfig,
        client_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let method = request.method().to_string();
        let uri = request.uri().to_string();
        let headers = request.headers().clone();

        println!("🔍 HTTP/3 {} {} from {}", method, uri, client_addr);

        // Extract domain from Host header
        let domain = extract_domain_from_host_header(&headers);

        // Create CGI environment for this request
        let mut cgi_env = CgiEnvironment::new();
        cgi_env.set_request_method(&method);
        cgi_env.set_request_uri(&uri);
        cgi_env.set_remote_addr(&client_addr.to_string());
        cgi_env.set_server_protocol("HTTP/3");

        // Add headers to CGI environment
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                cgi_env.set_header(name.as_str(), value_str);
            }
        }

        // Process the request using existing file server logic
        let response = Self::process_request(
            &method,
            &uri,
            &headers,
            &domain,
            &cgi_env,
            file_server,
            security_config,
            client_addr,
        ).await?;

        // Send HTTP/3 response
        Self::send_response(response, stream).await?;

        // Update stats
        if let Ok(mut stats) = stats_collector.lock() {
            stats.record_request(&method, &uri, client_addr);
        }

        Ok(())
    }

    /// Process HTTP/3 request using existing file server logic
    async fn process_request(
        method: &str,
        uri: &str,
        headers: &h3::HeaderMap,
        domain: &Option<String>,
        cgi_env: &CgiEnvironment,
        file_server: &SecureFileServer,
        security_config: &SecurityConfig,
        client_addr: SocketAddr,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Convert h3::HeaderMap to HashMap<String, String> for compatibility
        let mut header_map = HashMap::new();
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                header_map.insert(name.to_string(), value_str.to_string());
            }
        }

        // Use existing file server logic
        file_server.handle_request(
            method,
            uri,
            &header_map,
            domain,
            cgi_env,
            security_config,
            client_addr,
        ).await
    }

    /// Send HTTP/3 response
    async fn send_response(
        response: HttpResponse,
        mut stream: RequestStream<bytes::Bytes, h3::server::OpenStreams>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Convert HttpResponse to h3::Response
        let mut h3_response = h3::Response::new(response.status_code);

        // Add headers
        for (name, value) in response.headers {
            h3_response.headers_mut().insert(
                h3::HeaderName::from_bytes(name.as_bytes())?,
                h3::HeaderValue::from_bytes(value.as_bytes())?,
            );
        }

        // Send response headers
        stream.send_response(h3_response).await?;

        // Send response body if present
        if !response.body.is_empty() {
            stream.send_data(bytes::Bytes::from(response.body)).await?;
        }

        // Finish the stream
        stream.finish().await?;

        Ok(())
    }

    /// Get the local address this handler is bound to
    pub fn local_addr(&self) -> Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
        self.endpoint.local_addr()
    }
}

/// HTTP/3 handler when feature is disabled
#[cfg(not(feature = "http3"))]
pub struct Http3Handler;

#[cfg(not(feature = "http3"))]
impl Http3Handler {
    pub fn new(
        _server_config: Arc<RustlsServerConfig>,
        _file_server: Arc<SecureFileServer>,
        _stats_collector: Arc<Mutex<HourlyStatsCollector>>,
        _security_config: SecurityConfig,
        _bind_addr: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err("HTTP/3 support not enabled. Compile with --features http3".into())
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("HTTP/3 support not enabled. Compile with --features http3".into())
    }

    pub fn local_addr(&self) -> Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
        Err("HTTP/3 support not enabled. Compile with --features http3".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[cfg(feature = "http3")]
    #[tokio::test]
    async fn test_http3_handler_creation() {
        // This test would require setting up a proper server config
        // For now, just test that the struct can be created
        let _handler = Http3Handler;
    }

    #[test]
    fn test_feature_gate() {
        // Test that the module compiles with and without the feature
        let _handler = Http3Handler;
    }
}
