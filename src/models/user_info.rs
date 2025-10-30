use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Serialize, Deserialize, FromRow)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub active: bool,
    pub role: String,
    pub belonging_area_division: String,
    pub belonging_area_sub_area: String,
}