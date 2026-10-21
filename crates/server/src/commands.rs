//! What each CLI command does. `serve` grows as later tasks add state.

use std::sync::Arc;

use anyhow::Context;
use carsharing_core::Coordinate;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::app::router;
use crate::config::{self, IntegrityMode, ServeConfig};
use crate::db;
use crate::expand::Expander;
use crate::integrity::{GoogleIntegrity, Integrity};
use crate::limits::RateLimits;
use crate::ors::Ors;
use crate::owner;
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
        IntegrityMode::Key(key) => {
            info!("PLAY_INTEGRITY=key: only builds with the registration key can register");
            Integrity::Key(key.clone())
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

async fn owner_pool() -> anyhow::Result<sqlx::PgPool> {
    let url = config::database_url(|name| std::env::var(name).ok())?;
    db::connect(&url)
        .await
        .context("cannot connect to the database")
}

pub async fn show_offer(offer: &str) -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    print!("{}", owner::show_offer(&pool, offer).await?);
    pool.close().await;
    Ok(())
}

pub async fn set_locked(offer: &str, locked: bool) -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    owner::set_locked(&pool, offer, locked).await?;
    println!("{offer}: {}", if locked { "locked" } else { "unlocked" });
    pool.close().await;
    Ok(())
}

pub async fn set_hidden(offer: &str, hidden: bool) -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    owner::set_hidden(&pool, offer, hidden).await?;
    println!("{offer}: {}", if hidden { "hidden" } else { "shown" });
    pool.close().await;
    Ok(())
}

pub async fn close_report(report: uuid::Uuid) -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    owner::close_report(&pool, report, crate::system_today()).await?;
    println!("report {report}: closed");
    pool.close().await;
    Ok(())
}

pub async fn revert_offer(offer: &str) -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    owner::revert_offer(&pool, offer, crate::system_today()).await?;
    println!("{offer}: reverted to its previous price");
    println!("if the abuse may go on, consider lock-offer {offer} too");
    pool.close().await;
    Ok(())
}

pub async fn revoke_installs() -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    let revoked = owner::revoke_installs(&pool).await?;
    println!("{revoked} installs revoked; each registers again on its next request");
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

/// Alexanderplatz → Berlin Hauptbahnhof: public landmarks, not anyone's location.
const CHECK_FROM: (i32, i32) = (525_219_184, 134_132_147);
const CHECK_TO: (i32, i32) = (525_250_839, 133_694_195);

pub async fn check_routing() -> anyhow::Result<()> {
    let get = |name: &str| std::env::var(name).ok();
    let key = get("ORS_API_KEY")
        .filter(|k| !k.trim().is_empty())
        .context("ORS_API_KEY is not set")?;
    let base = get("ORS_BASE_URL").unwrap_or_else(|| ServeConfig::DEFAULT_ORS_BASE_URL.to_owned());
    let ors = Ors::new(url::Url::parse(&base)?, key.trim().to_owned());
    let from = Coordinate::new(CHECK_FROM.0, CHECK_FROM.1)?;
    let to = Coordinate::new(CHECK_TO.0, CHECK_TO.1)?;
    match ors.route(from, to).await {
        Ok(trip) => {
            println!("routing ok: {} m, {} s", trip.metres, trip.seconds);
            Ok(())
        }
        Err(error) => anyhow::bail!("routing failed: {error:?}"),
    }
}
