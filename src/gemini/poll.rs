use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use dashmap::{DashMap, DashSet};
use tokio::{
    sync::{
        Notify,
        mpsc::{Receiver, Sender},
    },
    time::sleep,
};
use tokio_util::sync::CancellationToken;

// this is priority one
// need to solve for the following:
// 1. maintain an accurate list of subscriptions
// 2. ensure event metadata is up to date
// 3. be able to map contracts to events
//
// the parent event id is embedded in the contract id
// the contractStatus stream will tell us when new events/contracts are created and when the strike changes

use crate::{
    gemini::{
        client::GeminiClient,
        messages::{ContractStatus, Stream, SubscriptionError, Subscriptions},
        orderbook::GeminiOrderbook,
        responses::{BinaryPredictionMarket, Contract, Event, Strike},
    },
    session::{Payload, Request},
    stats::matcher::MetadataTransportMsg,
    traits::Metadata,
};

pub struct MetaDataRepo {
    client: Arc<GeminiClient>,
    metadata: HashSet<String>,
    metadata_tx: Sender<MetadataTransportMsg<BinaryPredictionMarket>>,
}

impl MetaDataRepo {
    pub fn new(
        client: Arc<GeminiClient>,
        metadata_tx: Sender<MetadataTransportMsg<BinaryPredictionMarket>>,
    ) -> Self {
        Self {
            client,
            metadata: HashSet::new(),
            metadata_tx,
        }
    }

    async fn fetch_and_forward(&mut self, event_ticker: &str, contract_ticker: &str) -> Result<()> {
        if self.metadata.contains(contract_ticker) {
            return Ok(());
        }

        let mut event = self.client.get_event_by_ticker(event_ticker).await?;
        let contracts = event.take_contracts();
        let event = Arc::new(event);

        for contract in contracts {
            let metadata = BinaryPredictionMarket::new(event.clone(), contract);
            self.metadata.insert(metadata.ticker().to_string());
            self.metadata_tx.send(metadata.into()).await?;
        }

        Ok(())
    }

    async fn remove_and_forward(&mut self, contract_ticker: &str) -> Result<()> {
        self.metadata.remove(contract_ticker);
        self.metadata_tx
            .send(MetadataTransportMsg::Remove(contract_ticker.to_string()))
            .await?;

        Ok(())
    }

    pub async fn run(&mut self, mut contract_status_rx: Receiver<Arc<ContractStatus>>) {
        while let Some(msg) = contract_status_rx.recv().await {
            match msg.new_status.as_str() {
                "Settled" => {
                    if let Err(e) = self.remove_and_forward(&msg.symbol).await {
                        tracing::error!("GEMINI: metadata error {e}")
                    }
                }

                _ => {
                    if let Err(e) = self.fetch_and_forward(&msg.event_ticker, &msg.symbol).await {
                        tracing::error!("GEMINI: metadata error {e}")
                    }
                }
            }
        }
    }
}

pub struct SubscriptionManager {
    request_tx: Sender<Payload<Subscriptions>>,
    resub_rx: Receiver<String>,
    subscriptions: HashSet<String>,
    orderbooks: Arc<DashMap<String, GeminiOrderbook>>,
}

impl SubscriptionManager {
    pub fn new(
        request_tx: Sender<Payload<Subscriptions>>,
        resub_rx: Receiver<String>,
        orderbooks: Arc<DashMap<String, GeminiOrderbook>>,
    ) -> Self {
        Self {
            request_tx,
            resub_rx,
            orderbooks,
            subscriptions: HashSet::new(),
        }
    }

    async fn subscribe_static_streams(&self) -> Result<()> {
        let payload = Subscriptions::Subscribe(Stream::ContractStatus);
        self.request_tx.send(payload.into()).await?;

        Ok(())
    }

    async fn subscribe(&mut self, symbol: String) -> Result<()> {
        let payload = Subscriptions::Subscribe(Stream::DifferentialDepth(symbol));
        self.request_tx.send(payload.into()).await?;

        Ok(())
    }

    async fn unsubscribe(&mut self, symbol: String) -> Result<()> {
        let payload = Subscriptions::Unsubscribe(Stream::DifferentialDepth(symbol));
        self.request_tx.send(payload.into()).await?;

        Ok(())
    }

    async fn resubscribe(&mut self, symbol: String) -> Result<()> {
        let payload = vec![
            Subscriptions::Unsubscribe(Stream::DifferentialDepth(symbol.clone())),
            Subscriptions::Subscribe(Stream::DifferentialDepth(symbol)),
        ];

        self.request_tx.send(payload.into()).await?;

        Ok(())
    }

    async fn handle_status_msg(&mut self, msg: Arc<ContractStatus>) -> Result<()> {
        match msg.new_status.as_str() {
            "Settled" => {
                tracing::info!(
                    "GEMINI: {} settled, unsubscribing and removing orderbook",
                    &msg.symbol
                );
                self.subscriptions.remove(&msg.symbol);
                self.orderbooks.remove(&msg.symbol);
                self.unsubscribe(msg.symbol.clone()).await?
            }

            _ => {
                let subscribed = self.subscriptions.contains(&msg.symbol);
                if !subscribed {
                    tracing::info!(
                        "GEMINI: subscribing to previously untracked market {}",
                        &msg.symbol
                    );
                    self.subscribe(msg.symbol.clone()).await?;
                    self.subscriptions.insert(msg.symbol.clone());
                }
            }
        }

        Ok(())
    }

    pub async fn run(
        &mut self,
        mut subscription_err_rx: Receiver<SubscriptionError>,
        mut contract_status_rx: Receiver<Arc<ContractStatus>>,
        cancel: CancellationToken,
    ) {
        // let mut interval = tokio::time::interval(Duration::from_secs(60));
        let _ = self.subscribe_static_streams().await;
        // let _ = self.seed().await;

        loop {
            tokio::select! {
                biased;

                _ = cancel.cancelled() => break,

                resub = self.resub_rx.recv() => {
                    match resub {
                        Some(symbol) => {
                            let _ = self.resubscribe(symbol).await;
                        }

                        None => break
                    }
                },

                sub_err = subscription_err_rx.recv() => {
                    match sub_err {
                        Some(error) => {
                            tracing::error!("subscription error: {error:#?}")
                        }

                        None => break
                    }
                },

                contract_status = contract_status_rx.recv() => {
                    match contract_status {
                        Some(status) => {
                            let _ = self.handle_status_msg(status).await;

                        }

                        None => break
                    }
                }
            }
        }

        println!("market poller exiting")
    }
}
