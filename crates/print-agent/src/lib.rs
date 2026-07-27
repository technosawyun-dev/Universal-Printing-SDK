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

use axum::routing::{get, post};
use axum::Router;
use state::AppState;
use std::sync::Arc;

pub const DEFAULT_PORT: u16 = 18999;

pub fn build_app(state: Arc<AppState>) -> Router {
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
        .with_state(state)
}
