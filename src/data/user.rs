use crate::data::errors::DataError;
use crate::models::user_info::UserInfo;
use sqlx::PgPool;
use bcrypt;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct User {
    id: i32,
    password_hash: String,
}

/// Hash checked when the username does not exist, so that a failed login takes
/// the same time whether or not the account exists.
static DUMMY_HASH: Lazy<String> = Lazy::new(|| bcrypt::hash("dummy", 10).expect("bcrypt hash"));

pub async fn authenticate_user(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<i32, DataError> {
    let user: Option<User> = sqlx::query_as!(
        User,
        "SELECT id, password_hash FROM users WHERE username = $1",
        username
    )
    .fetch_optional(pool)
    .await
    .map_err(DataError::Query)?;

    let hashed_password = user.as_ref().map_or(DUMMY_HASH.as_str(), |u| u.password_hash.as_str());
    let valid_password = bcrypt::verify(password, hashed_password)?;
    match user {
        Some(user) if valid_password => Ok(user.id),
        _ => Err(DataError::Unauthorized("Invalid credentials".to_string())),
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
pub fn get_assignable_roles() -> Vec<String> {
    // Roles are plain text with no table of their own, so the list lives here
    // (it must match the roles checked in routes.rs).
    vec!["advisor".to_string(), "board".to_string()]
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
    get_role_and_password_hash(pool, user_id).await.map(|(role, _)| role)
}

/// Role and current password hash of an active user. Deactivated (or deleted)
/// users resolve to an error: they cannot log in and any session they still
/// hold stops being authenticated.
pub async fn get_role_and_password_hash(pool: &PgPool, user_id: i32) -> Result<(String, String), DataError> {
    sqlx::query_as::<_, (String, String)>("SELECT role, password_hash FROM users WHERE id = $1 AND active")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| DataError::Unauthorized("Invalid credentials".to_string()))
}
