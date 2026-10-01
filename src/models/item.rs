#[derive(sqlx::FromRow, Debug, Clone)]
pub struct OrderItem {
    pub order_id: i32,
    pub manufacturer: String,
    pub manufacturer_pn: String,
    pub quantity: i32,
    pub proposal: String,
    pub project: String,
    pub mouser_pn: Option<String>,
    pub digikey_pn: Option<String>,
    // Result of the last BOM generation: price (excl. VAT) from the chosen supplier,
    // or the reason the item could not be sourced.
    pub unit_price: Option<f64>,
    pub bom_note: Option<String>,
}