use crate::{
    gemini::{messages::Message, types::BinaryPredictionMarket},
    metadata::Category,
    traits::{Metadata, Parser},
};

use memchr::memmem::Finder;
use regex::Regex;

pub struct GeminiParser<'f> {
    depth_finder: Finder<'f>,
    balance_finder: Finder<'f>,
    error_finder: Finder<'f>,
    code_finder: Finder<'f>,
    contract_status_finder: Finder<'f>,
}

impl<'f> GeminiParser<'f> {
    pub fn new() -> Self {
        Self {
            depth_finder: Finder::new("\"depthUpdate\""),
            balance_finder: Finder::new("\"balanceUpdate\""),
            error_finder: Finder::new("\"error\""),
            code_finder: Finder::new("\"code\""),
            contract_status_finder: Finder::new("\"contractStatus\""),
        }
    }
}

impl<'f> Parser<Message> for GeminiParser<'f> {
    #[inline]
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Message> {
        if self.depth_finder.find(bytes).is_some() {
            return Ok(Message::L2DifferentialDepth(sonic_rs::from_slice(bytes)?));
        }
        if self.contract_status_finder.find(bytes).is_some() {
            return Ok(Message::ContractStatus(sonic_rs::from_slice(bytes)?));
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

pub struct TickerParser {
    crypto_regex: Regex,
    individual_sports_regex: Regex,
    team_sports_regex: Regex,
}

impl TickerParser {
    pub fn new() -> Self {
        Self {
            crypto_regex: Regex::new(
                r"^gemi-([a-z]+)(?:(05m|15m))?(\d{10})-(up|hi(\d+(?:d\d+)?))$",
            )
            .unwrap(),
            individual_sports_regex: Regex::new(r"^gemi-[a-z0-9]+-[a-z]{2,5}-[a-z]+-\d{8}-[a-z]+$")
                .unwrap(),
            team_sports_regex: Regex::new(
                r"^gemi-[a-z]{2,6}-\d{10}-[a-z]{2,4}-[a-z]{2,4}-[a-z]{1,7}-[a-z0-9]+$",
            )
            .unwrap(),
        }
    }

    pub fn parse(&self, market: &BinaryPredictionMarket) {
        match market.event.category {
            Category::Crypto => self.parse_crypto_ticker(&market.contract_id()),
            Category::Sports => self.parse_sports_ticker(&market.contract_id()),
            _ => {}
        }
    }

    fn parse_sports_ticker(&self, ticker: &str) {
        let team = self.team_sports_regex.captures(ticker);
        let individual = self.individual_sports_regex.captures(ticker);

        println!("team: {team:?}");
        println!("single: {individual:?}")
    }

    fn parse_crypto_ticker(&self, ticker: &str) {
        if let Some(caps) = self.crypto_regex.captures(ticker) {
            let symbol = caps.get(1).map(|m| m.as_str());
            let interval = caps.get(2).map(|m| m.as_str());
            let id = caps.get(3).map(|m| m.as_str());
            let mode = caps.get(4).map(|m| m.as_str());
            let hi_detail = caps.get(5).map(|m| m.as_str());
        }
    }
}
