use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use dashmap::{DashMap, DashSet};
use tokio::sync::{
    Notify,
    mpsc::{Receiver, Sender},
};
use tokio_util::sync::CancellationToken;

use crate::{
    gemini::{
        client::GeminiClient,
        messages::{Stream, Subscriptions},
        orderbook::Orderbook,
        responses::{Contract, Event, Market},
    },
    session::{Payload, Request},
};

pub struct MarketPoller {
    api_client: Arc<GeminiClient>,
    request_tx: Sender<Payload<Subscriptions>>,
    resub_rx: Receiver<String>,
    subscriptions: DashMap<String, Event>,
    contract_to_event_index: HashMap<String, String>,
    orderbooks: Arc<DashMap<String, Orderbook>>,
}

impl MarketPoller {
    pub fn new(
        api_client: Arc<GeminiClient>,
        request_tx: Sender<Payload<Subscriptions>>,
        resub_rx: Receiver<String>,
        orderbooks: Arc<DashMap<String, Orderbook>>,
    ) -> Self {
        Self {
            api_client,
            request_tx,
            resub_rx,
            subscriptions: DashMap::new(),
            contract_to_event_index: HashMap::new(),
            orderbooks,
        }
    }

    async fn subscribe(&mut self, mut event: Event) -> Result<()> {
        for contract in &event.contracts {
            if contract
                .market_state
                .as_ref()
                .is_some_and(|state| state == "open")
            {
                let payload = Subscriptions::Subscribe(Stream::DifferentialDepth(
                    contract.instrument_symbol.clone(),
                ));
                self.request_tx.send(payload.into()).await?;

                // as a first pass we'll just clone, this should be optimized later
                self.contract_to_event_index
                    .insert(contract.instrument_symbol.clone(), event.ticker.clone());
            }
        }

        self.subscriptions
            .insert(std::mem::take(&mut event.ticker), event);

        Ok(())
    }

    async fn unsubscribe(&mut self, event: Event) -> Result<()> {
        self.subscriptions.remove(&event.ticker);

        for contract in event.contracts {
            self.contract_to_event_index
                .remove(&contract.instrument_symbol);

            // drop dead book
            tracing::info!("removing dead book: {}", &contract.instrument_symbol);
            self.orderbooks.remove(&contract.instrument_symbol);

            let payload =
                Subscriptions::Unsubscribe(Stream::DifferentialDepth(contract.instrument_symbol));
            self.request_tx.send(payload.into()).await?;
        }

        Ok(())
    }

    async fn resubscribe(&mut self, symbol: String) -> Result<()> {
        tracing::info!("resubbbing to {symbol}");
        let payload = vec![
            Subscriptions::Unsubscribe(Stream::DifferentialDepth(symbol.clone())),
            Subscriptions::Subscribe(Stream::DifferentialDepth(symbol)),
        ];

        self.request_tx.send(payload.into()).await?;

        Ok(())
    }

    pub async fn poll(&mut self) -> Result<()> {
        println!("Performing scheduled api poll for Gemini");
        let events = self.api_client.list_prediction_market_events().await?;

        for event in events {
            // check if we have this market in our sub map
            if self.subscriptions.contains_key(&event.ticker) {
                // if we have it and its active, no action
                if event.status == "active" {
                    continue;
                } else {
                    // if we have it and its not active, unsub
                    self.unsubscribe(event).await?
                }
            } else if event.status == "active" {
                self.subscribe(event).await?
            }
        }

        tracing::info!("subscribed to {} markets", self.subscriptions.len());

        Ok(())
    }

    pub async fn run(&mut self, subscription_err_rx: Receiver<SubscriptionError>, cancel: CancellationToken) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));

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
                            self.subscriptions.remove(&error.id);
                        }

                        None => break
                    }
                },
                
                _ = interval.tick() => {
                    if let Err(e) = self.poll().await {
                        eprintln!("api poll error: {e}")
                    }
                }
            }
        }

        println!("market poller exiting")
    }
}
