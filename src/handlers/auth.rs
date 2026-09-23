use crate::models::templates::LoginPageTemplate;
use askama::Template;
use crate::{
    models::{user_form_model::AuthFormModel, app::AppState},
    data::{user, errors},
};
use axum::{
    extract::State, response::{Html, IntoResponse, Redirect, Response}, Form
};
use tower_sessions::Session;

pub async fn login() -> impl IntoResponse {
    let html_string = LoginPageTemplate{}.render().unwrap();
    Html(html_string).into_response()
}

pub async fn login_handler(
    State(app_state): State<AppState>,
    session: Session,
    Form(user_form): Form<AuthFormModel>,
) -> Result<Response, errors::AppError> {
    let user_id = user::authenticate_user(
        &app_state.connection_pool,
        &user_form.username.trim(),
        &user_form.password.trim(),
    ).await?;
    session.insert("authenticated_user_id", user_id).await?;
    println!("User logged in with id: {}.", user_id);
    // redirect the user to the homepage matching their role
    let user_role = user::get_user_role(&app_state.connection_pool, user_id).await?;
    let destination = match user_role.as_str() {
        "prof" => "/prof",
        "board" => "/board/home",
        _ => "/home",
    };
    Ok(Redirect::to(destination).into_response())
}