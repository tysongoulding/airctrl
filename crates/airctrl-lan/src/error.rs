use std::net::SocketAddr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LanError {
    #[error("Failed to bind HTTP server to {addr}: {source}")]
    BindError {
        addr: SocketAddr,
        source: std::io::Error,
    },

    #[error("I/O error encountered: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Artifact not found at path: {0}")]
    ArtifactNotFound(String),

    #[error("No active LAN network interface discovered")]
    NoLanInterface,
}
