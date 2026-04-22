use std::{sync::Arc, time::Duration};

use anyhow::Result;
use dashmap::{DashMap, DashSet};
use serde_json::json;
use tokio::sync::{Notify, mpsc::Sender};

use crate::{
    gemini::{client::GeminiClient, responses::Contract},
    session::Request,
};

pub struct MarketPoller {
    api_client: Arc<GeminiClient>,
    request_tx: Sender<Request>,
    resub_notification: Arc<Notify>,
    resub_list: Arc<DashSet<String>>,
    subscriptions: DashMap<String, Contract>,
}

impl MarketPoller {
    pub fn new(
        api_client: Arc<GeminiClient>,
        request_tx: Sender<Request>,
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

    pub fn build_request(&self, method: &str, contract: &Contract) -> Request {
        let body = json!(
            {
              "id": format!("{}", contract.instrument_symbol),
              "method": method,
              "params": [
                format!("{}@depth@100ms", contract.instrument_symbol)
              ]
            }
        )
        .to_string();

        Request::LowPriority(body)
    }

    pub fn build_request_raw(&self, method: &str, stream: &str) -> Request {
        let body = json!(
            {
              "id": stream,
              "method": method,
              "params": [
                format!("{}@depth@100ms", stream)
              ]
            }
        )
        .to_string();

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

                    for resub in self.resub_list.iter() {
                        let request = self.build_request_raw("unsubscribe", resub.key());
                        let _ = self.request_tx.send(request).await;

                        let request = self.build_request_raw("subscribe", resub.key());
                        let _ = self.request_tx.send(request).await;

                        self.resub_list.remove(resub.key());
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
