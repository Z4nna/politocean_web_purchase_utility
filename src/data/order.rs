use crate::data::{item, errors::DataError};
use crate::models::digikey_api_models::DigiKeyPart;
use crate::models::item::OrderItem;
use crate::models::mouser_api_models::MouserPart;
use futures::stream::{self, StreamExt};
use sqlx::{PgPool, types::time::Date};
use time::format_description;
use umya_spreadsheet::{Spreadsheet};
use crate::data::{mouser_apis};

use crate::data::excel;

use super::digikey_apis;

/// Outcome of the two supplier lookups for one item: for each supplier, the part
/// that can be bought, or a short reason why it cannot.
#[derive(Debug, Clone)]
struct ItemProcessingResult {
    item: OrderItem,
    mouser_part: Result<MouserPart, String>,
    digikey_part: Result<DigiKeyPart, String>,
}

#[derive(Debug)]
enum SupplierChoice {
    Mouser(MouserPart),
    Digikey(DigiKeyPart),
    /// Neither supplier can deliver the item; carries the reason given by each.
    Unresolved(String),
}

/// Picks the supplier for an item. The lookups only return a part that is in stock
/// and has a price for the requested quantity, so when both have it the cheaper
/// one wins (Digikey on a tie).
fn choose_supplier(mouser_part: Result<MouserPart, String>, digikey_part: Result<DigiKeyPart, String>) -> SupplierChoice {
    match (mouser_part, digikey_part) {
        (Ok(m), Ok(d)) => {
            if m.unit_price < d.unit_price {
                SupplierChoice::Mouser(m)
            } else {
                SupplierChoice::Digikey(d)
            }
        }
        (Ok(m), Err(_)) => SupplierChoice::Mouser(m),
        (Err(_), Ok(d)) => SupplierChoice::Digikey(d),
        (Err(m), Err(d)) => SupplierChoice::Unresolved(format!("Mouser: {}; Digikey: {}", m, d)),
    }
}

// Note: one fixed cap on concurrent item lookups to stay under the suppliers'
// rate limits; switch to a per-supplier limiter if one of them needs its own rate.
const MAX_CONCURRENT_LOOKUPS: usize = 4;

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
    // RETURNING gives the id of the row just inserted. Looking it up afterwards by
    // its fields would return another order with the same description and date.
    let order_id: i32 = sqlx::query_scalar!(
        "INSERT INTO orders (author_id, description, area_division, area_sub_area) VALUES ($1, $2, $3, $4) RETURNING id",
        author_id,
        description,
        area_division,
        area_sub_area
    )
    .fetch_one(pool)
    .await
    .map_err(|e| DataError::Query(e))?;
    Ok(order_id)
}

/// A confirmed order has been approved and sent to the professor: the board must
/// unconfirm it before it can be changed again.
pub async fn ensure_not_confirmed(pool: &PgPool, order_id: i32) -> Result<(), DataError> {
    let confirmed = sqlx::query_scalar!("SELECT confirmed FROM orders WHERE id = $1", order_id)
        .fetch_optional(pool)
        .await
        .map_err(DataError::Query)?;
    match confirmed {
        Some(false) => Ok(()),
        Some(true) => Err(DataError::BadRequest("This order is confirmed and can no longer be modified.".to_string())),
        None => Err(DataError::BadRequest("Order not found.".to_string())),
    }
}

