use std::collections::BTreeMap;

use shared::{BookSnapshot, BookUpdate, Level, Side, Symbol};

const PRICE_SCALE: f64 = 100.0;

fn to_ticks(price: f64) -> u64 {
    (price * PRICE_SCALE).round() as u64
}

fn from_ticks(ticks: u64) -> f64 {
    ticks as f64 / PRICE_SCALE
}

pub struct Book {
    pub symbol: Symbol,
    bids: BTreeMap<u64, f64>,
    asks: BTreeMap<u64, f64>,
    seq: u64,
}

impl Book {
    pub fn new(symbol: Symbol) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            seq: 0,
        }
    }

    pub fn apply(&mut self, side: Side, price: f64, qty: f64) -> BookUpdate {
        let ticks = to_ticks(price);
        let side_map = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };
        let entry = side_map.entry(ticks).or_insert(0.0);
        *entry += qty;
        let new_qty = *entry;
        if new_qty <= 0.0 {
            side_map.remove(&ticks);
        }
        self.seq += 1;
        BookUpdate::Delta {
            symbol: self.symbol.clone(),
            side,
            price,
            qty: new_qty.max(0.0),
            seq: self.seq,
        }
    }

    pub fn snapshot(&self, depth: usize) -> BookSnapshot {
        let bids = self
            .bids
            .iter()
            .rev()
            .take(depth)
            .map(|(&t, &q)| Level {
                price: from_ticks(t),
                qty: q,
            })
            .collect();
        let asks = self
            .asks
            .iter()
            .take(depth)
            .map(|(&t, &q)| Level {
                price: from_ticks(t),
                qty: q,
            })
            .collect();
        BookSnapshot {
            symbol: self.symbol.clone(),
            bids,
            asks,
            seq: self.seq,
        }
    }
}
