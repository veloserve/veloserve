use anyhow::{Context, Result};
use tracing::info;
use veloserve::velopanel::{build_router, AppState, PanelConfig};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "velopanel=info,tower_http=info".into()),
        )
        .init();

    let config =
        PanelConfig::from_env().context("failed loading VeloPanel configuration from env")?;
    let bind_addr = config.bind_addr;
    let app = build_router(AppState::new(config)).context("failed building VeloPanel router")?;

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed binding VeloPanel listener at {bind_addr}"))?;
    info!("VeloPanel API listening on http://{bind_addr}");

    axum::serve(listener, app)
        .await
        .context("VeloPanel API server terminated with error")?;

    Ok(())
}
