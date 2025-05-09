use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct NewOrderFormModel {
    pub customer_name: String,
    pub surname: String,

    #[serde(flatten)]
    pub products: HashMap<String, String>,
}