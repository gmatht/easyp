//! HTTP/3 Monitor Module
//!
//! This module provides monitoring and detection capabilities for HTTP/3 connections,
//! including UDP firewall detection and connection metrics tracking.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::interval;

/// HTTP/3 monitoring metrics for tracking connection health and UDP firewall issues
#[cfg(feature = "http3")]
pub struct Http3Monitor {
    /// Number of Alt-Svc headers sent to clients
    alt_svc_sent: AtomicU64,

    /// Number of successful HTTP/3 connections established
    http3_connections: AtomicU64,

    /// Number of failed HTTP/3 connection attempts
    http3_failures: AtomicU64,

    /// Number of connection timeouts (indicates UDP blocking)
    connection_timeouts: AtomicU64,

    /// Number of clients that received Alt-Svc but never connected via HTTP/3
    alt_svc_without_http3: AtomicU64,

    /// Track clients by IP to detect patterns
    client_attempts: Arc<tokio::sync::RwLock<HashMap<String, ClientStats>>>,

    /// Start time for calculating rates
    start_time: Instant,
}

/// Statistics for individual clients
#[cfg(feature = "http3")]
#[derive(Debug, Clone)]
struct ClientStats {
    alt_svc_received: u64,
    http3_attempts: u64,
    http3_successes: u64,
    timeouts: u64,
    last_seen: Instant,
}

/// UDP firewall detection results
#[cfg(feature = "http3")]
#[derive(Debug, Clone)]
pub struct FirewallDetection {
    /// Likelihood that UDP is blocked (0.0 to 1.0)
    udp_blocked_probability: f64,

    /// Number of clients showing signs of UDP blocking
    affected_clients: u64,

    /// Recommended action
    recommendation: String,
}

