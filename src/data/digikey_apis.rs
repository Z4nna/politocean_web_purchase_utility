use reqwest::Client;
use std::{fs::File, io::Write, time::Duration};
use dotenvy::dotenv;
use crate::models::digikey_api_models::{
    TokenResponse,
    DigiKeyPart,
    DigiKeyRequestBody,
    FilterOptionsRequest,
    SortOptions,
    DigiKeySearchResult,
    Product
};
use serde_path_to_error::deserialize;

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
    println!("Searching for {} {} on Digikey", query_manufacturer, query_manufacturer_pn);
    // future addition: check if old token is still valid, and in this case use the old one
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
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    println!("Sent digikey search request successfully");

    if !search_response.status().is_success() {
        return Err(format!("Failed to search DigiKey: code {:?}", search_response.status()).into());
    }
    println!("Search successful");

    let bytes = search_response.bytes().await?;

    let json: serde_json::Value = serde_json::from_slice(&bytes)?;

    // Serialize the JSON with pretty formatting
    let pretty = serde_json::to_string_pretty(&json)?;

    // Write to a file
    let mut file = File::create("digikey_response_pretty.json")?;
    file.write_all(pretty.as_bytes())?;

    let myresponse: DigiKeySearchResult;
    /*
    match serde_json::from_slice(&bytes) {
        Ok(response) => {
            myresponse = response;
        }
        Err(e) => {
            println!("Error parsing JSON: {}", e);
            return Err(format!("Error parsing JSON: {}", e).into());
        }
    }*/

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
    //let myresponse = search_response.json::<DigiKeySearchResult>().await?;
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
