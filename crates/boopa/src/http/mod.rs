use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::IntoResponse,
    routing::{get, post, put},
};
use tower_http::services::ServeDir;

use crate::app_state::AppState;

pub mod routes;

pub fn router(state: Arc<AppState>) -> Router {
    let static_assets =
        ServeDir::new(state.config().frontend_dir.clone()).not_found_service(get(index_fallback));

    Router::new()
        .route("/api/health", get(routes::health::get_health))
        .route("/api/distros", get(routes::distros::get_distros))
        .route("/api/dhcp", get(routes::dhcp::get_dhcp))
        .route("/api/cache", get(routes::cache::get_cache_status))
        .route("/api/selection", put(routes::distros::put_selection))
        .route("/api/cache/refresh", post(routes::cache::refresh_cache))
        .route("/boot/{*path}", get(get_boot_asset))
        .fallback_service(static_assets)
        .with_state(state)
}

async fn index_fallback() -> impl IntoResponse {
    axum::response::Html(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>boopa</title>
  </head>
  <body>
    <div id="root">frontend assets not built yet</div>
  </body>
</html>"#,
    )
}

async fn get_boot_asset(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    match state.resolve_boot_asset(&path).await {
        Some(asset) => match asset.read_bytes().await {
            Ok(bytes) => {
                let mut response = axum::response::Response::new(Body::from(bytes));
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(asset.content_type()),
                );
                response.into_response()
            }
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        },
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
