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

    pub fn build_request(&self, method: &str, contract: &Contract) -> Request<Subscriptions> {
        let body = Subscriptions::Subscribe {
            id: contract.instrument_symbol.clone(),
            method: method.to_string(),
            params: [format!("{}@depth@100ms", contract.instrument_symbol)],
        };

        Request::LowPriority(body)
    }

    pub fn build_request_raw(&self, method: &str, stream: &str) -> Request<Subscriptions> {
        let method = match method {
            "subscribe" => "subscribe".to_string(),
            "unsubscribe" => "unsubscribe".to_string(),
            _ => "unknown".to_string(),
        };

        let stream = stream.to_string();

        let body = Subscriptions::Subscribe {
            id: stream.clone(),
            method,
            params: [format!("{}@depth@100ms", stream)],
        };

        Request::LowPriority(body)
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
                    let request = self.build_request("unsubscribe", &poll_market);
                    let _ = self.request_tx.send(request).await;
                    self.subscriptions.remove(&poll_market.instrument_symbol);
                }
            } else if poll_market.status == "active" {
                // subscribe
                let request = self.build_request("subscribe", &poll_market);
                let _ = self.request_tx.send(request).await;
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
                        // need to refactor to package the unsubscribe and subscribe requests into
                        // a single json array
                        // this ensures the ordering of the resub requests
                        // ws will handle detecting array

                        let request = self.build_request_raw("unsubscribe", &key);
                        let _ = self.request_tx.send(request).await;

                        let request = self.build_request_raw("subscribe", &key);
                        let _ = self.request_tx.send(request).await;
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
