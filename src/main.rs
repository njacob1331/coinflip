use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::{
    bookkeeper::BookKeeper,
    gemini::{
        client::GeminiClient, messages::Subscriptions, parser::GeminiParser, poll::MarketPoller,
        router::GeminiRouter,
    },
    session::{Request, SessionManager},
};

mod bookkeeper;
mod gemini;
mod session;
mod stats;
mod traits;
mod ws;

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
    let (request_tx, request_rx) = tokio::sync::mpsc::channel::<Request<Subscriptions>>(32);
    let order_tx = request_tx.clone();
    let (resub_tx, resub_rx) = tokio::sync::mpsc::channel::<String>(32);

    let parser = GeminiParser::new();
    let mut router = GeminiRouter::new();
    let orderbook_rx = router.connect("orderbook");
    let mut balance_rx = router.balance_rx();

    let mut bookeeper = BookKeeper::new(resub_tx);
    let client = Arc::new(GeminiClient::new());
    let mut session_manager = SessionManager::new();
    let connection_listener = session_manager.connection_listener();
    let mut poller = MarketPoller::new(client.clone(), request_tx, resub_rx);

    let balance_task = tokio::spawn(async move {
        while let Some(balance) = balance_rx.recv().await {
            tracing::info!("{balance:#?}")
        }
    });

    let cancel = shutdown.clone();
    let consumer_task = tokio::spawn(async move {
        bookeeper
            .run(orderbook_rx, connection_listener, cancel)
            .await;
    });

    let cancel = shutdown.clone();
    let manager_task = tokio::spawn(async move {
        session_manager
            .run(
                "wss://ws.gemini.com?snapshot=-1",
                parser,
                router,
                request_rx,
                cancel.clone(),
            )
            .await;
    });

    let cancel = shutdown.clone();
    let producer_task = tokio::spawn(async move {
        poller.run(cancel).await;
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("shutting down");
            shutdown.cancel()
        }
    }

    let _ = tokio::join!(consumer_task, manager_task, producer_task);

    Ok(())
}
