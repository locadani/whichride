//! What every request handler can reach.

use std::sync::Arc;

use carsharing_core::{Catalogue, Day};
use sqlx::PgPool;

use crate::integrity::Integrity;

#[derive(Clone)]
pub struct AppState {
    /// Today's date; tests replace the clock.
    pub today: Arc<dyn Fn() -> Day + Send + Sync>,
    pub db: PgPool,
    /// The seed catalogue: provider names, packages and refuel credits. Offers and prices come
    /// from the database.
    pub providers: Arc<Catalogue>,
    pub integrity: Arc<Integrity>,
}
