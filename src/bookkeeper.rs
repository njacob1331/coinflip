use dashmap::DashMap;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
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
    index: HashMap<String, u32>,
    reverse_index: Vec<String>,
    books: Vec<O>,
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
            index: HashMap::new(),
            reverse_index: Vec::new(),
            books: Vec::new(),
        }
    }

    pub fn orderbooks(&self) -> Arc<DashMap<String, O>> {
        self.orderbooks.clone()
    }

    fn reset_books(&mut self) {
        self.orderbooks.clear();
    }

    // fn update_books(&mut self, mut update: T) {
    //     if self.corrupted.contains(update.key()) {
    //         // if we have a symbol "blacklisted" and we receive a new snapshot,
    //         // that means the subscription manager did its job and resubscribed for us
    //         // at this point we can remove the symbol from the corrupted set and maintain the book
    //         if update.is_snapshot() {
    //             tracing::info!("new snapshot recieved for {}", update.key());
    //             self.corrupted.remove(update.key());
    //             self.orderbooks
    //                 .insert(update.take_key(), O::from_snapshot(update));
    //         }

    //         return;
    //     }

    //     let mut is_invalid = false;

    //     // Scope the borrow so it drops before we call .remove()
    //     if let Some(mut book) = self.orderbooks.get_mut(update.key()) {
    //         if !book.valid_sequence(&update) {
    //             is_invalid = true;
    //         } else {
    //             book.update(update);
    //             return;
    //         }
    //     }

    //     if is_invalid {
    //         tracing::info!("invalid sequence detected for {}", update.key());

    //         self.orderbooks.remove(update.key());

    //         let owned_key = update.key().to_string();

    //         self.corrupted.insert(owned_key.clone());
    //         let _ = self.resub_tx.try_send(owned_key);
    //         return;
    //     }

    //     self.orderbooks
    //         .insert(update.take_key(), O::from_snapshot(update));
    // }

    fn handle_snapshot(&mut self, mut update: T) {
        let key = update.key();
        tracing::info!("new snapshot received for {}", key);
        self.corrupted.remove(key);

        let idx = self.books.len();
        let owned_key = update.take_key(); // clone happens here, but only on snapshot

        self.index.insert(owned_key.clone(), idx as u32);
        self.reverse_index.push(owned_key);
        self.books.push(O::from_snapshot(update));
    }

    fn handle_update(&mut self, idx: usize, mut update: T) {
        let book = &mut self.books[idx];

        if !book.valid_sequence(&update) {
            self.index.remove(update.key());

            let last = self.books.len() - 1;

            self.books.swap(idx, last);
            self.reverse_index.swap(idx, last);

            self.books.pop();
            let swapped_key = self.reverse_index.pop().unwrap();

            if idx != last {
                self.index.insert(swapped_key, idx as u32);
            }

            let owned_key = update.take_key();
            self.corrupted.insert(owned_key.clone());
            let _ = self.resub_tx.try_send(owned_key);

            return;
        }

        book.update(update);
    }

    fn handle_new(&mut self, mut update: T) {
        let idx = self.books.len();
        let owned_key = update.take_key(); // clone happens here

        self.index.insert(owned_key.clone(), idx as u32);
        self.reverse_index.push(owned_key);
        self.books.push(O::from_snapshot(update));
    }

    fn update_books(&mut self, update: T) {
        // Handle corrupted symbols
        if self.corrupted.contains(update.key()) {
            if update.is_snapshot() {
                self.handle_snapshot(update);
            }

            return;
        }

        // Fast path: existing book
        if let Some(&idx) = self.index.get(update.key()) {
            self.handle_update(idx as usize, update);
            return;
        }

        self.handle_new(update);
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
