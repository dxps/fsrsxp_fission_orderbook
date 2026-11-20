use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};

use shared::{BookUpdate, NewOrder};

use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/symbols", get(get_symbols))
        .route("/book/:symbol", get(get_book))
        .route("/orders", post(post_order))
        .route("/ws/book/:symbol", get(ws_book))
        .with_state(state)
}

async fn get_symbols(State(state): State<AppState>) -> Json<Vec<String>> {
    Json(state.symbols())
}

async fn get_book(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> impl IntoResponse {
    match state.snapshot(&symbol) {
        Some(s) => Json(s).into_response(),
        None => (StatusCode::NOT_FOUND, "unknown symbol").into_response(),
    }
}

async fn post_order(
    State(state): State<AppState>,
    Json(order): Json<NewOrder>,
) -> impl IntoResponse {
    match state.place(order) {
        Ok(update) => Json(update).into_response(),
        Err(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
    }
}

async fn ws_book(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    match state.subscribe(&symbol) {
        Some((snapshot, rx)) => ws
            .on_upgrade(move |socket| run_socket(socket, snapshot, rx))
            .into_response(),
        None => (StatusCode::NOT_FOUND, "unknown symbol").into_response(),
    }
}

async fn run_socket(
    socket: WebSocket,
    snapshot: shared::BookSnapshot,
    mut rx: tokio::sync::broadcast::Receiver<BookUpdate>,
) {
    let (mut sink, mut stream) = socket.split();

    let initial = BookUpdate::Snapshot(snapshot);
    if let Ok(payload) = serde_json::to_string(&initial) {
        if sink.send(Message::Text(payload)).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            incoming = stream.next() => match incoming {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            },
            update = rx.recv() => match update {
                Ok(upd) => {
                    let Ok(payload) = serde_json::to_string(&upd) else { continue };
                    if sink.send(Message::Text(payload)).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    }
}
