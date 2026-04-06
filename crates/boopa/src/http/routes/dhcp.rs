use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use boot_recipe::DistroId;
use serde::Deserialize;

use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct DhcpQuery {
    pub distro: Option<DistroId>,
}

pub async fn get_dhcp(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DhcpQuery>,
) -> Result<Json<crate::app_state::DhcpResponse>, (axum::http::StatusCode, String)> {
    state
        .dhcp_guide(query.distro)
        .await
        .map(Json)
        .map_err(internal_error)
}

fn internal_error(error: anyhow::Error) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        error.to_string(),
    )
}
