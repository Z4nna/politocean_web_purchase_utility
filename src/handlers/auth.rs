use crate::models::templates::LoginPageTemplate;
use askama::Template;
use crate::{
    models::{user_form_model::AuthFormModel, app::AppState},
    data::{user, errors},
};
use axum::{
    extract::State, response::{Html, IntoResponse, Redirect, Response}, Form
};
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};
use tower_sessions::Session;

pub async fn login() -> impl IntoResponse {
    let html_string = LoginPageTemplate{}.render().unwrap();
    Html(html_string).into_response()
}

pub async fn logout_handler(session: Session) -> Result<Response, errors::AppError> {
    // Invalidate the session entirely (clears data and deletes it from the store).
    session.flush().await.map_err(errors::AppError::Session)?;
    Ok(Redirect::to("/").into_response())
}

const MAX_FAILED_LOGINS: u32 = 10;
const FAILED_LOGIN_WINDOW: Duration = Duration::from_secs(5 * 60);

// Failed logins per username: (count, time of the first failure). Kept in
// memory, so it resets on restart and is not shared between server instances.
static FAILED_LOGINS: Lazy<Mutex<HashMap<String, (u32, Instant)>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn login_blocked(username: &str) -> bool {
    FAILED_LOGINS
        .lock()
        .unwrap()
        .get(username)
        .is_some_and(|(count, first)| *count >= MAX_FAILED_LOGINS && first.elapsed() < FAILED_LOGIN_WINDOW)
}

fn record_failed_login(username: &str) {
    let mut failed = FAILED_LOGINS.lock().unwrap();
    // Dropping expired entries here also keeps the map from growing forever.
    failed.retain(|_, (_, first)| first.elapsed() < FAILED_LOGIN_WINDOW);
    failed.entry(username.to_string()).or_insert((0, Instant::now())).0 += 1;
}

pub async fn login_handler(
    State(app_state): State<AppState>,
    session: Session,
    Form(user_form): Form<AuthFormModel>,
) -> Result<Response, errors::AppError> {
    let username = user_form.username.trim();
    if login_blocked(username) {
        return Err(errors::DataError::TooManyRequests("Too many failed logins, try again in a few minutes".to_string()).into());
    }

    let user_id = match user::authenticate_user(&app_state.connection_pool, username, user_form.password.trim()).await {
        Ok(id) => id,
        Err(e) => {
            if matches!(e, errors::DataError::Unauthorized(_)) {
                record_failed_login(username);
            }
            return Err(e.into());
        }
    };
    FAILED_LOGINS.lock().unwrap().remove(username);

    // Fails for deactivated users, so they never get a session.
    let (user_role, password_hash) = user::get_role_and_password_hash(&app_state.connection_pool, user_id).await?;
    // New session id on login (prevents session fixation).
    session.cycle_id().await?;
    session.insert("authenticated_user_id", user_id).await?;
    session.insert("password_stamp", password_hash).await?;
    println!("User logged in with id: {}.", user_id);
    // redirect the user to the homepage matching their role
    let destination = match user_role.as_str() {
        "prof" => "/prof",
        "board" => "/board/home",
        _ => "/home",
    };
    Ok(Redirect::to(destination).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_too_many_failed_logins() {
        let username = "rate-limit-test-user";
        for _ in 0..MAX_FAILED_LOGINS {
            assert!(!login_blocked(username));
            record_failed_login(username);
        }
        assert!(login_blocked(username));
        assert!(!login_blocked("someone-else"));
    }
}
