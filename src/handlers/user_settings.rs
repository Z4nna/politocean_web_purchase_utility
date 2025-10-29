use askama::Template;
use axum::{extract::State, response::{Html, IntoResponse, Response}};
use tower_sessions::Session;

use crate::{data::errors, models::{app::AppState, templates::UserSettingsPageTemplate}};

pub async fn user_settings_handler(
    State(_app_state): State<AppState>,
    _session: Session,
) -> Result<Response, errors::AppError>{
    Ok(Html(UserSettingsPageTemplate{}.render().unwrap()).into_response())
}
