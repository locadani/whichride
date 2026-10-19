//! What each CLI command does. `serve` grows as later tasks add state.

use std::sync::Arc;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::app::router;
use crate::config::{self, IntegrityMode, ServeConfig};
use crate::db;
use crate::expand::Expander;
use crate::integrity::{GoogleIntegrity, Integrity};
use crate::limits::RateLimits;
use crate::ors::Ors;
use crate::state::AppState;

pub async fn serve() -> anyhow::Result<()> {
    let get = |name: &str| std::env::var(name).ok();
    let config = ServeConfig::from_lookup(get)?;
    let pool = db::connect(&config::database_url(get)?)
        .await
        .context("cannot connect to the database")?;
    let providers = db::seed_catalogue().context("crates/server/catalogue.json is invalid")?;
    db::prepare(&pool, &providers, crate::system_today()).await?;
    let integrity = match &config.integrity {
        IntegrityMode::Google { package } => {
            Integrity::Google(GoogleIntegrity::new(package.clone()))
        }
        IntegrityMode::Off => {
            warn!("PLAY_INTEGRITY=off: writes are not checked (local development only)");
            Integrity::Off
        }
    };
    let state = AppState {
        today: Arc::new(crate::system_today),
        db: pool,
        providers: Arc::new(providers),
        integrity: Arc::new(integrity),
        ors: Arc::new(Ors::new(
            config.ors_base_url.clone(),
            config.ors_api_key.clone(),
        )),
        limits: Arc::new(RateLimits::new()),
        expander: Arc::new(Expander::new()),
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
    let get = |name: &str| std::env::var(name).ok();
    let pool = db::connect(&config::database_url(get)?)
        .await
        .context("cannot connect to the database")?;
    let providers = db::seed_catalogue().context("crates/server/catalogue.json is invalid")?;
    db::prepare(&pool, &providers, crate::system_today()).await?;
    pool.close().await;
    Ok(())
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
