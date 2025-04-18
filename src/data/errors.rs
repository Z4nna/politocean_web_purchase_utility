use thiserror::Error;
use axum::{
    response::{IntoResponse, Response, Html},
    body::Body,
    http::StatusCode,
    
};

#[derive(Error, Debug)]
pub enum DataError {
    #[error("Failed database query: {0}")]
    Query(#[from] sqlx::Error),

    #[error("Failed to query: {0}")]
    FailedQuery(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Failed to hash: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),

    #[error("Failed to convert from utf8: {0}")]
    Utf8Conversion(#[from] std::string::FromUtf8Error),
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error")]
    Database(#[from] DataError),

    #[error("Template error")]
    Template(#[from] askama::Error),

    #[error("Failed loading session")]
    Session(#[from] tower_sessions::session::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let (status, response) = match self {
            AppError::Database(e) => server_error(e.to_string()),
            AppError::Template(e) => server_error(e.to_string()),
            AppError::Session(e) => server_error(e.to_string()),
        };

        (status, response).into_response()
    }
}

fn server_error(e: String) -> (StatusCode, Response) {
    let html_string = format!(
        "<!DOCTYPE html>
        <html>
        <head><title>500 Internal Server Error</title></head>
        <body>
            <h1>Internal Server Error</h1>
            <pre>{}</pre>
        </body>
        </html>",
        e
    );

    (StatusCode::INTERNAL_SERVER_ERROR, Html(html_string).into_response())
}