# fission.rs + axum order-book demo

A tiny full-stack Rust demo so you can digest how a [fission.rs](https://fission.rs/) web UI talks to an [axum](https://github.com/tokio-rs/axum) backend over **REST + WebSocket**.

## What's in it

```
shared/          # serde DTOs shared by both sides
backend/         # axum: GET /symbols, GET /book/:sym, POST /orders, GET /ws/book/:sym
frontend/        # fission.rs web app (WASM): UI + state + HTTP jobs + WS inbox
  src/state.rs   # AppState — the GlobalState
  src/net.rs     # gloo-net HTTP + web-sys WebSocket into a thread-local inbox
  src/jobs.rs    # JobSpec per HTTP call, registered on the WebApp
  src/app.rs     # OrderBookApp component + reducers
  src/lib.rs     # WASM entry; with_async + with_frame_hook wire everything
  index.html     # host page with #fission-web-mount
```

## Architecture at a glance

- **HTTP**: each endpoint is a `JobSpec` (defined in `frontend/src/jobs.rs`). The `WebApp::with_async(|r| r.register_job(JOB, handler))` registers a `gloo-net` handler for each one. A reducer fires one with
  ```rust
  let on_ok = ctx.effects.bind(SnapshotLoaded, reduce_with!(on_snapshot_loaded));
  let on_err = ctx.effects.bind(NetError,      reduce_with!(on_net_error));
  ctx.effects.app(FETCH_SNAPSHOT, symbol).on_ok(on_ok).on_err(on_err).dispatch();
  ```
  The result-handling reducer reads the response with `ctx.input.job_ok(FETCH_SNAPSHOT)`.
- **WebSocket**: `web-sys` opens the socket once per symbol. `onmessage` deserializes a `BookUpdate` and pushes it into a `thread_local!` `VecDeque`. `WebApp::with_frame_hook(|state: &mut AppState| { drain_inbox(|u| state.apply(u)) })` drains the queue into state every frame; returning `true` tells fission the view needs redraw.
- **Backend**: `tokio::sync::broadcast` per symbol. REST writes apply to the book and emit a `BookUpdate::Delta`, which is also sent down any open WS subscriber channel.

## Run it

Backend (port 8080):

```bash
cargo run -p backend
```

Frontend — using the fission CLI (from the fission docs):

```bash
cargo install cargo-fission
cd frontend
fission add-target web --project-dir .
fission run --target web --project-dir .
```

The dev server port is printed on start. Open it in a browser; the UI should show the three seeded symbols (BTCUSD, ETHUSD, TLV), load the first one's book, and receive live deltas over the WebSocket.

## Status

- `cargo check -p backend` — clean (native).
- `cargo check -p frontend --target wasm32-unknown-unknown` — clean.
- Tested at the type-check level, not yet run end-to-end against the live dev server; a `cargo run -p backend` next to `fission run --target web` in the `frontend` folder is the next step.

## Patterns worth noting

- The component is **not** a `fission_component!` macro invocation — it's plain `impl From<OrderBookApp> for Widget`, following the `resize_app.rs` fixture in the `fission-test-driver` crate. The ctx + view pair comes from `fission::build::current::<AppState>()`.
- Widgets are **struct literals** with `..Default::default()`. `widgets![...]` collects children with automatic `.into()`.
- Reducers come from `#[fission_reducer(ActionName)]`. Payloads become positional args before an optional final `ctx: &mut ReducerContext<State>`.
- **Binding handlers requires `reduce_with!(handler)`** to coerce the fn item to the fn-pointer type `IntoHandler` wants.
- Hoist each `ctx.effects.bind(...)` into a `let` before the `ctx.effects.app(...)` chain — otherwise the second `bind` conflicts with the first borrow.
- CORS on the backend is wide open (`Any`) because the fission dev-server origin is runtime-decided; tighten before any real deployment.
