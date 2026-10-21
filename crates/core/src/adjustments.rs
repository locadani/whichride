//! A user's own price changes: overrides, subscriptions, discounts and refuel credits.
//!
//! They are applied in a fixed order (spec §6): override → subscription → percentage discounts →
//! fixed discounts and refuel credit → minimum charge.

use serde::{Deserialize, Serialize};

use crate::catalogue::{Offer, OfferId, Provider, ProviderId, validate_cards};
use crate::money::{Cents, Percent};
use crate::rate_card::{CardKind, Charges, RateCard};
use crate::usage::{Rounding, Trip};

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

/// What a provider credits this user for refuelling or charging during a trip. Providers do not
/// publish it, so the user enters it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefuelCredit {
    pub provider: ProviderId,
    pub amount: Cents,
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
    /// The first one for a provider applies.
    #[serde(default)]
    pub refuel_credits: Vec<RefuelCredit>,
    /// The user will refuel or charge during this trip.
    #[serde(default)]
    pub refuel: bool,
}

/// A trip's price and what it is made of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Price {
    pub total: Cents,
    /// In the order they were applied (spec §6); their amounts add up to `total`.
    pub lines: Vec<PriceLine>,
}

/// One part of a price. Amounts that lower the price are negative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PriceLine {
    /// The unlock fee of a standard tariff, the price of a package. Left out when zero.
    BaseFee {
        kind: CardKind,
        amount: Cents,
    },
    /// Left out when the card has no per-km price.
    Distance {
        billed_metres: u64,
        per_km: Cents,
        included_km: u32,
        rounding: Rounding,
        amount: Cents,
    },
    /// Left out when the card has no per-minute price.
    Time {
        billed_seconds: u64,
        per_minute: Cents,
        included_minutes: u32,
        rounding: Rounding,
        amount: Cents,
    },
    /// What the provider's subscription saves; left out when it saves nothing.
    Subscription {
        amount: Cents,
    },
    PercentDiscount {
        off: Percent,
        amount: Cents,
    },
    /// Only what the discount could take off: never below zero.
    AmountDiscount {
        amount: Cents,
    },
    RefuelCredit {
        amount: Cents,
    },
    /// What lifts the price to the card's minimum charge.
    Minimum {
        minimum: Cents,
        amount: Cents,
    },
}

impl PriceLine {
    pub fn amount(&self) -> Cents {
        match self {
            PriceLine::BaseFee { amount, .. }
            | PriceLine::Distance { amount, .. }
            | PriceLine::Time { amount, .. }
            | PriceLine::Subscription { amount }
            | PriceLine::PercentDiscount { amount, .. }
            | PriceLine::AmountDiscount { amount }
            | PriceLine::RefuelCredit { amount }
            | PriceLine::Minimum { amount, .. } => *amount,
        }
    }
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
        self.priced(provider, card, trip).total
    }

    /// The final price of a trip with one rate card of `provider`, and its lines.
    pub fn priced(&self, provider: &Provider, card: &RateCard, trip: &Trip) -> Price {
        let charges = card.charges(trip);
        let mut lines = Vec::new();
        if charges.base_fee != Cents::ZERO {
            lines.push(PriceLine::BaseFee {
                kind: card.kind,
                amount: charges.base_fee,
            });
        }
        if card.per_km != Cents::ZERO {
            lines.push(PriceLine::Distance {
                billed_metres: charges.billed_metres,
                per_km: card.per_km,
                included_km: card.included_km,
                rounding: card.km_rounding,
                amount: charges.distance,
            });
        }
        if card.per_minute != Cents::ZERO {
            lines.push(PriceLine::Time {
                billed_seconds: charges.billed_seconds,
                per_minute: card.per_minute,
                included_minutes: card.included_minutes,
                rounding: card.minute_rounding,
                amount: charges.time,
            });
        }
        let mut total = charges.total();
        if let Some(subscription) = self
            .subscriptions
            .iter()
            .find(|s| s.provider == provider.id)
        {
            let after = after_subscription(subscription, card.kind, charges);
            // A pass that changes nothing for this card (e.g. a package) gets no line.
            if after != total {
                lines.push(PriceLine::Subscription {
                    amount: after - total,
                });
            }
            total = after;
        }
        for discount in &self.discounts {
            if let Discount::Percent { provider: id, off } = discount
                && *id == provider.id
            {
                let after = off.off(total);
                if after != total {
                    lines.push(PriceLine::PercentDiscount {
                        off: *off,
                        amount: after - total,
                    });
                }
                total = after;
            }
        }
        for discount in &self.discounts {
            if let Discount::Amount {
                provider: id,
                amount,
            } = discount
                && *id == provider.id
                // A negative amount would raise the price; treat it as invalid input.
                && *amount > Cents::ZERO
            {
                let taken = (*amount).min(total.at_least_zero());
                lines.push(PriceLine::AmountDiscount {
                    amount: Cents::ZERO - taken,
                });
                total = total - taken;
            }
        }
        if self.refuel
            && let Some(credit) = self
                .refuel_credits
                .iter()
                .find(|c| c.provider == provider.id)
            // Like a fixed discount, a negative credit is invalid input.
            && credit.amount > Cents::ZERO
        {
            let taken = credit.amount.min(total.at_least_zero());
            lines.push(PriceLine::RefuelCredit {
                amount: Cents::ZERO - taken,
            });
            total = total - taken;
        }
        let floor = total.at_least_zero().max(card.minimum);
        if floor != total {
            lines.push(PriceLine::Minimum {
                minimum: card.minimum,
                amount: floor - total,
            });
        }
        Price {
            total: floor,
            lines,
        }
    }
}

