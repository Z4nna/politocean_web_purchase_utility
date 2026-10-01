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

/// Stores the result of a BOM generation on an order item: the chosen supplier's
/// part number and unit price, or a note saying why the item could not be sourced.
pub async fn set_bom_result(
    pool: &PgPool,
    item: &OrderItem,
    mouser_pn: Option<String>,
    digikey_pn: Option<String>,
    unit_price: Option<f64>,
    bom_note: Option<String>,
) -> Result<(), DataError> {
    sqlx::query!(
        "UPDATE order_items
         SET mouser_pn = $1, digikey_pn = $2, unit_price = $3, bom_note = $4
         WHERE order_id = $5 AND manufacturer = $6 AND manufacturer_pn = $7 AND proposal = $8 AND project = $9",
        mouser_pn,
        digikey_pn,
        unit_price,
        bom_note,
        item.order_id,
        item.manufacturer,
        item.manufacturer_pn,
        item.proposal,
        item.project
    )
    .execute(pool)
    .await
    .map_err(DataError::Query)?;

    Ok(())
}
