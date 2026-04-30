use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::{
    gemini::messages::{BalanceUpdate, Message, OrderbookUpdate},
    traits::Router,
};

pub struct GeminiRouter {
    orderbook_tx: Sender<OrderbookUpdate>,
    orderbook_rx: Option<Receiver<OrderbookUpdate>>,
    balance_tx: Sender<BalanceUpdate>,
    balance_rx: Option<Receiver<BalanceUpdate>>,
}

impl GeminiRouter {
    pub fn new() -> Self {
        let (orderbook_tx, orderbook_rx) = channel(256);
        let (balance_tx, balance_rx) = channel(32);

        Self {
            orderbook_tx,
            orderbook_rx: Some(orderbook_rx),
            balance_tx,
            balance_rx: Some(balance_rx),
        }
    }

    pub fn balance_rx(&mut self) -> Receiver<BalanceUpdate> {
        self.balance_rx.take().unwrap()
    }

    pub fn connect(&mut self, channel: &str) -> Receiver<OrderbookUpdate> {
        match channel {
            "orderbook" => self.orderbook_rx.take().unwrap(),
            _ => panic!("unknown channel: {channel}"),
        }
    }
}

#[async_trait]
impl Router<Message> for GeminiRouter {
    async fn route(&self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::OrderbookUpdate(update) => self.orderbook_tx.send(update).await?,
            Message::BalanceUpdate(update) => self.balance_tx.send(update).await?,
            Message::SubscriptionError(error) => tracing::info!("{error:#?}"),
            _ => {}
        }

        Ok(())
    }
}
