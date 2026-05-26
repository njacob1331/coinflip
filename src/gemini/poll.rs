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

// need to implement a seeding mechanism
// this serves both as the inital seed as well as
// a recovery mechanism when the websocket resets
// it should revalidate its current state cascade any changes to the
// appropriate consumers

use crate::{
    gemini::{
        client::GeminiClient,
        messages::{ContractStatus, Stream, SubscriptionError, Subscriptions},
        orderbook::GeminiOrderbook,
        types::BinaryPredictionMarket,
    },
    session::{Payload, Request},
    stats::matcher::MetadataTransportMsg,
    traits::Metadata,
};

pub struct MetaDataRepo {
    client: Arc<GeminiClient>,
    metadata: HashSet<Arc<str>>,
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
            let bp_market = BinaryPredictionMarket::new(event.clone(), contract);
            self.metadata.insert(bp_market.contract_id());
            self.metadata_tx.send(bp_market.into()).await?;
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
    resub_rx: Receiver<Arc<str>>,
    subscriptions: HashSet<Arc<str>>,
}

impl SubscriptionManager {
    pub fn new(request_tx: Sender<Payload<Subscriptions>>, resub_rx: Receiver<Arc<str>>) -> Self {
        Self {
            request_tx,
            resub_rx,
            subscriptions: HashSet::new(),
        }
    }

    async fn subscribe_static_streams(&self) -> Result<()> {
        let payload = Subscriptions::Subscribe(Stream::ContractStatus);
        self.request_tx.send(payload.into()).await?;

        Ok(())
    }

    async fn subscribe(&mut self, symbol: Arc<str>) -> Result<()> {
        let payload = Subscriptions::Subscribe(Stream::DifferentialDepth(symbol));
        self.request_tx.send(payload.into()).await?;

        Ok(())
    }

    async fn unsubscribe(&mut self, symbol: Arc<str>) -> Result<()> {
        let payload = Subscriptions::Unsubscribe(Stream::DifferentialDepth(symbol));
        self.request_tx.send(payload.into()).await?;

        Ok(())
    }

    async fn resubscribe(&mut self, symbol: Arc<str>) -> Result<()> {
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

        println!("subscription manager exiting")
    }
}
