use crate::data::errors::DataError;
use sqlx::PgPool;
use crate::models::item::OrderItem;

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

pub async fn set_item_pn(
    pool: &PgPool, 
    order_id: i32, 
    manufacturer: String, 
    manufacturer_pn: String,
    mouser_pn: Option<String>,
    digikey_pn: Option<String>
) -> Result<(), DataError> {
    
    sqlx::query!(
        "UPDATE order_items 
         SET mouser_pn = $1, digikey_pn = $2 
         WHERE order_id = $3 AND manufacturer = $4 AND manufacturer_pn = $5",
        mouser_pn,
        digikey_pn,
        order_id,
        manufacturer,
        manufacturer_pn
    )
    .execute(pool)
    .await
    .map_err(DataError::Query)?;

    Ok(())
}