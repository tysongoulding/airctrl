use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tokio_util::io::ReaderStream;
use crate::state::LanState;

pub async fn get_artifact(
    State(state): State<Arc<LanState>>,
) -> Response {
    let path = &state.artifact_path;

    // Verify path existence and ensure it is a regular file before opening or streaming
    let metadata = match tokio::fs::metadata(path).await {
        Ok(m) if m.is_file() => m,
        Ok(_) => {
            let err_json = serde_json::json!({
                "error": format!("Configured artifact path '{}' is a directory, expected an executable binary.", path.display())
            });
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                err_json.to_string(),
            ).into_response();
        }
        Err(_) => {
            let err_json = serde_json::json!({
                "error": format!("Executable artifact '{}' not found. Please compile airctrl.exe first.", path.display())
            });
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                err_json.to_string(),
            ).into_response();
        }
    };

    match tokio::fs::File::open(path).await {
        Ok(file) => {

            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/vnd.microsoft.portable-executable")
                .header(header::CONTENT_DISPOSITION, "attachment; filename=\"airctrl.exe\"")
                .header(header::CONTENT_LENGTH, metadata.len().to_string());

            let (_, _, hash) = state.inspect_artifact().await;
            if let Some(h) = hash {
                builder = builder.header(header::ETAG, format!("\"{}\"", h));
            }

            match builder.body(body) {
                Ok(resp) => resp,
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    serde_json::json!({"error": e.to_string()}).to_string(),
                ).into_response(),
            }
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                serde_json::json!({"error": e.to_string()}).to_string(),
            ).into_response()
        }
    }
}

