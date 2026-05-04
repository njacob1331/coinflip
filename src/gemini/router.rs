use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::{
    gemini::messages::{BalanceUpdate, Message, OrderbookUpdate, SubscriptionError},
    traits::Router,
};

pub struct GeminiRouter {
    orderbook_tx: Sender<OrderbookUpdate>,
    orderbook_rx: Option<Receiver<OrderbookUpdate>>,
    balance_tx: Sender<BalanceUpdate>,
    balance_rx: Option<Receiver<BalanceUpdate>>,
    subscription_err_tx: Sender<SubscriptionError>,
    subscription_err_rx: Option<Receiver<SubscriptionError>>,
}

impl GeminiRouter {
    pub fn new() -> Self {
        let (orderbook_tx, orderbook_rx) = channel(256);
        let (balance_tx, balance_rx) = channel(32);
        let (subscription_err_tx, subscription_err_rx) = channel(32);

        Self {
            orderbook_tx,
            orderbook_rx: Some(orderbook_rx),
            balance_tx,
            balance_rx: Some(balance_rx),
            subscription_err_tx,
            subscription_err_rx: Some(subscription_err_rx)
        }
    }

    pub fn balance_rx(&mut self) -> Receiver<BalanceUpdate> {
        self.balance_rx.take().unwrap()
    }

    pub fn subscription_err_rx(&mut self) -> Receiver<SubscriptionError> {
        self.subscription_err_rx.take().unwrap()
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
            Message::SubscriptionError(error) => self.subscription_err_tx.send(error).await?,
            _ => {}
        }

        Ok(())
    }
}
