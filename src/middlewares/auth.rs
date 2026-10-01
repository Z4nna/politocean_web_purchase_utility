use crate::data::errors::AppError;
use crate::data::user;
use crate::models::app::{AppState, CurrentUser};
use axum::{
    extract::{Path, Request, State},
    http::{header::CACHE_CONTROL, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Extension,
};
use tower_sessions::Session;

pub async fn authenticate(
    State(app_state): State<AppState>,
    session: Session,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user_id = session.get::<i32>("authenticated_user_id").await?;

    let mut current_user = CurrentUser {
        is_authenticated: false,
        user_id: None,
        role: None,
    };

    if let Some(id) = user_id {
        // Resolve the role once per request so both the guards and the page
        // templates (menu rendering) can rely on it. A session whose user was
        // deleted or deactivated is not authenticated.
        // The session also carries the password hash it was created with, so a
        // password change signs out every other session of that user.
        let password_stamp = session.get::<String>("password_stamp").await?;
        if let Ok((role, password_hash)) = user::get_role_and_password_hash(&app_state.connection_pool, id).await {
            if password_stamp.as_deref() == Some(password_hash.as_str()) {
                current_user.is_authenticated = true;
                current_user.user_id = Some(id);
                current_user.role = Some(role);
            }
        }
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

/// Middleware for `/orders/:id/...` routes: only the order's author or a board
/// member / the professor may touch the order.
pub async fn require_order_access(
    State(app_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(order_id): Path<i32>,
    req: Request,
    next: Next,
) -> Response {
    let author_id = sqlx::query_scalar::<_, i32>("SELECT author_id FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_optional(&app_state.connection_pool)
        .await
        .ok()
        .flatten();

    if current_user.can_access_board() || (author_id.is_some() && author_id == current_user.user_id) {
        next.run(req).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

/// State carried by the [`require_role`] middleware: the set of roles allowed to
/// reach the guarded route.
#[derive(Clone)]
pub struct RoleGuard {
    pub allowed_roles: Vec<String>,
}

/// Middleware that gates a route behind one or more roles.
///
/// The caller's role is resolved by [`authenticate`] and stored on
/// [`CurrentUser`]. If it is not among `allowed_roles`, the request is
/// redirected to that role's own homepage instead of being served.
pub async fn require_role(
    State(guard): State<RoleGuard>,
    Extension(current_user): Extension<CurrentUser>,
    req: Request,
    next: Next,
) -> Response {
    // Not authenticated (or role unknown): back to the login page.
    let Some(role) = current_user.role.as_deref() else {
        return Redirect::to("/").into_response();
    };

    if guard.allowed_roles.iter().any(|allowed| allowed == role) {
        let mut res = next.run(req).await;
        res.headers_mut()
            .insert(CACHE_CONTROL, "no-store".parse().unwrap());
        res
    } else {
        // Authenticated, but wrong role for this page: send the user to the
        // homepage they are allowed to see (avoids redirect loops).
        Redirect::to(homepage_for_role(role)).into_response()
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
