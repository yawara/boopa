use std::sync::Arc;

use actix_web::{HttpResponse, web};
use boot_recipe::DistroId;
use serde::Deserialize;

use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct DhcpQuery {
    pub distro: Option<DistroId>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/dhcp", web::get().to(get_dhcp));
}

pub async fn get_dhcp(
    state: web::Data<Arc<AppState>>,
    query: web::Query<DhcpQuery>,
) -> HttpResponse {
    match state.dhcp_guide(query.distro).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}
