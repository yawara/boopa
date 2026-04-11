use std::sync::Arc;

use actix_web::{HttpResponse, web};
use boot_recipe::DistroId;
use serde::Deserialize;

use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct SelectionRequest {
    pub distro: DistroId,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/distros", web::get().to(get_distros))
        .route("/api/selection", web::put().to(put_selection));
}

pub async fn get_distros(state: web::Data<Arc<AppState>>) -> HttpResponse {
    HttpResponse::Ok().json(state.supported_distros().await)
}

pub async fn put_selection(
    state: web::Data<Arc<AppState>>,
    request: web::Json<SelectionRequest>,
) -> HttpResponse {
    match state.set_selected_distro(request.distro).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}
