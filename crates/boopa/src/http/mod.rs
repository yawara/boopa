use std::sync::Arc;

use std::path::{Component, Path, PathBuf};

use actix_files::NamedFile;
use actix_web::{
    HttpRequest, HttpResponse,
    mime::APPLICATION_OCTET_STREAM,
    web::{self, Data, ServiceConfig},
};

use crate::{
    app_state::AppState,
    boot_assets::{BootAssetTransport, ResolvedBootAsset},
};

pub mod routes;

const INDEX_FALLBACK_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>boopa</title>
  </head>
  <body>
    <div id="root">frontend assets not built yet</div>
  </body>
</html>"#;

pub fn configure(cfg: &mut ServiceConfig, state: Arc<AppState>) {
    cfg.app_data(Data::new(state))
        .route("/api/health", web::get().to(routes::health::get_health))
        .route("/api/distros", web::get().to(routes::distros::get_distros))
        .route("/api/dhcp", web::get().to(routes::dhcp::get_dhcp))
        .route("/api/cache", web::get().to(routes::cache::get_cache_status))
        .route(
            "/api/selection",
            web::put().to(routes::distros::put_selection),
        )
        .route(
            "/api/cache/refresh",
            web::post().to(routes::cache::refresh_cache),
        )
        .route("/boot/{path:.*}", web::get().to(get_boot_asset))
        .route("/", web::get().to(get_frontend_asset))
        .route("/{path:.*}", web::get().to(get_frontend_asset));
}

async fn index_fallback() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(INDEX_FALLBACK_HTML)
}

async fn get_boot_asset(
    request: HttpRequest,
    state: Data<Arc<AppState>>,
    path: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let path = path.into_inner();

    match state
        .resolve_boot_asset(&path, BootAssetTransport::Http)
        .await
    {
        Some(ResolvedBootAsset::CachedFile { local_path, .. }) => {
            let file = NamedFile::open_async(local_path)
                .await?
                .set_content_type(APPLICATION_OCTET_STREAM);
            Ok(file.into_response(&request))
        }
        Some(asset) => match asset.read_bytes().await {
            Ok(bytes) => Ok(HttpResponse::Ok()
                .content_type(asset.content_type())
                .body(bytes)),
            Err(_) => Ok(HttpResponse::NotFound().finish()),
        },
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

async fn get_frontend_asset(
    request: HttpRequest,
    state: Data<Arc<AppState>>,
    path: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let requested_path = path.into_inner();
    let frontend_dir = &state.config().frontend_dir;

    if let Some(response) = serve_named_file(&request, frontend_dir, &requested_path).await? {
        return Ok(response);
    }

    if let Some(response) = serve_named_file(&request, frontend_dir, "index.html").await? {
        return Ok(response);
    }

    Ok(index_fallback().await)
}

async fn serve_named_file(
    request: &HttpRequest,
    frontend_dir: &Path,
    requested_path: &str,
) -> actix_web::Result<Option<HttpResponse>> {
    let Some(path) = resolve_frontend_path(frontend_dir, requested_path).await else {
        return Ok(None);
    };

    let file = NamedFile::open_async(path).await?;
    Ok(Some(file.into_response(request)))
}

async fn resolve_frontend_path(frontend_dir: &Path, requested_path: &str) -> Option<PathBuf> {
    let candidate = safe_join(frontend_dir, Path::new(requested_path))?;
    let metadata = tokio::fs::metadata(&candidate).await.ok()?;

    if metadata.is_file() {
        return Some(candidate);
    }

    if metadata.is_dir() {
        let index_path = candidate.join("index.html");
        if tokio::fs::metadata(&index_path)
            .await
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
        {
            return Some(index_path);
        }
    }

    None
}

fn safe_join(base_dir: &Path, requested_path: &Path) -> Option<PathBuf> {
    let mut joined = base_dir.to_path_buf();

    for component in requested_path.components() {
        match component {
            Component::Normal(segment) => joined.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    Some(joined)
}
