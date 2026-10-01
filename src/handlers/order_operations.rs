use crate::models::{order_operations::MergeOrderRequest, templates::OrderArithmeticPageTemplate};
use askama::Template;
use crate::{
    models::{app::AppState, order_operations::{Order, ScaleOrderRequest}},
    data::{errors, order},
};
use axum::{
    extract::State, response::{Html, IntoResponse, Response}, Extension, Json
};
use tower_sessions::{Session};
use crate::models::app::CurrentUser;

pub async fn order_op_page_handler(
    State(_app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Response, errors::AppError>{
    let html_string = OrderArithmeticPageTemplate {
        is_board: current_user.can_access_board(),
    }.render().unwrap();
    Ok(Html(html_string).into_response())
}

pub async fn list_orders_handler(
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> impl IntoResponse {
    // Board members / the professor see every order, everyone else only their own.
    let mut orders = sqlx::query_as!(
        Order,
        "SELECT id, description, author_id FROM orders ORDER BY id DESC",
        //session.get::<i32>("authenticated_user_id").await.unwrap_or(None).unwrap_or(-1)
    )
    .fetch_all(&app_state.connection_pool)
    .await
    .unwrap_or_default();

    if !current_user.can_access_board() {
        orders.retain(|o| Some(o.author_id) == current_user.user_id);
    }

    Json(orders)
}

pub async fn scale_order_handler (
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ScaleOrderRequest>
) -> Result<Json<serde_json::Value>, errors::AppError> {
    if !payload.scale_factor.is_finite() || payload.scale_factor <= 0.0 {
        return Err(errors::DataError::BadRequest("The scale factor must be greater than 0.".to_string()).into());
    }
    // check user is author of the requested order or board
    let order_author_id = sqlx::query! (
        "SELECT author_id FROM orders WHERE id = $1",
        payload.order_id
    ).fetch_one(&app_state.connection_pool).await
    .map_err(|_| errors::AppError::Database(errors::DataError::FailedQuery("Order not found.".to_string())))?.author_id;

    if !current_user.can_access_board() && current_user.user_id != Some(order_author_id) {
       return Err(errors::AppError::Database(errors::DataError::FailedQuery("Not authorized.".to_string())));
    }
    order::ensure_not_confirmed(&app_state.connection_pool, payload.order_id).await?;

    // scale order, using integer quantities (an item never drops below 1)
    let rows_updated = sqlx::query(
        "UPDATE order_items SET quantity = GREATEST(1, ROUND(quantity * $1)::int) WHERE order_id = $2"
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
    // Both orders must exist and be distinct (merging an order into itself would
    // delete it), and a non-board user must be the author of both.
    if payload.source_id == payload.target_id || author_ids.len() != 2 {
        return Err(errors::AppError::Database(errors::DataError::FailedQuery("Invalid orders.".to_string())));
    }
    if user_role != "board" && author_ids.iter().any(|a| *a != user_id) {
        return Err(errors::AppError::Database(errors::DataError::FailedQuery("Not authorized.".to_string())));
    }
    order::ensure_not_confirmed(&app_state.connection_pool, payload.source_id).await?;
    order::ensure_not_confirmed(&app_state.connection_pool, payload.target_id).await?;

    // Quantities of items present in both orders are summed.
    order::merge_orders(&app_state.connection_pool, payload.source_id, payload.target_id).await?;

    Ok(axum::Json(serde_json::json!({
        "status": "success"
    })))
}