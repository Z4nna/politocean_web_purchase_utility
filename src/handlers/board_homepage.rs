use askama::Template;
use crate::{
    data::{errors, order, user}, models::{app::AppState, templates::BoardHomepageTemplate}
};
use axum::{
    extract::State, response::{Html, IntoResponse, Redirect, Response}
};
use tower_sessions::Session;

pub async fn board_homepage_handler(
    State(app_state): State<AppState>,
    session: Session,
) -> Result<Response, errors::AppError>{
    // check if the user is authenticated
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?;
    match user_id {
        Some(id) => {
            // if user is logged in, check if he got enough permissions
            let user_role_result = user::get_user_role(&app_state.connection_pool, id).await;
            if let Ok(user_role) = user_role_result {
                if user_role != "board" {
                    return Ok(Redirect::to("/home").into_response());
                } else {
                    let html_string = BoardHomepageTemplate {
                        orders: order::get_ready_orders(&app_state.connection_pool).await?,
                    }.render().unwrap();
                    return Ok(Html(html_string).into_response());
                }
            }
            Ok(Redirect::to("/home").into_response())
        }
        None => {
            // If user is not logged in, redirect to login page
            Ok(Redirect::to("/").into_response())
        }
    }
}
