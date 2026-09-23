//! Management of the configurable order options: areas/sub-areas, projects and
//! proposals. These are edited from the board-reserved "manage options" page.
//!
//! Deletion policy (decided with the board):
//! * Areas/sub-areas: a rename cascades to orders and users through the existing
//!   `ON UPDATE CASCADE` foreign keys. A delete is *blocked* while any user still
//!   belongs to the area. Once no users reference it, it is hard-deleted if no
//!   order references it either, otherwise it is archived (kept for history but
//!   hidden from the dropdowns).
//! * Projects/proposals: a "delete" only ever archives (order history is
//!   preserved). A rename lets the caller pick between renaming in place
//!   (cascades to `order_items`) or archiving the old value and creating the new
//!   one alongside it.

use crate::data::errors::DataError;
use sqlx::PgPool;

/// A row of the areas management table.
#[derive(Debug, Clone)]
pub struct AreaRow {
    pub division: String,
    pub sub_area: String,
    pub archived: bool,
}

/// A project or proposal row. Both are just a unique name plus the archived flag.
#[derive(Debug, Clone)]
pub struct OptionRow {
    pub name: String,
    pub archived: bool,
}

/// What happened when the board asked to delete an area.
pub enum AreaDeleteOutcome {
    /// The row was physically removed (nothing referenced it).
    Deleted,
    /// The row was kept but archived (orders still reference it).
    Archived,
}

// ---------------------------------------------------------------------------
// Areas / sub-areas
// ---------------------------------------------------------------------------

/// Every area, including archived ones, for the management table.
pub async fn list_areas(pool: &PgPool) -> Result<Vec<AreaRow>, DataError> {
    sqlx::query_as!(
        AreaRow,
        "SELECT division, sub_area, archived FROM areas ORDER BY division, sub_area"
    )
    .fetch_all(pool)
    .await
    .map_err(DataError::Query)
}

/// Adds a new (division, sub_area) area.
pub async fn create_area(pool: &PgPool, division: &str, sub_area: &str) -> Result<(), DataError> {
    sqlx::query!(
        "INSERT INTO areas (division, sub_area) VALUES ($1, $2)",
        division,
        sub_area
    )
    .execute(pool)
    .await
    .map_err(DataError::Query)?;
    Ok(())
}

/// Renames an area in place. The `ON UPDATE CASCADE` foreign keys propagate the
/// new (division, sub_area) to every referencing order and user.
pub async fn rename_area(
    pool: &PgPool,
    old_division: &str,
    old_sub_area: &str,
    new_division: &str,
    new_sub_area: &str,
) -> Result<(), DataError> {
    sqlx::query!(
        "UPDATE areas SET division = $1, sub_area = $2 WHERE division = $3 AND sub_area = $4",
        new_division,
        new_sub_area,
        old_division,
        old_sub_area
    )
    .execute(pool)
    .await
    .map_err(DataError::Query)?;
    Ok(())
}

/// Restores an archived area so it shows up in the dropdowns again.
pub async fn unarchive_area(pool: &PgPool, division: &str, sub_area: &str) -> Result<(), DataError> {
    sqlx::query!(
        "UPDATE areas SET archived = FALSE WHERE division = $1 AND sub_area = $2",
        division,
        sub_area
    )
    .execute(pool)
    .await
    .map_err(DataError::Query)?;
    Ok(())
}

async fn count_users_for_area(pool: &PgPool, division: &str, sub_area: &str) -> Result<i64, DataError> {
    let record = sqlx::query!(
        "SELECT COUNT(*) AS count FROM users
         WHERE belonging_area_division = $1 AND belonging_area_sub_area = $2",
        division,
        sub_area
    )
    .fetch_one(pool)
    .await
    .map_err(DataError::Query)?;
    Ok(record.count.unwrap_or(0))
}

async fn count_orders_for_area(pool: &PgPool, division: &str, sub_area: &str) -> Result<i64, DataError> {
    let record = sqlx::query!(
        "SELECT COUNT(*) AS count FROM orders
         WHERE area_division = $1 AND area_sub_area = $2",
        division,
        sub_area
    )
    .fetch_one(pool)
    .await
    .map_err(DataError::Query)?;
    Ok(record.count.unwrap_or(0))
}

