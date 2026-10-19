//! What each CLI command does. `serve` grows as later tasks add state.

use std::sync::Arc;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::app::router;
use crate::config::ServeConfig;
use crate::state::AppState;

pub async fn serve() -> anyhow::Result<()> {
    let config = ServeConfig::from_lookup(|name| std::env::var(name).ok())?;
    let state = AppState {
        today: Arc::new(crate::system_today),
    };
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("cannot listen on {}", config.bind_addr))?;
    info!(addr = %config.bind_addr, "listening");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

pub async fn migrate() -> anyhow::Result<()> {
    anyhow::bail!("the database arrives in Task 4")
}

/// Resolves on Ctrl-C, or on SIGTERM (what Cloud Run and `docker stop` send).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                warn!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    info!("shutting down");
}
