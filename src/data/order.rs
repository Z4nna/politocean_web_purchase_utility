use crate::data::{item, errors::DataError};
use sqlx::{PgPool, types::time::Date};
use time::format_description;

use crate::data::mouser_apis;

use crate::data::excel;

use super::digikey_apis;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Order {
    pub id: i32,
    pub author_id: i32,
    pub date: Date,
    pub ready: bool,
    pub confirmed: bool,
    pub description: String,
    pub area_division: String,
    pub area_sub_area: String
}

impl Order {
    pub fn get_date(&self) -> String {
        let format = format_description::parse("[day]/[month]/[year]").unwrap();
        self.date.format(&format).unwrap_or("".to_string())
    }
    pub fn get_status(&self) -> &str {
        if self.confirmed {
            "All done! ✅"
        } else if self.ready {
            "Waiting for approvment ..."
        } else {
            "Not completed yet"
        }
    }
    pub fn get_bg_color(&self) -> &str {
        if self.confirmed {
          " #4CAF50"
        } else if self.ready {
            " #FFC107"
        } else {
            " #F44336"
        }
    }
}

pub async fn get_order_from_author_id(author_id: i32, pool: &PgPool) -> Result<Vec<Order>, DataError> {
    let user_orders = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE author_id = $1 ORDER BY date DESC",
        author_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(user_orders)
}

pub async fn get_order_from_id(order_id: i32, pool: &PgPool) -> Result<Order, DataError> {
    let user_orders = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE id = $1 ORDER BY date DESC",
        order_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(user_orders)
}

