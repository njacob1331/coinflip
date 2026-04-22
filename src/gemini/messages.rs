use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[derive(Debug, Serialize)]
enum Subscriptions {
    Subscribe,
    Unsubscribe,
    List,
}

#[derive(Debug, Clone)]
pub enum Message {
    OrderbookUpdate(OrderbookUpdate),
    SubscriptionStatus(SubscriptionStatus),
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionError {
    pub code: i16,
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionStatus {
    pub id: u32,
    pub status: i16,
    pub error: Option<SubscriptionError>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct OrderbookUpdate {
    // #[serde(rename = "e")]
    // pub event_type: String,

    // #[serde(rename = "E")]
    // pub event_time_ns: u64,
    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "U")]
    pub first_update_id: u64,

    #[serde(rename = "u")]
    pub last_update_id: u64,

    #[serde(rename = "b")]
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub bids: Vec<(Decimal, Decimal)>,

    #[serde(rename = "a")]
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub asks: Vec<(Decimal, Decimal)>,
}
