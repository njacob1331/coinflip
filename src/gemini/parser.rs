use crate::{
    gemini::messages::{Message, OrderbookUpdate, SubscriptionStatus},
    traits::Parser,
};

pub struct GeminiParser;

impl GeminiParser {
    pub fn new() -> Self {
        Self
    }
}

impl Parser<Message> for GeminiParser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Message> {
        if memchr::memmem::find(bytes, b"\"depthUpdate\"").is_some() {
            let parsed: OrderbookUpdate = sonic_rs::from_slice(bytes)?;
            return Ok(Message::OrderbookUpdate(parsed));
        }

        // if first_update_id == last_update_id its a snapshot

        // if memchr::memmem::find(bytes, b"\"status\"").is_some() {
        //     let parsed: SubscriptionStatus = sonic_rs::from_slice(bytes)?;
        //     return Ok(Message::SubscriptionStatus(parsed));
        // }

        Ok(Message::Unknown)
    }
}
