pub mod app_state;
pub mod autoinstall;
pub mod boot_assets;
pub mod config;
pub mod http;
pub mod persistence;
pub mod tftp;

use std::sync::Arc;

use actix_web::{App, HttpServer};
use app_state::AppState;
use tracing::info;

pub async fn run(config: config::Config) -> anyhow::Result<()> {
    let state = Arc::new(AppState::new(config.clone()).await?);

    let tftp_state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = tftp::serve(tftp_state).await {
            tracing::error!(?error, "tftp server exited");
        }
    });

    info!(api_bind=%config.api_bind, tftp_bind=%config.tftp_bind, "boopa started");

    HttpServer::new(move || {
        let state = state.clone();
        App::new().configure(move |cfg| http::configure(cfg, state.clone()))
    })
    .bind(config.api_bind)?
    .run()
    .await?;

    Ok(())
}
