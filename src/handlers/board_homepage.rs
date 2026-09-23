use askama::Template;
use crate::{
    data::{errors, order},
    models::{
        app::{AppState, CurrentUser},
        templates::{BoardHomeTemplate, BoardHomepageTemplate},
    },
};
use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
    Extension,
};

// Access to these handlers is gated by the `require_role` middleware, so the
// caller is guaranteed to be authorized for the board-reserved area.

/// Board homepage hub: entry point linking to board orders and manage users.
pub async fn board_home_handler(
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Response, errors::AppError> {
    let html_string = BoardHomeTemplate {
        is_board: current_user.can_access_board(),
    }
    .render()
    .unwrap();
    Ok(Html(html_string).into_response())
}

/// Board orders: the ready-orders table reserved to board members.
pub async fn board_orders_handler(
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Response, errors::AppError> {
    let html_string = BoardHomepageTemplate {
        orders: order::get_ready_orders(&app_state.connection_pool).await?,
        is_board: current_user.can_access_board(),
    }
    .render()
    .unwrap();
    Ok(Html(html_string).into_response())
}
