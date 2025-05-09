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
}