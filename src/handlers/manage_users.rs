use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    data::{errors, user},
    models::{app::AppState, templates::ManageUsersTemplate},
};

// Access to every handler in this module is gated by the `require_role`
// middleware (board and prof only).

pub async fn manage_users_page(
    State(app_state): State<AppState>,
    session: Session,
) -> Result<Response, errors::AppError> {
    let current_user_id = session
        .get::<i32>("authenticated_user_id")
        .await
        .map_err(errors::AppError::Session)?
        .unwrap_or(-1);

    let pool = &app_state.connection_pool;
    let html_string = ManageUsersTemplate {
        users: user::get_users_except(pool, current_user_id).await?,
        divisions: user::get_divisions(pool).await?,
        sub_areas: user::get_sub_areas(pool).await?,
        roles: user::get_assignable_roles(pool).await?,
    }
    .render()
    .unwrap();

    Ok(Html(html_string).into_response())
}

#[derive(Deserialize)]
pub struct CreateUserForm {
    username: String,
    email: String,
    password: String,
    role: String,
    active: String,
    belonging_area_division: String,
    belonging_area_sub_area: String,
}

pub async fn create_user_handler(
    State(app_state): State<AppState>,
    Form(form): Form<CreateUserForm>,
) -> Result<Response, errors::AppError> {
    // 'prof' is a unique account and can never be assigned to anyone.
    if form.role == "prof" {
        return Err(errors::DataError::FailedQuery("Cannot assign the 'prof' role".to_string()).into());
    }

    let email = form.email.trim();
    let email = if email.is_empty() { None } else { Some(email) };

    user::create_user(
        &app_state.connection_pool,
        form.username.trim(),
        email,
        form.password.trim(),
        form.active == "true",
        form.role.trim(),
        form.belonging_area_division.trim(),
        form.belonging_area_sub_area.trim(),
    )
    .await?;

    Ok(Redirect::to("/board/users").into_response())
}

#[derive(Deserialize)]
pub struct UpdateUserForm {
    role: String,
    active: String,
    belonging_area_division: String,
    belonging_area_sub_area: String,
}

pub async fn update_user_handler(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<UpdateUserForm>,
) -> Result<Response, errors::AppError> {
    // 'prof' is a unique account and can never be assigned to anyone.
    if form.role == "prof" {
        return Err(errors::DataError::FailedQuery("Cannot assign the 'prof' role".to_string()).into());
    }

    user::update_user(
        &app_state.connection_pool,
        id,
        form.belonging_area_division.trim(),
        form.belonging_area_sub_area.trim(),
        form.active == "true",
        form.role.trim(),
    )
    .await?;

    Ok(Redirect::to("/board/users").into_response())
}

pub async fn delete_user_handler(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Response, errors::AppError> {
    user::delete_user(&app_state.connection_pool, id).await?;
    Ok(Redirect::to("/board/users").into_response())
}