/// Deletes an area following the board policy: blocked while any user belongs to
/// it, archived while any order references it, otherwise hard-deleted.
pub async fn delete_or_archive_area(
    pool: &PgPool,
    division: &str,
    sub_area: &str,
) -> Result<AreaDeleteOutcome, DataError> {
    let users = count_users_for_area(pool, division, sub_area).await?;
    if users > 0 {
        return Err(DataError::FailedQuery(format!(
            "Cannot delete area '{division} / {sub_area}': {users} user(s) still belong to it. \
             Reassign them to another area first."
        )));
    }

    let orders = count_orders_for_area(pool, division, sub_area).await?;
    if orders > 0 {
        // Old orders still reference it: keep the row but hide it from dropdowns.
        sqlx::query!(
            "UPDATE areas SET archived = TRUE WHERE division = $1 AND sub_area = $2",
            division,
            sub_area
        )
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
        Ok(AreaDeleteOutcome::Archived)
    } else {
        sqlx::query!(
            "DELETE FROM areas WHERE division = $1 AND sub_area = $2",
            division,
            sub_area
        )
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
        Ok(AreaDeleteOutcome::Deleted)
    }
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

/// Every project, including archived ones, for the management table.
pub async fn list_projects(pool: &PgPool) -> Result<Vec<OptionRow>, DataError> {
    sqlx::query_as!(
        OptionRow,
        "SELECT name, archived FROM projects ORDER BY name"
    )
    .fetch_all(pool)
    .await
    .map_err(DataError::Query)
}

pub async fn create_project(pool: &PgPool, name: &str) -> Result<(), DataError> {
    sqlx::query!("INSERT INTO projects (name) VALUES ($1)", name)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}

/// Renames a project in place; cascades to `order_items` via `ON UPDATE CASCADE`.
pub async fn rename_project_cascade(pool: &PgPool, old: &str, new: &str) -> Result<(), DataError> {
    sqlx::query!("UPDATE projects SET name = $1 WHERE name = $2", new, old)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}

/// Archives the old project and introduces the new name alongside it, so existing
/// orders keep pointing at the (now archived) old project.
pub async fn rename_project_archive(pool: &PgPool, old: &str, new: &str) -> Result<(), DataError> {
    let mut tx = pool.begin().await.map_err(DataError::Query)?;
    sqlx::query!(
        "INSERT INTO projects (name, archived) VALUES ($1, FALSE)
         ON CONFLICT (name) DO UPDATE SET archived = FALSE",
        new
    )
    .execute(&mut *tx)
    .await
    .map_err(DataError::Query)?;
    sqlx::query!("UPDATE projects SET archived = TRUE WHERE name = $1", old)
        .execute(&mut *tx)
        .await
        .map_err(DataError::Query)?;
    tx.commit().await.map_err(DataError::Query)?;
    Ok(())
}

pub async fn archive_project(pool: &PgPool, name: &str) -> Result<(), DataError> {
    sqlx::query!("UPDATE projects SET archived = TRUE WHERE name = $1", name)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}

pub async fn unarchive_project(pool: &PgPool, name: &str) -> Result<(), DataError> {
    sqlx::query!("UPDATE projects SET archived = FALSE WHERE name = $1", name)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Proposals
// ---------------------------------------------------------------------------

/// Every proposal, including archived ones, for the management table.
pub async fn list_proposals(pool: &PgPool) -> Result<Vec<OptionRow>, DataError> {
    sqlx::query_as!(
        OptionRow,
        "SELECT name, archived FROM proposals ORDER BY name"
    )
    .fetch_all(pool)
    .await
    .map_err(DataError::Query)
}

pub async fn create_proposal(pool: &PgPool, name: &str) -> Result<(), DataError> {
    sqlx::query!("INSERT INTO proposals (name) VALUES ($1)", name)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}

/// Renames a proposal in place; cascades to `order_items` via `ON UPDATE CASCADE`.
pub async fn rename_proposal_cascade(pool: &PgPool, old: &str, new: &str) -> Result<(), DataError> {
    sqlx::query!("UPDATE proposals SET name = $1 WHERE name = $2", new, old)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}

/// Archives the old proposal and introduces the new name alongside it, so existing
/// orders keep pointing at the (now archived) old proposal.
pub async fn rename_proposal_archive(pool: &PgPool, old: &str, new: &str) -> Result<(), DataError> {
    let mut tx = pool.begin().await.map_err(DataError::Query)?;
    sqlx::query!(
        "INSERT INTO proposals (name, archived) VALUES ($1, FALSE)
         ON CONFLICT (name) DO UPDATE SET archived = FALSE",
        new
    )
    .execute(&mut *tx)
    .await
    .map_err(DataError::Query)?;
    sqlx::query!("UPDATE proposals SET archived = TRUE WHERE name = $1", old)
        .execute(&mut *tx)
        .await
        .map_err(DataError::Query)?;
    tx.commit().await.map_err(DataError::Query)?;
    Ok(())
}

pub async fn archive_proposal(pool: &PgPool, name: &str) -> Result<(), DataError> {
    sqlx::query!("UPDATE proposals SET archived = TRUE WHERE name = $1", name)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}

pub async fn unarchive_proposal(pool: &PgPool, name: &str) -> Result<(), DataError> {
    sqlx::query!("UPDATE proposals SET archived = FALSE WHERE name = $1", name)
        .execute(pool)
        .await
        .map_err(DataError::Query)?;
    Ok(())
}
