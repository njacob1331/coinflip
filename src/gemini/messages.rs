use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    common::{Message, Parser},
    session::Request,
};

// #[derive(Debug, Serialize)]
// struct SubscriptionActionInfo<'a> {
//     id: &'a str,
//     method: &'a str,
//     params: [String; 1],
// }

// #[derive(Debug, Serialize)]
// pub enum SubscriptionAction {
//     Subscribe(String),
//     Unsubscribe(String),
//     List,
// }

// impl From<SubscriptionAction> for Request {
//     fn from(value: SubscriptionAction) -> Self {
//         match value {
//             SubscriptionAction::Subscribe(inner) => {
//                 let action = SubscriptionActionInfo {
//                     id: &inner,
//                     method: "subscribe",
//                     params: [format!("{}@depth@100ms", inner)],
//                 };

//                 let str = serde_json::to_string(&action).unwrap();
//                 Request::LowPriority(str)
//             }
//             SubscriptionAction::Unsubscribe(inner) => {
//                 let action = SubscriptionActionInfo {
//                     id: &inner,
//                     method: "unsubscribe",
//                     params: [format!("{}@depth@100ms", inner)],
//                 };

//                 let str = serde_json::to_string(&action).unwrap();
//                 Request::LowPriority(str)
//             }
//             SubscriptionAction::List => {
//                 let action = SubscriptionActionInfo {
//                     id: "list",
//                     method: "list_subscriptions",
//                     params: [],
//                 };

//                 let str = serde_json::to_string(&action).unwrap();
//                 Request::LowPriority(str)
//             }
//         }
//     }
// }

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
pub struct DepthUpdate {
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

pub struct GeminiParser;

impl GeminiParser {
    pub fn new() -> Self {
        Self
    }
}

impl Parser<Message> for GeminiParser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Message> {
        if memchr::memmem::find(bytes, b"\"depthUpdate\"").is_some() {
            let parsed: DepthUpdate = sonic_rs::from_slice(bytes)?;
            return Ok(Message::OrderbookUpdate(parsed.into()));
        }

        Ok(Message::Unknown)
    }
}
