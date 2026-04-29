use dashmap::{DashMap, DashSet};
use std::{collections::HashSet, sync::Arc};
use tokio_util::sync::CancellationToken;

use tokio::sync::{
    Notify,
    mpsc::{self, Receiver, Sender},
};

use crate::gemini::{messages::OrderbookUpdate, orderbook::Orderbook};

pub struct BookKeeper {
    orderbooks: Arc<DashMap<String, Orderbook>>,
    corrupted: HashSet<String>,
    resub_tx: Sender<String>,
}

impl BookKeeper {
    pub fn new(resub_tx: Sender<String>) -> Self {
        Self {
            orderbooks: Arc::new(DashMap::new()),
            corrupted: HashSet::new(),
            resub_tx,
        }
    }

    fn orderbooks(&self) -> Arc<DashMap<String, Orderbook>> {
        self.orderbooks.clone()
    }

    fn reset_books(&mut self) {
        self.orderbooks.clear();
    }

    fn update_books(&mut self, mut update: OrderbookUpdate) {
        if self.corrupted.contains(&update.symbol) {
            // if we have a symbol "blacklisted" and we receive a new snapshot,
            // that means the subscription manager did its job and resubscribed for us
            // at this point we can remove the symbol from the corrupted set and maintain the book
            if update.first_update_id == update.last_update_id {
                tracing::info!("new snapshot recieved for {}", &update.symbol);
                self.corrupted.remove(&update.symbol);
                self.orderbooks.insert(
                    std::mem::take(&mut update.symbol),
                    Orderbook::from_snapshot(update),
                );
            }

            return;
        }

        let mut is_invalid = false;

        // Scope the borrow so it drops before we call .remove()
        if let Some(mut book) = self.orderbooks.get_mut(&update.symbol) {
            if !book.is_valid_sequence(&update) {
                is_invalid = true;
            } else {
                book.update(update);
                return;
            }
        }

        if is_invalid {
            tracing::info!("invalid sequence detected for {}", &update.symbol);
            self.orderbooks.remove(&update.symbol);
            self.corrupted.insert(update.symbol.clone());
            let _ = self.resub_tx.try_send(update.symbol);
            return;
        }

        self.orderbooks
            .entry(std::mem::take(&mut update.symbol))
            .or_insert(Orderbook::from_snapshot(update));
    }

    pub async fn run(
        &mut self,
        mut orderbook_rx: Receiver<OrderbookUpdate>,
        connection_reset: Arc<Notify>,
        cancel: CancellationToken,
    ) {
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