pub async fn mark_order_ready(pool: &PgPool, order_id: i32) -> Result<(), DataError> {
    sqlx::query!(
        "UPDATE orders SET ready = true WHERE id = $1",
        order_id
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(())
}

pub async fn mark_order_unready(pool: &PgPool, order_id: i32) -> Result<(), DataError> {
    sqlx::query!(
        "UPDATE orders SET ready = false WHERE id = $1",
        order_id
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(())
}

pub async fn mark_order_confirmed(pool: &PgPool, order_id: i32) -> Result<(), DataError> {
    sqlx::query!(
        "UPDATE orders SET confirmed = true WHERE id = $1",
        order_id
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(())
}

pub async fn mark_order_unconfirmed(pool: &PgPool, order_id: i32) -> Result<(), DataError> {

    sqlx::query!(
        "UPDATE orders SET confirmed = false WHERE id = $1",
        order_id
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(())
}

pub async fn create_order(
    pool: &PgPool,
    author_id: i32,
    description: String,
    area_division: String,
    area_sub_area: String,
) -> Result<i32, DataError> {
    sqlx::query!(
        "INSERT INTO orders (author_id, description, area_division, area_sub_area) VALUES ($1, $2, $3, $4)",
        author_id,
        description,
        area_division,
        area_sub_area
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;

    let order_id: i32 = sqlx::query!(
        "SELECT id FROM orders WHERE author_id = $1 AND date = CURRENT_DATE AND description = $2 AND area_division = $3 AND area_sub_area = $4",
        author_id,
        description,
        area_division,
        area_sub_area
    )
    .fetch_one(pool)
    .await
    .map_err(|e| DataError::Query(e))?
    .id;
    Ok(order_id)
}

pub async fn add_item_to_order(
    pool: &PgPool,
    order_id: i32,
    manifacturer: String,
    manifacturer_pn: String,
    quantity: i32,
    proposal: String,
    project: String,
) -> Result<(), DataError> {
    sqlx::query!(
        "INSERT INTO order_items (order_id, manifacturer, manifacturer_pn, quantity, proposal, project) VALUES ($1, $2, $3, $4, $5, $6)",
        order_id,
        manifacturer,
        manifacturer_pn,
        quantity,
        proposal,
        project,
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(())
}

pub async fn generate_bom(pool: &PgPool, order_id: i32) -> Result<(), DataError> {
    // get order info
    let order = get_order_from_id(order_id, pool).await?;
    let order_items = item::get_items_from_order(order_id, pool).await?;

    // create excel files
    let mut mouser_book = excel::create_bom_file();
    let mut digikey_book = excel::create_bom_file();

    // for each item, retrieve info from mouser / digikey
    for item in order_items {
        let mouser_part_opt: Option<mouser_apis::MouserPart> = mouser_apis::search_mouser(
            &item.manifacturer,
            &item.manifacturer_pn, 
            item.quantity as u32)
            .await
            .map_err(|e| DataError::FailedQuery(e.to_string()))?;
        let digikey_part_opt = digikey_apis::digikey_search(&item.manifacturer, 
            &item.manifacturer_pn, 
            item.quantity as u32)
            .await
            .map_err(|e| DataError::FailedQuery(e.to_string()))?;
        match (mouser_part_opt, digikey_part_opt) {
            (Some(mouser_part), Some(digikey_part)) => {
                // check if item is available on both mouser and digikey
                println!("id: {}, mouser_price: {}, digikey_price: {}", item.manifacturer_pn, mouser_part.unit_price, digikey_part.unit_price);
                if (mouser_part.availability >= item.quantity as u32)
                && mouser_part.unit_price > 0.0
                && (mouser_part.unit_price < digikey_part.unit_price || digikey_part.unit_price == 0.0)  {
                    excel::add_item_to_bom(
                        &mut mouser_book,
                        mouser_part.manufacturer,
                        mouser_part.manufacturer_pn,
                        item.quantity,
                        mouser_part.description,
                        mouser_part.unit_price,
                        item.proposal,
                        mouser_part.product_url,
                        item.project,
                        "".to_string()).map_err(|e| DataError::FailedQuery(e.to_string()))?;
                } else if digikey_part.availability >= item.quantity as u32 
                && digikey_part.unit_price > 0.0 {
                    excel::add_item_to_bom(
                        &mut digikey_book,
                        digikey_part.manufacturer,
                        digikey_part.manufacturer_pn,
                        item.quantity,
                        digikey_part.description,
                        digikey_part.unit_price,
                        item.proposal,
                        digikey_part.product_url,
                        item.project,
                        "".to_string()).map_err(|e| DataError::FailedQuery(e.to_string()))?;
                } else {
                    excel::add_item_to_bom(
                        &mut mouser_book,
                        item.manifacturer,
                        item.manifacturer_pn,
                        0,
                        "".to_string(),
                        0.0,
                        item.proposal,
                        "".to_string(),
                        item.project,
                        "".to_string()).map_err(|e| DataError::FailedQuery(e.to_string()))?;
                }
            },
            (None, Some(digikey_part)) => { // only available on digikey
                excel::add_item_to_bom(
                    &mut digikey_book, 
                    digikey_part.manufacturer, 
                    digikey_part.manufacturer_pn, 
                    item.quantity, 
                    digikey_part.description,
                    digikey_part.unit_price,
                    item.proposal,
                    digikey_part.product_url,
                    item.project,
                    "".to_string()).map_err(|e| DataError::FailedQuery(e.to_string()))?;
            }
            (Some(mouser_part), None) => { // only available on mouser
                excel::add_item_to_bom(
                    &mut mouser_book,
                    mouser_part.manufacturer,
                    mouser_part.manufacturer_pn,
                    item.quantity,
                    mouser_part.description,
                    mouser_part.unit_price,
                    item.proposal,
                    mouser_part.product_url,
                    item.project,
                    "".to_string()).map_err(|e| DataError::FailedQuery(e.to_string()))?;
            }
            (None, None) => { // part not found
                excel::add_item_to_bom(
                    &mut mouser_book,
                    item.manifacturer,
                    item.manifacturer_pn,
                    0,
                    "".to_string(),
                    0.0,
                    item.proposal,
                    "".to_string(),
                    item.project,
                    "".to_string()).map_err(|e| DataError::FailedQuery(e.to_string()))?;
            }
        }
    }
    let mouser_bom_bytes = excel::save_to_bytes(&mouser_book).map_err(|e| DataError::FailedQuery(e.to_string()))?;
    let digikey_bom_bytes = excel::save_to_bytes(&digikey_book).map_err(|e| DataError::FailedQuery(e.to_string()))?;
    // save bom file to db
    sqlx::query!(
        r#"INSERT INTO order_bom (order_id, bom_file_mouser, bom_file_digikey, filename)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (order_id) DO UPDATE 
            SET bom_file_mouser = EXCLUDED.bom_file_mouser,
            bom_file_digikey = EXCLUDED.bom_file_digikey,
            filename = EXCLUDED.filename"#r,
        order_id,
        mouser_bom_bytes,
        digikey_bom_bytes,
        order.description.replace(" ", "_").to_lowercase()
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;

    Ok(())
}