use askama::Template;
use crate::{
    data::{errors, order, item}, models::{app::AppState, templates::EditOrderTemplate}
};
use axum::{
    extract::{State, Path}, response::{Html, IntoResponse, Redirect, Response}
};
use tower_sessions::Session;

pub async fn edit_order_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError>{
    let html_string = EditOrderTemplate{
        order: order::get_order_from_id(order_id, &app_state.connection_pool).await?,
        items: item::get_items_from_order(order_id, &app_state.connection_pool).await?,
    }.render().unwrap();
    Ok(Html(html_string).into_response())
}

pub async fn submit_order_handler(
    State(app_state): State<AppState>,
    session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError>{
    //order::update_order(&app_state.connection_pool, order_id).await?;
    Ok(Redirect::to("/home").into_response())
}

pub async fn mark_order_ready_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>
) -> Result<Response, errors::AppError>{
    order::mark_order_ready(&app_state.connection_pool, order_id).await?;
    Ok(Redirect::to("/home").into_response())
}

pub async fn mark_order_unready_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError>{
    order::mark_order_unready(&app_state.connection_pool, order_id).await?;
    Ok(Redirect::to("/home").into_response())
}

pub async fn mark_order_confirmed_handler(
    State(app_state): State<AppState>,
    session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError>{
    // check user is logged in
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?;
    match user_id {
        Some(id) => {
            // first check if user is authorized to mark as confirmed (i.e. PM or CTO)
            if id == 1 {order::mark_order_confirmed(&app_state.connection_pool, order_id).await?;}
            Ok(Redirect::to("/home").into_response())
        }
        None => {
            // If user is not logged in, redirect to login page
            Ok(Redirect::to("/").into_response())
        }
    }
}

pub async fn mark_order_unconfirmed_handler(
    State(app_state): State<AppState>,
    session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError>{
    // check user is logged in
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?;
    match user_id {
        Some(id) => {
            // first check if user is authorized to mark as confirmed (i.e. PM or CTO)
            if id == 1 {order::mark_order_unconfirmed(&app_state.connection_pool, order_id).await?;}
            Ok(Redirect::to("/home").into_response())
        }
        None => {
            // If user is not logged in, redirect to login page
            Ok(Redirect::to("/").into_response())
        }
    }
}

pub async fn generate_bom_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError>{
    order::generate_bom(&app_state.connection_pool, order_id).await?;
    Ok(Redirect::to(&format!("/orders/{}/edit", order_id)).into_response())
}