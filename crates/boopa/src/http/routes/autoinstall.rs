use std::sync::Arc;

use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    autoinstall::{UbuntuAutoinstallConfigUpdate, UpdateError, ValidationErrorResponse},
};

pub async fn get_ubuntu_autoinstall(state: web::Data<Arc<AppState>>) -> HttpResponse {
    match state.ubuntu_autoinstall_config().await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}

pub async fn put_ubuntu_autoinstall(
    state: web::Data<Arc<AppState>>,
    request: web::Json<UbuntuAutoinstallConfigUpdate>,
) -> HttpResponse {
    match state.update_ubuntu_autoinstall(request.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(UpdateError::Validation(error)) => HttpResponse::BadRequest().json(error),
        Err(UpdateError::Internal(error)) => {
            HttpResponse::InternalServerError().json(ValidationErrorResponse {
                message: error.to_string(),
                field_errors: Default::default(),
            })
        }
    }
}
