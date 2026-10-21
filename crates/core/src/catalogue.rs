//! Providers, their offers and the offers' rate cards, loaded from versioned JSON.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::money::Cents;
use crate::rate_card::RateCard;

/// Stable identifier of a provider, e.g. `miles`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderId(pub String);

/// Identifier of an offer, unique across all providers, e.g. `miles.s`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OfferId(pub String);

/// A car model or size class with its tariffs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: OfferId,
    pub name: String,
    pub cards: Vec<RateCard>,
}

/// An app that can open a provider (decision 66), e.g. BVG Jelbi or SIXT for SIXT share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderApp {
    pub name: String,
    pub android_package: String,
}

/// A carsharing company.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub name: String,
    /// Android package used by the "Open <provider>" button.
    pub android_package: String,
    /// The name of the app `android_package` opens, when it isn't the provider's own
    /// (SIXT share opens BVG Jelbi, decision 48).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// Other apps the user may choose to open this provider with.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_apps: Vec<ProviderApp>,
    pub offers: Vec<Offer>,
}

impl Provider {
    /// The default app first, then the others.
    pub fn apps(&self) -> Vec<ProviderApp> {
        let default = ProviderApp {
            name: self.app_name.clone().unwrap_or_else(|| self.name.clone()),
            android_package: self.android_package.clone(),
        };
        std::iter::once(default)
            .chain(self.other_apps.iter().cloned())
            .collect()
    }
}

/// Every provider the app compares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalogue {
    pub providers: Vec<Provider>,
}

#[derive(Debug, Error)]
pub enum CatalogueError {
    #[error("the catalogue is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("provider `{0}` appears more than once")]
    DuplicateProvider(String),
    #[error("offer `{0}` appears more than once")]
    DuplicateOffer(String),
    #[error("provider `{0}` has no offers")]
    NoOffers(String),
    #[error("offer `{0}` has no rate cards")]
    NoCards(String),
    #[error("offer `{offer}` has two rate cards labelled `{label}`")]
    DuplicateCardLabel { offer: String, label: String },
    #[error("app `{0}` is listed twice for one provider")]
    DuplicateApp(String),
    #[error("offer `{0}` has a negative price")]
    NegativePrice(String),
}

