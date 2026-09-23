use askama::Template;
use umya_spreadsheet::{Spreadsheet};
use crate::{
    handlers,
    data::{errors::{self, DataError}, excel, item, options, order, user}, models::{app::{AppState, CurrentUser}, templates::{CoffeePageTemplate, EditOrderTemplate}}
};
use axum::{
    body::{Body, Bytes}, extract::{Multipart, Path, State}, http::{header, HeaderValue, StatusCode}, response::{Html, IntoResponse, Redirect, Response}, Extension, Form, Json
};
use tower_sessions::Session;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use zip::write::FileOptions;
use std::{collections::{HashMap, HashSet}, io::Write};
use std::io::Cursor;

pub async fn edit_order_handler(
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError> {
    // Fetch the order and its items first so we can keep their currently selected
    // area/proposal/project available in the dropdowns even if those options have
    // since been archived (archived options are shown but rendered disabled, so
    // they are not selectable for new items).
    let order = order::get_order_from_id(order_id, &app_state.connection_pool).await?;
    let items = item::get_items_from_order(order_id, &app_state.connection_pool).await?;

    // Every valid, non-archived (division, sub_area) pair, plus the order's own
    // pair kept available even if it has since been archived. The client uses
    // `area_pairs` to constrain the sub-area dropdown to combinations that
    // actually exist, so an edit can never produce an invalid composite key.
    let mut area_pairs: Vec<options::AreaRow> = sqlx::query_as!(
        options::AreaRow,
        "SELECT division, sub_area, archived FROM areas WHERE archived = FALSE ORDER BY division, sub_area"
    )
    .fetch_all(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::Query(e))?;

    if !area_pairs.iter().any(|p| p.division == order.area_division && p.sub_area == order.area_sub_area) {
        let archived = sqlx::query!(
            "SELECT archived FROM areas WHERE division = $1 AND sub_area = $2",
            order.area_division,
            order.area_sub_area
        )
        .fetch_optional(&app_state.connection_pool)
        .await
        .map_err(|e| DataError::Query(e))?
        .map(|r| r.archived)
        .unwrap_or(true);
        area_pairs.push(options::AreaRow {
            division: order.area_division.clone(),
            sub_area: order.area_sub_area.clone(),
            archived,
        });
    }

    // Distinct dropdown option lists derived from the available pairs.
    let mut divisions: Vec<String> = area_pairs.iter().map(|p| p.division.clone()).collect();
    divisions.sort();
    divisions.dedup();
    let mut sub_areas: Vec<String> = area_pairs.iter().map(|p| p.sub_area.clone()).collect();
    sub_areas.sort();
    sub_areas.dedup();

    // Active options first (alphabetical, selectable), then any archived option a
    // current item still references, appended and flagged so the template disables it.
    let mut proposals: Vec<options::OptionRow> = sqlx::query_as!(
        options::OptionRow,
        "SELECT name, archived FROM proposals WHERE archived = FALSE ORDER BY name"
    )
    .fetch_all(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::Query(e))?;

    let mut projects: Vec<options::OptionRow> = sqlx::query_as!(
        options::OptionRow,
        "SELECT name, archived FROM projects WHERE archived = FALSE ORDER BY name"
    )
    .fetch_all(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::Query(e))?;

    let mut seen_proposals: HashSet<String> = proposals.iter().map(|o| o.name.clone()).collect();
    let mut seen_projects: HashSet<String> = projects.iter().map(|o| o.name.clone()).collect();
    for item in &items {
        if seen_proposals.insert(item.proposal.clone()) {
            proposals.push(options::OptionRow { name: item.proposal.clone(), archived: true });
        }
        if seen_projects.insert(item.project.clone()) {
            projects.push(options::OptionRow { name: item.project.clone(), archived: true });
        }
    }

    let html_string = EditOrderTemplate{
        order: order,
        items: items,
        divisions: divisions,
        sub_areas: sub_areas,
        area_pairs: area_pairs,
        proposals: proposals,
        projects: projects,
        is_board: current_user.can_access_board(),
    }.render().unwrap();
    Ok(Html(html_string).into_response())
}

/// Extracts the submitted item rows from the order form. Rows are identified by
/// the `items_manufacturer_pn_<index>` keys produced by the client-side JS.
fn parse_order_items(form: &HashMap<String, String>) -> Vec<order::NewOrderItem> {
    let mut indices: HashSet<i32> = HashSet::new();
    for key in form.keys() {
        if let Some(index_str) = key.strip_prefix("items_manufacturer_pn_") {
            if let Ok(index) = index_str.parse::<i32>() {
                indices.insert(index);
            }
        }
    }

    indices
        .into_iter()
        .map(|index| {
            let get = |prefix: &str| form.get(&format!("{}{}", prefix, index)).map(|s| s.trim().to_string());
            order::NewOrderItem {
                manufacturer: get("items_manufacturer_").unwrap_or_default(),
                manufacturer_pn: get("items_manufacturer_pn_").unwrap_or_default(),
                proposal: get("items_proposal_").unwrap_or_else(|| "Elettronica generale".to_string()),
                project: get("items_project_").unwrap_or_else(|| "Varie per lab".to_string()),
                quantity: get("items_quantity_")
                    .and_then(|q| q.parse::<i32>().ok())
                    .unwrap_or(1),
            }
        })
        .collect()
}

pub async fn submit_order_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, errors::AppError> {
    let description = form.get("description").map(|s| s.trim().to_string()).unwrap_or_default();
    let area_division = form.get("area_division").map(|s| s.trim().to_string()).unwrap_or_default();
    let area_sub_area = form.get("area_sub_area").map(|s| s.trim().to_string()).unwrap_or_default();
    let items = parse_order_items(&form);

    // Applied atomically: on any failure (e.g. an invalid area/project/proposal)
    // the whole edit rolls back, so the order is never lost or half-updated.
    order::update_order_and_items(
        &app_state.connection_pool,
        order_id,
        description,
        area_division,
        area_sub_area,
        items,
    )
    .await?;

    Ok(Redirect::to(&format!("/orders/{}/edit", order_id)).into_response())
}

pub async fn mark_order_ready_handler(State(app_state): State<AppState>, _session: Session, Path(order_id): Path<i32>) -> Result<Response, errors::AppError>{
    order::mark_order_ready(&app_state.connection_pool, order_id).await?;
    Ok(Redirect::to("/home").into_response())
}

pub async fn mark_order_unready_handler(State(app_state): State<AppState>,_session: Session,Path(order_id): Path<i32>,) -> Result<Response, errors::AppError>{
    order::mark_order_unready(&app_state.connection_pool, order_id).await?;
    Ok(Redirect::to("/home").into_response())
}

pub async fn mark_order_confirmed_handler(
    State(app_state): State<AppState>,
    session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError> {
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
                }
                let payload = handlers::prof_homepage::OrderNotificationRequest {
                    order_id: order_id,
                    user_id: id,
                };
                println!("calling notify prof handler");
                handlers::prof_homepage::notify_prof_order_confirmed_handler(
                    State(app_state.clone()),
                    session.clone(),
                    Json(payload),
                ).await?;
                order::mark_order_confirmed(&app_state.connection_pool, order_id).await?;

                return Ok(Redirect::to("/board/home").into_response());
            }
            Ok(Redirect::to("/home").into_response())
        }
        None => {
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
    Path(order_id): Path<i32>
) -> Result<Response, errors::AppError>{

    println!("Starting BOM generation.");
    // spawn tokio task and move to the background
    {
        let mut jobs = app_state.bom_jobs.lock().await;
        jobs.insert(order_id, "in_progress".to_string());
    }

    tokio::spawn(async move {
        let result = order::generate_bom(&app_state.connection_pool, order_id).await;
        let mut jobs = app_state.bom_jobs.lock().await;
        jobs.insert(
            order_id,
            if result.is_ok() {
                "done".to_string()
            } else {
                "failed".to_string()
            },
        );
    });
    // immediately return the coffee page, waiting for the job to finish
    Ok(Redirect::to(&format!("/orders/{}/coffee", order_id)).into_response())
}

