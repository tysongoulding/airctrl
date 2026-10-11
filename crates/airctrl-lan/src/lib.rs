pub mod error;
pub mod handlers;
pub mod ip;
pub mod script;
pub mod state;

pub use error::LanError;
pub use ip::get_lan_ip;
pub use script::generate_run_ps1;
pub use state::LanState;

use axum::{routing::get, Router};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct LanServerConfig {
    pub bind_ip: Ipv4Addr,
    pub port: u16,
    pub advertised_ip: Option<Ipv4Addr>,
    pub artifact_path: PathBuf,
    pub version: String,
}

impl Default for LanServerConfig {
    fn default() -> Self {
        Self {
            bind_ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 8080,
            advertised_ip: None,
            artifact_path: PathBuf::from("target/x86_64-pc-windows-msvc/release/airctrl.exe"),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

pub struct ServerHandle {
    pub local_addr: SocketAddr,
    pub base_url: String,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
}

impl ServerHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Starts the embedded async HTTP LAN distribution daemon.
pub async fn start_lan_server(config: LanServerConfig) -> Result<ServerHandle, LanError> {
    let lan_ip = match config.advertised_ip {
        Some(ip) => ip,
        None => get_lan_ip()?,
    };

    let bind_addr = SocketAddr::from((config.bind_ip, config.port));
    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|e| LanError::BindError { addr: bind_addr, source: e })?;

    let actual_addr = listener.local_addr().map_err(LanError::Io)?;

    let advertised_port = if config.port == 0 {
        actual_addr.port()
    } else {
        config.port
    };

    let base_url = format!("http://{}:{}", lan_ip, advertised_port);

    let state = Arc::new(LanState::new(
        lan_ip,
        advertised_port,
        config.artifact_path,
        config.version,
    ));

    let app = Router::new()
        .route("/run.ps1", get(handlers::get_run_ps1))
        .route("/airctrl.exe", get(handlers::get_artifact))
        .route("/health", get(handlers::get_health))
        .route("/version", get(handlers::get_version))
        .with_state(state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let graceful = server.with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = graceful.await {
            tracing::error!("LAN HTTP daemon error: {}", e);
        }
    });

    Ok(ServerHandle {
        local_addr: actual_addr,
        base_url,
        shutdown_tx: Some(shutdown_tx),
    })
}
