use fission::GlobalState;
use shared::{BookSnapshot, BookUpdate, Level, Side, Symbol};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct AppState {
    pub symbols: Vec<Symbol>,
    pub selected: Symbol,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
    pub seq: u64,
    pub status: String,
    pub form_price: String,
    pub form_qty: String,
    pub form_side: Side,
}

impl AppState {
    pub fn apply(&mut self, update: BookUpdate) {
        match update {
            BookUpdate::Snapshot(s) => self.apply_snapshot(s),
            BookUpdate::Delta { symbol, side, price, qty, seq } => {
                if symbol == self.selected {
                    merge_level(&mut self.bids, &mut self.asks, side, price, qty);
                    self.seq = seq;
                }
            }
        }
    }

    pub fn apply_snapshot(&mut self, snap: BookSnapshot) {
        if snap.symbol == self.selected || self.selected.is_empty() {
            self.selected = snap.symbol;
            self.bids = snap.bids;
            self.asks = snap.asks;
            self.seq = snap.seq;
        }
    }
}

impl GlobalState for AppState {}

fn merge_level(bids: &mut Vec<Level>, asks: &mut Vec<Level>, side: Side, price: f64, qty: f64) {
    let (target, descending) = match side {
        Side::Bid => (bids, true),
        Side::Ask => (asks, false),
    };
    if let Some(pos) = target.iter().position(|l| l.price == price) {
        if qty <= 0.0 {
            target.remove(pos);
        } else {
            target[pos].qty = qty;
        }
        return;
    }
    if qty <= 0.0 {
        return;
    }
    let insert_at = target
        .iter()
        .position(|l| if descending { l.price < price } else { l.price > price })
        .unwrap_or(target.len());
    target.insert(insert_at, Level { price, qty });
}
