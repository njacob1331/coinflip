// bookkeeper.rs
use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
};
use tokio::sync::{
    Notify,
    mpsc::{Receiver, Sender},
};
use tokio_util::sync::CancellationToken;

use crate::{
    common::{OrderbookSequence, OrderbookUpdate},
    traits::OrderBook,
};

pub struct BookKeeper<K, O, S, D> {
    corrupted: HashSet<K>,
    resub_tx: Sender<K>,
    index: HashMap<K, u32>,
    reverse_index: Vec<K>,
    books: Vec<O>,
    _phantom: std::marker::PhantomData<(S, D)>,
}

impl<K, O, S, D> BookKeeper<K, O, S, D>
where
    K: Clone + Eq + Hash + Display,
    O: OrderBook<S, D>,
    S: Debug,
    D: Debug,
{
    pub fn new(resub_tx: Sender<K>) -> Self {
        Self {
            corrupted: HashSet::new(),
            resub_tx,
            index: HashMap::new(),
            reverse_index: Vec::new(),
            books: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    fn reset_books(&mut self) {
        self.index.clear();
        self.reverse_index.clear();
        self.books.clear();
        self.corrupted.clear();
    }

    fn handle_snapshot(&mut self, key: K, data: S) {
        tracing::info!("snapshot received for {}", key);
        self.corrupted.remove(&key);

        let idx = self.books.len() as u32;
        self.index.insert(key.clone(), idx);
        self.reverse_index.push(key);
        self.books.push(O::from_snapshot(data));
    }

    fn handle_diff(&mut self, key: K, data: D) {
        if self.corrupted.contains(&key) {
            // Drop diffs for corrupted books; wait for a snapshot
            return;
        }

        match self.index.get(&key) {
            Some(&idx) => {
                let idx = idx as usize;
                let book = &mut self.books[idx];
                match book.sequence(&data) {
                    OrderbookSequence::Valid => book.update(data),
                    OrderbookSequence::Stale => {}
                    OrderbookSequence::Gap => {
                        tracing::info!("gap detected for {}", key);
                        self.remove_book(idx, &key);
                        self.corrupted.insert(key.clone());
                        let _ = self.resub_tx.try_send(key);
                    }
                }
            }
            None => {
                // Diff arrived before snapshot — common on connect, just drop it
                tracing::debug!("diff for unknown key {}, waiting for snapshot", key);
                let _ = self.resub_tx.try_send(key);
            }
        }
    }

    fn handle_terminal(&mut self, key: K) {
        tracing::info!("terminal received for {} - removing book", &key);
        if let Some(&idx) = self.index.get(&key) {
            self.remove_book(idx as usize, &key);
        }
        self.corrupted.remove(&key);
    }

    /// Swap-remove a book at `idx`, keeping index/reverse_index consistent.
    fn remove_book(&mut self, idx: usize, key: &K) {
        self.index.remove(key);
        let last = self.books.len() - 1;
        if idx != last {
            self.books.swap(idx, last);
            self.reverse_index.swap(idx, last);
            // The key that was at `last` is now at `idx` — update its index entry
            let swapped_key = &self.reverse_index[idx];
            self.index.insert(swapped_key.clone(), idx as u32);
        }
        self.books.pop();
        self.reverse_index.pop();
    }

    pub async fn run(
        &mut self,
        mut orderbook_rx: Receiver<OrderbookUpdate<K, S, D>>,
        _connection_reset: Arc<Notify>,
        cancel: CancellationToken,
    ) {
        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                msg = orderbook_rx.recv() => {
                    match msg {
                        Some(OrderbookUpdate::Snapshot { key, data }) => {

                            self.handle_snapshot(key, data);
                        }
                        Some(OrderbookUpdate::Diff { key, data }) => {
                            self.handle_diff(key, data);
                        }
                        Some(OrderbookUpdate::Terminal(key)) => {
                            self.handle_terminal(key);
                        }
                        None => break, // channel closed
                    }
                }
            }
        }
        tracing::info!("BookKeeper exiting");
    }
}
