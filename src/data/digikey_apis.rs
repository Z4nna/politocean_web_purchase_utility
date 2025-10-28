use reqwest::Client;
use tokio::{sync::RwLock, time::Instant};
use std::{sync::Arc, time::Duration};
use dotenvy::dotenv;
use crate::models::digikey_api_models::{
    DigiKeyPart, DigiKeyRequestBody, DigiKeySearchResult, FilterOptionsRequest, Product, ProductVariation, SortOptions, TokenResponse
};
use serde_path_to_error::deserialize;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
struct TokenCache {
    token: String,
    expires_at: Instant,
}

static DIGIKEY_TOKEN: Lazy<Arc<RwLock<Option<TokenCache>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

async fn digikey_get_token() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    {
        let token_lock = DIGIKEY_TOKEN.read().await;

        if let Some(ref token_cache) = *token_lock {
            if Instant::now() < token_cache.expires_at {
                return Ok(token_cache.token.clone());
            }
        }
    }

    let mut token_lock = DIGIKEY_TOKEN.write().await;

    dotenv().ok();

    let client_id = std::env::var("DIGIKEY_CLIENT_ID")?;
    let client_secret = std::env::var("DIGIKEY_CLIENT_SECRET")?;

    println!("🔐 Fetching new Digi-Key token...");

    let client = Client::new();
    let token_response = client
        .post("https://api.digikey.com/v1/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "client_id={}&client_secret={}&grant_type=client_credentials",
            client_id, client_secret
        ))
        .send()
        .await?;

    if !token_response.status().is_success() {
        return Err(format!(
            "Failed to get Digi-Key token: {}",
            token_response.text().await?
        )
        .into());
    }

    let token: TokenResponse = token_response.json().await?;

    let expires_in = token.expires_in;
    *token_lock = Some(TokenCache {
        token: token.access_token.clone(),
        expires_at: Instant::now() + Duration::from_secs(expires_in.saturating_sub(60)),
    });

    Ok(token.access_token)
}

pub async fn digikey_search(query_manufacturer: &str, query_manufacturer_pn: &str, quantity: u32,) -> Result<Option<DigiKeyPart>, Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    let client_id = std::env::var("DIGIKEY_CLIENT_ID").expect("DIGIKEY_CLIENT_ID not set");
    println!("Searching for {} {} on Digikey", query_manufacturer, query_manufacturer_pn);

    let token = digikey_get_token().await?;
    let client = Client::new();
    // Step 2: Perform product search
    let url = format!("https://api.digikey.com/products/v4/search/keyword");

    let request_body = DigiKeyRequestBody {
        keywords: format!("{} {}", query_manufacturer, query_manufacturer_pn).into(),
        limit: 20,
        offset: 0,
        filter_options_request: FilterOptionsRequest {
            minimum_quantity_available: quantity,
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
        .timeout(Duration::from_secs(100))
        .send()
        .await?;

    if !search_response.status().is_success() {
        return Err(format!("Failed to search DigiKey: code {:?}", search_response.status()).into());
    }

    let bytes = search_response.bytes().await?;

    //let json: serde_json::Value = serde_json::from_slice(&bytes)?;

    // Serialize the JSON with pretty formatting
    //let pretty = serde_json::to_string_pretty(&json)?;

    // Write to a file
    //let mut file = File::create("digikey_response_pretty.json")?;
    //file.write_all(pretty.as_bytes())?;

    let myresponse: DigiKeySearchResult;

    let mut de = serde_json::Deserializer::from_slice(&bytes);
    match deserialize::<_, DigiKeySearchResult>(&mut de) {
        Ok(result) => {
            myresponse = result;
        },
        Err(e) => {
            eprintln!("❌ Path error: {}", e);
            return Err(format!("Error parsing JSON: {}", e).into());
        }
    };

    let possible_products: Vec<Product> = myresponse
    .products
    .into_iter()
    .filter(|p| p.manufacturer_product_number == *query_manufacturer_pn)
    .collect();

    if possible_products.is_empty() {
        return Ok(None);
    }

    let mut valid_variations: Vec<(&Product, &ProductVariation)> = Vec::new();
    for product in &possible_products {
        for variation in &product.product_variations {
            if variation.quantity_availablefor_package_type >= quantity
                && variation.minimum_order_quantity <= quantity
            {
                valid_variations.push((product, variation));
            }
        }
    }

    if valid_variations.is_empty() {
        return Ok(None);
    }

    let best_pair = valid_variations
        .into_iter()
        .min_by(|(_, v1), (_, v2)| {
            let p1 = v1.get_price(quantity).unwrap_or(f64::INFINITY);
            let p2 = v2.get_price(quantity).unwrap_or(f64::INFINITY);
            p1.partial_cmp(&p2).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    let (best_product, best_variation) = best_pair;

    let product = DigiKeyPart {
        manufacturer: best_product.manufacturer.name.clone(),
        manufacturer_pn: best_product.manufacturer_product_number.clone(),
        description: best_product.description.product_description.clone(),
        digikey_pn: best_variation.digi_key_product_number.clone(),
        product_url: best_product.product_url.clone(),
        unit_price: best_variation.get_price(quantity).unwrap_or(0.0),
        availability: best_product.quantity_available,
    };

    Ok(Some(product))
}