impl Catalogue {
    /// Parses and validates a catalogue.
    pub fn from_json(json: &str) -> Result<Catalogue, CatalogueError> {
        let catalogue: Catalogue = serde_json::from_str(json)?;
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn validate(&self) -> Result<(), CatalogueError> {
        let mut provider_ids = HashSet::new();
        let mut offer_ids = HashSet::new();
        for provider in &self.providers {
            if !provider_ids.insert(&provider.id) {
                return Err(CatalogueError::DuplicateProvider(provider.id.0.clone()));
            }
            if provider.offers.is_empty() {
                return Err(CatalogueError::NoOffers(provider.id.0.clone()));
            }
            let mut packages = HashSet::new();
            for app in provider.apps() {
                if !packages.insert(app.android_package.clone()) {
                    return Err(CatalogueError::DuplicateApp(app.android_package));
                }
            }
            for offer in &provider.offers {
                if !offer_ids.insert(&offer.id) {
                    return Err(CatalogueError::DuplicateOffer(offer.id.0.clone()));
                }
                validate_cards(&offer.id, &offer.cards)?;
            }
        }
        Ok(())
    }

    pub fn provider(&self, id: &ProviderId) -> Option<&Provider> {
        self.providers.iter().find(|provider| &provider.id == id)
    }
}

/// Checks the rate cards of one offer; also used for personal overrides.
pub fn validate_cards(offer: &OfferId, cards: &[RateCard]) -> Result<(), CatalogueError> {
    if cards.is_empty() {
        return Err(CatalogueError::NoCards(offer.0.clone()));
    }
    let mut labels = HashSet::new();
    for card in cards {
        if !labels.insert(&card.label) {
            return Err(CatalogueError::DuplicateCardLabel {
                offer: offer.0.clone(),
                label: card.label.clone(),
            });
        }
        let prices = [card.base_fee, card.per_km, card.per_minute, card.minimum];
        if prices.iter().any(|price| *price < Cents::ZERO) {
            return Err(CatalogueError::NegativePrice(offer.0.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use crate::rate_card::fixtures::*;

    pub fn catalogue() -> Catalogue {
        Catalogue {
            providers: vec![
                Provider {
                    id: ProviderId("miles".into()),
                    name: "MILES".into(),
                    android_package: "com.driveby.app".into(),
                    app_name: None,
                    other_apps: Vec::new(),
                    offers: vec![Offer {
                        id: OfferId("miles.s".into()),
                        name: "S".into(),
                        cards: vec![per_km_card(), package_card()],
                    }],
                },
                Provider {
                    id: ProviderId("bolt".into()),
                    name: "Bolt Drive".into(),
                    android_package: "ee.mtakso.client".into(),
                    app_name: None,
                    other_apps: Vec::new(),
                    offers: vec![Offer {
                        id: OfferId("bolt.corsa".into()),
                        name: "Opel Corsa".into(),
                        cards: vec![per_minute_and_km_card()],
                    }],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::catalogue;
    use super::*;

    #[test]
    fn apps_list_the_default_first_named_after_the_provider_unless_named() {
        let json = r#"{"providers":[{"id":"sixt","name":"SIXT share","android_package":"com.trafi.whitelabel.bvg",
            "app_name":"BVG Jelbi","other_apps":[{"name":"SIXT","android_package":"com.sixt.reservation"}],
            "offers":[{"id":"sixt.a","name":"A","cards":[]}]}]}"#;
        // Cards are irrelevant here; build the provider directly from JSON without validation.
        let catalogue: Catalogue = serde_json::from_str(json).unwrap();
        let apps = catalogue.providers[0].apps();
        assert_eq!(
            apps[0],
            ProviderApp {
                name: "BVG Jelbi".into(),
                android_package: "com.trafi.whitelabel.bvg".into()
            }
        );
        assert_eq!(apps[1].android_package, "com.sixt.reservation");
        let miles = &fixtures::catalogue().providers[0];
        assert_eq!(
            miles.apps(),
            vec![ProviderApp {
                name: "MILES".into(),
                android_package: "com.driveby.app".into()
            }]
        );
    }

    #[test]
    fn an_app_listed_twice_is_refused() {
        let mut catalogue = fixtures::catalogue();
        catalogue.providers[0].other_apps = vec![ProviderApp {
            name: "Again".into(),
            android_package: catalogue.providers[0].android_package.clone(),
        }];
        assert!(matches!(
            catalogue.validate(),
            Err(CatalogueError::DuplicateApp(_))
        ));
    }

    #[test]
    fn older_catalogues_without_apps_still_load() {
        let catalogue = fixtures::catalogue();
        let json = serde_json::to_string(&catalogue).unwrap();
        assert!(!json.contains("other_apps"), "empty lists are not written");
        assert_eq!(Catalogue::from_json(&json).unwrap(), catalogue);
    }

    #[test]
    fn valid_catalogue_round_trips_through_json() {
        let json = serde_json::to_string(&catalogue()).unwrap();
        assert_eq!(Catalogue::from_json(&json).unwrap(), catalogue());
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(matches!(
            Catalogue::from_json("{"),
            Err(CatalogueError::Json(_))
        ));
    }

    #[test]
    fn rejects_duplicate_providers() {
        let mut c = catalogue();
        c.providers.push(c.providers[0].clone());
        assert!(
            matches!(c.validate(), Err(CatalogueError::DuplicateProvider(id)) if id == "miles")
        );
    }

    #[test]
    fn rejects_offer_ids_repeated_across_providers() {
        let mut c = catalogue();
        c.providers[1].offers[0].id = OfferId("miles.s".into());
        assert!(matches!(c.validate(), Err(CatalogueError::DuplicateOffer(id)) if id == "miles.s"));
    }

    #[test]
    fn rejects_provider_without_offers() {
        let mut c = catalogue();
        c.providers[1].offers.clear();
        assert!(matches!(c.validate(), Err(CatalogueError::NoOffers(id)) if id == "bolt"));
    }

    #[test]
    fn rejects_offer_without_cards() {
        let mut c = catalogue();
        c.providers[1].offers[0].cards.clear();
        assert!(matches!(c.validate(), Err(CatalogueError::NoCards(id)) if id == "bolt.corsa"));
    }

    #[test]
    fn rejects_duplicate_card_labels() {
        let mut c = catalogue();
        let card = c.providers[1].offers[0].cards[0].clone();
        c.providers[1].offers[0].cards.push(card);
        assert!(matches!(
            c.validate(),
            Err(CatalogueError::DuplicateCardLabel { .. })
        ));
    }

    #[test]
    fn rejects_negative_prices() {
        let mut c = catalogue();
        c.providers[1].offers[0].cards[0].per_km = Cents(-1);
        assert!(
            matches!(c.validate(), Err(CatalogueError::NegativePrice(id)) if id == "bolt.corsa")
        );
    }

    #[test]
    fn finds_a_provider_by_id() {
        let c = catalogue();
        assert_eq!(
            c.provider(&ProviderId("bolt".into())).unwrap().name,
            "Bolt Drive"
        );
        assert!(c.provider(&ProviderId("sixt".into())).is_none());
    }
}
