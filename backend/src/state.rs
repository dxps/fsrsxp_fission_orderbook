use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio::sync::broadcast;

use shared::{BookSnapshot, BookUpdate, NewOrder, Symbol};

use crate::book::Book;

const BROADCAST_CAPACITY: usize = 256;
const DEFAULT_DEPTH: usize = 10;

struct SymbolChannel {
    book: Book,
    tx: broadcast::Sender<BookUpdate>,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<HashMap<Symbol, SymbolChannel>>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        for sym in ["BTCUSD", "ETHUSD", "TLV"] {
            let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
            map.insert(
                sym.to_string(),
                SymbolChannel {
                    book: Book::new(sym.to_string()),
                    tx,
                },
            );
        }
        Self {
            inner: Arc::new(RwLock::new(map)),
        }
    }

    pub fn symbols(&self) -> Vec<Symbol> {
        let guard = self.inner.read().unwrap();
        let mut syms: Vec<_> = guard.keys().cloned().collect();
        syms.sort();
        syms
    }

    pub fn snapshot(&self, symbol: &str) -> Option<BookSnapshot> {
        let guard = self.inner.read().unwrap();
        guard.get(symbol).map(|c| c.book.snapshot(DEFAULT_DEPTH))
    }

    pub fn subscribe(&self, symbol: &str) -> Option<(BookSnapshot, broadcast::Receiver<BookUpdate>)> {
        let guard = self.inner.read().unwrap();
        let c = guard.get(symbol)?;
        Some((c.book.snapshot(DEFAULT_DEPTH), c.tx.subscribe()))
    }

    pub fn place(&self, order: NewOrder) -> Result<BookUpdate, &'static str> {
        let mut guard = self.inner.write().unwrap();
        let c = guard.get_mut(&order.symbol).ok_or("unknown symbol")?;
        let update = c.book.apply(order.side, order.price, order.qty);
        let _ = c.tx.send(update.clone());
        Ok(update)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
