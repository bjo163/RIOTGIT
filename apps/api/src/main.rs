use std::{env, net::SocketAddr};

use axum::{Json, Router, http::StatusCode, routing::get};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct MetaResponse {
    name: &'static str,
    display_name: &'static str,
    tagline: &'static str,
    api_version: &'static str,
}

async fn live() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "live",
            service: "riotgit-api",
        }),
    )
}

async fn ready() -> (StatusCode, Json<HealthResponse>) {
    // Dependency checks will be added when the database and storage adapters land.
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ready",
            service: "riotgit-api",
        }),
    )
}

async fn meta() -> Json<MetaResponse> {
    Json(MetaResponse {
        name: "riotgit",
        display_name: "R!OTGIT",
        tagline: "THE UNDERGROUND CODE NETWORK",
        api_version: "v1",
    })
}

fn bind_address() -> anyhow::Result<SocketAddr> {
    let raw = env::var("RIOTGIT_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_owned());
    raw.parse()
        .map_err(|error| anyhow::anyhow!("invalid RIOTGIT_BIND '{raw}': {error}"))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            warn!(%error, "failed to listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => warn!(%error, "failed to listen for SIGTERM"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("riotgit_api=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let app = Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/api/v1/meta", get(meta));

    let address = bind_address()?;
    let listener = TcpListener::bind(address).await?;
    info!(%address, "R!OTGIT API listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
