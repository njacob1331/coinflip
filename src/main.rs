use tracing_subscriber;

use std::sync::Arc;

use anyhow::Result;
use dashmap::{DashMap, DashSet};
use futures_util::future::join_all;

use tokio::sync::{
    Notify,
    mpsc::{self, Receiver, Sender},
};
use tokio_util::sync::CancellationToken;

use crate::{
    gemini::{
        client::GeminiClient,
        messages::{OrderbookUpdate, Subscriptions},
        orderbook::Orderbook,
        parser::GeminiParser,
        poll::MarketPoller,
        router::GeminiRouter,
    },
    session::{Request, SessionManager},
};

mod common;
mod gemini;
mod session;
mod traits;
mod ws;

struct BookKeeper {
    orderbooks: Arc<DashMap<String, Orderbook>>,
    corrupted: Arc<DashSet<String>>,
    resub_notification: Arc<Notify>,
}

impl BookKeeper {
    fn new() -> Self {
        Self {
            orderbooks: Arc::new(DashMap::new()),
            corrupted: Arc::new(DashSet::new()),
            resub_notification: Arc::new(Notify::new()),
        }
    }

    fn orderbooks(&self) -> Arc<DashMap<String, Orderbook>> {
        self.orderbooks.clone()
    }

    fn corrupted(&self) -> Arc<DashSet<String>> {
        self.corrupted.clone()
    }

    fn resub_notification(&self) -> Arc<Notify> {
        self.resub_notification.clone()
    }

    fn reset_books(&mut self) {
        self.orderbooks.clear();
    }

    fn update_books(&self, mut update: OrderbookUpdate) {
        if self.corrupted.contains(&update.symbol) {
            // if we have a symbol "blacklisted" and we receive a new snapshot,
            // that means the subscription manager did its job and resubscribed for us
            // at this point we can remove the symbol from the corrupted set and maintain the book
            if update.first_update_id == update.last_update_id {
                tracing::info!("new snapshot recieved for {}", &update.symbol);
                self.corrupted.remove(&update.symbol);
                self.orderbooks
                    .insert(std::mem::take(&mut update.symbol), Orderbook::from_snapshot(update));
            }

            return;
        }

        if let Some(mut book) = self.orderbooks.get_mut(&update.symbol) {
            if !book.is_valid_sequence(&update) {
                // tracing::info!("invalid sequence detected for {}", &update.symbol);
                self.orderbooks.remove(&update.symbol);
                self.corrupted.insert(update.symbol);
                self.resub_notification.notify_one();
                return;
            }

            book.update(update);
            return;
        }

        self.orderbooks
            .entry(std::mem::take(&mut update.symbol))
            .or_insert(Orderbook::from_snapshot(update));
    }

    async fn run(
        &mut self,
        mut orderbook_rx: Receiver<OrderbookUpdate>,
        connection_reset: Arc<Notify>,
    ) {
        loop {
            tokio::select! {
                feed = orderbook_rx.recv() => {
                    match feed {
                        Some(update) => self.update_books(update),
                        None => break
                    }
                }

                _ = connection_reset.notified() => self.reset_books()
            }
        }

        println!("exchange consumer exiting");
    }
}

use tracing_subscriber::{EnvFilter, fmt};

fn init_tracing() {
    let filter = EnvFilter::new("coinflip=debug");

    fmt()
        .with_env_filter(filter)
        .with_target(false) // optional: hides module paths
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let shutdown = CancellationToken::new();

    // the type on Request should be something like enum GeminiSend
    // which carries every possible type that can be sent via gemini ws
    let (request_tx, request_rx) = mpsc::channel::<Request<Subscriptions>>(32);
    let order_tx = request_tx.clone();

    let parser = GeminiParser::new();
    let mut router = GeminiRouter::new();
    let orderbook_rx = router.connect("orderbook");

    let mut bookeeper = BookKeeper::new();
    let client = Arc::new(GeminiClient::new());
    let mut session_manager = SessionManager::new();
    let connection_listener = session_manager.connection_listener();
    let mut poller = MarketPoller::new(
        client.clone(),
        request_tx,
        bookeeper.resub_notification(),
        bookeeper.corrupted(),
    );

    let consumer_task = tokio::spawn(async move {
        bookeeper.run(orderbook_rx, connection_listener).await;
    });

    let manager_task = tokio::spawn(async move {
        session_manager
            .run(
                "wss://ws.gemini.com?snapshot=-1",
                parser,
                router,
                request_rx,
            )
            .await;
    });

    let producer_task = tokio::spawn(async move {
        poller.run().await;
    });

    let handles = [consumer_task, manager_task, producer_task];

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("shutting down");
            shutdown.cancel()
        },
        _ = join_all(handles) => {},
    }

    Ok(())
}
