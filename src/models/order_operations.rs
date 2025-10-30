use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: i32,
    pub description: String,
    pub author_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ScaleOrderRequest {
    pub order_id: i32,
    pub scale_factor: f64,
}

#[derive(Serialize, Deserialize)]
pub enum MergeOrderOption {
    KeepSrcQuantity = 0,
    KeepTargetQuantity = 1,
    KeepHighestQuantity = 2,
    KeepLowestQuantity = 3,
    AddQuantities = 4
}

#[derive(Serialize, Deserialize)]
pub struct MergeOrderRequest {
    pub source_id: i32,
    pub target_id: i32,
    //options: MergeOrderOption
}