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

// #[serde_as]
// #[derive(Debug, Clone, Deserialize)]
// pub struct OrderbookUpdate {
//     // #[serde(rename = "e")]
//     // pub event_type: String,

//     // #[serde(rename = "E")]
//     // pub event_time_ns: u64,
//     #[serde(rename = "s")]
//     pub symbol: String,

//     #[serde(rename = "U")]
//     pub first_update_id: u64,

//     #[serde(rename = "u")]
//     pub last_update_id: u64,

//     #[serde(rename = "b")]
//     #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
//     pub bids: Vec<(Decimal, Decimal)>,

//     #[serde(rename = "a")]
//     #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
//     pub asks: Vec<(Decimal, Decimal)>,
// }

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
