use axum::{middleware, routing::{get, post}, Router};
use crate::handlers::{advisors_homepage, auth, board_homepage, edit_order, new_order, prof_homepage};
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
    .nest_service("/static", server_dir)
    .layer(middleware::from_fn(middlewares::auth::authenticate))
    .with_state(app_state)
}

fn auth_routes() -> Router<app::AppState> {
    Router::new()
        .route("/log-in", post(auth::login_handler))
}

fn home_routes() -> Router<app::AppState> {
    Router::new()
        .route("/home", get(advisors_homepage::advisors_homepage_handler))
        .route("/board/home", get(board_homepage::board_homepage_handler))
        .route("/prof", get(prof_homepage::prof_homepage_handler))
        .route_layer(middleware::from_fn(middlewares::auth::required_authentication))
}

fn orders_routes() -> Router<app::AppState> {
    Router::new()
        .route("/orders/new", get(new_order::new_order_handler))
        .route("/orders/new/submit", post(new_order::submit_order_handler))
        .route("/orders/new/upload-kicad-bom", post(new_order::upload_kicad_bom_handler))
        .route("/orders/:id/edit", get(edit_order::edit_order_handler))
        .route("/orders/:id/edit/submit", post(edit_order::submit_order_handler))
        .route("/orders/:id/edit/generate-bom", post(edit_order::generate_bom_handler))
        .route("/orders/:id/edit/download-bom", post(edit_order::download_bom_handler))
        .route("/orders/:id/ready", post(edit_order::mark_order_ready_handler))
        .route("/orders/:id/unready", post(edit_order::mark_order_unready_handler))
        .route("/orders/:id/confirm", post(edit_order::mark_order_confirmed_handler))
        .route("/orders/:id/unconfirm", post(edit_order::mark_order_unconfirmed_handler))
        .route("/orders/:id/delete", post(edit_order::delete_order_handler))
        .route_layer(middleware::from_fn(middlewares::auth::required_authentication)) // require authentication
}