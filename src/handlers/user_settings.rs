use askama::Template;
use axum::{Form, extract::State, response::{Html, IntoResponse, Redirect, Response}};
use serde::Deserialize;
use tower_sessions::Session;
use validator::Validate;
use crate::{data::errors, models::{app::AppState, templates::UserSettingsPageTemplate, user_info::UserInfo}};

pub async fn user_settings_handler(
    State(app_state): State<AppState>,
    session: Session,
) -> Result<Response, errors::AppError> {
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?;

    let user_info: UserInfo = sqlx::query_as!(
        UserInfo,
        "SELECT id, username, email, active, role, belonging_area_division, belonging_area_sub_area FROM users WHERE id = $1",
        user_id
    )
    .fetch_one(&app_state.connection_pool)
    .await
    .map_err(|e| errors::DataError::FailedQuery("Failed to fetch user informations".to_string() + &e.to_string()))?;

    Ok(Html(UserSettingsPageTemplate{
        user_info: user_info
    }.render().unwrap()).into_response())
}

#[derive(Deserialize, Validate)]
pub struct UpdateEmailForm {
    #[validate(email(message = "Invalid email format"))]
    email: String,
}

pub async fn update_email(
    State(app_state): State<AppState>,
    session: Session,
    Form(form): Form<UpdateEmailForm>,
) -> Result<Response, errors::AppError> {
    if let Err(e) = form.validate() {
        eprintln!("Invalid email: {}", e);
        return Err(errors::AppError::Database(errors::DataError::Mail(e.to_string())));
    }

    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?.unwrap_or(-1);

    if let Err(e) = sqlx::query("UPDATE users SET email = $1 WHERE id = $2")
        .bind(&form.email)
        .bind(user_id)
        .execute(&app_state.connection_pool)
        .await
    {
        eprintln!("DB error: {}", e);
        return Err(errors::AppError::Database(errors::DataError::FailedQuery(e.to_string())));
    }

    Ok(Redirect::to("/settings").into_response())
}