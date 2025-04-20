use core::f64;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use dotenvy::dotenv;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct DigiKeyRequestBody {
    keywords: String,
    limit: u32,
    offset: u32,
    filter_options_request: FilterOptionsRequest,
    sort_options: SortOptions,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SortOptions {
    field: String,
    sort_order: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct FilterOptionsRequest {
    minimum_quantity_available: u32,
    market_place_filter: String,

}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct DigiKeySearchResult {
    products: Vec<Product>,
    products_count: u32,
    exact_matches: Vec<Product>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub description: ProductDescription,
    pub manufacturer: Manufacturer,
    pub manufacturer_product_number: String,
    pub product_url: String,
    pub datasheet_url: String,
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
    fn get_price(&self, quantity: u32) -> Option<f64> {
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

async fn digikey_get_token() -> Result<String, Box<dyn std::error::Error>> {
    dotenv().ok();
    let client_id = std::env::var("DIGIKEY_CLIENT_ID").expect("DIGIKEY_CLIENT_ID not set");
    let client_secret = std::env::var("DIGIKEY_CLIENT_SECRET").expect("DIGIKEY_CLIENT_SECRET not set");

    let client = Client::new();

    let token_response = client
        .post("https://api.digikey.com/v1/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("client_id={client_id}&client_secret={client_secret}&grant_type=client_credentials"))
        .send()
        .await?;

    if !token_response.status().is_success() {
        return Err(format!("Failed to get token: {:?}", token_response.text().await?).into());
    }

    let token: TokenResponse = token_response.json().await?;
    Ok(token.access_token)
}

pub async fn digikey_search(query_manufacturer: &str, query_manufacturer_pn: &str, quantity: u32,) -> Result<Option<DigiKeyPart>, Box<dyn std::error::Error>> {
    dotenv().ok();
    let client_id = std::env::var("DIGIKEY_CLIENT_ID").expect("DIGIKEY_CLIENT_ID not set");
    println!("Searching for {} {}", query_manufacturer, query_manufacturer_pn);
    // future addition: check if old token is still valid, and in this case use the old one
    let token = digikey_get_token().await?;
    println!("Got token.");
    let client = Client::new();
    // Step 2: Perform product search
    let url = format!("https://api.digikey.com/products/v4/search/keyword");

    let request_body = DigiKeyRequestBody {
        keywords: format!("{} {}", query_manufacturer, query_manufacturer_pn).into(),
        limit: 20,
        offset: 0,
        filter_options_request: FilterOptionsRequest {
            minimum_quantity_available: 0,
            market_place_filter: "NoFilter".to_string(),
        },
        sort_options: SortOptions {
            field: "None".to_string(),
            sort_order: "Ascending".to_string(),
        },
    };

    let search_response = client
        .post(&url)
        .header("X-DIGIKEY-Client-Id", client_id)
        .header("X-DIGIKEY-Locale-Language", "en")
        .header("X-DIGIKEY-Locale-Currency", "EUR")
        .header("X-DIGIKEY-Locale-Site", "IT")
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;
    println!("Sent search request");

    if !search_response.status().is_success() {
        return Err(format!("Failed to search DigiKey: {:?}", search_response.status()).into());
    }
    println!("Search successful");

    let myresponse = search_response.json::<DigiKeySearchResult>().await?;
    println!("Response parsed");

    let mut possible_products: Vec<Product> = Vec::new();
    for product in myresponse.products {
        if &product.manufacturer_product_number == query_manufacturer_pn {
            possible_products.push(product);
        }
    }

    if possible_products.len() > 1 {
        // handle multiple products with the same manufacturer part number
    }

    // choose the right product variation, by selecting the variation with the lowest price and enough availability
    let best_product = if possible_products.len() > 0 {
        possible_products[0].clone()
    } else {
        return Ok(None);
    };
    let variation = match best_product
    .product_variations
    .iter()
    .filter(|v| v.quantity_availablefor_package_type >= quantity && v.minimum_order_quantity <= quantity)
    .min() {
        Some(v) => v,
        None => {
            return Ok(None);
        }
    };
    
    let product = DigiKeyPart {
        manufacturer: possible_products[0].manufacturer.name.clone(),
        manufacturer_pn: possible_products[0].manufacturer_product_number.clone(),
        description: possible_products[0].description.product_description.clone(),
        digikey_pn: variation.digi_key_product_number.clone(),
        product_url: possible_products[0].product_url.clone(),
        unit_price: variation.get_price(quantity).unwrap_or(0.0),
        availability: possible_products[0].quantity_available,
    };
    Ok(Some(product))
}