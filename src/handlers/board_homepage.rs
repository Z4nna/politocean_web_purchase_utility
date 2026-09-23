use askama::Template;
use crate::{
    data::{errors, order}, models::{app::AppState, templates::BoardHomepageTemplate}
};
use axum::{
    extract::State, response::{Html, IntoResponse, Response}
};

// Access to these handlers is gated by the `require_role` middleware (board only),
// so the caller is guaranteed to be an authenticated board member here.
pub async fn board_homepage_handler(
    State(app_state): State<AppState>,
) -> Result<Response, errors::AppError>{
    let html_string = BoardHomepageTemplate {
        orders: order::get_ready_orders(&app_state.connection_pool).await?,
    }.render().unwrap();
    Ok(Html(html_string).into_response())
}

pub async fn board_manage_users() -> Response {
    // get all users except for board ones
    // display them in a table (add possibility to remove an user and add a new one)
    Html("<h1>Work in progress</h1> <a href=\"/home\"></a>").into_response()
}