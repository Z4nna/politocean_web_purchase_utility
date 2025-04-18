use crate::data::errors::DataError;
use sqlx::{PgPool, types::time::Date};
use time::format_description;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct OrderItem {
    pub order_id: i32,
    pub manifacturer: String,
    pub manifacturer_pn: String,
    pub quantity: i32,
    pub proposal: String,
    pub project: String
}

pub async fn get_items_from_order(order_id: i32, pool: &PgPool) -> Result<Vec<OrderItem>, DataError> {
    let user_orders = sqlx::query_as!(
        OrderItem,
        "SELECT * FROM order_items WHERE order_id = $1",
        order_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(user_orders)
}
