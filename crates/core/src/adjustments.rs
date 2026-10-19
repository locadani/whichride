//! A user's own price changes: overrides, subscriptions, discounts and refuel credits.
//!
//! They are applied in a fixed order (spec §6): override → subscription → percentage discounts →
//! fixed discounts and refuel credit → minimum charge.

use serde::{Deserialize, Serialize};

use crate::catalogue::{Offer, OfferId, Provider, ProviderId, validate_cards};
use crate::money::{Cents, Percent};
use crate::rate_card::{CardKind, Charges, RateCard};
use crate::usage::Trip;

/// The user's own rate cards for an offer, used instead of the shared ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceOverride {
    pub offer: OfferId,
    pub cards: Vec<RateCard>,
}

/// A paid membership that lowers specific parts of a provider's price.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub provider: ProviderId,
    /// Off the distance charge of standard tariffs.
    #[serde(default)]
    pub distance_off: Percent,
    /// Off the time charge of standard tariffs.
    #[serde(default)]
    pub time_off: Percent,
    /// The unlock fee of standard tariffs is not charged.
    #[serde(default)]
    pub waive_base_fee: bool,
    /// Off the whole charge of packages.
    #[serde(default)]
    pub package_off: Percent,
}

/// A promotion on one provider. The app passes only discounts that are valid right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Discount {
    Percent { provider: ProviderId, off: Percent },
    Amount { provider: ProviderId, amount: Cents },
}

/// Everything personal that changes a price. Stored on the device only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adjustments {
    #[serde(default)]
    pub overrides: Vec<PriceOverride>,
    /// At most one per provider is expected; the first one for a provider applies.
    #[serde(default)]
    pub subscriptions: Vec<Subscription>,
    #[serde(default)]
    pub discounts: Vec<Discount>,
    /// The user will refuel or charge during this trip.
    #[serde(default)]
    pub refuel: bool,
}

