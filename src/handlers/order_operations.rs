use crate::models::{templates::OrderArithmeticPageTemplate};
use askama::Template;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use crate::{
    models::app::AppState,
    data::{errors, order},
};
use axum::{
    extract::State, response::{Html, IntoResponse, Response}, Json
};
use tower_sessions::{Session};

pub async fn order_op_page_handler(
    State(_app_state): State<AppState>,
) -> Result<Response, errors::AppError>{
    
    let html_string = OrderArithmeticPageTemplate {
        
    }.render().unwrap();
    Ok(Html(html_string).into_response())
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct Order {
    id: i32,
    description: String,
    author_id: i32,
}

pub async fn list_orders_handler(
    State(app_state): State<AppState>,
    _session: Session
) -> impl IntoResponse {
    // should change behavoir based on whether the calling user is board or not
    let orders = sqlx::query_as!(
        Order,
        "SELECT id, description, author_id FROM orders ORDER BY id DESC",
        //session.get::<i32>("authenticated_user_id").await.unwrap_or(None).unwrap_or(-1)
    )
    .fetch_all(&app_state.connection_pool)
    .await
    .unwrap_or_default();

    Json(orders)
}

#[derive(Serialize, Deserialize)]
pub struct ScaleOrderRequest {
    order_id: i32,
    scale_factor: f64,
}

pub async fn scale_order_handler (
    State(app_state): State<AppState>,
    session: Session,
    Json(payload): Json<ScaleOrderRequest>
) -> Result<Json<serde_json::Value>, errors::AppError> {
    // check user is author of the requested order or board
    let order_author_id = sqlx::query! (
        "SELECT author_id FROM orders WHERE id = $1",
        payload.order_id
    ).fetch_one(&app_state.connection_pool).await.unwrap().author_id;
    
    if session.get::<i32>("authenticated_user_id").await.unwrap_or(None).unwrap_or(-1) != order_author_id {
       return Err(errors::AppError::Database(errors::DataError::FailedQuery("Not authorized.".to_string())));
    }

    // scale order, using integer quantities
    let rows_updated = sqlx::query(
        "UPDATE order_items SET quantity = ROUND(quantity * $1)::int WHERE order_id = $2"
    )
    .bind(payload.scale_factor)
    .bind(payload.order_id)
    .execute(&app_state.connection_pool)
    .await.map_err(|e| errors::AppError::Database(errors::DataError::FailedQuery(e.to_string())))?
    .rows_affected();
    println!("Scaled order {} by factor {}, updated {} rows", payload.order_id, payload.scale_factor, rows_updated);

    sqlx::query!(
        "UPDATE orders SET date = CURRENT_DATE WHERE id = $1",
        payload.order_id
    )
    .execute(&app_state.connection_pool)
    .await.map_err(|e| errors::AppError::Database(errors::DataError::FailedQuery(e.to_string())))?;

    Ok(axum::Json(serde_json::json!({
        "status": "success",
        "rows_updated": rows_updated
    })))
}

#[derive(Serialize, Deserialize)]
pub enum MergeOrderOption {
    KeepSrcQuantity = 0,
    KeepTargetQuantity = 1,
    KeepHighestQuantity = 2,
    KeepLowestQuantity = 3,
    AddQuantities = 4
}
#[derive(Serialize, Deserialize)]
pub struct MergeOrderRequest {
    source_id: i32,
    target_id: i32,
    //options: MergeOrderOption
}

pub async fn merge_order_handler (
    State(app_state): State<AppState>,
    session: Session,
    Json(payload): Json<MergeOrderRequest>
) -> Result<Json<serde_json::Value>, errors::AppError> {
    println!("merging...");
    // check user is author of both orders or board
    // board
    let user_id = session.get::<i32>("authenticated_user_id").await.unwrap_or(None).unwrap_or(-1);
    let user_role = sqlx::query!(
        "SELECT role FROM users WHERE id = $1",
        user_id
    ).fetch_one(&app_state.connection_pool)
    .await.map_err(|e| errors::AppError::Database(errors::DataError::FailedQuery(e.to_string())))?.role;
    
    let author_ids: Vec<i32>= sqlx::query!(
        "SELECT author_id FROM orders WHERE id = $1 OR id = $2",
        payload.source_id, payload.target_id
    ).fetch_all(&app_state.connection_pool)
    .await.map_err(|e| errors::AppError::Database(errors::DataError::FailedQuery(e.to_string())))?
    .iter().map(|a| a.author_id).collect();
    if user_role != "board".to_string() && user_id != author_ids[0] && user_id != author_ids[1] {
        return Err(errors::AppError::Database(errors::DataError::FailedQuery("Not authorized.".to_string())));
    }
    println!("authorised");
    // edit target based on merge options
    let options = MergeOrderOption::AddQuantities;
    match options {
        MergeOrderOption::KeepSrcQuantity => {
            
        }
        MergeOrderOption::KeepTargetQuantity => {

        }
        MergeOrderOption::KeepLowestQuantity => {

        }
        MergeOrderOption::KeepHighestQuantity => {

        }
        MergeOrderOption::AddQuantities => {
            // get all items from src - stupid but funny trick to do everything inside map :)
            join_all(sqlx::query!(
                "SELECT * FROM order_items WHERE order_id = $1",
                payload.source_id
            ).fetch_all(&app_state.connection_pool)
            .await
            .map_err(|e| errors::AppError::Database(errors::DataError::FailedQuery(e.to_string())))?
            .iter()
            .map(async |item| order::add_item_to_order( // by defaults sums quantities on conflict
                    &app_state.connection_pool, 
                    payload.target_id, 
                    item.manufacturer.clone(),
                    item.manufacturer_pn.clone(),
                    item.quantity.clone(), 
                    item.proposal.clone(), 
                    item.project.clone(), 
                    item.mouser_pn.clone(), 
                    item.digikey_pn.clone()
                ).await)).await;
        }
    }

    // remove source
    crate::data::order::delete_order(&app_state.connection_pool, payload.source_id).await?;

    Ok(axum::Json(serde_json::json!({
        "status": "success"
    })))
}