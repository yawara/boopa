use axum::{Json, extract::State, http::StatusCode};
use boot_recipe::DistroId;
use serde::{Deserialize, Serialize};

use crate::app_state::SharedState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionResponse {
    pub distro_id: DistroId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRequest {
    pub distro_id: DistroId,
}

pub async fn get_selection(State(state): State<SharedState>) -> Json<SelectionResponse> {
    Json(SelectionResponse {
        distro_id: state.current_distro().await,
    })
}

pub async fn set_selection(
    State(state): State<SharedState>,
    Json(request): Json<SelectionRequest>,
) -> Result<(StatusCode, Json<SelectionResponse>), StatusCode> {
    state
        .set_distro(request.distro_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        StatusCode::OK,
        Json(SelectionResponse {
            distro_id: request.distro_id,
        }),
    ))
}

