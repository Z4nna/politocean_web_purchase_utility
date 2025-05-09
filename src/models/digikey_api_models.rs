use core::f64;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DigiKeyRequestBody {
    pub keywords: String,
    pub limit: u32,
    pub offset: u32,
    pub filter_options_request: FilterOptionsRequest,
    pub sort_options: SortOptions,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SortOptions {
    pub field: String,
    pub sort_order: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FilterOptionsRequest {
    pub minimum_quantity_available: u32,
    pub market_place_filter: String,

}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DigiKeySearchResult {
    pub products: Vec<Product>,
    pub products_count: u32,
    pub exact_matches: Vec<Product>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub description: ProductDescription,
    pub manufacturer: Manufacturer,
    pub manufacturer_product_number: String,
    pub product_url: String,
    pub datasheet_url: Option<String>,
    pub quantity_available: u32,
    pub product_variations: Vec<ProductVariation>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ProductDescription {
    pub product_description: String,
    pub detailed_description: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Manufacturer {
    pub id: i32,
    pub name: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ProductVariation {
    pub digi_key_product_number: String,
    pub standard_pricing: Option<Vec<PriceBreak>>,
    pub quantity_availablefor_package_type: u32,
    pub minimum_order_quantity: u32,
}

impl ProductVariation {
    pub fn get_price(&self, quantity: u32) -> Option<f64> {
        match self.clone().standard_pricing {
            Some(price_breaks) => {
                let mut unit_price = 0.0;
                for price in price_breaks {
                    if quantity >= price.break_quantity {
                        unit_price = price.unit_price
                    }
                }
                Some(unit_price)
            },
            None => {
                return None;
            }
        }
    }
}

impl PartialEq for ProductVariation {
    fn eq(&self, other: &Self) -> bool {
        self.get_price(0) == other.get_price(0)
    }
}

impl Eq for ProductVariation {}

impl PartialOrd for ProductVariation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ProductVariation {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_price = self.get_price(0).unwrap_or(f64::INFINITY);
        let other_price = other.get_price(0).unwrap_or(f64::INFINITY);

        self_price
            .partial_cmp(&other_price)
            .unwrap_or(Ordering::Equal)
    }
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PriceBreak {
    pub break_quantity: u32,
    pub unit_price: f64,
    pub total_price: f64,
}

#[derive(Deserialize, Debug)]
pub struct DigiKeyPart {
    pub manufacturer: String,
    pub manufacturer_pn: String,
    pub description: String,
    pub digikey_pn: String,
    pub product_url: String,
    pub unit_price: f64,
    pub availability: u32,
}