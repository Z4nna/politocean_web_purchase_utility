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

pub async fn digikey_search(
    query_manufacturer: &str, 
    query_manufacturer_pn: &str, 
    quantity: u32
) -> Result<Result<DigiKeyPart, String>, Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    let client_id = std::env::var("DIGIKEY_CLIENT_ID").expect("DIGIKEY_CLIENT_ID not set");
    println!("Searching for {} {} on Digikey", query_manufacturer, query_manufacturer_pn);

    let token = digikey_get_token().await?;
    let client = Client::new();
    // Step 2: Perform product search
    let url = format!("https://api.digikey.com/products/v4/search/keyword");

    // The search is by keyword and paged, so the requested part is not guaranteed
    // to be on the first page: keep fetching until it shows up or results run out.
    let mut products: Vec<Product> = Vec::new();
    let mut offset = 0;
    loop {
        let request_body = DigiKeyRequestBody {
            keywords: format!("{} {}", query_manufacturer, query_manufacturer_pn).into(),
            limit: PAGE_SIZE,
            offset,
            filter_options_request: FilterOptionsRequest {
                // Stock is checked below instead, so that a part that cannot be bought
                // is still returned and we can tell why.
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
            .header("X-DIGIKEY-Client-Id", &client_id)
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
        let mut de = serde_json::Deserializer::from_slice(&bytes);
        let page: DigiKeySearchResult = match deserialize(&mut de) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("❌ Path error: {}", e);
                return Err(format!("Error parsing JSON: {}", e).into());
            }
        };

        let page_len = page.products.len() as u32;
        offset += page_len;
        products.extend(page.products);
        // Parts Digikey itself flags as exact matches, wherever they rank.
        products.extend(page.exact_matches);

        if has_part(&products, query_manufacturer_pn)
            || page_len == 0
            || offset >= page.products_count
            || offset >= MAX_RESULTS_SCANNED
        {
            break;
        }
    }

    let (best_product, best_variation) = match pick_variation(&products, query_manufacturer_pn, quantity) {
        Ok(pair) => pair,
        Err(reason) => return Ok(Err(reason)),
    };

    let product = DigiKeyPart {
        manufacturer: best_product.manufacturer.name.clone(),
        manufacturer_pn: best_product.manufacturer_product_number.clone(),
        description: best_product.description.product_description.clone(),
        digikey_pn: best_variation.digi_key_product_number.clone(),
        product_url: best_product.product_url.clone(),
        unit_price: best_variation.get_price(quantity).unwrap_or(0.0),
        availability: best_product.quantity_available,
    };

    Ok(Ok(product))
}

const PAGE_SIZE: u32 = 20;
// Note: stop after 5 pages so a part Digikey does not carry costs at most 5
// calls; a part ranked below 100 keyword results is reported as not found.
const MAX_RESULTS_SCANNED: u32 = 100;

/// Whether the requested part number (manufacturer's or Digikey's) is among `products`.
fn has_part(products: &[Product], pn: &str) -> bool {
    products.iter().any(|p| p.manufacturer_product_number == pn || p.product_variations.iter().any(|v| v.digi_key_product_number == pn))
}

