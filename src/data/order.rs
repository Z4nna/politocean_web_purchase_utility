use crate::data::{item, errors::DataError};
use crate::models::mouser_api_models;
use sqlx::{PgPool, types::time::Date};
use time::format_description;
use umya_spreadsheet::Spreadsheet;
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
            "Waiting for approval ..."
        } else {
            "To be completed ..."
        }
    }
    pub fn get_bg_color(&self) -> &str {
        if self.confirmed {
          " #ACF39D"
        } else if self.ready {
            " #FFC107"
        } else {
            " #E85F5C"
        }
    }
}

pub async fn get_order_from_author_id(author_id: i32, pool: &PgPool) -> Result<Vec<Order>, DataError> {
    let user_orders = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE author_id = $1 ORDER BY date DESC, id DESC",
        author_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(user_orders)
}

pub async fn get_ready_orders(pool: &PgPool) -> Result<Vec<Order>, DataError> {
    let user_orders = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE ready = true ORDER BY date DESC, id DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(user_orders)
}

pub async fn get_confirmed_orders(pool: &PgPool) -> Result<Vec<Order>, DataError> {
    let user_orders = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE confirmed = true ORDER BY date DESC, id DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(user_orders)
}

pub async fn get_order_from_id(order_id: i32, pool: &PgPool) -> Result<Order, DataError> {
    let user_orders = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE id = $1",
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

pub async fn create_order_with_id(
    pool: &PgPool,
    order_id: i32,
    author_id: i32,
    description: String,
    area_division: String,
    area_sub_area: String,
) -> Result<(), DataError> {
    sqlx::query!(
        "INSERT INTO orders (id, author_id, description, area_division, area_sub_area) VALUES ($1, $2, $3, $4, $5)",
        order_id,
        author_id,
        description,
        area_division,
        area_sub_area
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(())
}

pub async fn delete_order(pool: &PgPool, order_id: i32) -> Result<(), DataError> {
    sqlx::query!(
        r#"DELETE FROM orders WHERE id = $1"#,
        order_id
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(())
}

pub async fn add_item_to_order(
    pool: &PgPool,
    order_id: i32,
    manifacturer: String,
    manifacturer_pn: String,
    quantity: i32,
    proposal: String,
    project: String,
    mouser_pn: Option<String>,
    digikey_pn: Option<String>,
) -> Result<(), DataError> {
    sqlx::query!(
        "INSERT INTO order_items (order_id, manufacturer, manufacturer_pn, quantity, proposal, project, mouser_pn, digikey_pn) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        order_id,
        manifacturer,
        manifacturer_pn,
        quantity,
        proposal,
        project,
        mouser_pn,
        digikey_pn
    )
    .execute(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(())
}

pub async fn generate_bom(pool: &PgPool, order_id: i32) -> Result<(), DataError> {
    println!("Generating BOM for order {}", order_id);
    // get order info
    let order: Order = get_order_from_id(order_id, pool).await?;
    let order_items = item::get_items_from_order(order_id, pool).await?;

    // create excel files
    let mut mouser_book = excel::create_bom_file();
    let mut digikey_book = excel::create_bom_file();

    // for each item, retrieve info from mouser / digikey
    for item in order_items {
        println!("Searching for item: {}", item.manufacturer_pn);
        let mouser_part_opt: Option<mouser_api_models::MouserPart> = mouser_apis::search_mouser(
            &item.manufacturer,
            &item.manufacturer_pn, 
            item.quantity as u32)
            .await
            .map_err(|e| DataError::FailedQuery(e.to_string()))?;
        println!("Found {} mouser parts", mouser_part_opt.is_some() as i32);
        let digikey_part_opt = digikey_apis::digikey_search(&item.manufacturer, 
            &item.manufacturer_pn, 
            item.quantity as u32)
            .await
            .map_err(|e| DataError::FailedQuery(e.to_string()))?;
        println!("Found {} digikey parts", digikey_part_opt.is_some() as i32);

        match (mouser_part_opt, digikey_part_opt) {
            (Some(mouser_part), Some(digikey_part)) => {
                println!("id: {}, mouser_price: {}, digikey_price: {}", item.manufacturer_pn, mouser_part.unit_price, digikey_part.unit_price);
                // check if item is available on both mouser and digikey
                if (mouser_part.availability >= item.quantity as u32)
                && mouser_part.unit_price > 0.0
                && (mouser_part.unit_price < digikey_part.unit_price || digikey_part.unit_price == 0.0)  {
                    // adding to mouser book, set mouser_pn in db
                    item::set_item_pn(
                        pool, 
                        order_id, 
                        item.manufacturer.clone(), 
                        item.manufacturer_pn.clone(), 
                        Some(mouser_part.mouser_pn.clone()), 
                        None
                    ).await?;
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
                    // adding to digikey book, set digikey_pn in db
                    item::set_item_pn(
                        pool, 
                        order_id, 
                        item.manufacturer.clone(),
                        item.manufacturer_pn.clone(), 
                        None, 
                        Some(digikey_part.digikey_pn.clone())
                    ).await?;
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
                    println!("Item not available on both mouser and digikey");
                    excel::add_item_to_bom(
                        &mut mouser_book,
                        item.manufacturer,
                        item.manufacturer_pn,
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
                println!("id: {}, digikey_price: {}", item.manufacturer_pn, digikey_part.unit_price);
                item::set_item_pn(
                    pool, 
                    order_id, 
                    item.manufacturer.clone(), 
                    item.manufacturer_pn.clone(), 
                    None, 
                    Some(digikey_part.digikey_pn.clone())
                ).await?;
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
                println!("id: {}, mouser_price: {}", item.manufacturer_pn, mouser_part.unit_price);
                item::set_item_pn(
                    pool, 
                    order_id, 
                    item.manufacturer.clone(), 
                    item.manufacturer_pn.clone(), 
                    Some(mouser_part.mouser_pn.clone()), 
                    None
                ).await?;
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
                println!("id: {}, not found", item.manufacturer_pn);
                excel::add_item_to_bom(
                    &mut mouser_book,
                    item.manufacturer,
                    item.manufacturer_pn,
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

pub async fn create_order_from_kicad_bom(
    pool: &PgPool,
    author_id: i32,
    description: String,
    area_division: String,
    area_sub_area: String,
    proposal: String,
    project: String,
    kicad_bom_file: &Spreadsheet
) -> Result<(), DataError> {
    // create order
    let order_id = create_order(pool, author_id, description, area_division, area_sub_area).await?;
    // read kicad bom file, for each item, nsert into db
    let bom_items = excel::parse_kicad_bom_file(kicad_bom_file).map_err(|e| DataError::FailedQuery(e))?;
    for item in bom_items {
        println!("{}: {}x {}", item.manifacturer, item.quantity, item.manifacturer_pn);
        add_item_to_order(pool, order_id, item.manifacturer, item.manifacturer_pn, item.quantity, proposal.clone(), project.clone(), None, None).await?;
    }
    Ok(())
}