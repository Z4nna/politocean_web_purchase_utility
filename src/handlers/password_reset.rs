use axum::{Form, extract::{Query, State}, response::{Html, IntoResponse, Redirect, Response}};
use lettre::transport::smtp::commands::Data;
use serde::Deserialize;
use tower_sessions::Session;
use rand::RngCore;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use crate::{data::errors::{self, AppError, DataError}, models::app::AppState};

pub async fn request_password_reset(
    State(app_state): State<AppState>,
    session: Session,
) -> Result<Response, errors::AppError> {
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e)).map_err(|e| errors::DataError::FailedQuery(e.to_string()))?;

    if let Some(user) = user_id {
        let token = {
            let mut random_bytes = [0u8; 24];
            rand::thread_rng().fill_bytes(&mut random_bytes);
            URL_SAFE_NO_PAD.encode(&random_bytes)
        };

        sqlx::query!(
            "INSERT INTO password_reset_tokens (user_id, token, expires_at)
            VALUES ($1, $2, NOW() + INTERVAL '15 minutes')
            ON CONFLICT (user_id)
            DO UPDATE SET
                token = EXCLUDED.token,
                expires_at = EXCLUDED.expires_at",
            user,
            token
        )
        .execute(&app_state.connection_pool)
        .await.map_err(|e| errors::DataError::FailedQuery(e.to_string()))?;

        return Ok(Redirect::to(&format!("/reset-password?token={}", token)).into_response());
    }

    Err(errors::AppError::Database(errors::DataError::Internal("user not logged in while attempting password reset".to_string())))
}

#[derive(Deserialize)]
pub struct ResetQuery {
    token: String,
}

pub async fn reset_password_page(
    State(app_state): State<AppState>,
    Query(params): Query<ResetQuery>,
) -> Result<Response, errors::AppError> {
    // Validate token in DB
    let record = sqlx::query!(
        "SELECT user_id FROM password_reset_tokens WHERE token = $1 AND expires_at > NOW()",
        params.token
    )
    .fetch_optional(&app_state.connection_pool)
    .await
    .map_err(|e| errors::DataError::FailedQuery(e.to_string()))?;

    if let Some(_record) = record {
        Ok(Html(format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Reset Password</title>
                <style>
                    body {{
                        font-family: sans-serif;
                        display: flex;
                        flex-direction: column;
                        align-items: center;
                        justify-content: center;
                        height: 100vh;
                        background-color: #f9fafb;
                    }}
                    form {{
                        background: white;
                        padding: 2rem;
                        border-radius: 10px;
                        box-shadow: 0 4px 10px rgba(0,0,0,0.1);
                    }}
                </style>
            </head>
            <body>
                <h1>Reset Password</h1>
                <form action="/reset-password" method="post">
                    <input type="hidden" name="token" value="{}">
                    <label>New Password:</label><br>
                    <input type="password" name="new_password" required><br><br>
                    <button type="submit">Change Password</button>
                </form>
            </body>
            </html>
            "#,
            params.token
        )).into_response())
    } else {
        Err(errors::AppError::Database(errors::DataError::TokenError("Token not found or expired".to_string())))
    }
}

#[derive(Deserialize)]
pub struct ResetForm {
    token: String,
    new_password: String,
}

pub async fn reset_password_submit(
    State(app_state): State<AppState>,
    Form(form): Form<ResetForm>,
) -> Result<Response, errors::AppError> {
    println!("resetting password...");
    let record_opt = sqlx::query!(
        "SELECT user_id FROM password_reset_tokens WHERE token = $1 AND expires_at > NOW()",
        form.token
    )
    .fetch_optional(&app_state.connection_pool)
    .await
    .map_err(|e| errors::DataError::FailedQuery(e.to_string()))?;

    if let Some(record) = record_opt {
        println!("valid user and token");
        let hashed = bcrypt::hash(&form.new_password, 10).map_err(|e| DataError::Internal(e.to_string()))?; // implement your hashing function
        sqlx::query!(
            "UPDATE users SET password_hash = $1 WHERE id = $2",
            hashed,
            record.user_id
        )
        .execute(&app_state.connection_pool)
        .await.map_err(|e| DataError::FailedQuery(e.to_string()))?;

        sqlx::query!("DELETE FROM password_reset_tokens WHERE token = $1", form.token)
            .execute(&app_state.connection_pool)
            .await.map_err(|e| DataError::FailedQuery(e.to_string()))?;

        Ok(Html("<h1>Password successfully reset!</h1>".to_string()).into_response())
    } else {
        Err(errors::AppError::Database(DataError::TokenError("Invalid token".to_string())))
    }
}
