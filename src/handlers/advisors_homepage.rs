use crate::models::templates::AdvisorHomepageTemplate;
use askama::Template;
use crate::{
    models::app::AppState,
    data::{errors, order},
};
use axum::{
    extract::State, response::{Html, IntoResponse, Redirect, Response}
};
use tower_sessions::Session;

pub async fn advisors_homepage_handler(
    State(app_state): State<AppState>,
    session: Session,
) -> Result<Response, errors::AppError>{
    // check if the user is authenticated
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?;
    match user_id {
        Some(id) => {
            // if user is logged in, get the user's orders, render them in a table
            let html_string = AdvisorHomepageTemplate {
                orders: order::get_order_from_author_id(id, &app_state.connection_pool).await?,
            }.render().unwrap();
            Ok(Html(html_string).into_response())
        }
        None => {
            // If user is not logged in, redirect to login page
            Ok(Redirect::to("/").into_response())
        }
    }
}
