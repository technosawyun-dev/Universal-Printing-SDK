//! `ws://127.0.0.1:18999/events`, per docs/PROTOCOL.md. Browsers can't set
//! custom headers on a WebSocket handshake, so the pairing token travels as
//! a `?token=` query param instead of `Authorization: Bearer` here -- Origin
//! is still a plain header and already checked by `middleware::auth` before
//! this handler runs.

use crate::error::AgentError;
use crate::state::AppState;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
use std::sync::Arc;

pub async fn events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, AgentError> {
    let origin = headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    let token = params.get("token").ok_or_else(|| AgentError::Unauthorized("missing ?token= query param".to_string()))?;
    if !state.tokens.is_valid(&origin, token) {
        return Err(AgentError::Unauthorized("invalid or expired pairing token".to_string()));
    }
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state)).into_response())
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.events_tx.subscribe();
    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            incoming = socket.recv() => {
                if incoming.is_none() {
                    break; // client closed the connection
                }
                if matches!(incoming, Some(Err(_))) {
                    break;
                }
                // Any client -> server message (e.g. a ping) is ignored; this
                // channel is server -> client notifications only.
            }
        }
    }
}
