use crate::data::errors::AppError;
use crate::data::user;
use crate::models::app::CurrentUser;
use axum::{
    extract::{Request, State},
    http::header::CACHE_CONTROL,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Extension,
};
use sqlx::PgPool;
use tower_sessions::Session;

pub async fn authenticate(
    session: Session,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user_id = session.get::<i32>("authenticated_user_id").await?;

    let mut current_user = CurrentUser {
        is_authenticated: false,
        user_id: None,
    };

    if let Some(id) = user_id {
        current_user.is_authenticated = true;
        current_user.user_id = Some(id);
    }
    req.extensions_mut().insert(current_user);
    Ok(next.run(req).await)
}

pub async fn required_authentication(
    Extension(current_user): Extension<CurrentUser>,
    req: Request,
    next: Next,
) -> Response {
    if !current_user.is_authenticated {
        return Redirect::to("/").into_response();
    }

    let mut res = next.run(req).await;

    res.headers_mut()
        .insert(CACHE_CONTROL, "no-store".parse().unwrap());

    res
}

/// State carried by the [`require_role`] middleware: the database pool used to
/// look up the caller's role and the set of roles allowed to reach the route.
#[derive(Clone)]
pub struct RoleGuard {
    pub pool: PgPool,
    pub allowed_roles: Vec<String>,
}

/// Middleware that gates a route behind one or more roles.
///
/// The caller must be authenticated (populated by [`authenticate`]). Its role
/// is read from the database and, if it is not among `allowed_roles`, the
/// request is redirected to that role's own homepage instead of being served.
pub async fn require_role(
    State(guard): State<RoleGuard>,
    Extension(current_user): Extension<CurrentUser>,
    req: Request,
    next: Next,
) -> Response {
    // Not logged in at all: back to the login page.
    let Some(user_id) = current_user.user_id else {
        return Redirect::to("/").into_response();
    };

    // Resolve the role from the database; treat any lookup failure as "no access".
    let role = match user::get_user_role(&guard.pool, user_id).await {
        Ok(role) => role,
        Err(_) => return Redirect::to("/").into_response(),
    };

    if guard.allowed_roles.iter().any(|allowed| allowed == &role) {
        let mut res = next.run(req).await;
        res.headers_mut()
            .insert(CACHE_CONTROL, "no-store".parse().unwrap());
        res
    } else {
        // Authenticated, but wrong role for this page: send the user to the
        // homepage they are allowed to see (avoids redirect loops).
        Redirect::to(homepage_for_role(&role)).into_response()
    }
}

/// Landing page each role is redirected to when it lacks access to a route.
fn homepage_for_role(role: &str) -> &'static str {
    match role {
        "prof" => "/prof",
        "board" => "/board/home",
        // "advisor" and any unknown role default to the advisor homepage.
        _ => "/home",
    }
}