/// Moves every item of `source_id` into `target_id` (summing the quantities of
/// items present in both) and deletes the source order. Runs in one transaction,
/// so the source is only deleted if all of its items were moved.
pub async fn merge_orders(pool: &PgPool, source_id: i32, target_id: i32) -> Result<(), DataError> {
    let mut tx = pool.begin().await.map_err(DataError::Query)?;
    sqlx::query!(
        "INSERT INTO order_items (order_id, manufacturer, manufacturer_pn, quantity, proposal, project, mouser_pn, digikey_pn, unit_price, bom_note)
         SELECT $2, manufacturer, manufacturer_pn, quantity, proposal, project, mouser_pn, digikey_pn, unit_price, bom_note
         FROM order_items WHERE order_id = $1
         ON CONFLICT (order_id, manufacturer, manufacturer_pn, proposal, project)
         DO UPDATE SET quantity = order_items.quantity + EXCLUDED.quantity",
        source_id,
        target_id
    )
    .execute(&mut *tx)
    .await
    .map_err(DataError::Query)?;
    sqlx::query!("DELETE FROM orders WHERE id = $1", source_id)
        .execute(&mut *tx)
        .await
        .map_err(DataError::Query)?;
    tx.commit().await.map_err(DataError::Query)?;
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

/// A single item as submitted from the order form (before it is persisted).
pub struct NewOrderItem {
    pub manufacturer: String,
    pub manufacturer_pn: String,
    pub quantity: i32,
    pub proposal: String,
    pub project: String,
}

/// Applies an edit to an existing order atomically: updates the order fields and
/// fully replaces its items, all in one transaction. If anything fails (e.g. an
/// invalid area/project/proposal foreign key) the whole change is rolled back, so
/// the order is never left deleted or half-updated.
///
/// Replacing the items (delete + re-insert) rather than relying on the old
/// delete-and-recreate path guarantees that changed project/proposal values are
/// persisted, instead of being silently kept by the `ON CONFLICT` upsert.
pub async fn update_order_and_items(
    pool: &PgPool,
    order_id: i32,
    description: String,
    area_division: String,
    area_sub_area: String,
    items: Vec<NewOrderItem>,
) -> Result<(), DataError> {
    let mut tx = pool.begin().await.map_err(DataError::Query)?;

    sqlx::query!(
        "UPDATE orders SET description = $1, area_division = $2, area_sub_area = $3 WHERE id = $4",
        description,
        area_division,
        area_sub_area,
        order_id
    )
    .execute(&mut *tx)
    .await
    .map_err(DataError::Query)?;

    sqlx::query!("DELETE FROM order_items WHERE order_id = $1", order_id)
        .execute(&mut *tx)
        .await
        .map_err(DataError::Query)?;

    for item in items {
        sqlx::query!(
            "INSERT INTO order_items (order_id, manufacturer, manufacturer_pn, quantity, proposal, project)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (order_id, manufacturer, manufacturer_pn, proposal, project)
             DO UPDATE SET quantity = order_items.quantity + EXCLUDED.quantity",
            order_id,
            item.manufacturer,
            item.manufacturer_pn,
            item.quantity,
            item.proposal,
            item.project
        )
        .execute(&mut *tx)
        .await
        .map_err(DataError::Query)?;
    }

    tx.commit().await.map_err(DataError::Query)?;
    Ok(())
}

pub async fn add_item_to_order(
    pool: &PgPool,
    order_id: i32,
    manufacturer: String,
    manufacturer_pn: String,
    quantity: i32,
    proposal: String,
    project: String,
    mouser_pn: Option<String>,
    digikey_pn: Option<String>,
) -> Result<(), DataError> {
    sqlx::query!(
        "INSERT INTO order_items (order_id, manufacturer, manufacturer_pn, quantity, proposal, project, mouser_pn, digikey_pn) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (order_id, manufacturer, manufacturer_pn, proposal, project)
        DO UPDATE SET quantity = order_items.quantity + EXCLUDED.quantity",
        order_id,
        manufacturer,
        manufacturer_pn,
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

/// Adds a row to a BOM sheet and stores the result on the order item. The item is
/// looked up by the manufacturer / part number the user typed (`item`), while the
/// sheet shows the names as the supplier spells them. With no supplier part number
/// the item is unresolved and `description` is the reason, kept as its note.
async fn add_to_bom_and_db(
    pool: &PgPool,
    item: &OrderItem,
    manufacturer: String,
    manufacturer_pn: String,
    quantity: i32,
    description: String,
    unit_price: f64,
    product_url: String,
    mouser_pn: Option<String>,
    digikey_pn: Option<String>,
    book: &mut Spreadsheet,
) -> Result<(), DataError>{
    let resolved = mouser_pn.is_some() || digikey_pn.is_some();
    item::set_bom_result(
        pool,
        item,
        mouser_pn,
        digikey_pn,
        resolved.then_some(unit_price),
        (!resolved).then(|| description.clone()),
    ).await?;
    excel::add_item_to_bom(
        book,
        manufacturer,
        manufacturer_pn,
        quantity,
        description,
        unit_price,
        item.proposal.clone(),
        product_url,
        item.project.clone(),
        "".to_string()).map_err(|e| DataError::FailedQuery(e.to_string()))?;
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

    let results: Vec<ItemProcessingResult> = stream::iter(order_items)
        .map(|item| async move {
            let (mouser_part_res, digikey_part_res) = tokio::join!(
                mouser_apis::search_mouser(
                &item.manufacturer,
                &item.manufacturer_pn,
                item.quantity as u32),
                digikey_apis::digikey_search(&item.manufacturer,
                &item.manufacturer_pn,
                item.quantity as u32)
            );
            // A failed lookup (API error) is not "not found": say so, details go to the log.
            let mouser_part = mouser_part_res.unwrap_or_else(|e| {
                eprintln!("Mouser lookup failed for {} {}: {}", item.manufacturer, item.manufacturer_pn, e);
                Err("lookup failed, regenerate the BOM".to_string())
            });
            let digikey_part = digikey_part_res.unwrap_or_else(|e| {
                eprintln!("Digikey lookup failed for {} {}: {}", item.manufacturer, item.manufacturer_pn, e);
                Err("lookup failed, regenerate the BOM".to_string())
            });
            ItemProcessingResult { item, mouser_part, digikey_part }
        })
        .buffer_unordered(MAX_CONCURRENT_LOOKUPS)
        .collect()
        .await;

    for result in results {
        let item = result.item;
        match choose_supplier(result.mouser_part, result.digikey_part) {
            SupplierChoice::Mouser(part) => {
                println!("man: {} - id: {} - mouser_price: {}", item.manufacturer, item.manufacturer_pn, part.unit_price);
                add_to_bom_and_db(
                    pool,
                    &item,
                    part.manufacturer,
                    part.manufacturer_pn,
                    item.quantity,
                    part.description,
                    part.unit_price,
                    part.product_url,
                    Some(part.mouser_pn),
                    None,
                    &mut mouser_book
                ).await?;
            }
            SupplierChoice::Digikey(part) => {
                println!("man: {} - id: {} - digikey_price: {}", item.manufacturer, item.manufacturer_pn, part.unit_price);
                add_to_bom_and_db(
                    pool,
                    &item,
                    part.manufacturer,
                    part.manufacturer_pn,
                    item.quantity,
                    part.description,
                    part.unit_price,
                    part.product_url,
                    None,
                    Some(part.digikey_pn),
                    &mut digikey_book
                ).await?;
            }
            SupplierChoice::Unresolved(reason) => {
                println!("man: {} - id: {} - {}", item.manufacturer, item.manufacturer_pn, reason);
                // Listed with quantity 0 on the Mouser sheet, with the reason as description.
                add_to_bom_and_db(
                    pool,
                    &item,
                    item.manufacturer.clone(),
                    item.manufacturer_pn.clone(),
                    0,
                    reason,
                    0.0,
                    "".to_string(),
                    None,
                    None,
                    &mut mouser_book
                ).await?;
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

pub async fn bulk_add_from_bom(
    pool: &PgPool,
    order_id: i32,
    proposal: String,
    project: String,
    bom: &Spreadsheet
) -> Result<(), DataError> {
    // read bom file, for each item, nsert into db
    let bom_items = excel::parse_kicad_bom_file(bom).map_err(|e| DataError::FailedQuery(e))?;
    for item in bom_items {
        println!("{}: {}x {}", item.manifacturer, item.quantity, item.manifacturer_pn);
        add_item_to_order(pool, order_id, item.manifacturer, item.manifacturer_pn, item.quantity, proposal.clone(), project.clone(), None, None).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mouser(unit_price: f64, availability: u32) -> MouserPart {
        MouserPart {
            manufacturer: "TI".to_string(),
            manufacturer_pn: "PN".to_string(),
            description: String::new(),
            mouser_pn: "M-PN".to_string(),
            product_url: String::new(),
            unit_price,
            availability,
        }
    }

    fn digikey(unit_price: f64, availability: u32) -> DigiKeyPart {
        DigiKeyPart {
            manufacturer: "Texas Instruments".to_string(),
            manufacturer_pn: "PN".to_string(),
            description: String::new(),
            digikey_pn: "D-PN".to_string(),
            product_url: String::new(),
            unit_price,
            availability,
        }
    }

    #[test]
    fn chooses_cheapest_supplier_or_reports_both_reasons() {
        use SupplierChoice::*;
        assert!(matches!(choose_supplier(Ok(mouser(1.0, 10)), Ok(digikey(2.0, 10))), Mouser(_)));
        assert!(matches!(choose_supplier(Ok(mouser(2.0, 10)), Ok(digikey(1.0, 10))), Digikey(_)));
        assert!(matches!(choose_supplier(Ok(mouser(1.0, 10)), Err("not in stock".to_string())), Mouser(_)));
        assert!(matches!(choose_supplier(Err("not in stock".to_string()), Ok(digikey(1.0, 10))), Digikey(_)));
        match choose_supplier(Err("only 2 in stock".to_string()), Err("discontinued".to_string())) {
            Unresolved(reason) => assert_eq!(reason, "Mouser: only 2 in stock; Digikey: discontinued"),
            other => panic!("expected Unresolved, got {:?}", other),
        }
    }
}
