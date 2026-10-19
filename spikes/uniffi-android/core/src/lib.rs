//! Feasibility spike: one record type and one function crossing the Rust ↔ Kotlin boundary.

uniffi::setup_scaffolding!();

/// Prices of one offer, in euro cents.
#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct RateCard {
    pub unlock_cents: u32,
    pub per_km_cents: u32,
    pub per_minute_cents: u32,
    pub minimum_cents: u32,
}

/// Cost of a trip in cents; km are rounded up to the next full km.
#[uniffi::export]
pub fn trip_cost_cents(rate: RateCard, metres: u32, minutes: u32) -> u32 {
    let km = metres.div_ceil(1000);
    let cost = rate.unlock_cents + km * rate.per_km_cents + minutes * rate.per_minute_cents;
    cost.max(rate.minimum_cents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn km_only() -> RateCard {
        RateCard { unlock_cents: 100, per_km_cents: 79, per_minute_cents: 0, minimum_cents: 0 }
    }

    #[test]
    fn rounds_km_up() {
        assert_eq!(trip_cost_cents(km_only(), 10_001, 30), 100 + 11 * 79);
    }

    #[test]
    fn applies_minimum() {
        let rate = RateCard { unlock_cents: 0, per_km_cents: 18, per_minute_cents: 11, minimum_cents: 250 };
        assert_eq!(trip_cost_cents(rate, 500, 3), 250);
    }
}
