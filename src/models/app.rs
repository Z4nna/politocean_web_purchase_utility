use std::{collections::HashMap, sync::Arc};
use sqlx::PgPool;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub connection_pool: PgPool,
    pub current_user: CurrentUser,
    pub bom_jobs: Arc<Mutex<HashMap<i32, String>>>, // i32 = order_id
}

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub is_authenticated: bool,
    pub user_id: Option<i32>,
}
