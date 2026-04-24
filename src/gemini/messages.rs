use serde::{
    Deserialize, Serialize, Serializer,
    ser::{SerializeSeq, SerializeStruct},
};

use crate::session::Request;

#[derive(Debug)]
pub enum Subscriptions {
    Subscribe(String),
    Unsubscribe(String),
    Resubscribe(String),
}

impl Into<Request<Subscriptions>> for Subscriptions {
    fn into(self) -> Request<Subscriptions> {
        match self {
            Self::Resubscribe(_) => Request::LowPriority {
                batch: true,
                inner: self,
            },

            _ => Request::LowPriority {
                batch: false,
                inner: self,
            },
        }
    }
}

use serde_json::json;

impl Serialize for Subscriptions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        fn channel_for(stream: &str) -> String {
            let mut s = String::with_capacity(stream.len() + 12);
            s.push_str(stream);
            s.push_str("@depth@100ms");
            s
        }

        fn serialize_single<S: Serializer>(
            serializer: S,
            stream: &str,
            method: &str,
        ) -> Result<S::Ok, S::Error> {
            let mut state = serializer.serialize_struct("Subscriptions", 3)?;
            let channel = channel_for(stream);

            state.serialize_field("id", stream)?;
            state.serialize_field("method", method)?;
            state.serialize_field("params", &[channel])?;
            state.end()
        }

        match self {
            Self::Subscribe(stream) => serialize_single(serializer, stream, "subscribe"),
            Self::Unsubscribe(stream) => serialize_single(serializer, stream, "unsubscribe"),

            Self::Resubscribe(stream) => {
                let channel = channel_for(stream);

                let mut seq = serializer.serialize_seq(Some(2))?;

                seq.serialize_element(&json!({
                    "id": stream,
                    "method": "unsubscribe",
                    "params": [channel.clone()]
                }))?;

                seq.serialize_element(&json!({
                    "id": stream,
                    "method": "subscribe",
                    "params": [channel]
                }))?;

                seq.end()
            }
        }
    }
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
