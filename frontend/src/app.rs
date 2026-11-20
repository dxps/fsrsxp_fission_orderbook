//! Root widget + reducers for the order-book demo.
//!
//! Follows the component shape documented in `fission-macros`'s README and the
//! `resize_app.rs` fixture: no `fission_component!` macro — just an
//! `impl From<App> for Widget`, with `fission::build::current::<State>()` to
//! obtain the build `ctx` + `view`.

use fission::prelude::*;
use shared::{NewOrder, Side};

use crate::jobs::{FETCH_SNAPSHOT, FETCH_SYMBOLS, POST_ORDER};
use crate::state::AppState;

#[derive(Clone)]
pub struct OrderBookApp;

impl From<OrderBookApp> for Widget {
    fn from(_: OrderBookApp) -> Widget {
        let (ctx, view) = fission::build::current::<AppState>();
        let tokens = &view.env().theme.tokens;
        let state = view.state();

        let mut children: Vec<Widget> = widgets![
            Text::new("Fission Order Book").size(tokens.typography.heading1_size),
            Text::new(format!(
                "symbol: {}   seq: {}   {}",
                state.selected, state.seq, state.status
            ))
            .size(tokens.typography.font_size_sm),
        ];

        // Symbol selector row
        let mut sym_buttons: Vec<Widget> = Vec::new();
        for sym in &state.symbols {
            let label = sym.clone();
            let payload = sym.clone();
            sym_buttons.push(
                Button {
                    child: Some(Text::new(label).into()),
                    on_press: Some(with_reducer!(
                        ctx,
                        SelectSymbol(payload),
                        on_select_symbol
                    )),
                    ..Default::default()
                }
                .into(),
            );
        }
        if !sym_buttons.is_empty() {
            children.push(
                Row {
                    gap: Some(tokens.spacing.s),
                    children: sym_buttons,
                    ..Default::default()
                }
                .into(),
            );
        }

        // Book view: BID column | ASK column
        let mut bid_rows: Vec<Widget> = widgets![
            Text::new("BIDS").size(tokens.typography.heading_size),
        ];
        for lvl in &state.bids {
            bid_rows.push(
                Text::new(format!("{:>10.2}   {:>10.4}", lvl.price, lvl.qty))
                    .size(tokens.typography.body_medium_size)
                    .into(),
            );
        }
        let mut ask_rows: Vec<Widget> = widgets![
            Text::new("ASKS").size(tokens.typography.heading_size),
        ];
        for lvl in &state.asks {
            ask_rows.push(
                Text::new(format!("{:>10.2}   {:>10.4}", lvl.price, lvl.qty))
                    .size(tokens.typography.body_medium_size)
                    .into(),
            );
        }
        children.push(
            Row {
                gap: Some(tokens.spacing.xl),
                children: widgets![
                    Column {
                        gap: Some(tokens.spacing.xs),
                        children: bid_rows,
                        ..Default::default()
                    },
                    Column {
                        gap: Some(tokens.spacing.xs),
                        children: ask_rows,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }
            .into(),
        );

        // Order form
        children.push(
            Text::new("Place order")
                .size(tokens.typography.heading_size)
                .into(),
        );
        children.push(
            Row {
                gap: Some(tokens.spacing.s),
                children: widgets![
                    TextInput {
                        value: state.form_price.clone(),
                        placeholder: Some("price".into()),
                        on_input: Some(with_reducer!(ctx, EditPrice, on_edit_price)),
                        ..Default::default()
                    },
                    TextInput {
                        value: state.form_qty.clone(),
                        placeholder: Some("qty".into()),
                        on_input: Some(with_reducer!(ctx, EditQty, on_edit_qty)),
                        ..Default::default()
                    },
                    Button {
                        child: Some(
                            Text::new(match state.form_side {
                                Side::Bid => "BID",
                                Side::Ask => "ASK",
                            })
                            .into(),
                        ),
                        on_press: Some(with_reducer!(ctx, ToggleSide, on_toggle_side)),
                        ..Default::default()
                    },
                    Button {
                        child: Some(Text::new("Submit").into()),
                        on_press: Some(with_reducer!(ctx, Submit, on_submit)),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }
            .into(),
        );

        Container::new(Column {
            gap: Some(tokens.spacing.l),
            children,
            ..Default::default()
        })
        .padding_all(tokens.spacing.xl)
        .bg(tokens.colors.background)
        .into()
    }
}

// ─── Reducers ───────────────────────────────────────────────────────────────

#[fission_reducer(BootstrapStart)]
pub fn on_bootstrap(state: &mut AppState, ctx: &mut ReducerContext<AppState>) {
    state.status = "loading symbols…".into();
    let on_ok = ctx.effects.bind(SymbolsLoaded, reduce_with!(on_symbols_loaded));
    let on_err = ctx.effects.bind(NetError, reduce_with!(on_net_error));
    ctx.effects
        .app(FETCH_SYMBOLS, ())
        .on_ok(on_ok)
        .on_err(on_err)
        .dispatch();
}

#[fission_reducer(SymbolsLoaded)]
pub fn on_symbols_loaded(state: &mut AppState, ctx: &mut ReducerContext<AppState>) {
    let Some(syms) = ctx.input.job_ok(FETCH_SYMBOLS) else {
        return;
    };
    state.symbols = syms;
    if state.selected.is_empty() {
        if let Some(first) = state.symbols.first().cloned() {
            state.selected = first.clone();
            dispatch_load(state, ctx, &first);
            let _ = crate::net::connect_ws(&first);
        }
    }
}

#[fission_reducer(SelectSymbol)]
pub fn on_select_symbol(
    state: &mut AppState,
    symbol: String,
    ctx: &mut ReducerContext<AppState>,
) {
    if symbol == state.selected {
        return;
    }
    state.selected = symbol.clone();
    state.bids.clear();
    state.asks.clear();
    state.seq = 0;
    dispatch_load(state, ctx, &symbol);
    let _ = crate::net::connect_ws(&symbol);
}

fn dispatch_load(state: &mut AppState, ctx: &mut ReducerContext<AppState>, symbol: &str) {
    state.status = format!("loading {symbol}…");
    let on_ok = ctx.effects.bind(SnapshotLoaded, reduce_with!(on_snapshot_loaded));
    let on_err = ctx.effects.bind(NetError, reduce_with!(on_net_error));
    ctx.effects
        .app(FETCH_SNAPSHOT, symbol.to_string())
        .on_ok(on_ok)
        .on_err(on_err)
        .dispatch();
}

#[fission_reducer(SnapshotLoaded)]
pub fn on_snapshot_loaded(state: &mut AppState, ctx: &mut ReducerContext<AppState>) {
    if let Some(snap) = ctx.input.job_ok(FETCH_SNAPSHOT) {
        state.apply_snapshot(snap);
        state.status = "connected".into();
    }
}

#[fission_reducer(EditPrice)]
pub fn on_edit_price(state: &mut AppState, ctx: &mut ReducerContext<AppState>) {
    if let Some(c) = ctx.input.text_change() {
        state.form_price = c.new_text.clone();
    }
}

#[fission_reducer(EditQty)]
pub fn on_edit_qty(state: &mut AppState, ctx: &mut ReducerContext<AppState>) {
    if let Some(c) = ctx.input.text_change() {
        state.form_qty = c.new_text.clone();
    }
}

#[fission_reducer(ToggleSide)]
pub fn on_toggle_side(state: &mut AppState) {
    state.form_side = match state.form_side {
        Side::Bid => Side::Ask,
        Side::Ask => Side::Bid,
    };
}

#[fission_reducer(Submit)]
pub fn on_submit(state: &mut AppState, ctx: &mut ReducerContext<AppState>) {
    let (Ok(price), Ok(qty)) = (
        state.form_price.parse::<f64>(),
        state.form_qty.parse::<f64>(),
    ) else {
        state.status = "price/qty invalid".into();
        return;
    };
    if qty <= 0.0 {
        state.status = "qty must be > 0".into();
        return;
    }
    let order = NewOrder {
        symbol: state.selected.clone(),
        side: state.form_side,
        price,
        qty,
    };
    state.status = "submitting…".into();
    let on_ok = ctx.effects.bind(OrderPlaced, reduce_with!(on_order_placed));
    let on_err = ctx.effects.bind(NetError, reduce_with!(on_net_error));
    ctx.effects
        .app(POST_ORDER, order)
        .on_ok(on_ok)
        .on_err(on_err)
        .dispatch();
}

#[fission_reducer(OrderPlaced)]
pub fn on_order_placed(state: &mut AppState, _ctx: &mut ReducerContext<AppState>) {
    state.status = "order accepted".into();
    state.form_price.clear();
    state.form_qty.clear();
}

#[fission_reducer(NetError)]
pub fn on_net_error(state: &mut AppState, ctx: &mut ReducerContext<AppState>) {
    let msg = ctx
        .input
        .job_error_message(FETCH_SYMBOLS)
        .or_else(|| ctx.input.job_error_message(FETCH_SNAPSHOT))
        .or_else(|| ctx.input.job_error_message(POST_ORDER))
        .unwrap_or("network error");
    state.status = format!("error: {msg}");
}
