pub mod app_state;
pub mod boot_assets;
pub mod config;
pub mod http;
pub mod persistence;
pub mod tftp;

use std::sync::Arc;

use app_state::AppState;
use axum::serve;
use tokio::net::TcpListener;
use tracing::info;

pub async fn run(config: config::Config) -> anyhow::Result<()> {
    let state = Arc::new(AppState::new(config.clone()).await?);
    let router = http::router(state.clone());
    let tcp_listener = TcpListener::bind(config.api_bind).await?;

    let tftp_state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = tftp::serve(tftp_state).await {
            tracing::error!(?error, "tftp server exited");
        }
    });

    info!(api_bind=%config.api_bind, tftp_bind=%config.tftp_bind, "boopa started");

    serve(tcp_listener, router).await?;
    Ok(())
}
