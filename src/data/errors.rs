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

    #[error("Failed to configure SMTP: {0}")]
    Mail(String),

    #[error("Token error: {0}")]
    TokenError(String),

    #[error("{0}")]
    Unauthorized(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    TooManyRequests(String),
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
        // Full detail goes to the server log only. Raw database / internal errors
        // are never sent to the client; hand-written messages are.
        eprintln!("Request failed: {:?}", self);
        let (status, message) = match self {
            AppError::Database(e @ DataError::Unauthorized(_)) => (StatusCode::UNAUTHORIZED, e.to_string()),
            AppError::Database(e @ DataError::BadRequest(_)) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::Database(e @ DataError::TooManyRequests(_)) => (StatusCode::TOO_MANY_REQUESTS, e.to_string()),
            AppError::Database(e @ (DataError::FailedQuery(_) | DataError::TokenError(_) | DataError::Mail(_))) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong.".to_string()),
        };

        let title = status.canonical_reason().unwrap_or("Error");
        let html_string = format!(
            "<!DOCTYPE html>
        <html>
        <head><title>{code} {title}</title></head>
        <body>
            <h1>{title}</h1>
            <pre>{message}</pre>
        </body>
        </html>",
            code = status.as_u16(),
            message = message.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"),
        );

        (status, Html(html_string)).into_response()
    }
}
