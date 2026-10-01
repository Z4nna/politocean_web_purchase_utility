use serde::Deserialize;

#[derive(Deserialize)]
pub struct ResetQuery {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ResetForm {
    pub token: String,
    pub current_password: String,
    pub new_password: String,
}