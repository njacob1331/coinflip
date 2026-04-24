use std::{sync::Arc, time::Duration};

use anyhow::Result;
use dashmap::{DashMap, DashSet};
use tokio::sync::{Notify, mpsc::Sender};

use crate::{
    gemini::{client::GeminiClient, messages::Subscriptions, responses::Contract},
    session::Request,
};

pub struct MarketPoller {
    api_client: Arc<GeminiClient>,
    request_tx: Sender<Request<Subscriptions>>,
    resub_notification: Arc<Notify>,
    resub_list: Arc<DashSet<String>>,
    subscriptions: DashMap<String, Contract>,
}

impl MarketPoller {
    pub fn new(
        api_client: Arc<GeminiClient>,
        request_tx: Sender<Request<Subscriptions>>,
        resub_notification: Arc<Notify>,
        resub_list: Arc<DashSet<String>>,
    ) -> Self {
        Self {
            api_client,
            request_tx,
            resub_notification,
            resub_list,
            subscriptions: DashMap::new(),
        }
    }

    pub async fn poll(&mut self) -> Result<()> {
        println!("Performing scheduled api poll for Gemini");
        let poll_markets: Vec<Contract> = self
            .api_client
            .list_prediction_market_events()
            .await?
            .into_iter()
            .flat_map(|e| e.contracts)
            .map(|mut c| {
                c.instrument_symbol = c.instrument_symbol.to_lowercase();
                c
            })
            .collect();

        for poll_market in poll_markets {
            // check if we have this market in our sub map
            if self
                .subscriptions
                .contains_key(&poll_market.instrument_symbol)
            {
                // if we have it and its active, no action
                if poll_market.status == "active" {
                    continue;
                } else {
                    // if we have it and its not active, unsub
                    self.subscriptions.remove(&poll_market.instrument_symbol);
                    let _ = self
                        .request_tx
                        .send(Subscriptions::Unsubscribe(poll_market.instrument_symbol).into())
                        .await;
                }
            } else if poll_market.status == "active" {
                // subscribe
                let _ = self
                    .request_tx
                    .send(Subscriptions::Resubscribe(poll_market.instrument_symbol.clone()).into())
                    .await;
                self.subscriptions
                    .insert(poll_market.instrument_symbol.clone(), poll_market);
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                biased;

                _ = self.resub_notification.notified() => {
                    let keys: Vec<_> = self.resub_list.iter().map(|e| e.key().clone()).collect();
                    for key in keys {
                        let _ = self.request_tx.send(Subscriptions::Resubscribe(key).into()).await;
                    }
                },
                _ = interval.tick() => {
                    if let Err(e) = self.poll().await {
                        eprintln!("api poll error: {e}")
                    }
                }
            }
        }
    }
}