/// Picks the cheapest packaging of the requested part that can be bought in
/// `quantity` pieces right now, or a short reason why none can.
fn pick_variation<'a>(products: &'a [Product], pn: &str, quantity: u32) -> Result<(&'a Product, &'a ProductVariation), String> {
    let price = |v: &ProductVariation| v.get_price(quantity).unwrap_or(0.0);
    let candidates: Vec<(&Product, &ProductVariation)> = products
        .iter()
        .flat_map(|p| p.product_variations.iter().map(move |v| (p, v)))
        .filter(|(p, v)| p.manufacturer_product_number == pn || v.digi_key_product_number == pn)
        .collect();
    let Some((product, _)) = candidates.first() else {
        return Err("part number not found".to_string());
    };

    let best = candidates
        .iter()
        .filter(|(_, v)| v.quantity_availablefor_package_type >= quantity && v.minimum_order_quantity <= quantity && price(v) > 0.0)
        .min_by(|(_, v1), (_, v2)| price(v1).partial_cmp(&price(v2)).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(best) = best {
        return Ok(*best);
    }

    let status = product.product_status.as_ref().map(|s| s.status.as_str()).filter(|s| !s.is_empty() && *s != "Active");
    let in_stock: Vec<&ProductVariation> = candidates.iter().map(|(_, v)| *v).filter(|v| v.quantity_availablefor_package_type >= quantity).collect();
    if in_stock.is_empty() {
        let max_stock = candidates.iter().map(|(_, v)| v.quantity_availablefor_package_type).max().unwrap_or(0);
        return Err(match (product.discontinued || product.end_of_life, status, max_stock) {
            (true, status, _) => status.unwrap_or("discontinued").to_lowercase(),
            (_, Some(status), 0) => format!("not in stock ({})", status),
            (_, None, 0) => "not in stock".to_string(),
            (_, _, n) => format!("only {} in stock", n),
        });
    }
    match in_stock.iter().map(|v| v.minimum_order_quantity).min() {
        Some(min) if min > quantity => Err(format!("minimum order quantity is {}", min)),
        _ => Err("no price available".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::digikey_api_models::{Manufacturer, PriceBreak, ProductDescription, ProductStatus};

    fn variation(dk_pn: &str, stock: u32, moq: u32, unit_price: f64) -> ProductVariation {
        ProductVariation {
            digi_key_product_number: dk_pn.to_string(),
            standard_pricing: Some(vec![PriceBreak { break_quantity: moq, unit_price, total_price: unit_price * moq as f64 }]),
            quantity_availablefor_package_type: stock,
            minimum_order_quantity: moq,
        }
    }

    fn product(variations: Vec<ProductVariation>) -> Product {
        Product {
            description: ProductDescription { product_description: String::new(), detailed_description: String::new() },
            manufacturer: Manufacturer { id: 1, name: "Texas Instruments".to_string() },
            manufacturer_product_number: "LM358P".to_string(),
            product_url: String::new(),
            datasheet_url: None,
            quantity_available: variations.iter().map(|v| v.quantity_availablefor_package_type).sum(),
            product_variations: variations,
            product_status: None,
            discontinued: false,
            end_of_life: false,
        }
    }

    #[test]
    fn picks_cheapest_buyable_variation_or_explains() {
        let reason = |products: &[Product], qty| pick_variation(products, "LM358P", qty).map(|(_, v)| v.digi_key_product_number.clone());

        let both = [product(vec![variation("TUBE", 500, 1, 0.40), variation("REEL", 5000, 2500, 0.10)])];
        assert_eq!(reason(&both, 10).unwrap(), "TUBE");
        assert_eq!(reason(&both, 3000).unwrap(), "REEL");

        assert!(has_part(&both, "LM358P") && has_part(&both, "REEL") && !has_part(&both, "LM358"));
        assert_eq!(reason(&[], 10).unwrap_err(), "part number not found");
        assert_eq!(reason(&[product(vec![variation("REEL", 5000, 2500, 0.10)])], 10).unwrap_err(), "minimum order quantity is 2500");
        assert_eq!(reason(&[product(vec![variation("TUBE", 4, 1, 0.40)])], 10).unwrap_err(), "only 4 in stock");
        assert_eq!(reason(&[product(vec![variation("TUBE", 0, 1, 0.40)])], 10).unwrap_err(), "not in stock");

        let mut obsolete = product(vec![variation("TUBE", 0, 1, 0.40)]);
        obsolete.discontinued = true;
        obsolete.product_status = Some(ProductStatus { status: "Obsolete".to_string() });
        assert_eq!(reason(&[obsolete], 10).unwrap_err(), "obsolete");
    }
}
