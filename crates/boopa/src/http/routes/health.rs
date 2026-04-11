use actix_web::{HttpResponse, web};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/health", web::get().to(get_health));
}

pub async fn get_health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse { status: "ok" })
}
