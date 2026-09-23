use crate::models::templates::AdvisorHomepageTemplate;
use askama::Template;
use crate::{
    models::app::{AppState, CurrentUser},
    data::{errors, order},
};
use axum::{
    extract::State, response::{Html, IntoResponse, Redirect, Response}, Extension
};

pub async fn advisors_homepage_handler(
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Response, errors::AppError>{
    match current_user.user_id {
        Some(id) => {
            // if user is logged in, get the user's orders, render them in a table
            let html_string = AdvisorHomepageTemplate {
                orders: order::get_order_from_author_id(id, &app_state.connection_pool).await?,
                is_board: current_user.can_access_board(),
            }.render().unwrap();
            Ok(Html(html_string).into_response())
        }
        None => {
            // If user is not logged in, redirect to login page
            Ok(Redirect::to("/").into_response())
        }
    }
}
