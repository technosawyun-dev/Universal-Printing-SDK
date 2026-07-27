//! End-to-end HTTP tests against the real router (`build_app`), using
//! `tower::ServiceExt::oneshot` -- no socket bind needed except for the
//! print-tcp round trip, which needs a real listener to prove the whole
//! chain (HTTP -> job -> transport -> bytes-on-the-wire) actually works.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use printers_print_agent::auth::TokenStore;
use printers_print_agent::state::AppState;
use printers_print_agent::build_app;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

const ORIGIN: &str = "http://test.local";

fn test_state() -> Arc<AppState> {
    Arc::new(AppState::with_tokens(vec![ORIGIN.to_string()], TokenStore::in_memory()))
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn req(method: &str, uri: &str, origin: Option<&str>, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(o) = origin {
        builder = builder.header("Origin", o);
    }
    if let Some(t) = token {
        builder = builder.header("Authorization", format!("Bearer {t}"));
    }
    match body {
        Some(v) => builder.header("content-type", "application/json").body(Body::from(v.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn health_requires_origin_but_not_a_token() {
    let app = build_app(test_state());

    let no_origin = app.clone().oneshot(req("GET", "/health", None, None, None)).await.unwrap();
    assert_eq!(no_origin.status(), StatusCode::UNAUTHORIZED);

    let with_origin = app.oneshot(req("GET", "/health", Some(ORIGIN), None, None)).await.unwrap();
    assert_eq!(with_origin.status(), StatusCode::OK);
    let body = body_json(with_origin).await;
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn disallowed_origin_is_rejected_even_with_a_valid_token() {
    let state = test_state();
    let token = state.tokens.pair(ORIGIN);
    let app = build_app(state);

    let resp = app.oneshot(req("GET", "/printers", Some("http://evil.example"), Some(&token), None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_routes_require_a_bearer_token_after_pairing() {
    let app = build_app(test_state());

    // Pair to get a token.
    let pair_resp = app
        .clone()
        .oneshot(req("POST", "/pair", Some(ORIGIN), None, Some(json!({ "appName": "Test App" }))))
        .await
        .unwrap();
    assert_eq!(pair_resp.status(), StatusCode::OK);
    let token = body_json(pair_resp).await["token"].as_str().unwrap().to_string();

    // Without the token: rejected.
    let unauthorized = app.clone().oneshot(req("GET", "/printers", Some(ORIGIN), None, None)).await.unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    // With the token: allowed.
    let authorized = app.oneshot(req("GET", "/printers", Some(ORIGIN), Some(&token), None)).await.unwrap();
    assert_eq!(authorized.status(), StatusCode::OK);
}

#[tokio::test]
async fn print_tcp_end_to_end_delivers_rendered_escpos_bytes() {
    let state = test_state();
    let token = state.tokens.pair(ORIGIN);
    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = Vec::new();
        // Read until the peer closes -- the agent closes its write side once
        // it's flushed, so this returns once the whole ESC/POS payload lands.
        tokio::io::AsyncReadExt::read_to_end(&mut socket, &mut buf).await.unwrap();
        buf
    });

    let document = json!({
        "version": "1.0",
        "paper": { "kind": "thermal-80" },
        "content": [{ "type": "text", "value": "hi" }, { "type": "cut" }]
    });
    let print_resp = app
        .clone()
        .oneshot(req(
            "POST",
            "/print-tcp",
            Some(ORIGIN),
            Some(&token),
            Some(json!({ "host": addr.ip().to_string(), "port": addr.port(), "document": document })),
        ))
        .await
        .unwrap();
    assert_eq!(print_resp.status(), StatusCode::OK);
    let job_id = body_json(print_resp).await["jobId"].as_str().unwrap().to_string();

    // Poll the job until it's done (bounded, since the transport runs on a
    // spawned background task -- this isn't synchronous with the HTTP response).
    let mut status = String::new();
    for _ in 0..50 {
        let job_resp = app.clone().oneshot(req("GET", &format!("/jobs/{job_id}"), Some(ORIGIN), Some(&token), None)).await.unwrap();
        let body = body_json(job_resp).await;
        status = body["status"].as_str().unwrap().to_string();
        if status == "done" || status == "error" {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }
    assert_eq!(status, "done");

    let received = server.await.unwrap();
    assert_eq!(&received[0..2], &[0x1b, 0x40]); // ESC @ initialize
    assert!(received.windows(2).any(|w| w == b"hi"));
}

#[tokio::test]
async fn unknown_job_id_returns_404() {
    let state = test_state();
    let token = state.tokens.pair(ORIGIN);
    let app = build_app(state);

    let resp = app.oneshot(req("GET", "/jobs/job_does_not_exist", Some(ORIGIN), Some(&token), None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pair_revoke_invalidates_the_token() {
    let app = build_app(test_state());
    let pair_resp = app.clone().oneshot(req("POST", "/pair", Some(ORIGIN), None, Some(json!({})))).await.unwrap();
    let token = body_json(pair_resp).await["token"].as_str().unwrap().to_string();

    let revoke_resp = app.clone().oneshot(req("POST", "/pair/revoke", Some(ORIGIN), Some(&token), None)).await.unwrap();
    assert_eq!(revoke_resp.status(), StatusCode::NO_CONTENT);

    let after_revoke = app.oneshot(req("GET", "/printers", Some(ORIGIN), Some(&token), None)).await.unwrap();
    assert_eq!(after_revoke.status(), StatusCode::UNAUTHORIZED);
}

/// A browser page always sends a CORS preflight OPTIONS request before a
/// cross-origin POST with a JSON body -- without a CORS layer, this request
/// gets no Access-Control-Allow-Origin header back and the browser blocks
/// the real request client-side (surfaces to JS as an opaque "Failed to
/// fetch", never even reaching this server's own auth/origin checks).
#[tokio::test]
async fn preflight_for_an_allowed_origin_gets_cors_headers_without_a_token() {
    let app = build_app(test_state());

    let preflight = Request::builder()
        .method("OPTIONS")
        .uri("/pair")
        .header("Origin", ORIGIN)
        .header("Access-Control-Request-Method", "POST")
        .header("Access-Control-Request-Headers", "content-type,authorization")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(preflight).await.unwrap();
    assert!(resp.status().is_success());
    assert_eq!(resp.headers().get("access-control-allow-origin").unwrap(), ORIGIN);
}

#[tokio::test]
async fn preflight_for_a_disallowed_origin_gets_no_cors_headers() {
    let app = build_app(test_state());

    let preflight = Request::builder()
        .method("OPTIONS")
        .uri("/pair")
        .header("Origin", "http://evil.example")
        .header("Access-Control-Request-Method", "POST")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(preflight).await.unwrap();
    assert!(resp.headers().get("access-control-allow-origin").is_none());
}
