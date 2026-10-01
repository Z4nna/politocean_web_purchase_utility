use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct KeywordSearchRequest {
    #[serde(rename = "SearchByKeywordRequest")]
    pub request: InnerRequest,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerRequest {
    pub keyword: String,
    pub records: u32,
    pub starting_record: u32,
    pub search_options: String,
    pub search_with_your_sign_up_language: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MouserResponse {
    pub search_results: Option<SearchResults>,
    // API-level problems (bad key, rate limit) are reported here, in a 200 response.
    pub errors: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct SearchResults {
    pub parts: Vec<Part>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Part {
    pub manufacturer: Option<String>,
    pub manufacturer_part_number: Option<String>,
    pub description: Option<String>,
    pub mouser_part_number: Option<String>,
    pub product_detail_url: Option<String>,
    pub price_breaks: Option<Vec<PriceBreak>>,
    pub availability: Option<String>,
    pub availability_in_stock: Option<String>,
    pub lifecycle_status: Option<String>,
    // Sent as the string "true"/"false"; kept loose so a format change cannot break parsing.
    pub is_discontinued: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
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