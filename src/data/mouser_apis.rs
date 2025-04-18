use serde::{Deserialize, Serialize};
use dotenvy::dotenv;
use reqwest::Client;

#[derive(Serialize)]
struct KeywordSearchRequest {
    #[serde(rename = "SearchByKeywordRequest")]
    request: InnerRequest,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct InnerRequest {
    keyword: String,
    records: u32,
    startingRecord: u32,
    searchOptions: String,
    searchWithYourSignUpLanguage: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct MouserResponse {
    SearchResults: Option<SearchResults>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct SearchResults {
    Parts: Vec<Part>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Part {
    pub Manufacturer: Option<String>,
    pub ManufacturerPartNumber: Option<String>,
    pub Description: Option<String>,
    pub MouserPartNumber: Option<String>,
    pub ProductDetailUrl: Option<String>,
    pub PriceBreaks: Option<Vec<PriceBreak>>,
    pub Availability: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MouserPart {
    pub manufacturer: String,
    pub manufacturer_pn: String,
    pub description: String,
    pub mouser_pn: String,
    pub product_url: String,
    pub unit_price: f64,
    pub availability: u32,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct PriceBreak {
    pub Quantity: u32,
    pub Price: String,
    pub Currency: String,
}

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
            startingRecord: 0,
            searchOptions: "".into(),
            searchWithYourSignUpLanguage: "".into(),
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
    match response.SearchResults {
        Some(search_results) => {
            for part in search_results.Parts {
                let manufacturer = part.Manufacturer.unwrap_or_default();
                let manufacturer_pn = part.ManufacturerPartNumber.unwrap_or_default();
                // assure we return only the requested item
                if manufacturer != query_manufacturer || manufacturer_pn != query_manufacturer_pn {
                    continue;
                }
                let mouser_part = MouserPart {
                    manufacturer: manufacturer,
                    manufacturer_pn: manufacturer_pn,
                    description: part.Description.unwrap_or_default(),
                    mouser_pn: part.MouserPartNumber.unwrap_or_default(),
                    product_url: part.ProductDetailUrl.unwrap_or_default(),
                    unit_price: match part.PriceBreaks {
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
                    availability: part.Availability
                        .clone()
                        .unwrap_or_default()
                        .strip_suffix(" In Stock")
                        .unwrap_or_default()
                        .parse::<u32>()
                        .unwrap_or_default(),
                };
                println!("Availability: {:?}", part.Availability.as_ref());
                return Ok(Some(mouser_part));
            }
            Ok(None)
        },
        None => Ok(None),
    }
}