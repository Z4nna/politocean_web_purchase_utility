use crate::data::errors::DataError;
use sqlx::PgPool;
use bcrypt;

#[derive(Debug, Clone)]
pub struct User {
    id: i32,
    password_hash: String,
}

pub async fn authenticate_user(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<i32, DataError> {
    let user: User = sqlx::query_as!(
        User,
        "SELECT id, password_hash FROM users WHERE username = $1",
        username
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DataError::FailedQuery("Invalid credentials".to_string()),
        e => DataError::Query(e),
    })?;

    let hashed_password: &str = user.password_hash.as_str();
    let valid_password = bcrypt::verify(password, hashed_password)?;
    if !valid_password {
        Err(DataError::FailedQuery("Invalid credentials".to_string()))
    } else {
        Ok(user.id)
    }
}