use serde::{Deserialize, Serialize, Serializer};

use crate::session::Request;

// ====
// Send
// ====

#[derive(Debug)]
pub enum Stream {
    BookTicker(String),
    PartialDepth(String),
    DifferentialDepth(String),
    Trade(String),
    Order,
    Balance,
}

#[derive(Debug)]
pub enum Subscriptions {
    Subscribe(Stream),
    Unsubscribe(Stream),
    Resubscribe(Stream),
}

impl From<Subscriptions> for Request<Subscriptions> {
    fn from(value: Subscriptions) -> Self {
        match value {
            Subscriptions::Resubscribe(_) => Request::LowPriority {
                batch: true,
                inner: value,
            },

            _ => Request::LowPriority {
                batch: false,
                inner: value,
            },
        }
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
            }
        }

        match self {
            Self::Subscribe(stream) => build_message(stream, "subscribe").serialize(serializer),
            Self::Unsubscribe(stream) => build_message(stream, "unsubscribe").serialize(serializer),
            Self::Resubscribe(stream) => {
                let messages = [
                    build_message(stream, "unsubscribe"),
                    build_message(stream, "subscribe"),
                ];
                messages.serialize(serializer)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    OrderbookUpdate(OrderbookUpdate),
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
    pub bids: Vec<PriceLevel>,

    #[serde(rename = "a")]
    pub asks: Vec<PriceLevel>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceLevel {
    #[serde(deserialize_with = "deserialize_gemini_prediction_market_price")]
    pub price: u8,
    #[serde(deserialize_with = "deserialize_gemini_prediction_market_qty")]
    pub qty: i32,
}

#[inline(always)]
fn deserialize_gemini_prediction_market_qty<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = <&str>::deserialize(deserializer)?;
    let s = &s[..s.len() - 3];
    let value = sonic_rs::from_str(s).map_err(serde::de::Error::custom)?;

    Ok(value)
}

#[inline(always)]
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
