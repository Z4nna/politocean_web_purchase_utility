use crate::data::errors::AppError;
use crate::data::user;
use crate::models::app::{AppState, CurrentUser};
use axum::{
    extract::{Request, State},
    http::header::CACHE_CONTROL,
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
        current_user.is_authenticated = true;
        current_user.user_id = Some(id);
        // Resolve the role once per request so both the guards and the page
        // templates (menu rendering) can rely on it.
        if let Ok(role) = user::get_user_role(&app_state.connection_pool, id).await {
            current_user.role = Some(role);
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
