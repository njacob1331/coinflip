// bookkeeper.rs
use kanal::AsyncSender;
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
    common::{OrderbookSequence, OrderbookSnapshot, OrderbookUpdate, SharedStr},
    data_structures::IndexedStore,
    stats::TransportMsg,
    traits::OrderBook,
};

pub struct BookKeeper<O, S, D> {
    corrupted: HashMap<SharedStr, u32>,
    resub_tx: Sender<SharedStr>,
    store: IndexedStore<SharedStr, O>,
    forward_tx: AsyncSender<TransportMsg<SharedStr, OrderbookSnapshot>>,
    _phantom: std::marker::PhantomData<(S, D)>,
}

impl<O, S, D> BookKeeper<O, S, D>
where
    O: OrderBook<S, D>,
    S: Debug,
    D: Debug,
{
    pub fn new(
        resub_tx: Sender<SharedStr>,
        forward_tx: AsyncSender<TransportMsg<SharedStr, OrderbookSnapshot>>,
    ) -> Self {
        Self {
            corrupted: HashMap::new(),
            resub_tx,
            store: IndexedStore::new(),
            forward_tx,
            _phantom: std::marker::PhantomData,
        }
    }

    fn reset_books(&mut self) {
        self.store.clear();
        self.corrupted.clear();
    }

    fn forward_data(&self, data: OrderbookSnapshot) {
        let msg = TransportMsg::HandleData(data);
        if let Err(e) = self.forward_tx.try_send(msg) {
            tracing::error!("BOOKKEEPER: forwarding error {e}")
        }
    }

    fn forward_terminal(&self, key: SharedStr) {
        let msg = TransportMsg::RemoveData(key);
        if let Err(e) = self.forward_tx.try_send(msg) {
            tracing::error!("BOOKKEEPER: forwarding error {e}")
        }
    }

    fn handle_snapshot(&mut self, key: SharedStr, snapshot: S) {
        tracing::info!("snapshot received for {}", key);

        if let Some(idx) = self.corrupted.remove(&key) {
            self.store
                .update_data_at(idx as usize, O::from_snapshot(snapshot));
            return;
        }

        self.store.insert(key, O::from_snapshot(snapshot));
    }

    fn handle_diff(&mut self, key: SharedStr, diff: D) {
        if self.corrupted.contains_key(&key) {
            // Drop diffs for corrupted books; wait for a snapshot
            return;
        }

        match self.store.get_mut(&key) {
            Some(entry) => {
                let book = entry.data;
                match book.sequence(&diff) {
                    OrderbookSequence::Valid => {
                        book.update(diff);
                        let snapshot = book.snapshot(key.clone());
                        self.forward_data(snapshot);
                    }
                    OrderbookSequence::Gap => {
                        tracing::info!("gap detected for {}", key);
                        self.corrupted.insert(key.clone(), entry.index);
                        let _ = self.resub_tx.try_send(key);
                    }
                    OrderbookSequence::Stale => {}
                }
            }

            None => {
                // Diff arrived before snapshot — common on connect, just drop it
                tracing::debug!("diff for unknown key {}, waiting for snapshot", key);
                let _ = self.resub_tx.try_send(key);
            }
        }
    }

    fn handle_terminal(&mut self, key: SharedStr) {
        tracing::info!("terminal received for {} - removing book", &key);
        self.store.remove(&key);
        self.corrupted.remove(&key);
        self.forward_terminal(key);
    }

    pub async fn run(
        &mut self,
        mut orderbook_rx: Receiver<OrderbookUpdate<S, D>>,
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
