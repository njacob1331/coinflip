use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::{
    bookkeeper::BookKeeper,
    common::{OrderbookSnapshot, SharedStr},
    gemini::{
        client::GeminiClient,
        messages::{L2DifferentialDepth, Subscriptions},
        orderbook::GeminiOrderbook,
        parser::GeminiParser,
        poll::{MetaDataRepo, SubscriptionManager},
        router::GeminiRouter,
        types::BinaryPredictionMarket,
    },
    session::{Payload, Request, SessionManager},
    stats::{
        TransportMsg,
        engine::StatsEngine,
        matcher::{MetadataTransportMsg, StructuralCorrelationGraph},
    },
};

mod bookkeeper;
mod common;
mod data_structures;
mod gemini;
mod metadata;
mod ontology;
mod session;
mod stats;
mod traits;
mod utils;
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

    let (request_tx, request_rx) = tokio::sync::mpsc::channel::<Payload<Subscriptions>>(32);
    let (resub_tx, resub_rx) = tokio::sync::mpsc::channel::<SharedStr>(32);
    let (market_metadata_tx, mut market_metadata_rx) =
        tokio::sync::mpsc::channel::<MetadataTransportMsg<BinaryPredictionMarket>>(32);
    let (stats_tx, stats_rx) =
        kanal::bounded_async::<TransportMsg<SharedStr, OrderbookSnapshot>>(256);

    let parser = GeminiParser::new();
    let mut router = GeminiRouter::new();
    let mut orderbook_rx = router.orderbook_rx();
    let mut balance_rx = router.balance_rx();
    let mut subscription_err_rx = router.subscription_err_rx();
    let mut contract_status_rx = router.contract_status_rx();
    let contract_metadata_rx = router.contract_metadata_rx();

    let mut bookeeper =
        BookKeeper::<GeminiOrderbook, L2DifferentialDepth, L2DifferentialDepth>::new(
            resub_tx, stats_tx,
        );
    let client = Arc::new(GeminiClient::new());
    let mut session_manager = SessionManager::new();
    let connection_listener = session_manager.connection_listener();
    let mut sub_manager = SubscriptionManager::new(request_tx, resub_rx);

    let matcher_task = tokio::spawn(async move {
        let mut matcher = StructuralCorrelationGraph::new();
        matcher.run(&mut market_metadata_rx).await;
    });

    let price_corr_task = tokio::spawn(async move {
        let mut corr = StatsEngine::new();
        corr.run(stats_rx).await;
    });

    let metadata_task = tokio::spawn(async move {
        let mut metadata_repo = MetaDataRepo::new(client.clone(), market_metadata_tx);
        metadata_repo.run(contract_metadata_rx).await;
    });

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
    let sub_manager_task = tokio::spawn(async move {
        sub_manager
            .run(subscription_err_rx, contract_status_rx, cancel)
            .await;
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("shutting down");
            shutdown.cancel()
        }
    }

    let _ = tokio::join!(consumer_task, manager_task, sub_manager_task);

    Ok(())
}
