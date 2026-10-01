use std::collections::{HashSet, HashMap};
use crate::{data::{errors::{DataError}, excel}, models::{templates::NewOrderTemplate}};
use askama::Template;
use crate::{
    models::app::AppState,
    data::{errors, order},
};
use axum::{
    body::Bytes, extract::{Form, Multipart, State}, response::{Html, IntoResponse, Redirect, Response}, Extension
};
use tower_sessions::Session;
use crate::models::app::CurrentUser;

pub async fn new_order_handler(
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Response, errors::AppError> {

    let (areas, sub_areas): (Vec<String>, Vec<String>) = sqlx::query!("SELECT division, sub_area FROM areas WHERE archived = FALSE")
    .fetch_all(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::Query(e))?
    .into_iter()
    .map(|r| (r.division, r.sub_area))
    .unzip();

    let proposals = sqlx::query!("SELECT name FROM proposals WHERE archived = FALSE")
    .fetch_all(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::Query(e))?
    .into_iter()
    .map(|r| r.name)
    .collect();

    let projects = sqlx::query!("SELECT name FROM projects WHERE archived = FALSE")
    .fetch_all(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::Query(e))?
    .into_iter()
    .map(|r| r.name)
    .collect();


    let html_string = NewOrderTemplate{
        areas: HashSet::<String>::from_iter(areas).into_iter().collect(),
        sub_areas: HashSet::<String>::from_iter(sub_areas).into_iter().collect(),
        proposals: proposals,
        projects: projects,
        is_board: current_user.can_access_board(),
    }.render().unwrap();

    Ok(Html(html_string).into_response())
}

/// A required text field of a submitted form, trimmed.
pub fn required_field(fields: &HashMap<String, String>, name: &str) -> Result<String, errors::AppError> {
    fields
        .get(name)
        .map(|s| s.trim().to_string())
        .ok_or_else(|| DataError::BadRequest(format!("Missing field: {}", name)).into())
}

/// Reads a multipart BOM upload: its text fields and the spreadsheet in its `file` part.
pub async fn read_bom_upload(mut multipart: Multipart) -> Result<(HashMap<String, String>, umya_spreadsheet::Spreadsheet), errors::AppError> {
    let bad_upload = |e: axum::extract::multipart::MultipartError| DataError::BadRequest(e.to_string());
    let mut fields: HashMap<String, String> = HashMap::new();
    let mut file_bytes: Option<Bytes> = None;

    while let Some(field) = multipart.next_field().await.map_err(bad_upload)? {
        let name = field.name().unwrap_or_default().to_string();

        if name == "file" {
            file_bytes = Some(field.bytes().await.map_err(bad_upload)?);
        } else {
            // Normal text field
            let text = field.text().await.map_err(bad_upload)?;
            fields.insert(name, text);
        }
    }
    let file_bytes = file_bytes.ok_or_else(|| DataError::BadRequest("Missing file".to_string()))?;
    let spreadsheet = excel::load_from_bytes(&file_bytes)
        .map_err(|_| DataError::BadRequest("The uploaded file is not a valid spreadsheet".to_string()))?;
    Ok((fields, spreadsheet))
}

pub async fn submit_order_handler(
    State(app_state): State<AppState>,
    session: Session,
    Form(user_form): Form<HashMap<String, String>>,
) -> Result<Response, errors::AppError> {
    let order_author_id = session
    .get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?
    .ok_or_else(|| DataError::Unauthorized("Not logged in".to_string()))?;
    let description = required_field(&user_form, "description")?;
    let area_division = required_field(&user_form, "area_division")?;
    let area_sub_area = required_field(&user_form, "area_sub_area")?;
    let items = crate::handlers::edit_order::parse_order_items(&user_form)?;
    order::create_order_with_items(&app_state.connection_pool, order_author_id, description, area_division, area_sub_area, items).await?;

    Ok(Redirect::to("/home").into_response())
}

pub async fn upload_kicad_bom_handler(
    State(app_state): State<AppState>,
    session: Session,
    multipart: Multipart
) -> Result<Response, errors::AppError> {
    let (fields, spreadsheet) = read_bom_upload(multipart).await?;
    let author_id = session
        .get::<i32>("authenticated_user_id")
        .await?
        .ok_or_else(|| DataError::Unauthorized("Not logged in".to_string()))?;
    order::create_order_from_kicad_bom(
        &app_state.connection_pool,
        author_id,
        required_field(&fields, "description")?,
        required_field(&fields, "area_division")?,
        required_field(&fields, "area_sub_area")?,
        required_field(&fields, "proposal")?,
        required_field(&fields, "project")?,
        &spreadsheet
    ).await?;
    return Ok(Redirect::to("/home").into_response());
}
