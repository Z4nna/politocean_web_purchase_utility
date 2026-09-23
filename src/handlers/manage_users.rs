use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Extension, Form,
};
use serde::Deserialize;

use crate::{
    data::{errors, mail, user},
    models::{app::{AppState, CurrentUser}, templates::ManageUsersTemplate},
};

// Access to every handler in this module is gated by the `require_role`
// middleware (board and prof only).

pub async fn manage_users_page(
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Response, errors::AppError> {
    let current_user_id = current_user.user_id.unwrap_or(-1);

    let pool = &app_state.connection_pool;
    let html_string = ManageUsersTemplate {
        users: user::get_users_except(pool, current_user_id).await?,
        divisions: user::get_divisions(pool).await?,
        sub_areas: user::get_sub_areas(pool).await?,
        roles: user::get_assignable_roles(pool).await?,
        is_board: current_user.can_access_board(),
    }
    .render()
    .unwrap();

    Ok(Html(html_string).into_response())
}

#[derive(Deserialize)]
pub struct CreateUserForm {
    username: String,
    email: String,
    role: String,
    // A checkbox is only submitted when checked, so absence means "inactive".
    active: Option<String>,
    belonging_area_division: String,
    belonging_area_sub_area: String,
}

/// Derives a throwaway initial password from the username. It is intentionally
/// not secure: the user is expected to change it as soon as they log in.
fn temporary_password(username: &str) -> String {
    format!("{}-PoliTOcean1", username)
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
    if email.is_empty() {
        return Err(errors::DataError::FailedQuery("Email is required".to_string()).into());
    }

    let username = form.username.trim();
    let temp_password = temporary_password(username);

    user::create_user(
        &app_state.connection_pool,
        username,
        Some(email),
        &temp_password,
        form.active.is_some(),
        form.role.trim(),
        form.belonging_area_division.trim(),
        form.belonging_area_sub_area.trim(),
    )
    .await?;

    // Send the new user their username and temporary password.
    let subject = "PoliTOcean: il tuo account è stato creato";
    let body = format!(
        "Ciao {username},\n\n\
         È stato creato un account per te sul portale PoliTOcean.\n\n\
         Username: {username}\n\
         Password temporanea: {temp_password}\n\n\
         Effettua l'accesso e cambia la password il prima possibile.\n\n\
         Team PoliTOcean."
    );
    mail::send_plaintext_email(email, subject, body).await?;

    Ok(Redirect::to("/board/users").into_response())
}

#[derive(Deserialize)]
pub struct UpdateUserForm {
    role: String,
    // A checkbox is only submitted when checked, so absence means "inactive".
    active: Option<String>,
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
        form.active.is_some(),
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
