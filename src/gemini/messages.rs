use serde::{Deserialize, Serialize, Serializer};

use crate::{
    common::SharedStr,
    session::{Payload, Priority, Request},
    traits::{OrderbookData, Prioritize},
};
use serde::Deserializer;
use std::sync::Arc;

pub fn deserialize_arc_str<'de, D>(deserializer: D) -> Result<SharedStr, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <&str>::deserialize(deserializer)?;
    Ok(Arc::from(s))
}

#[derive(Debug)]
pub enum Stream {
    BookTicker(SharedStr),
    PartialDepth(SharedStr),
    DifferentialDepth(SharedStr),
    Trade(SharedStr),
    Order,
    Balance,
    ContractStatus,
    ConnInfo,
}

#[derive(Debug)]
pub enum Subscriptions {
    Subscribe(Stream),
    Unsubscribe(Stream),
}

impl Prioritize for Subscriptions {
    fn priority(&self) -> Priority {
        Priority::Low
    }
}

impl Serialize for Subscriptions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct SubscriptionMessage<'a> {
            id: &'a str,
            method: &'a str,
            params: [String; 1],
        }

        fn build_message<'a>(stream: &'a Stream, method: &'a str) -> SubscriptionMessage<'a> {
            match stream {
                Stream::BookTicker(symbol) => SubscriptionMessage {
                    id: symbol,
                    method,
                    params: [format!("{}@bookTicker", symbol)],
                },
                Stream::PartialDepth(symbol) => SubscriptionMessage {
                    id: symbol,
                    method,
                    params: [format!("{}@depth@5@100ms", symbol)],
                },
                Stream::DifferentialDepth(symbol) => SubscriptionMessage {
                    id: symbol,
                    method,
                    params: [format!("{}@depth@100ms", symbol)],
                },
                Stream::Trade(symbol) => SubscriptionMessage {
                    id: symbol,
                    method,
                    params: ["trade".to_string()],
                },
                Stream::Order => SubscriptionMessage {
                    id: "order",
                    method,
                    params: ["orders@account".to_string()],
                },
                Stream::Balance => SubscriptionMessage {
                    id: "balance",
                    method,
                    params: ["balances@account".to_string()],
                },
                Stream::ContractStatus => SubscriptionMessage {
                    id: "contract-status",
                    method,
                    params: ["contractStatus".to_string()],
                },
                Stream::ConnInfo => SubscriptionMessage {
                    id: "conn-info",
                    method: "conninfo",
                    params: ["{}".to_string()],
                },
            }
        }

        match self {
            Self::Subscribe(stream) => build_message(stream, "subscribe").serialize(serializer),
            Self::Unsubscribe(stream) => build_message(stream, "unsubscribe").serialize(serializer),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeInForce {
    Gtc,
    Ioc,
    Fok,
    Moc,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PredictionMarketOutcome {
    Yes,
    No,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOrder {
    client_order_id: String,
    symbol: String,
    side: OrderSide,
    #[serde(rename = "type")]
    kind: OrderType,
    time_in_force: TimeInForce,
    price: String,
    quantity: String,
    event_outcome: PredictionMarketOutcome,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrder {
    order_id: String,
}

#[derive(Debug)]
pub enum Order {
    Place(PlaceOrder),
    Cancel(CancelOrder),
    // need to add the batch cancel variants
    // CancelAll,
    // CancelSession,
}

impl Serialize for Order {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct PlaceOrderMessage<'a> {
            id: &'a str,
            method: &'a str,
            params: &'a PlaceOrder,
        }

        #[derive(Serialize)]
        struct CancelOrderMessage<'a> {
            id: &'a str,
            method: &'a str,
            params: &'a CancelOrder,
        }

        match self {
            Order::Place(order) => PlaceOrderMessage {
                id: &order.client_order_id,
                method: "order.place",
                params: order,
            }
            .serialize(serializer),
            Order::Cancel(cancel) => CancelOrderMessage {
                id: &cancel.order_id,
                method: "order.cancel",
                params: cancel,
            }
            .serialize(serializer),
            // need to add the batch cancel variants
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ContractStatus(ContractStatus),
    L2DifferentialDepth(L2DifferentialDepth),
    SubscriptionError(SubscriptionError),
    BalanceUpdate(BalanceUpdate),
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionError {
    pub id: String,
    pub status: i16,
    pub error: SubscriptionErrorMsg,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionErrorMsg {
    code: i16,
    msg: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct L2DifferentialDepth {
    // #[serde(rename = "e")]
    // pub event_type: String,

    // #[serde(rename = "E")]
    // pub event_time_ns: u64,
    #[serde(rename = "s")]
    #[serde(deserialize_with = "deserialize_arc_str")]
    pub symbol: SharedStr,

    #[serde(rename = "U")]
    pub first_update_id: u64,

    #[serde(rename = "u")]
    pub last_update_id: u64,

    #[serde(rename = "b")]
    pub bids: Vec<PriceLevel>,

    #[serde(rename = "a")]
    pub asks: Vec<PriceLevel>,
}

impl OrderbookData for L2DifferentialDepth {
    fn key(&self) -> &str {
        &self.symbol
    }

    fn take_key(&mut self) -> SharedStr {
        self.symbol.clone()
    }

    fn is_snapshot(&self) -> bool {
        self.first_update_id == self.last_update_id
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceLevel {
    #[serde(deserialize_with = "deserialize_gemini_prediction_market_price")]
    pub price: u8,
    #[serde(deserialize_with = "deserialize_gemini_prediction_market_qty")]
    pub qty: i32,
}

#[inline]
fn deserialize_gemini_prediction_market_qty<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = <&str>::deserialize(deserializer)?;
    let s = &s[..s.len() - 3];
    let value = sonic_rs::from_str(s).map_err(serde::de::Error::custom)?;

    Ok(value)
}

#[inline]
fn deserialize_gemini_prediction_market_price<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = <&str>::deserialize(deserializer)?;
    let digits = &s.as_bytes()[2..]; // b"4100"
    let value = (digits[0] - b'0') * 10 + (digits[1] - b'0');

    Ok(value)
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceUpdate {
    // #[serde(rename = "e")]
    // pub event_type: String,

    // #[serde(rename = "E")]
    // pub event_time_ns: u64,
    #[serde(rename = "u")]
    pub update_ts: u64,

    #[serde(rename = "B")]
    pub balance_update: Vec<AssetBalance>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AssetBalance {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "f")]
    pub balance: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractStatus {
    #[serde(rename = "k")]
    pub event_ticker: String,
    #[serde(rename = "s")]
    #[serde(deserialize_with = "deserialize_arc_str")]
    pub symbol: SharedStr,
    #[serde(rename = "p")]
    pub strike: Option<String>,
    #[serde(rename = "o")]
    pub previous_status: String,
    #[serde(rename = "n")]
    pub new_status: String,
}

impl ContractStatus {
    pub fn is_strike_update(&self) -> bool {
        self.previous_status == self.new_status
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PositionUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_ts: u64,
    #[serde(rename = "u")]
    pub account_update_ts: u64,
    #[serde(rename = "A")]
    pub account_id: u64,
    #[serde(rename = "P")]
    pub positions: Vec<Position>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Position {
    #[serde(rename = "t")]
    pub product_type: String,
    #[serde(rename = "s")]
    pub instrument_symbol: String,
    #[serde(rename = "a")]
    pub amount: Vec<Amount>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Amount {
    #[serde(rename = "t")]
    pub label: String,
    #[serde(rename = "v")]
    pub size: String,
    #[serde(rename = "c")]
    pub asset_code: Option<String>,
}
