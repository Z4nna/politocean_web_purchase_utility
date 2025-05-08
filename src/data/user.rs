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

pub async fn get_user_role(pool: &PgPool, user_id: i32) -> Result<String, DataError> {
    let user_role_result = sqlx::query!(
        "SELECT role FROM users WHERE id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| DataError::Query(e));

    if let Ok(user_role) = user_role_result {
        Ok(user_role.role)
    } else {
        Err(DataError::FailedQuery("User not found".to_string()))
    }
}