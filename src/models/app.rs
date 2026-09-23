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
    pub role: Option<String>,
}

impl CurrentUser {
    /// Whether the user may access the board-reserved area (board members and
    /// the professor). Used both by the route guards and the menu rendering.
    pub fn can_access_board(&self) -> bool {
        matches!(self.role.as_deref(), Some("board") | Some("prof"))
    }
}
