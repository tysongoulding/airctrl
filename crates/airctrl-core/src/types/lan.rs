use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMetadata {
    pub server_ip: String,
    pub port: u16,
    pub version: String,
    pub binary_name: String,
    pub sha256: Option<String>,
    pub build_timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub uptime_secs: u64,
    pub active_connections: usize,
    pub artifact_available: bool,
}
