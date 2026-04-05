use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use boot_recipe::DistroId;
use serde::Deserialize;

use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct SelectionRequest {
    pub distro: DistroId,
}

pub async fn get_distros(
    State(state): State<Arc<AppState>>,
) -> Json<crate::app_state::DistrosResponse> {
    Json(state.supported_distros().await)
}

pub async fn put_selection(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SelectionRequest>,
) -> Result<Json<crate::app_state::SelectionResponse>, (StatusCode, String)> {
    state
        .set_selected_distro(request.distro)
        .await
        .map(Json)
        .map_err(internal_error)
}

fn internal_error(error: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
