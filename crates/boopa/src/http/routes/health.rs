use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

pub async fn get_health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse { status: "ok" })
}