impl Adjustments {
    /// The rate cards to price an offer with, and whether they are the user's override.
    /// An override without cards, or with cards that fail validation, is ignored rather than hiding
    /// the offer or mispricing it.
    pub fn cards_for<'a>(&'a self, offer: &'a Offer) -> (&'a [RateCard], bool) {
        let found = self
            .overrides
            .iter()
            .find(|o| o.offer == offer.id && validate_cards(&o.offer, &o.cards).is_ok());
        match found {
            Some(price_override) => (&price_override.cards, true),
            None => (&offer.cards, false),
        }
    }

    /// The final price of a trip with one rate card of `provider`.
    pub fn price(&self, provider: &Provider, card: &RateCard, trip: &Trip) -> Cents {
        let charges = card.charges(trip);
        let mut total = self.after_subscription(&provider.id, card.kind, charges);
        for discount in &self.discounts {
            if let Discount::Percent { provider: id, off } = discount
                && *id == provider.id
            {
                total = off.off(total);
            }
        }
        for discount in &self.discounts {
            if let Discount::Amount {
                provider: id,
                amount,
            } = discount
                && *id == provider.id
            {
                // A negative amount would raise the price; treat it as invalid input.
                total = total - (*amount).at_least_zero();
            }
        }
        if self.refuel
            && let Some(credit) = provider.refuel_credit
        {
            total = total - credit;
        }
        total.at_least_zero().max(card.minimum)
    }

    fn after_subscription(&self, provider: &ProviderId, kind: CardKind, charges: Charges) -> Cents {
        let Some(subscription) = self.subscriptions.iter().find(|s| &s.provider == provider) else {
            return charges.total();
        };
        match kind {
            CardKind::Standard => {
                let base_fee = if subscription.waive_base_fee {
                    Cents::ZERO
                } else {
                    charges.base_fee
                };
                base_fee
                    + subscription.distance_off.off(charges.distance)
                    + subscription.time_off.off(charges.time)
            }
            CardKind::Package => subscription.package_off.off(charges.total()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::fixtures::catalogue;
    use crate::rate_card::fixtures::*;

    const TRIP: Trip = Trip {
        metres: 10_200,
        seconds: 25 * 60,
    };

    fn miles() -> Provider {
        catalogue().providers[0].clone()
    }

    fn bolt() -> Provider {
        catalogue().providers[1].clone()
    }

    fn pct(basis_points: u16) -> Percent {
        Percent::from_basis_points(basis_points).unwrap()
    }

    #[test]
    fn no_adjustments_gives_the_rate_card_price() {
        assert_eq!(
            Adjustments::default().price(&miles(), &per_km_card(), &TRIP),
            Cents(969)
        );
    }

    #[test]
    fn override_replaces_the_offer_cards() {
        let offer = &miles().offers[0];
        let mut cheaper = per_km_card();
        cheaper.per_km = Cents(59);
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: offer.id.clone(),
                cards: vec![cheaper.clone()],
            }],
            ..Adjustments::default()
        };
        assert_eq!(adjustments.cards_for(offer), (&[cheaper][..], true));
    }

    #[test]
    fn override_for_another_offer_is_ignored() {
        let offer = &miles().offers[0];
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: OfferId("unknown".into()),
                cards: vec![],
            }],
            ..Adjustments::default()
        };
        assert_eq!(adjustments.cards_for(offer), (&offer.cards[..], false));
    }

    #[test]
    fn subscription_discounts_standard_components_and_waives_unlock() {
        let adjustments = Adjustments {
            subscriptions: vec![Subscription {
                provider: miles().id,
                distance_off: pct(1_000),
                time_off: Percent::ZERO,
                waive_base_fee: true,
                package_off: Percent::ZERO,
            }],
            ..Adjustments::default()
        };
        // 869 × 0.9 = 782.1 → 782, unlock waived
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents(782)
        );
    }

    #[test]
    fn subscription_discounts_packages_as_a_whole() {
        let adjustments = Adjustments {
            subscriptions: vec![Subscription {
                provider: miles().id,
                distance_off: pct(1_000),
                time_off: Percent::ZERO,
                waive_base_fee: true,
                package_off: pct(500),
            }],
            ..Adjustments::default()
        };
        assert_eq!(
            adjustments.price(&miles(), &package_card(), &TRIP),
            Cents(2_755)
        );
    }

    #[test]
    fn subscription_of_another_provider_is_ignored() {
        let adjustments = Adjustments {
            subscriptions: vec![Subscription {
                provider: bolt().id,
                distance_off: pct(5_000),
                time_off: pct(5_000),
                waive_base_fee: true,
                package_off: pct(5_000),
            }],
            ..Adjustments::default()
        };
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents(969)
        );
    }

    #[test]
    fn percentage_discounts_compound_in_order() {
        let adjustments = Adjustments {
            discounts: vec![
                Discount::Percent {
                    provider: miles().id,
                    off: pct(2_000),
                },
                Discount::Percent {
                    provider: miles().id,
                    off: pct(1_000),
                },
            ],
            ..Adjustments::default()
        };
        // 969 × 0.8 = 775.2 → 775; 775 × 0.9 = 697.5 → 698
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents(698)
        );
    }

    #[test]
    fn percentage_applies_before_fixed_amount() {
        let adjustments = Adjustments {
            discounts: vec![
                Discount::Amount {
                    provider: miles().id,
                    amount: Cents(500),
                },
                Discount::Percent {
                    provider: miles().id,
                    off: pct(5_000),
                },
            ],
            ..Adjustments::default()
        };
        // (969 × 0.5 = 484.5 → 485) − 500 → below zero → 0
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents::ZERO
        );
    }

    #[test]
    fn discounts_larger_than_the_price_stop_at_the_minimum() {
        let adjustments = Adjustments {
            discounts: vec![Discount::Amount {
                provider: bolt().id,
                amount: Cents(10_000),
            }],
            ..Adjustments::default()
        };
        assert_eq!(
            adjustments.price(&bolt(), &per_minute_and_km_card(), &TRIP),
            Cents(250)
        );
    }

    #[test]
    fn refuel_credit_applies_only_when_refuelling_and_offered() {
        let refuelling = Adjustments {
            refuel: true,
            ..Adjustments::default()
        };
        assert_eq!(
            refuelling.price(&miles(), &per_km_card(), &TRIP),
            Cents(469)
        );
        assert_eq!(
            Adjustments::default().price(&miles(), &per_km_card(), &TRIP),
            Cents(969)
        );
        // Bolt offers no credit in the fixture.
        assert_eq!(
            refuelling.price(&bolt(), &per_minute_and_km_card(), &TRIP),
            Cents(473)
        );
    }

    #[test]
    fn minimum_applies_to_short_trips() {
        let short = Trip {
            metres: 500,
            seconds: 120,
        };
        assert_eq!(
            Adjustments::default().price(&bolt(), &per_minute_and_km_card(), &short),
            Cents(250)
        );
    }

    #[test]
    fn adjustments_read_missing_lists_as_empty() {
        let adjustments: Adjustments = serde_json::from_str("{}").unwrap();
        assert_eq!(adjustments, Adjustments::default());
        let json = r#"{"discounts":[{"type":"amount","provider":"miles","amount":500}]}"#;
        let adjustments: Adjustments = serde_json::from_str(json).unwrap();
        assert_eq!(adjustments.discounts.len(), 1);
    }

    #[test]
    fn override_with_invalid_cards_is_ignored() {
        let offer = &bolt().offers[0];
        let mut negative = per_minute_and_km_card();
        negative.per_km = Cents(-1_000);
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: offer.id.clone(),
                cards: vec![negative],
            }],
            ..Adjustments::default()
        };
        assert_eq!(adjustments.cards_for(offer), (&offer.cards[..], false));
    }

    #[test]
    fn negative_fixed_discounts_are_ignored() {
        for amount in [Cents(-100_000), Cents(i64::MIN)] {
            let adjustments = Adjustments {
                discounts: vec![Discount::Amount {
                    provider: bolt().id,
                    amount,
                }],
                ..Adjustments::default()
            };
            assert_eq!(
                adjustments.price(&bolt(), &per_minute_and_km_card(), &TRIP),
                Cents(473)
            );
        }
    }
}
