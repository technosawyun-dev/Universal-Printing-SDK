//! Origin + Bearer-token enforcement, per docs/PROTOCOL.md's Auth section:
//! Origin is checked on *every* request including `/health`; the Bearer
//! token is required on everything except `/health` and `/pair`. The
//! WebSocket upgrade at `/events` can't send custom headers, so its token
//! check happens separately, inside the handler, against a `?token=` query
//! param -- this middleware only handles the Origin check for that path.

use crate::auth::origin_allowed;
use crate::error::AgentError;
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;

const TOKEN_EXEMPT_PATHS: &[&str] = &["/health", "/pair", "/events"];

pub async fn auth(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Result<Response, AgentError> {
    let origin = req
        .headers()
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AgentError::OriginNotAllowed("missing Origin header".to_string()))?
        .to_string();

    if !origin_allowed(&state.allowed_origins, &origin) {
        return Err(AgentError::OriginNotAllowed(format!("origin '{origin}' is not allowlisted")));
    }

    let path = req.uri().path().to_string();
    if TOKEN_EXEMPT_PATHS.contains(&path.as_str()) {
        return Ok(next.run(req).await);
    }

    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AgentError::Unauthorized("missing Authorization: Bearer <token> header".to_string()))?;

    if !state.tokens.is_valid(&origin, token) {
        return Err(AgentError::Unauthorized("invalid or expired pairing token".to_string()));
    }

    Ok(next.run(req).await)
}
