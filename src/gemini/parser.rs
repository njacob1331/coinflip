use crate::{gemini::messages::Message, traits::Parser};

use memchr::memmem::Finder;

pub struct GeminiParser<'f> {
    depth_finder: Finder<'f>,
    balance_finder: Finder<'f>,
    error_finder: Finder<'f>,
    code_finder: Finder<'f>,
}

impl<'f> GeminiParser<'f> {
    pub fn new() -> Self {
        Self {
            depth_finder: Finder::new("\"depthUpdate\""),
            balance_finder: Finder::new("\"balanceUpdate\""),
            error_finder: Finder::new("\"error\""),
            code_finder: Finder::new("\"code\""),
        }
    }
}

impl<'f> Parser<Message> for GeminiParser<'f> {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Message> {
        if self.depth_finder.find(bytes).is_some() {
            return Ok(Message::OrderbookUpdate(sonic_rs::from_slice(bytes)?));
        }
        if self.balance_finder.find(bytes).is_some() {
            return Ok(Message::BalanceUpdate(sonic_rs::from_slice(bytes)?));
        }
        if self.error_finder.find(bytes).is_some() && self.code_finder.find(bytes).is_some() {
            return Ok(Message::SubscriptionError(sonic_rs::from_slice(bytes)?));
        }

        Ok(Message::Unknown)
    }
}