pub async fn get_generate_bom_job_status_handler (
    State(app_state): State<AppState>,
    Path(order_id): Path<i32>
) -> Result<Response, errors::AppError> {
    let jobs = app_state.bom_jobs.lock().await;
    let status = jobs
        .get(&order_id)
        .cloned()
        .unwrap_or_else(|| "not_started".to_string());

    let body = serde_json::json!({ "status": status });

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    ).into_response())
}

pub async fn coffee_page_handler(
    State(_app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>
) -> Result<Response, errors::AppError> {
    let html_string = CoffeePageTemplate{order_id: order_id}.render().unwrap();
    Ok(Html(html_string).into_response())
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
) -> Result<Response, errors::AppError> {
    order::delete_order(&app_state.connection_pool, order_id).await?;
    println!("Deleted order {}", order_id);
    Ok(Redirect::to("/home").into_response())
}

pub async fn download_digikey_cart_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError> {
    // 1. create xlsx file (column 1 quantity, column 2 part number), no header
    // 1.1 get digikey items from db
    let items = item::get_items_from_order(order_id, &app_state.connection_pool).await?;

    let mut book: Spreadsheet = umya_spreadsheet::new_file();

    let order_sheet = book.get_sheet_mut(&0).unwrap();
    // insert items
    let mut row = 1;
    for item in items {
        if let Some(pn) = item.digikey_pn {
            order_sheet.get_cell_mut((1, row)).set_value(item.quantity.to_string()); // quantity
            order_sheet.get_cell_mut((2, row)).set_value(pn); // PN
            row += 1;
        }
    }
    // 2. download file
    let mut buffer = Cursor::new(Vec::new());
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut buffer).map_err(|e| errors::DataError::Internal(e.to_string()))?;

    let content_disposition = format!(r#"attachment; filename="digikey_cart_{}.xlsx""#, order_id);
    let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
            .header(header::CONTENT_DISPOSITION, HeaderValue::from_str(&content_disposition).unwrap())
            .body(Body::from(buffer.into_inner()))
            .unwrap();
    Ok(response)
}

pub async fn download_mouser_cart_handler(
    State(app_state): State<AppState>,
    _session: Session,
    Path(order_id): Path<i32>,
) -> Result<Response, errors::AppError> {
    // 1. create xlsx file (column 1 quantity, column 2 part number), no header
    // 1.1 get mouser items from db
    let items = item::get_items_from_order(order_id, &app_state.connection_pool).await?;

    let mut book: Spreadsheet = umya_spreadsheet::new_file();

    let order_sheet = book.get_sheet_mut(&0).unwrap();
    // insert items
    let mut row = 1;
    for item in items {
        if let Some(pn) = item.mouser_pn {
            order_sheet.get_cell_mut((1, row)).set_value(item.quantity.to_string()); // quantity
            order_sheet.get_cell_mut((2, row)).set_value(pn); // PN
            row += 1;
        }
    }
    // 2. download file
    let mut buffer = Cursor::new(Vec::new());
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut buffer).map_err(|e| errors::DataError::Internal(e.to_string()))?;

    let content_disposition = format!(r#"attachment; filename="mouser_cart_{}.xlsx""#, order_id);
    let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
            .header(header::CONTENT_DISPOSITION, HeaderValue::from_str(&content_disposition).unwrap())
            .body(Body::from(buffer.into_inner()))
            .unwrap();
    Ok(response)
}

pub async fn bulk_add_handler(
    State(app_state): State<AppState>,
    Path(order_id): Path<i32>,
    mut multipart: Multipart,
) -> Result<Response, errors::AppError> {
    let mut fields: HashMap<String, String> = HashMap::new();
    let mut file_bytes: Option<Bytes> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();

        if name == "file" {
            file_bytes = Some(field.bytes().await.unwrap());
        } else {
            // Normal text field
            let text = field.text().await.unwrap();
            fields.insert(name, text);
        }
    }
    let spreadsheet = excel::load_from_bytes(&file_bytes.unwrap()).map_err(|e| errors::DataError::Internal(e))?;
    order::bulk_add_from_bom(
        &app_state.connection_pool,
        order_id,
        fields.get("proposal").unwrap().to_string(),
        fields.get("project").unwrap().to_string(),
        &spreadsheet
    ).await?;
    return Ok(Redirect::to("/home").into_response());
}
