use serde::Deserialize;

#[derive(Deserialize)]
pub struct ResetQuery {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ResetForm {
    pub token: String,
    pub new_password: String,
}