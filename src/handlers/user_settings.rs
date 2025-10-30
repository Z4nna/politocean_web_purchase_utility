use askama::Template;
use axum::{extract::State, response::{Html, IntoResponse, Response}};
use tower_sessions::Session;

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
