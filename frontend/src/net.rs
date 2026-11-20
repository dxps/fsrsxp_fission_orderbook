//! HTTP + WebSocket glue for the fission frontend.
//!
//! HTTP calls are modelled as fission `JobSpec`s and registered on the `WebApp`
//! via `with_async(|r| r.register_job(...))`. Reducers launch them with
//! `ctx.effects.app(JOB, request).on_ok(...).on_err(...).dispatch()`.
//!
//! WebSocket is opened once at app start and pushes incoming `BookUpdate`s into
//! a thread-local queue; a `with_frame_hook` on the `WebApp` drains the queue
//! into `AppState` between frames.

use std::cell::RefCell;
use std::collections::VecDeque;

use serde::Serialize;
use shared::{BookSnapshot, BookUpdate, NewOrder, Symbol};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

pub const API_BASE: &str = "http://127.0.0.1:8080";
pub const WS_BASE: &str = "ws://127.0.0.1:8080";

thread_local! {
    pub(crate) static WS_INBOX: RefCell<VecDeque<BookUpdate>> = RefCell::new(VecDeque::new());
    static WS_HANDLE: RefCell<Option<WebSocket>> = RefCell::new(None);
}

/// Replace the currently open WebSocket with one subscribed to `symbol`.
pub fn connect_ws(symbol: &str) -> Result<(), JsValue> {
    WS_HANDLE.with(|slot| {
        if let Some(prev) = slot.borrow_mut().take() {
            let _ = prev.close();
        }
    });

    let url = format!("{WS_BASE}/ws/book/{symbol}");
    let ws = WebSocket::new(&url)?;

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(|event: MessageEvent| {
        if let Some(text) = event.data().as_string() {
            match serde_json::from_str::<BookUpdate>(&text) {
                Ok(update) => WS_INBOX.with(|q| q.borrow_mut().push_back(update)),
                Err(err) => log::warn!("ws parse error: {err}"),
            }
        }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    WS_HANDLE.with(|slot| *slot.borrow_mut() = Some(ws));
    Ok(())
}

/// Drain pending WS messages. Returns true if any were applied.
pub fn drain_inbox<F: FnMut(BookUpdate)>(mut apply: F) -> bool {
    let mut touched = false;
    WS_INBOX.with(|q| {
        let mut q = q.borrow_mut();
        while let Some(update) = q.pop_front() {
            apply(update);
            touched = true;
        }
    });
    touched
}

// ---------- HTTP via gloo-net ----------

pub async fn fetch_symbols() -> Result<Vec<Symbol>, String> {
    get_json(&format!("{API_BASE}/symbols")).await
}

pub async fn fetch_snapshot(symbol: Symbol) -> Result<BookSnapshot, String> {
    get_json(&format!("{API_BASE}/book/{symbol}")).await
}

pub async fn post_order(order: NewOrder) -> Result<BookUpdate, String> {
    post_json(&format!("{API_BASE}/orders"), &order).await
}

async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let resp = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("{} {}", resp.status(), resp.status_text()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn post_json<R: Serialize, T: serde::de::DeserializeOwned>(
    url: &str,
    body: &R,
) -> Result<T, String> {
    let resp = gloo_net::http::Request::post(url)
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("{} {}", resp.status(), resp.status_text()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}