fn after_subscription(subscription: &Subscription, kind: CardKind, charges: Charges) -> Cents {
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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::catalogue::fixtures::catalogue;
    use crate::rate_card::fixtures::*;
    use crate::usage::Rounding;

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
    fn refuel_credit_applies_only_when_refuelling_and_entered() {
        let refuelling = Adjustments {
            refuel_credits: vec![RefuelCredit {
                provider: miles().id,
                amount: Cents(500),
            }],
            refuel: true,
            ..Adjustments::default()
        };
        assert_eq!(
            refuelling.price(&miles(), &per_km_card(), &TRIP),
            Cents(469)
        );
        let not_refuelling = Adjustments {
            refuel: false,
            ..refuelling.clone()
        };
        assert_eq!(
            not_refuelling.price(&miles(), &per_km_card(), &TRIP),
            Cents(969)
        );
        // The user entered no credit for Bolt.
        assert_eq!(
            refuelling.price(&bolt(), &per_minute_and_km_card(), &TRIP),
            Cents(473)
        );
    }

    #[test]
    fn negative_refuel_credit_is_ignored() {
        let adjustments = Adjustments {
            refuel_credits: vec![RefuelCredit {
                provider: miles().id,
                amount: Cents(-500),
            }],
            refuel: true,
            ..Adjustments::default()
        };
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents(969)
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

    fn lines(
        adjustments: &Adjustments,
        provider: &Provider,
        card: &RateCard,
        trip: &Trip,
    ) -> Vec<PriceLine> {
        adjustments.priced(provider, card, trip).lines
    }

    #[test]
    fn lines_show_the_unlock_fee_and_the_billed_km() {
        assert_eq!(
            lines(&Adjustments::default(), &miles(), &per_km_card(), &TRIP),
            [
                PriceLine::BaseFee {
                    kind: CardKind::Standard,
                    amount: Cents(100)
                },
                PriceLine::Distance {
                    billed_metres: 11_000,
                    per_km: Cents(79),
                    included_km: 0,
                    rounding: Rounding::StartedUnit,
                    amount: Cents(869)
                },
            ]
        );
    }

    #[test]
    fn lines_leave_out_parts_the_card_does_not_charge() {
        assert_eq!(
            lines(
                &Adjustments::default(),
                &bolt(),
                &per_minute_and_km_card(),
                &TRIP
            ),
            [
                PriceLine::Distance {
                    billed_metres: 11_000,
                    per_km: Cents(18),
                    included_km: 0,
                    rounding: Rounding::StartedUnit,
                    amount: Cents(198)
                },
                PriceLine::Time {
                    billed_seconds: 1_500,
                    per_minute: Cents(11),
                    included_minutes: 0,
                    rounding: Rounding::StartedUnit,
                    amount: Cents(275)
                },
            ]
        );
    }

    #[test]
    fn lines_of_a_package_show_its_price_and_the_included_units() {
        assert_eq!(
            lines(&Adjustments::default(), &miles(), &package_card(), &TRIP),
            [
                PriceLine::BaseFee {
                    kind: CardKind::Package,
                    amount: Cents(2_900)
                },
                PriceLine::Distance {
                    billed_metres: 0,
                    per_km: Cents(39),
                    included_km: 40,
                    rounding: Rounding::StartedUnit,
                    amount: Cents::ZERO
                },
                PriceLine::Time {
                    billed_seconds: 0,
                    per_minute: Cents(29),
                    included_minutes: 180,
                    rounding: Rounding::StartedUnit,
                    amount: Cents::ZERO
                },
            ]
        );
    }

    #[test]
    fn a_subscription_is_one_line_with_what_it_saves() {
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
        let lines = lines(&adjustments, &miles(), &per_km_card(), &TRIP);
        // 969 → 782
        assert_eq!(
            lines[2],
            PriceLine::Subscription {
                amount: Cents(-187)
            }
        );
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn a_subscription_or_discount_that_changes_nothing_gets_no_line() {
        let adjustments = Adjustments {
            subscriptions: vec![Subscription {
                provider: miles().id,
                distance_off: pct(1_000),
                time_off: Percent::ZERO,
                waive_base_fee: false,
                package_off: Percent::ZERO,
            }],
            discounts: vec![Discount::Percent {
                provider: miles().id,
                off: Percent::ZERO,
            }],
            ..Adjustments::default()
        };
        // A package under a pass that only discounts standard tariffs, and a 0 % discount.
        assert_eq!(
            lines(&adjustments, &miles(), &package_card(), &TRIP).len(),
            3
        );
    }

    #[test]
    fn each_percentage_discount_is_a_line_in_order() {
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
        // 969 → 775 → 698
        assert_eq!(
            lines(&adjustments, &miles(), &per_km_card(), &TRIP)[2..],
            [
                PriceLine::PercentDiscount {
                    off: pct(2_000),
                    amount: Cents(-194)
                },
                PriceLine::PercentDiscount {
                    off: pct(1_000),
                    amount: Cents(-77)
                },
            ]
        );
    }

    #[test]
    fn a_fixed_discount_shows_only_what_it_took_off() {
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
        // 969 → 485, then the 500 discount can only take 485.
        assert_eq!(
            lines(&adjustments, &miles(), &per_km_card(), &TRIP)[2..],
            [
                PriceLine::PercentDiscount {
                    off: pct(5_000),
                    amount: Cents(-484)
                },
                PriceLine::AmountDiscount {
                    amount: Cents(-485)
                },
            ]
        );
    }

    #[test]
    fn the_refuel_credit_is_a_line_when_refuelling() {
        let adjustments = Adjustments {
            refuel_credits: vec![RefuelCredit {
                provider: miles().id,
                amount: Cents(500),
            }],
            refuel: true,
            ..Adjustments::default()
        };
        assert_eq!(
            lines(&adjustments, &miles(), &per_km_card(), &TRIP)[2..],
            [PriceLine::RefuelCredit {
                amount: Cents(-500)
            }]
        );
    }

    #[test]
    fn invalid_negative_amounts_get_no_line() {
        let adjustments = Adjustments {
            discounts: vec![Discount::Amount {
                provider: miles().id,
                amount: Cents(-500),
            }],
            refuel_credits: vec![RefuelCredit {
                provider: miles().id,
                amount: Cents(-500),
            }],
            refuel: true,
            ..Adjustments::default()
        };
        assert_eq!(
            lines(&adjustments, &miles(), &per_km_card(), &TRIP).len(),
            2
        );
    }

    #[test]
    fn the_minimum_is_a_line_with_the_top_up() {
        let short = Trip {
            metres: 500,
            seconds: 120,
        };
        let lines = lines(
            &Adjustments::default(),
            &bolt(),
            &per_minute_and_km_card(),
            &short,
        );
        // 18 + 22 = 40, topped up to 250
        assert_eq!(
            lines.last(),
            Some(&PriceLine::Minimum {
                minimum: Cents(250),
                amount: Cents(210)
            })
        );
    }

    proptest! {
        #[test]
        fn lines_always_add_up_to_the_price(
            metres in 0u32..2_000_000,
            seconds in 0u32..1_000_000,
            percent in 0u16..=10_000,
            amount in -1_000i64..100_000,
            credit in -1_000i64..100_000,
            refuel in any::<bool>(),
            subscribed in any::<bool>(),
            card in 0usize..3,
        ) {
            let trip = Trip { metres, seconds };
            let provider = miles();
            let card = [per_km_card(), per_minute_and_km_card(), package_card()][card].clone();
            let adjustments = Adjustments {
                subscriptions: if subscribed {
                    vec![Subscription {
                        provider: provider.id.clone(),
                        distance_off: pct(1_500),
                        time_off: pct(500),
                        waive_base_fee: true,
                        package_off: pct(1_000),
                    }]
                } else {
                    vec![]
                },
                discounts: vec![
                    Discount::Percent { provider: provider.id.clone(), off: pct(percent) },
                    Discount::Amount { provider: provider.id.clone(), amount: Cents(amount) },
                ],
                refuel_credits: vec![RefuelCredit { provider: provider.id.clone(), amount: Cents(credit) }],
                refuel,
                ..Adjustments::default()
            };
            let price = adjustments.priced(&provider, &card, &trip);
            let sum: Cents = price.lines.iter().map(PriceLine::amount).sum();
            prop_assert_eq!(sum, price.total);
            prop_assert_eq!(price.total, adjustments.price(&provider, &card, &trip));
        }
    }
}
