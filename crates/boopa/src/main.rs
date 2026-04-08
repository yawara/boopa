use anyhow::Context;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("boopa=info,actix_web=info,image_cache=info"))
                .context("failed to build log filter")?,
        )
        .init();

    let config = boopa::config::Config::from_env()?;
    boopa::run(config).await
}
