use axum::{
    extract::State,
    http::header,
    response::IntoResponse,
};
use serde::Serialize;
use std::sync::Arc;
use crate::state::LanState;

#[derive(Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub target: String,
    pub artifact_name: String,
    pub artifact_sha256: Option<String>,
    pub artifact_size_bytes: Option<u64>,
    pub artifact_available: bool,
    pub build_timestamp: String,
}

pub async fn get_version(
    State(state): State<Arc<LanState>>,
) -> impl IntoResponse {
    let (available, size, hash) = state.inspect_artifact().await;

    let resp = VersionResponse {
        version: state.version.clone(),
        target: "x86_64-pc-windows-msvc".to_string(),
        artifact_name: "airctrl.exe".to_string(),
        artifact_sha256: hash,
        artifact_size_bytes: size,
        artifact_available: available,
        build_timestamp: state.build_timestamp.clone(),
    };

    (
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&resp).unwrap_or_default(),
    )
}
