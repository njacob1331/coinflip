use dashmap::DashMap;
use std::{collections::HashSet, sync::Arc};
use tokio_util::sync::CancellationToken;

use tokio::sync::{
    Notify,
    mpsc::{Receiver, Sender},
};

use crate::traits::{OrderBook, OrderbookData};

pub struct BookKeeper<O, T> {
    orderbooks: Arc<DashMap<String, O>>,
    corrupted: HashSet<String>,
    resub_tx: Sender<String>,
    _phantom: std::marker::PhantomData<T>,
}

impl<O, T> BookKeeper<O, T>
where
    O: OrderBook<T>,
    T: OrderbookData,
{
    pub fn new(resub_tx: Sender<String>) -> Self {
        Self {
            orderbooks: Arc::new(DashMap::new()),
            corrupted: HashSet::new(),
            resub_tx,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn orderbooks(&self) -> Arc<DashMap<String, O>> {
        self.orderbooks.clone()
    }

    fn reset_books(&mut self) {
        self.orderbooks.clear();
    }

    fn update_books(&mut self, mut update: T) {
        if self.corrupted.contains(update.key()) {
            // if we have a symbol "blacklisted" and we receive a new snapshot,
            // that means the subscription manager did its job and resubscribed for us
            // at this point we can remove the symbol from the corrupted set and maintain the book
            if update.is_snapshot() {
                tracing::info!("new snapshot recieved for {}", update.key());
                self.corrupted.remove(update.key());
                self.orderbooks
                    .insert(update.take_key(), O::from_snapshot(update));
            }

            return;
        }

        let mut is_invalid = false;

        // Scope the borrow so it drops before we call .remove()
        if let Some(mut book) = self.orderbooks.get_mut(update.key()) {
            if !book.valid_sequence(&update) {
                is_invalid = true;
            } else {
                book.update(update);
                return;
            }
        }

        if is_invalid {
            tracing::info!("invalid sequence detected for {}", update.key());

            self.orderbooks.remove(update.key());

            let owned_key = update.key().to_string();

            self.corrupted.insert(owned_key.clone());
            let _ = self.resub_tx.try_send(owned_key);
            return;
        }

        self.orderbooks
            .insert(update.take_key(), O::from_snapshot(update));
    }

    pub async fn run(
        &mut self,
        mut orderbook_rx: Receiver<T>,
        connection_reset: Arc<Notify>,
        cancel: CancellationToken,
    ) {
        // we don't need multiple branches here
        // this should simply read from the channel until it dies
        // the lifecycle can be controlled upstream and centralized to the session

        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,

                feed = orderbook_rx.recv() => {
                    match feed {
                        Some(update) => self.update_books(update),
                        None => break
                    }
                }

                // _ = connection_reset.notified() => self.reset_books(),
            }
        }

        println!("exchange consumer exiting");
    }
}
