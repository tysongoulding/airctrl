use axum::{
    extract::State,
    http::header,
    response::IntoResponse,
};
use serde::Serialize;
use std::sync::Arc;
use crate::state::LanState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
    pub server_ip: String,
    pub port: u16,
    pub artifact_available: bool,
    pub artifact_size_bytes: Option<u64>,
}

pub async fn get_health(
    State(state): State<Arc<LanState>>,
) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    let (available, size, _) = state.inspect_artifact().await;

    let resp = HealthResponse {
        status: "healthy".to_string(),
        uptime_secs: uptime,
        server_ip: state.advertised_ip.to_string(),
        port: state.advertised_port,
        artifact_available: available,
        artifact_size_bytes: size,
    };

    (
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&resp).unwrap_or_default(),
    )
}
