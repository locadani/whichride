//! The core for the website, through wasm-bindgen. Logic lives in `prices` and is tested
//! natively; this layer only converts to and from JavaScript values (as `crates/ffi` for UniFFI).

mod prices;

pub use prices::{OfferInfo, Priced, ProviderInfo, Ranking, WebPrices};

use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Prices(WebPrices);

#[wasm_bindgen]
impl Prices {
    #[wasm_bindgen(js_name = fromServer)]
    pub fn from_server(json: &str) -> Result<Prices, JsError> {
        WebPrices::from_server(json)
            .map(Prices)
            .map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = fromSeed)]
    pub fn from_seed(json: &str) -> Result<Prices, JsError> {
        WebPrices::from_seed(json)
            .map(Prices)
            .map_err(|e| JsError::new(&e))
    }

    pub fn providers(&self) -> Result<JsValue, JsError> {
        to_js(&self.0.providers())
    }

    pub fn offers(&self, metres: u32, seconds: u32) -> Result<JsValue, JsError> {
        to_js(&self.0.offers(metres, seconds))
    }

    pub fn options(
        &self,
        metres: u32,
        seconds: u32,
        offer_ids: Vec<String>,
    ) -> Result<JsValue, JsError> {
        to_js(&self.0.options(metres, seconds, &offer_ids))
    }
}

fn to_js(value: &impl Serialize) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsError::new(&e.to_string()))
}
