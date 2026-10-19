//! What every request handler can reach.

use std::sync::Arc;

use carsharing_core::Day;

#[derive(Clone)]
pub struct AppState {
    /// Today's date; tests replace the clock.
    pub today: Arc<dyn Fn() -> Day + Send + Sync>,
}
