use dotenvy::dotenv;
use reqwest::Client;
use crate::models::mouser_api_models::{
    MouserPart,
    KeywordSearchRequest,
    InnerRequest,
    MouserResponse,
};

pub async fn search_mouser(
    query_manufacturer: &str,
    query_manufacturer_pn: &str,
    quantity: u32,
) -> Result<Option<MouserPart>, Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = std::env::var("MOUSER_API_KEY").expect("MOUSER_API_KEY must be set");
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
    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await?
        .json::<MouserResponse>()
        .await?;
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