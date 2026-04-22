use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::common::{Message, OrderbookUpdate, Router};

pub struct GeminiRouter {
    orderbook_tx: Sender<OrderbookUpdate>,
    orderbook_rx: Option<Receiver<OrderbookUpdate>>,
}

impl GeminiRouter {
    pub fn new() -> Self {
        let (orderbook_tx, orderbook_rx) = channel::<OrderbookUpdate>(256);

        Self {
            orderbook_tx,
            orderbook_rx: Some(orderbook_rx),
        }
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
            _ => {}
        }

        Ok(())
    }
}
