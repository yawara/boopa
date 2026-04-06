use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use boot_recipe::DistroId;
use serde::Deserialize;

use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub distro: Option<DistroId>,
}

pub async fn get_cache_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<crate::app_state::CacheResponse>, (StatusCode, String)> {
    state.cache_status().await.map(Json).map_err(internal_error)
}

pub async fn refresh_cache(
    State(state): State<Arc<AppState>>,
    payload: Option<Json<RefreshRequest>>,
) -> Result<Json<crate::app_state::CacheResponse>, (StatusCode, String)> {
    state
        .refresh_cache(payload.and_then(|Json(request)| request.distro))
        .await
        .map(Json)
        .map_err(internal_error)
}

fn internal_error(error: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, error.to_string())
}
