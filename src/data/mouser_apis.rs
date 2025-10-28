use std::{time::Duration};
use dotenvy::dotenv;
use reqwest::{Client, Response};
use tokio::time::sleep;
use crate::models::mouser_api_models::{
    MouserPart,
    KeywordSearchRequest,
    InnerRequest,
    MouserResponse,
};
use serde_path_to_error::deserialize;

pub async fn search_mouser(
    query_manufacturer: &str,
    query_manufacturer_pn: &str,
    quantity: u32,
) -> Result<Option<MouserPart>, Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    let api_key = std::env::var("MOUSER_API_KEY").expect("MOUSER_API_KEY must be set");
    println!("Searching for {} {} on Mouser", query_manufacturer, query_manufacturer_pn);
    let url = format!(
        "https://api.mouser.com/api/v1/search/keyword?apiKey={}",
        api_key
    );
    let request_body = KeywordSearchRequest {
        request: InnerRequest {
            keyword: format!("{} {}", query_manufacturer, query_manufacturer_pn).into(),
            records: 0,
            starting_record: 0,
            search_options: "".into(),
            search_with_your_sign_up_language: "".into(),
        },
    };
    let client = Client::new();
    let mut search_response: Response;

    let mut attempts = 0;
    let max_attempts = 15;
    loop {
        search_response = client
        .post(&url)
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .timeout(Duration::from_secs(100))
        .send()
        .await?;

        if !search_response.status().is_success() {
            println!("Failed to search Mouser: code {:?}", search_response.status());
        } else {
            break;
        }
        if attempts >= max_attempts {
            break;
        } else {
            attempts += 1;
            println!("Retrying Mouser search, attempt {}", attempts);
            // slow down api call rate exponentially to prevent limiting from mouser -> 403
            sleep(Duration::from_millis(2u64.pow(attempts))).await;
            continue;
        }
    }

    let bytes = search_response.bytes().await?;

    //let json: serde_json::Value = serde_json::from_slice(&bytes)?;

    // Serialize the JSON with pretty formatting
    //let pretty = serde_json::to_string_pretty(&json)?;

    // Write to a file
    //let mut file = File::create("mouser_response.json")?;
    //file.write_all(pretty.as_bytes())?;

    let response: MouserResponse;

    let mut de = serde_json::Deserializer::from_slice(&bytes);
    match deserialize::<_, MouserResponse>(&mut de) {
        Ok(result) => {
            response = result;
        },
        Err(e) => {
            println!("❌ Path error: {}", e);
            // return Err(format!("Error parsing JSON: {}", e).into());
            return Box::pin(search_mouser(query_manufacturer, query_manufacturer_pn, quantity)).await;
        }
    };

    match response.search_results {
        Some(search_results) => {
            for part in search_results.parts {
                let manufacturer = part.manufacturer.unwrap_or_default();
                let manufacturer_pn = part.manufacturer_part_number.unwrap_or_default();
                // assure we return only the requested item
                if manufacturer_pn != query_manufacturer_pn {
                    continue;
                }
                let mouser_part = MouserPart {
                    manufacturer: manufacturer,
                    manufacturer_pn: manufacturer_pn,
                    description: part.description.unwrap_or_default(),
                    mouser_pn: part.mouser_part_number.unwrap_or_default(),
                    product_url: part.product_detail_url.unwrap_or_default(),
                    unit_price: match part.price_breaks {
                        Some(price_breaks) => {
                            let mut unit_price = 0.0;
                            for price in price_breaks {
                                if quantity >= price.Quantity {
                                    unit_price = price.Price
                                        .strip_suffix(" €")
                                        .unwrap_or("0.0")
                                        .replace(",", ".")
                                        .parse::<f64>()
                                        .unwrap_or(0.0);
                                }
                            }
                            unit_price
                        },
                        None => {
                            return Ok(None);
                        }
                    },
                    availability: part.availability
                        .clone()
                        .unwrap_or_default()
                        .strip_suffix(" In Stock")
                        .unwrap_or_default()
                        .parse::<u32>()
                        .unwrap_or_default(),
                };
                return Ok(Some(mouser_part));
            }
            Ok(None)
        },
        None => Ok(None),
    }
}
