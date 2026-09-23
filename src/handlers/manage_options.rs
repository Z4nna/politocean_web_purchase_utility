use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Extension, Form,
};
use serde::Deserialize;

use crate::{
    data::{errors::{self, DataError}, options},
    models::{app::{AppState, CurrentUser}, templates::ManageOptionsTemplate},
};

// Access to every handler in this module is gated by the `require_role`
// middleware (board and prof only).

const REDIRECT: &str = "/board/options";

pub async fn manage_options_page(
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Response, errors::AppError> {
    let pool = &app_state.connection_pool;
    let html_string = ManageOptionsTemplate {
        areas: options::list_areas(pool).await?,
        projects: options::list_projects(pool).await?,
        proposals: options::list_proposals(pool).await?,
        is_board: current_user.can_access_board(),
    }
    .render()
    .unwrap();

    Ok(Html(html_string).into_response())
}

/// Rejects blank names before they reach the database.
fn non_empty(value: &str, field: &str) -> Result<(), DataError> {
    if value.trim().is_empty() {
        Err(DataError::FailedQuery(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Areas
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateAreaForm {
    division: String,
    sub_area: String,
}

pub async fn create_area_handler(
    State(app_state): State<AppState>,
    Form(form): Form<CreateAreaForm>,
) -> Result<Response, errors::AppError> {
    non_empty(&form.division, "Division")?;
    non_empty(&form.sub_area, "Sub area")?;
    options::create_area(&app_state.connection_pool, form.division.trim(), form.sub_area.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

#[derive(Deserialize)]
pub struct RenameAreaForm {
    old_division: String,
    old_sub_area: String,
    new_division: String,
    new_sub_area: String,
}

pub async fn rename_area_handler(
    State(app_state): State<AppState>,
    Form(form): Form<RenameAreaForm>,
) -> Result<Response, errors::AppError> {
    non_empty(&form.new_division, "Division")?;
    non_empty(&form.new_sub_area, "Sub area")?;
    options::rename_area(
        &app_state.connection_pool,
        form.old_division.trim(),
        form.old_sub_area.trim(),
        form.new_division.trim(),
        form.new_sub_area.trim(),
    )
    .await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

#[derive(Deserialize)]
pub struct AreaKeyForm {
    division: String,
    sub_area: String,
}

pub async fn delete_area_handler(
    State(app_state): State<AppState>,
    Form(form): Form<AreaKeyForm>,
) -> Result<Response, errors::AppError> {
    // The board policy (block / archive / delete) is enforced in the data layer.
    options::delete_or_archive_area(&app_state.connection_pool, form.division.trim(), form.sub_area.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

pub async fn unarchive_area_handler(
    State(app_state): State<AppState>,
    Form(form): Form<AreaKeyForm>,
) -> Result<Response, errors::AppError> {
    options::unarchive_area(&app_state.connection_pool, form.division.trim(), form.sub_area.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

// ---------------------------------------------------------------------------
// Projects & proposals (both are just a name)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateOptionForm {
    name: String,
}

#[derive(Deserialize)]
pub struct RenameOptionForm {
    old_name: String,
    new_name: String,
    // "cascade" renames in place; anything else archives the old name and
    // creates the new one alongside it (the default).
    mode: String,
}

#[derive(Deserialize)]
pub struct OptionKeyForm {
    name: String,
}

// --- Projects ---

pub async fn create_project_handler(
    State(app_state): State<AppState>,
    Form(form): Form<CreateOptionForm>,
) -> Result<Response, errors::AppError> {
    non_empty(&form.name, "Project name")?;
    options::create_project(&app_state.connection_pool, form.name.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

pub async fn rename_project_handler(
    State(app_state): State<AppState>,
    Form(form): Form<RenameOptionForm>,
) -> Result<Response, errors::AppError> {
    non_empty(&form.new_name, "Project name")?;
    let (old, new) = (form.old_name.trim(), form.new_name.trim());
    if old == new {
        return Ok(Redirect::to(REDIRECT).into_response());
    }
    if form.mode == "cascade" {
        options::rename_project_cascade(&app_state.connection_pool, old, new).await?;
    } else {
        options::rename_project_archive(&app_state.connection_pool, old, new).await?;
    }
    Ok(Redirect::to(REDIRECT).into_response())
}

pub async fn delete_project_handler(
    State(app_state): State<AppState>,
    Form(form): Form<OptionKeyForm>,
) -> Result<Response, errors::AppError> {
    // "Delete" only ever archives, to preserve the history of past orders.
    options::archive_project(&app_state.connection_pool, form.name.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

pub async fn unarchive_project_handler(
    State(app_state): State<AppState>,
    Form(form): Form<OptionKeyForm>,
) -> Result<Response, errors::AppError> {
    options::unarchive_project(&app_state.connection_pool, form.name.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

// --- Proposals ---

pub async fn create_proposal_handler(
    State(app_state): State<AppState>,
    Form(form): Form<CreateOptionForm>,
) -> Result<Response, errors::AppError> {
    non_empty(&form.name, "Proposal name")?;
    options::create_proposal(&app_state.connection_pool, form.name.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

pub async fn rename_proposal_handler(
    State(app_state): State<AppState>,
    Form(form): Form<RenameOptionForm>,
) -> Result<Response, errors::AppError> {
    non_empty(&form.new_name, "Proposal name")?;
    let (old, new) = (form.old_name.trim(), form.new_name.trim());
    if old == new {
        return Ok(Redirect::to(REDIRECT).into_response());
    }
    if form.mode == "cascade" {
        options::rename_proposal_cascade(&app_state.connection_pool, old, new).await?;
    } else {
        options::rename_proposal_archive(&app_state.connection_pool, old, new).await?;
    }
    Ok(Redirect::to(REDIRECT).into_response())
}

pub async fn delete_proposal_handler(
    State(app_state): State<AppState>,
    Form(form): Form<OptionKeyForm>,
) -> Result<Response, errors::AppError> {
    // "Delete" only ever archives, to preserve the history of past orders.
    options::archive_proposal(&app_state.connection_pool, form.name.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}

pub async fn unarchive_proposal_handler(
    State(app_state): State<AppState>,
    Form(form): Form<OptionKeyForm>,
) -> Result<Response, errors::AppError> {
    options::unarchive_proposal(&app_state.connection_pool, form.name.trim()).await?;
    Ok(Redirect::to(REDIRECT).into_response())
}
