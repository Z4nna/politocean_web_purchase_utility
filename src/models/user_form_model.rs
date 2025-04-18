use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthFormModel {
    pub username: String,
    pub password: String,
}
