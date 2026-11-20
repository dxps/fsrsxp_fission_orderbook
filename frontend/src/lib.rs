//! WASM entry point for the order-book demo.
//!
//! Boot order (inside `#[wasm_bindgen(start)] run_web`):
//!   1. Install panic hook
//!   2. Open the WebSocket to the default symbol (its `onmessage` pushes into
//!      a thread-local inbox; a `with_frame_hook` drains it into `AppState`)
//!   3. Build the `WebApp<AppState, OrderBookApp>`
//!   4. `with_async` registers HTTP jobs (handlers run via
//!      `wasm_bindgen_futures::spawn_local`; non-Send futures are fine)
//!   5. `with_frame_hook` drains the WS inbox into `AppState` every frame
//!   6. `with_startup_action(BootstrapStart)` kicks off the HTTP bootstrap
//!   7. `.mount(...).run()` attaches to the host page's `<div>`

pub mod app;
pub mod jobs;
pub mod net;
pub mod state;

use fission::shell::{BrowserDefaults, WebApp};

use crate::app::{BootstrapStart, OrderBookApp};
use crate::jobs::{FETCH_SNAPSHOT, FETCH_SYMBOLS, POST_ORDER};
use crate::net::{connect_ws, drain_inbox, fetch_snapshot, fetch_symbols, post_order};
use crate::state::AppState;

const DEFAULT_SYMBOL: &str = "BTCUSD";

#[cfg(all(target_arch = "wasm32", feature = "standalone-entry"))]
fn web_app() -> WebApp<AppState, OrderBookApp> {
    WebApp::<AppState, _>::new(OrderBookApp)
        .with_title("Fission Order Book")
        .mount("#fission-web-mount")
        .with_browser_defaults(BrowserDefaults::KEYBOARD)
        .with_async(|r| {
            r.register_job(FETCH_SYMBOLS, |_req, _ctx| async move { fetch_symbols().await });
            r.register_job(FETCH_SNAPSHOT, |sym, _ctx| async move {
                fetch_snapshot(sym).await
            });
            r.register_job(POST_ORDER, |order, _ctx| async move {
                post_order(order).await
            });
        })
        .with_frame_hook(|state: &mut AppState| -> bool {
            drain_inbox(|update| state.apply(update))
        })
        .with_startup_action(BootstrapStart)
}

#[cfg(all(target_arch = "wasm32", feature = "standalone-entry"))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    let _ = connect_ws(DEFAULT_SYMBOL);
    web_app()
        .run()
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
}

// Keep the crate buildable on non-wasm targets for host-side `cargo check`.
#[cfg(not(target_arch = "wasm32"))]
pub fn _noop() {}
