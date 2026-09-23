use crate::data::errors::DataError;
use crate::models::user_info::UserInfo;
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

/// All users except the one making the request (used by the manage-users page).
pub async fn get_users_except(pool: &PgPool, exclude_id: i32) -> Result<Vec<UserInfo>, DataError> {
    sqlx::query_as!(
        UserInfo,
        "SELECT id, username, email, active, role, belonging_area_division, belonging_area_sub_area
         FROM users WHERE id != $1 ORDER BY username",
        exclude_id
    )
    .fetch_all(pool)
    .await
    .map_err(DataError::Query)
}

/// Distinct area divisions defined in the database.
pub async fn get_divisions(pool: &PgPool) -> Result<Vec<String>, DataError> {
    let rows = sqlx::query!("SELECT DISTINCT division FROM areas WHERE archived = FALSE ORDER BY division")
        .fetch_all(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(rows.into_iter().map(|r| r.division).collect())
}

/// Distinct area sub-areas defined in the database.
pub async fn get_sub_areas(pool: &PgPool) -> Result<Vec<String>, DataError> {
    let rows = sqlx::query!("SELECT DISTINCT sub_area FROM areas WHERE archived = FALSE ORDER BY sub_area")
        .fetch_all(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(rows.into_iter().map(|r| r.sub_area).collect())
}

/// Roles that can be assigned to a user. 'prof' is excluded because it is a
/// unique account that must never be granted to anyone else.
pub async fn get_assignable_roles(pool: &PgPool) -> Result<Vec<String>, DataError> {
    let rows = sqlx::query!("SELECT DISTINCT role FROM users WHERE role != 'prof' ORDER BY role")
        .fetch_all(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(rows.into_iter().map(|r| r.role).collect())
}

/// Creates a new user with a freshly hashed password.
pub async fn create_user(
    pool: &PgPool,
    username: &str,
    email: Option<&str>,
    password: &str,
    active: bool,
    role: &str,
    division: &str,
    sub_area: &str,
) -> Result<(), DataError> {
    let password_hash = bcrypt::hash(password, 10)?;
    sqlx::query!(
        "INSERT INTO users (username, email, password_hash, active, role, belonging_area_division, belonging_area_sub_area)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        username,
        email,
        password_hash,
        active,
        role,
        division,
        sub_area
    )
    .execute(pool)
    .await
    .map_err(DataError::Query)?;
    Ok(())
}

/// Deletes a user. The unique 'prof' account is protected from deletion.
pub async fn delete_user(pool: &PgPool, id: i32) -> Result<(), DataError> {
    sqlx::query!("DELETE FROM users WHERE id = $1 AND role != 'prof'", id)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}

/// Updates the editable fields of a user. The unique 'prof' account is protected
/// from changes made through this page.
pub async fn update_user(
    pool: &PgPool,
    id: i32,
    division: &str,
    sub_area: &str,
    active: bool,
    role: &str,
) -> Result<(), DataError> {
    sqlx::query!(
        "UPDATE users
         SET belonging_area_division = $1, belonging_area_sub_area = $2, active = $3, role = $4
         WHERE id = $5 AND role != 'prof'",
        division,
        sub_area,
        active,
        role,
        id
    )
    .execute(pool)
    .await
    .map_err(DataError::Query)?;
    Ok(())
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