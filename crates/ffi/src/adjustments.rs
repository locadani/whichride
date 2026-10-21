//! The user's own price changes (spec §6) across the boundary. Entries the core can't use (a
//! percentage above 100 %) are dropped instead of failing the comparison (plan R8).

use carsharing_core as cc;

use crate::cards::{RateCard, to_core};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PriceOverride {
    pub offer_id: String,
    pub cards: Vec<RateCard>,
}

/// A pass; percentages in basis points (1 500 = 15 %).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Subscription {
    pub provider_id: String,
    pub distance_off: u16,
    pub time_off: u16,
    pub waive_base_fee: bool,
    pub package_off: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Discount {
    /// `off` in basis points.
    Percent {
        provider_id: String,
        off: u16,
    },
    Amount {
        provider_id: String,
        cents: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RefuelCredit {
    pub provider_id: String,
    pub cents: i64,
}

/// What applies to this trip; the app passes only deals valid today (plan R5).
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct Adjustments {
    pub overrides: Vec<PriceOverride>,
    pub subscriptions: Vec<Subscription>,
    pub discounts: Vec<Discount>,
    pub refuel_credits: Vec<RefuelCredit>,
    pub refuel: bool,
}

fn percent(basis_points: u16) -> Option<cc::Percent> {
    cc::Percent::from_basis_points(basis_points).ok()
}

fn provider(id: &str) -> cc::ProviderId {
    cc::ProviderId(id.to_owned())
}

impl From<&Adjustments> for cc::Adjustments {
    fn from(a: &Adjustments) -> Self {
        cc::Adjustments {
            overrides: a
                .overrides
                .iter()
                .map(|o| cc::PriceOverride {
                    offer: cc::OfferId(o.offer_id.clone()),
                    cards: to_core(&o.cards),
                })
                .collect(),
            subscriptions: a
                .subscriptions
                .iter()
                .filter_map(|s| {
                    Some(cc::Subscription {
                        provider: provider(&s.provider_id),
                        distance_off: percent(s.distance_off)?,
                        time_off: percent(s.time_off)?,
                        waive_base_fee: s.waive_base_fee,
                        package_off: percent(s.package_off)?,
                    })
                })
                .collect(),
            discounts: a
                .discounts
                .iter()
                .filter_map(|d| match d {
                    Discount::Percent { provider_id, off } => Some(cc::Discount::Percent {
                        provider: provider(provider_id),
                        off: percent(*off)?,
                    }),
                    Discount::Amount { provider_id, cents } => Some(cc::Discount::Amount {
                        provider: provider(provider_id),
                        amount: cc::Cents(*cents),
                    }),
                })
                .collect(),
            refuel_credits: a
                .refuel_credits
                .iter()
                .map(|c| cc::adjustments::RefuelCredit {
                    provider: provider(&c.provider_id),
                    amount: cc::Cents(c.cents),
                })
                .collect(),
            refuel: a.refuel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentages_above_100_are_dropped() {
        let adjustments = Adjustments {
            discounts: vec![
                Discount::Percent {
                    provider_id: "bolt".into(),
                    off: 10_001,
                },
                Discount::Percent {
                    provider_id: "bolt".into(),
                    off: 2_000,
                },
                Discount::Amount {
                    provider_id: "miles".into(),
                    cents: 500,
                },
            ],
            subscriptions: vec![Subscription {
                provider_id: "miles".into(),
                distance_off: 1_000,
                time_off: 20_000,
                waive_base_fee: true,
                package_off: 0,
            }],
            ..Adjustments::default()
        };
        let core: cc::Adjustments = (&adjustments).into();
        assert_eq!(core.discounts.len(), 2);
        assert!(core.subscriptions.is_empty());
    }
}
