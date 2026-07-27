//! Local HTTP+WS print-agent, listening on `127.0.0.1:18999` by default (see
//! docs/PROTOCOL.md for why that port and the full API). Exposed as a lib so
//! tests can build the router in-process (no real socket bind needed) via
//! `tower::ServiceExt::oneshot`, and so `main.rs` stays a thin binary shell.

pub mod auth;
pub mod error;
pub mod jobs;
pub mod middleware;
pub mod printers;
pub mod routes;
pub mod state;
pub mod transport;
pub mod ws;

use axum::http::{header, HeaderValue, Method};
use axum::routing::{get, post};
use axum::Router;
use state::AppState;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub const DEFAULT_PORT: u16 = 18999;

pub fn build_app(state: Arc<AppState>) -> Router {
    // Every real caller is a browser page on a *different* origin than this
    // agent (different scheme/host/port -- even "http://localhost:5173" vs
    // this agent's "http://127.0.0.1:18999" counts, and a POST with a JSON
    // body always triggers a CORS preflight). Without this layer every
    // fetch() call fails client-side with an opaque "Failed to fetch" before
    // the request-level Origin/token checks in `middleware::auth` ever run.
    let cors_state = state.clone();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
            origin.to_str().is_ok_and(|o| auth::origin_allowed(&cors_state.allowed_origins, o))
        }))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    Router::new()
        .route("/health", get(routes::health))
        .route("/pair", post(routes::pair))
        .route("/pair/revoke", post(routes::pair_revoke))
        .route("/printers", get(routes::printers))
        .route("/discover", post(routes::discover))
        .route("/print", post(routes::print))
        .route("/print-tcp", post(routes::print_tcp))
        .route("/print-serial", post(routes::print_serial))
        .route("/print-bluetooth", post(routes::print_bluetooth))
        .route("/print-silent", post(routes::print_silent))
        .route("/jobs/{jobId}", get(routes::get_job))
        .route("/events", get(ws::events))
        .layer(axum::middleware::from_fn_with_state(state.clone(), middleware::auth))
        .layer(cors)
        .with_state(state)
}
