use std::net::SocketAddr;
use politocean_backend::{routes, init, models::app};

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    let pool = init::database_connection().await;

    let result = sqlx::query!("SELECT id FROM users").fetch_all(&pool).await;
    println!("DB result: {:?}", result);

    let session_layer = init::session(pool.clone()).await;

    let app_state = app::AppState {
        connection_pool: pool,
    };

    println!("Server running on {addr:?}");

    let app = routes::get_router(app_state).layer(session_layer);
    axum::serve(
        listener, 
        app.into_make_service_with_connect_info::<SocketAddr>()
    )
    .await.unwrap();
}
