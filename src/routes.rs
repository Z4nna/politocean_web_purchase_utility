use axum::{middleware::{self, from_fn_with_state}, routing::{get, post}, Router};
use crate::handlers::{advisors_homepage, auth, board_homepage, edit_order, manage_users, new_order, order_operations, password_reset, prof_homepage, user_settings};
use crate::middlewares::auth::RoleGuard;
use crate::models::app;
use tower_http::services::ServeDir;
use crate::middlewares;

pub fn get_router(app_state: app::AppState) -> Router {
    let server_dir = ServeDir::new("static");

    Router::new()
    .route("/", get(auth::login))
    .merge(auth_routes())
    .merge(home_routes())
    .merge(orders_routes())
    .merge(settings_routes())
    .route("/reset-password", get(password_reset::reset_password_page))
    .route("/reset-password", post(password_reset::reset_password_submit))
    .route("/request-pwd-reset", get(password_reset::request_password_reset))
    .nest_service("/static", server_dir)
    .layer(from_fn_with_state(app_state.clone(), middlewares::auth::authenticate))
    .with_state(app_state)
}

fn auth_routes() -> Router<app::AppState> {
    Router::new()
        .route("/log-in", post(auth::login_handler))
        .route("/logout", post(auth::logout_handler))
}

/// Builds a role-gating layer for the given roles. The caller's role is resolved
/// by the `authenticate` middleware and read from `CurrentUser`.
fn require_role(roles: &[&str]) -> RoleGuard {
    RoleGuard {
        allowed_roles: roles.iter().map(|r| r.to_string()).collect(),
    }
}

fn home_routes() -> Router<app::AppState> {
    Router::new()
        .merge(advisor_home_routes())
        .merge(board_home_routes())
        .merge(manage_users_routes())
        .merge(prof_home_routes())
}

fn advisor_home_routes() -> Router<app::AppState> {
    Router::new()
        .route("/home", get(advisors_homepage::advisors_homepage_handler))
        .route_layer(from_fn_with_state(
            require_role(&["advisor", "board"]),
            middlewares::auth::require_role,
        ))
}

fn board_home_routes() -> Router<app::AppState> {
    Router::new()
        // Board homepage hub: entry point to the board-reserved area.
        .route("/board/home", get(board_homepage::board_home_handler))
        .route_layer(from_fn_with_state(
            require_role(&["board", "prof"]),
            middlewares::auth::require_role,
        ))
        // Board orders (ready orders) are reserved to board members.
        .merge(board_orders_routes())
}

fn board_orders_routes() -> Router<app::AppState> {
    Router::new()
        .route("/board/orders", get(board_homepage::board_orders_handler))
        .route_layer(from_fn_with_state(
            require_role(&["board"]),
            middlewares::auth::require_role,
        ))
}

fn manage_users_routes() -> Router<app::AppState> {
    Router::new()
        .route("/board/users", get(manage_users::manage_users_page))
        .route("/board/users/create", post(manage_users::create_user_handler))
        .route("/board/users/:id/update", post(manage_users::update_user_handler))
        .route("/board/users/:id/delete", post(manage_users::delete_user_handler))
        .route_layer(from_fn_with_state(
            require_role(&["board", "prof"]),
            middlewares::auth::require_role,
        ))
}

fn prof_home_routes() -> Router<app::AppState> {
    Router::new()
        .route("/prof", get(prof_homepage::prof_homepage_handler))
        .route_layer(from_fn_with_state(
            require_role(&["prof"]),
            middlewares::auth::require_role,
        ))
}

fn settings_routes() -> Router<app::AppState> {
    Router::new()
        .route("/settings", get(user_settings::user_settings_handler))
        .route("/settings/set-email", post(user_settings::update_email))
}

fn orders_routes() -> Router<app::AppState> {
    Router::new()
        .route("/orders/list", get(order_operations::list_orders_handler))
        .route("/orders/new", get(new_order::new_order_handler))
        .route("/orders/new/submit", post(new_order::submit_order_handler))
        .route("/orders/new/upload-kicad-bom", post(new_order::upload_kicad_bom_handler))
        .route("/orders/:id/coffee", get(edit_order::coffee_page_handler))
        .route("/orders/:id/get_bom_gen_status", get(edit_order::get_generate_bom_job_status_handler))
        .merge(edit_order_routes())
        .merge(order_arithmetic_routes())
        .route_layer(middleware::from_fn(middlewares::auth::required_authentication)) // require authentication
}

fn edit_order_routes() -> Router<app::AppState> {
    Router::new()
        .route("/orders/:id/edit", get(edit_order::edit_order_handler))
        .route("/orders/:id/edit/submit", post(edit_order::submit_order_handler))
        .route("/orders/:id/edit/bulk-add", post(edit_order::bulk_add_handler))
        .route("/orders/:id/edit/generate-bom", post(edit_order::generate_bom_handler))
        .route("/orders/:id/edit/download-bom", post(edit_order::download_bom_handler))
        .route("/orders/:id/edit/create-mouser-cart", post(edit_order::download_mouser_cart_handler))
        .route("/orders/:id/edit/download-digikey-cart", post(edit_order::download_digikey_cart_handler))
        .route("/orders/:id/ready", post(edit_order::mark_order_ready_handler))
        .route("/orders/:id/unready", post(edit_order::mark_order_unready_handler))
        .route("/orders/:id/confirm", post(edit_order::mark_order_confirmed_handler))
        .route("/orders/:id/unconfirm", post(edit_order::mark_order_unconfirmed_handler))
        .route("/orders/:id/delete", post(edit_order::delete_order_handler))
}

fn order_arithmetic_routes() -> Router<app::AppState> {
    Router::new()
        .route("/orders/arithmetic", get(order_operations::order_op_page_handler))
        .route("/orders/scale", post(order_operations::scale_order_handler))
        .route("/orders/merge", post(order_operations::merge_order_handler))
}