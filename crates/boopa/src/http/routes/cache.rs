use std::sync::Arc;

use actix_web::{HttpResponse, web};
use boot_recipe::DistroId;
use serde::Deserialize;

use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub distro: Option<DistroId>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/cache", web::get().to(get_cache_status))
        .route("/api/cache/refresh", web::post().to(refresh_cache));
}

pub async fn get_cache_status(state: web::Data<Arc<AppState>>) -> HttpResponse {
    match state.cache_status().await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => HttpResponse::BadGateway().body(error.to_string()),
    }
}

pub async fn refresh_cache(
    state: web::Data<Arc<AppState>>,
    payload: Option<web::Json<RefreshRequest>>,
) -> HttpResponse {
    match state
        .refresh_cache(payload.and_then(|request| request.distro))
        .await
    {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => HttpResponse::BadGateway().body(error.to_string()),
    }
}
