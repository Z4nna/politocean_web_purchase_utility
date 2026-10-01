use std::{time::Duration};
use dotenvy::dotenv;
use reqwest::{Client, Response};
use tokio::time::sleep;
use crate::models::mouser_api_models::{
    MouserPart,
    KeywordSearchRequest,
    InnerRequest,
    MouserResponse,
    Part,
};
use serde_path_to_error::deserialize;

pub async fn search_mouser(
    query_manufacturer: &str,
    query_manufacturer_pn: &str,
    quantity: u32,
) -> Result<Result<MouserPart, String>, Box<dyn std::error::Error + Send + Sync>> {
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
    let max_attempts = 16;
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
            // The HTTP retries above are already exhausted (or the body is not a
            // search result): report the failure instead of retrying forever.
            return Err(format!("Error parsing Mouser response: {}", e).into());
        }
    };

    if let Some(error) = response.errors.as_ref().and_then(|errors| errors.first()) {
        return Err(format!("Mouser API error: {}", error).into());
    }

    // Several listings can share a manufacturer part number (e.g. reel / cut tape):
    // take the first one that can be bought, otherwise report why the first cannot.
    let mut reason: Option<String> = None;
    for part in response.search_results.map(|r| r.parts).unwrap_or_default() {
        // assure we return only the requested item
        if part.manufacturer_part_number.as_deref() != Some(query_manufacturer_pn)
            && part.mouser_part_number.as_deref() != Some(query_manufacturer_pn) {
            continue;
        }
        match evaluate_part(part, quantity) {
            Ok(mouser_part) => return Ok(Ok(mouser_part)),
            Err(e) => { reason.get_or_insert(e); }
        }
    }
    Ok(Err(reason.unwrap_or_else(|| "part number not found".to_string())))
}

/// Whether `quantity` pieces of a Mouser listing can be bought right now: `Ok` with
/// the unit price at that quantity, or `Err` with a short reason why not.
fn evaluate_part(part: Part, quantity: u32) -> Result<MouserPart, String> {
    // `Availability` is localized text ("8234 A magazzino", "8234 In Stock"), so the
    // numeric `AvailabilityInStock` is preferred and the text's leading number is the fallback.
    let availability = part.availability_in_stock
        .as_deref()
        .or_else(|| part.availability.as_deref().and_then(|a| a.split_whitespace().next()))
        .and_then(|n| n.parse::<u32>().ok())
        .unwrap_or_default();
    if availability < quantity {
        let discontinued = part.is_discontinued.is_some_and(|v| v == "true" || v == true);
        let lifecycle = part.lifecycle_status.filter(|s| !s.is_empty());
        return Err(match (discontinued, lifecycle, availability) {
            (true, _, _) => "discontinued".to_string(),
            (_, Some(status), 0) => format!("not in stock ({})", status),
            (_, None, 0) => "not in stock".to_string(),
            (_, _, n) => format!("only {} in stock", n),
        });
    }

    let price_breaks = part.price_breaks.unwrap_or_default();
    let mut unit_price = 0.0;
    for price in &price_breaks {
        if quantity >= price.Quantity {
            unit_price = price.Price
                .strip_suffix(" €")
                .unwrap_or("0.0")
                .replace(",", ".")
                .parse::<f64>()
                .unwrap_or(0.0);
        }
    }
    if unit_price <= 0.0 {
        return Err(match price_breaks.iter().map(|p| p.Quantity).min() {
            Some(min) if min > quantity => format!("minimum order quantity is {}", min),
            _ => "no price available".to_string(),
        });
    }

    Ok(MouserPart {
        manufacturer: part.manufacturer.unwrap_or_default(),
        manufacturer_pn: part.manufacturer_part_number.unwrap_or_default(),
        description: part.description.unwrap_or_default(),
        mouser_pn: part.mouser_part_number.unwrap_or_default(),
        product_url: part.product_detail_url.unwrap_or_default(),
        unit_price,
        availability,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::mouser_api_models::PriceBreak;

    fn part(availability: &str, breaks: &[(u32, &str)]) -> Part {
        Part {
            manufacturer: Some("Texas Instruments".to_string()),
            manufacturer_part_number: Some("LM358P".to_string()),
            description: None,
            mouser_part_number: Some("595-LM358P".to_string()),
            product_detail_url: None,
            price_breaks: Some(breaks.iter().map(|(q, p)| PriceBreak { Quantity: *q, Price: p.to_string(), Currency: "EUR".to_string() }).collect()),
            availability: Some(availability.to_string()),
            availability_in_stock: None,
            lifecycle_status: None,
            is_discontinued: None,
        }
    }

    #[test]
    fn evaluates_stock_and_price_breaks() {
        let breaks = [(10, "0,50 €"), (100, "0,30 €")];
        assert_eq!(evaluate_part(part("500 In Stock", &breaks), 100).unwrap().unit_price, 0.30);
        assert_eq!(evaluate_part(part("500 In Stock", &breaks), 5).unwrap_err(), "minimum order quantity is 10");
        assert_eq!(evaluate_part(part("3 In Stock", &breaks), 10).unwrap_err(), "only 3 in stock");
        assert_eq!(evaluate_part(part("None", &breaks), 10).unwrap_err(), "not in stock");
        // Localized text, and the numeric field taking precedence over it.
        assert_eq!(evaluate_part(part("500 A magazzino", &breaks), 10).unwrap().availability, 500);
        let mut numeric = part("Non disponibile", &breaks);
        numeric.availability_in_stock = Some("40".to_string());
        assert_eq!(evaluate_part(numeric, 10).unwrap().availability, 40);
        assert_eq!(evaluate_part(part("500 In Stock", &[]), 10).unwrap_err(), "no price available");

        let mut discontinued = part("None", &breaks);
        discontinued.is_discontinued = Some(serde_json::json!("true"));
        assert_eq!(evaluate_part(discontinued, 10).unwrap_err(), "discontinued");
    }
}
