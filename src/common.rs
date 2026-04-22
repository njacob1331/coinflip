use anyhow::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::gemini::messages::DepthUpdate;

pub trait Parser<T>: Send + Sync
where
    T: Send + 'static,
{
    fn parse(&self, bytes: &[u8]) -> Result<T>;
}

#[async_trait]
pub trait Router<T>: Send + Sync
where
    T: Send + 'static,
{
    async fn route(&self, msg: T) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum Message {
    OrderbookUpdate(OrderbookUpdate),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct OrderbookUpdate {
    pub exchange_id: String,
    pub symbol: String,
    pub first_update_id: u64,
    pub last_update_id: u64,
    pub bids: Vec<(Decimal, Decimal)>,
    pub asks: Vec<(Decimal, Decimal)>,
}

impl From<DepthUpdate> for OrderbookUpdate {
    fn from(value: DepthUpdate) -> Self {
        Self {
            exchange_id: String::from("gemini"),
            symbol: value.symbol,
            first_update_id: value.first_update_id,
            last_update_id: value.last_update_id,
            bids: value.bids,
            asks: value.asks,
        }
    }
}
