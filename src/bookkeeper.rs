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
    data: Vec<O>,
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
            data: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    fn reset_books(&mut self) {
        self.index.clear();
        self.data.clear();
        self.corrupted.clear();
    }

    fn handle_snapshot(&mut self, key: K, snapshot: S) {
        tracing::info!("snapshot received for {}", key);
        self.corrupted.remove(&key);

        let idx = self.data.len() as u32;
        self.index.insert(key, idx);
        self.data.push(O::from_snapshot(snapshot));
    }

    fn handle_diff(&mut self, key: K, diff: D) {
        if self.corrupted.contains(&key) {
            // Drop diffs for corrupted books; wait for a snapshot
            return;
        }

        match self.index.get(&key) {
            Some(&idx) => {
                let idx = idx as usize;
                let book = &mut self.data[idx];
                match book.sequence(&diff) {
                    OrderbookSequence::Valid => book.update(diff),
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
        let last = self.data.len() - 1;
        
        if idx != last {
            self.data.swap(idx, last);
            self.index.insert(key.clone(), idx as u32);
        }
        
        self.data.pop();
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
