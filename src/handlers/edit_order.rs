use askama::Template;
use crate::{
    data::{errors, order, item}, models::{app::AppState, templates::EditOrderTemplate}
};
use axum::{
    body::Body, extract::{Path, State}, http::{header, HeaderValue, StatusCode}, response::{Html, IntoResponse, Redirect, Response}
};
use tower_sessions::Session;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use zip::write::FileOptions;
use std::io::Write;
use std::io::Cursor;

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

pub async fn download_bom_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response<Body>, errors::AppError> {
    order::generate_bom(&app_state.connection_pool, order_id).await?;

    let bom_result = sqlx::query!(
        "SELECT bom_file_mouser, bom_file_digikey, filename FROM order_bom WHERE order_id = $1",
        order_id
    )
    .fetch_optional(&app_state.connection_pool)
    .await
    .map_err(|e| errors::AppError::Database(errors::DataError::Query(e)))?;

    if let Some(record) = bom_result {
        let mouser_bytes = record.bom_file_mouser.ok_or_else(|| {
            errors::AppError::Database(errors::DataError::FailedQuery(
                "Missing Mouser BOM".to_string(),
            ))
        })?;

        let digikey_bytes = record.bom_file_digikey.ok_or_else(|| {
            errors::AppError::Database(errors::DataError::FailedQuery(
                "Missing Digikey BOM".to_string(),
            ))
        })?;

        let raw_filename = record
            .filename
            .unwrap_or_else(|| format!("bom_{}", order_id));
        let base_filename = raw_filename.trim_end_matches(".xlsx");

        let mut buffer = Cursor::new(Vec::new());

        {
            let mut zip = zip::ZipWriter::new(&mut buffer);

            let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

            zip.start_file(format!("{}_mouser.xlsx", base_filename), options).map_err(|e| {
                errors::AppError::Database(errors::DataError::FailedQuery(e.to_string()))
            })?;
            zip.write_all(&mouser_bytes).map_err(|e| {
                errors::AppError::Database(errors::DataError::FailedQuery(e.to_string()))
            })?;

            zip.start_file(format!("{}_digikey.xlsx", base_filename), options).map_err(|e| {
                errors::AppError::Database(errors::DataError::FailedQuery(e.to_string()))
            })?;
            zip.write_all(&digikey_bytes).map_err(|e| {
                errors::AppError::Database(errors::DataError::FailedQuery(e.to_string()))
            })?;

            zip.finish().map_err(|e| {
                errors::AppError::Database(errors::DataError::FailedQuery(e.to_string()))
            })?;
        }

        let zip_filename = format!("{}_bom.zip", base_filename);
        let encoded = utf8_percent_encode(&zip_filename, NON_ALPHANUMERIC).to_string();
        let content_disposition = format!(
            r#"attachment; filename="{}"; filename*=UTF-8''{}"#,
            zip_filename, encoded
        );

        let zip_bytes = buffer.clone().into_inner();

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/zip")
            .header(header::CONTENT_DISPOSITION, HeaderValue::from_str(&content_disposition).unwrap())
            .body(Body::from(zip_bytes))
            .unwrap();

        Ok(response)
    } else {
        Err(errors::AppError::Database(errors::DataError::FailedQuery(
            "No BOM found for order".to_string(),
        )))
    }
}