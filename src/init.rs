use sqlx::PgPool;
use dotenvy::dotenv;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::time};
use tower_sessions_sqlx_store::PostgresStore;

pub async fn database_connection() -> PgPool {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to migrate");

    pool
}

pub async fn session(pool: PgPool) -> SessionManagerLayer<PostgresStore> {
    let session_store = PostgresStore::new(pool);

    session_store
        .migrate()
        .await
        .expect("Failed to run session migration");

    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(1)));

    session_layer
}