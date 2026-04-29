use crate::{
    gemini::messages::{BalanceUpdate, Message, OrderbookUpdate, SubscriptionError},
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

        if memchr::memmem::find(bytes, b"\"balanceUpdate\"").is_some() {
            let parsed: BalanceUpdate = sonic_rs::from_slice(bytes)?;
            return Ok(Message::BalanceUpdate(parsed));
        }

        if memchr::memmem::find(bytes, b"\"error\"").is_some()
            && memchr::memmem::find(bytes, b"\"code\"").is_some()
        {
            let parsed: SubscriptionError = sonic_rs::from_slice(bytes)?;
            return Ok(Message::SubscriptionError(parsed));
        }

        Ok(Message::Unknown)
    }
}
