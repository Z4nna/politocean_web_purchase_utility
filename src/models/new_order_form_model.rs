use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct NewOrderFormModel {
    pub customer_name: String,
    pub surname: String,

    // Catch-all for dynamic quantity_X fields
    #[serde(flatten)]
    pub products: HashMap<String, String>,
}