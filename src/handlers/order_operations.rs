use crate::models::{app, templates::OrderArithmeticPageTemplate};
use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use crate::{
    models::app::AppState,
    data::{errors, order},
};
use axum::{
    Json, extract::State, response::{Html, IntoResponse, Redirect, Response}
};
use tower_sessions::{session, Session};

pub async fn order_op_page_handler(
    State(app_state): State<AppState>,
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
    State(app_state): State<AppState>
) -> impl IntoResponse {
    let orders = sqlx::query_as!(
        Order,
        "SELECT id, description, author_id FROM orders ORDER BY id DESC"
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
    // check user is author of the requested order
    let order_author_id = sqlx::query!(
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

    Ok(axum::Json(serde_json::json!({
        "status": "success",
        "rows_updated": rows_updated
    })))
}