use askama::Template;
use crate::{
    data::{errors, item, order, user}, models::{app::AppState, templates::EditOrderTemplate}
};
use axum::{
    body::Body, extract::{Path, State}, http::{header, HeaderValue, StatusCode}, response::{Html, IntoResponse, Redirect, Response}, Form
};
use tower_sessions::Session;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use zip::write::FileOptions;
use std::{collections::{HashMap, HashSet}, io::Write};
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

pub async fn new_order_with_id_handler(
    State(app_state): State<AppState>,
    session: Session,
    Form(user_form): Form<HashMap<String, String>>,
    order_id: i32,
) -> Result<Response, errors::AppError> {
    let order_author_id = session
    .get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?.unwrap();
    let description = user_form.get("description").unwrap().to_string();
    let area_division = user_form.get("area_division").unwrap().to_string();
    let area_sub_area = user_form.get("area_sub_area").unwrap().to_string();
    order::create_order_with_id(
        &app_state.connection_pool, 
        order_id, 
        order_author_id, 
        description, 
        area_division, 
        area_sub_area).await?;

    let mut indices: HashSet<i32> = HashSet::new();
    // Collect valid indices based on existing keys
    for key in user_form.keys().map(|s| s.to_string()) {
        let maybe_index = key.strip_prefix("items_manifacturer_pn_");
        match maybe_index {
            Some(index_str) => {
                let index = index_str.parse::<i32>().unwrap();
                indices.insert(index);
            }
            None => {
                continue;
            }
        }
    }
    // Now process only the indices that exist
    for index in indices {
        let man_key = format!("items_manifacturer_{}", index);
        let pn_key = format!("items_manifacturer_pn_{}", index);
        let quantity_key = format!("items_quantity_{}", index);
        let proposal_key = format!("items_proposal_{}", index);
        let project_key = format!("items_project_{}", index);

        let manifacturer = user_form.get(&man_key).unwrap_or(&"".to_string()).to_string();
        let manifacturer_pn = user_form.get(&pn_key).unwrap_or(&"".to_string()).to_string();
        let proposal = user_form.get(&proposal_key).unwrap_or(&"Elettronica generale".to_string()).to_string();
        let project = user_form.get(&project_key).unwrap_or(&"Varie per lab".to_string()).to_string();
        let quantity = user_form
            .get(&quantity_key)
            .unwrap()
            .to_string()
            .parse::<i32>()
            .unwrap_or(1);
        order::add_item_to_order(
            &app_state.connection_pool,
            order_id,
            manifacturer,
            manifacturer_pn,
            quantity,
            proposal,
            project,
        )
        .await?;
    }

    Ok(Redirect::to("/home").into_response())
}

pub async fn submit_order_handler(
    State(app_state): State<AppState>,
    session: Session,
    Path(order_id): Path<i32>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, errors::AppError>{
    // delete old order
    order::delete_order(&app_state.connection_pool, order_id).await?;
    println!("Deleted order {}", order_id);
    // forward request to new order
    let response = new_order_with_id_handler(State(app_state), session, Form(form), order_id).await;
    match response {
        Ok(_) => {
            Ok(Redirect::to(&format!("/orders/{}/edit", order_id)).into_response())
        },
        Err(e) => Err(e)
    }
}

pub async fn mark_order_ready_handler(State(app_state): State<AppState>, _session: Session, Path(order_id): Path<i32>) -> Result<Response, errors::AppError>{
    order::mark_order_ready(&app_state.connection_pool, order_id).await?;
    Ok(Redirect::to("/home").into_response())
}

pub async fn mark_order_unready_handler(State(app_state): State<AppState>,_session: Session,Path(order_id): Path<i32>,) -> Result<Response, errors::AppError>{
    order::mark_order_unready(&app_state.connection_pool, order_id).await?;
    Ok(Redirect::to("/home").into_response())
}

pub async fn mark_order_confirmed_handler(State(app_state): State<AppState>,session: Session,Path(order_id): Path<i32>,) -> Result<Response, errors::AppError>{
    // check user is logged in
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?;
    match user_id {
        Some(id) => {
            // first check if user is authorized to mark as confirmed (i.e. PM or CTO)
            let user_role_result = user::get_user_role(&app_state.connection_pool, id).await;
            if let Ok(user_role) = user_role_result {
                if user_role != "board" {
                    return Ok(Redirect::to("/home").into_response());
                } else {
                    order::mark_order_confirmed(&app_state.connection_pool, order_id).await?;
                    return Ok(Redirect::to("/board/home").into_response());
                }
            }
            Ok(Redirect::to("/home").into_response())
        }
        None => {
            // If user is not logged in, redirect to login page
            Ok(Redirect::to("/").into_response())
        }
    }
}

pub async fn mark_order_unconfirmed_handler(State(app_state): State<AppState>,session: Session,Path(order_id): Path<i32>,) -> Result<Response, errors::AppError>{
    // check user is logged in
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?;
    match user_id {
        Some(id) => {
            // first check if user is authorized to mark as confirmed (i.e. PM or CTO)
            let user_role_result = user::get_user_role(&app_state.connection_pool, id).await;
            if let Ok(user_role) = user_role_result {
                if user_role != "board" {
                    return Ok(Redirect::to("/home").into_response());
                } else {
                    order::mark_order_unconfirmed(&app_state.connection_pool, order_id).await?;
                    return Ok(Redirect::to("/board/home").into_response());
                }
            }
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

pub async fn delete_order_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError>{
    println!("Handler called {}", order_id);
    order::delete_order(&app_state.connection_pool, order_id).await?;
    println!("Deleted order {}", order_id);
    Ok(Redirect::to("/home").into_response())
}