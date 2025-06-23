use std::collections::{HashSet, HashMap};
use crate::{data::{errors::{AppError, DataError}, excel}, models::{app, templates::NewOrderTemplate}};
use askama::Template;
use crate::{
    models::app::AppState,
    data::{errors, order},
};
use axum::{
    body::Bytes, extract::{Form, Multipart, State}, response::{Html, IntoResponse, Redirect, Response}
};
use tower_sessions::Session;

pub async fn new_order_handler(
    State(app_state): State<AppState>,
    session: Session,
) -> Result<Response, errors::AppError> {
    
    let (areas, sub_areas): (Vec<String>, Vec<String>) = sqlx::query!("SELECT division, sub_area FROM areas")
    .fetch_all(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::Query(e))?
    .into_iter()
    .map(|r| (r.division, r.sub_area))
    .unzip();

    let proposals = sqlx::query!("SELECT name FROM proposals")
    .fetch_all(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::Query(e))?
    .into_iter()
    .map(|r| r.name)
    .collect();

    let projects = sqlx::query!("SELECT name FROM projects")
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
    }.render().unwrap();

    Ok(Html(html_string).into_response())
}

pub async fn submit_order_handler(
    State(app_state): State<AppState>,
    session: Session,
    Form(user_form): Form<HashMap<String, String>>,
) -> Result<Response, errors::AppError> {
    let order_author_id = session
    .get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?.unwrap();
    let description = user_form.get("description").unwrap().to_string();
    let area_division = user_form.get("area_division").unwrap().to_string();
    let area_sub_area = user_form.get("area_sub_area").unwrap().to_string();
    let order_id = order::create_order(&app_state.connection_pool, order_author_id, description, area_division, area_sub_area).await?;

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
            None,
            None
        )
        .await?;
    }

    Ok(Redirect::to("/home").into_response())
}

pub async fn upload_kicad_bom_handler(
    State(app_state): State<AppState>,
    session: Session,
    mut multipart: Multipart
) -> Result<Response, errors::AppError> {
    println!("Uploading file...");
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
    println!("Spreadsheet loaded");
    order::create_order_from_kicad_bom(
        &app_state.connection_pool,
        session.get::<i32>("authenticated_user_id").await.unwrap().unwrap(),
        fields.get("description").unwrap().to_string(),
        fields.get("area_division").unwrap().to_string(), 
        fields.get("area_sub_area").unwrap().to_string(), 
        fields.get("proposal").unwrap().to_string(), 
        fields.get("project").unwrap().to_string(), 
        &spreadsheet
    ).await?;
    return Ok(Redirect::to("/home").into_response());
}