#[cfg(feature = "http3")]
impl Http3Monitor {
    /// Create a new HTTP/3 monitor
    pub fn new() -> Self {
        Self {
            alt_svc_sent: AtomicU64::new(0),
            http3_connections: AtomicU64::new(0),
            http3_failures: AtomicU64::new(0),
            connection_timeouts: AtomicU64::new(0),
            alt_svc_without_http3: AtomicU64::new(0),
            client_attempts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Record that an Alt-Svc header was sent to a client
    pub fn record_alt_svc_sent(&self, client_ip: &str) {
        self.alt_svc_sent.fetch_add(1, Ordering::Relaxed);

        // Update client stats
        tokio::spawn({
            let client_attempts = Arc::clone(&self.client_attempts);
            let client_ip = client_ip.to_string();
            async move {
                let mut clients = client_attempts.write().await;
                let stats = clients.entry(client_ip).or_insert_with(|| ClientStats {
                    alt_svc_received: 0,
                    http3_attempts: 0,
                    http3_successes: 0,
                    timeouts: 0,
                    last_seen: Instant::now(),
                });
                stats.alt_svc_received += 1;
                stats.last_seen = Instant::now();
            }
        });
    }

    /// Record a successful HTTP/3 connection
    pub fn record_http3_connection(&self, client_ip: &str) {
        self.http3_connections.fetch_add(1, Ordering::Relaxed);

        // Update client stats
        tokio::spawn({
            let client_attempts = Arc::clone(&self.client_attempts);
            let client_ip = client_ip.to_string();
            async move {
                let mut clients = client_attempts.write().await;
                let stats = clients.entry(client_ip).or_insert_with(|| ClientStats {
                    alt_svc_received: 0,
                    http3_attempts: 0,
                    http3_successes: 0,
                    timeouts: 0,
                    last_seen: Instant::now(),
                });
                stats.http3_attempts += 1;
                stats.http3_successes += 1;
                stats.last_seen = Instant::now();
            }
        });
    }

    /// Record a failed HTTP/3 connection attempt
    pub fn record_http3_failure(&self, client_ip: &str, is_timeout: bool) {
        if is_timeout {
            self.connection_timeouts.fetch_add(1, Ordering::Relaxed);
        } else {
            self.http3_failures.fetch_add(1, Ordering::Relaxed);
        }

        // Update client stats
        tokio::spawn({
            let client_attempts = Arc::clone(&self.client_attempts);
            let client_ip = client_ip.to_string();
            async move {
                let mut clients = client_attempts.write().await;
                let stats = clients.entry(client_ip).or_insert_with(|| ClientStats {
                    alt_svc_received: 0,
                    http3_attempts: 0,
                    http3_successes: 0,
                    timeouts: 0,
                    last_seen: Instant::now(),
                });
                stats.http3_attempts += 1;
                if is_timeout {
                    stats.timeouts += 1;
                }
                stats.last_seen = Instant::now();
            }
        });
    }

    /// Get current monitoring statistics
    pub fn get_stats(&self) -> Http3Stats {
        let alt_svc_sent = self.alt_svc_sent.load(Ordering::Relaxed);
        let http3_connections = self.http3_connections.load(Ordering::Relaxed);
        let http3_failures = self.http3_failures.load(Ordering::Relaxed);
        let timeouts = self.connection_timeouts.load(Ordering::Relaxed);

        let uptime = self.start_time.elapsed();
        let connection_rate = if uptime.as_secs() > 0 {
            http3_connections as f64 / uptime.as_secs() as f64
        } else {
            0.0
        };

        let success_rate = if http3_connections + http3_failures > 0 {
            http3_connections as f64 / (http3_connections + http3_failures) as f64
        } else {
            0.0
        };

        let alt_svc_conversion_rate = if alt_svc_sent > 0 {
            http3_connections as f64 / alt_svc_sent as f64
        } else {
            0.0
        };

        Http3Stats {
            alt_svc_sent,
            http3_connections,
            http3_failures,
            connection_timeouts: timeouts,
            connection_rate,
            success_rate,
            alt_svc_conversion_rate,
            uptime,
        }
    }

    /// Detect potential UDP firewall issues
    pub async fn detect_firewall_issues(&self) -> FirewallDetection {
        let stats = self.get_stats();
        let clients = self.client_attempts.read().await;

        // Calculate UDP blocked probability based on various factors
        let mut udp_blocked_probability = 0.0;
        let mut affected_clients = 0;

        // Factor 1: High timeout rate
        if stats.connection_timeouts > 0 {
            let timeout_rate = stats.connection_timeouts as f64 / (stats.http3_connections + stats.http3_failures) as f64;
            udp_blocked_probability += timeout_rate * 0.4;
        }

        // Factor 2: Low Alt-Svc conversion rate
        if stats.alt_svc_conversion_rate < 0.1 && stats.alt_svc_sent > 10 {
            udp_blocked_probability += 0.3;
        }

        // Factor 3: Client-specific patterns
        for (client_ip, client_stats) in clients.iter() {
            if client_stats.alt_svc_received > 0 && client_stats.http3_successes == 0 {
                affected_clients += 1;

                // If client received Alt-Svc but never successfully connected
                if client_stats.timeouts > client_stats.http3_attempts * 2 {
                    udp_blocked_probability += 0.1;
                }
            }
        }

        // Cap probability at 1.0
        udp_blocked_probability = udp_blocked_probability.min(1.0);

        let recommendation = if udp_blocked_probability > 0.7 {
            "High probability of UDP blocking. Consider disabling Alt-Svc headers or using a different port."
        } else if udp_blocked_probability > 0.4 {
            "Moderate probability of UDP blocking. Monitor connection patterns and consider fallback strategies."
        } else {
            "Low probability of UDP blocking. HTTP/3 should work normally for most clients."
        };

        FirewallDetection {
            udp_blocked_probability,
            affected_clients,
            recommendation: recommendation.to_string(),
        }
    }

    /// Start periodic monitoring and cleanup
    pub async fn start_monitoring(&self) {
        let mut interval = interval(Duration::from_secs(60)); // Check every minute
        let client_attempts = Arc::clone(&self.client_attempts);

        tokio::spawn(async move {
            loop {
                interval.tick().await;

                // Clean up old client entries (older than 1 hour)
                let cutoff = Instant::now() - Duration::from_secs(3600);
                let mut clients = client_attempts.write().await;
                clients.retain(|_, stats| stats.last_seen > cutoff);

                // Log current stats
                let stats = self.get_stats();
                println!("🔍 HTTP/3 Monitor: Alt-Svc sent: {}, HTTP/3 connections: {}, Failures: {}, Timeouts: {}",
                    stats.alt_svc_sent, stats.http3_connections, stats.http3_failures, stats.connection_timeouts);
            }
        });
    }
}

/// HTTP/3 monitoring statistics
#[cfg(feature = "http3")]
#[derive(Debug, Clone)]
pub struct Http3Stats {
    pub alt_svc_sent: u64,
    pub http3_connections: u64,
    pub http3_failures: u64,
    pub connection_timeouts: u64,
    pub connection_rate: f64,
    pub success_rate: f64,
    pub alt_svc_conversion_rate: f64,
    pub uptime: Duration,
}

/// HTTP/3 monitor when feature is disabled
#[cfg(not(feature = "http3"))]
pub struct Http3Monitor;

#[cfg(not(feature = "http3"))]
impl Http3Monitor {
    pub fn new() -> Self {
        Self
    }

    pub fn record_alt_svc_sent(&self, _client_ip: &str) {
        // No-op when feature is disabled
    }

    pub fn record_http3_connection(&self, _client_ip: &str) {
        // No-op when feature is disabled
    }

    pub fn record_http3_failure(&self, _client_ip: &str, _is_timeout: bool) {
        // No-op when feature is disabled
    }

    pub fn get_stats(&self) -> Http3Stats {
        Http3Stats {
            alt_svc_sent: 0,
            http3_connections: 0,
            http3_failures: 0,
            connection_timeouts: 0,
            connection_rate: 0.0,
            success_rate: 0.0,
            alt_svc_conversion_rate: 0.0,
            uptime: Duration::from_secs(0),
        }
    }

    pub async fn detect_firewall_issues(&self) -> FirewallDetection {
        FirewallDetection {
            udp_blocked_probability: 0.0,
            affected_clients: 0,
            recommendation: "HTTP/3 support not enabled".to_string(),
        }
    }

    pub async fn start_monitoring(&self) {
        // No-op when feature is disabled
    }
}

#[cfg(not(feature = "http3"))]
#[derive(Debug, Clone)]
pub struct Http3Stats {
    pub alt_svc_sent: u64,
    pub http3_connections: u64,
    pub http3_failures: u64,
    pub connection_timeouts: u64,
    pub connection_rate: f64,
    pub success_rate: f64,
    pub alt_svc_conversion_rate: f64,
    pub uptime: Duration,
}

#[cfg(not(feature = "http3"))]
#[derive(Debug, Clone)]
pub struct FirewallDetection {
    pub udp_blocked_probability: f64,
    pub affected_clients: u64,
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "http3")]
    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = Http3Monitor::new();
        let stats = monitor.get_stats();
        assert_eq!(stats.alt_svc_sent, 0);
        assert_eq!(stats.http3_connections, 0);
    }

    #[cfg(feature = "http3")]
    #[tokio::test]
    async fn test_firewall_detection() {
        let monitor = Http3Monitor::new();
        let detection = monitor.detect_firewall_issues().await;
        assert_eq!(detection.udp_blocked_probability, 0.0);
    }

    #[test]
    fn test_feature_gate() {
        let monitor = Http3Monitor::new();
        let stats = monitor.get_stats();
        assert_eq!(stats.alt_svc_sent, 0);
    }
}
