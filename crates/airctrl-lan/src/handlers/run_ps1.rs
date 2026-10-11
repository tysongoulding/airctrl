use axum::{
    extract::State,
    http::header,
    response::IntoResponse,
};
use std::sync::Arc;
use crate::script::generate_run_ps1;
use crate::state::LanState;

pub async fn get_run_ps1(
    State(state): State<Arc<LanState>>,
) -> impl IntoResponse {
    let script = generate_run_ps1(&state.advertised_ip.to_string(), state.advertised_port);
    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
        ],
        script,
    )
}
