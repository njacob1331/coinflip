use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{common::MarketCategory, traits::Metadata};

#[derive(Debug, Deserialize)]
pub struct ListMarketEvents {
    #[serde(default)]
    pub data: Vec<Event>,
    pub pagination: Pagination,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit: usize,
    pub offset: u64,
    pub total: u64,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(deserialize_with = "string_to_u32")]
    pub id: u32,
    // pub title: String,
    // pub slug: String,
    // pub description: Option<String>,
    // #[serde(rename = "imageUrl")]
    // pub image_url: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub category: MarketCategory,
    // pub series: Option<String>,
    #[serde(deserialize_with = "lowercase")]
    pub ticker: String,
    pub status: String,
    // #[serde(rename = "resolvedAt")]
    // pub resolved_at: Option<DateTime<Utc>>,
    // #[serde(rename = "createdAt")]
    // pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub contracts: Vec<Contract>,
    // #[serde(rename = "contractOrderbooks")]
    // pub contract_orderbooks: Option<Value>,
    // pub volume: Option<String>,
    // #[serde(rename = "volume24h")]
    // pub volume_24h: Option<String>,
    // pub liquidity: Option<String>,
    // pub tags: Option<Value>,
    // #[serde(rename = "effectiveDate")]
    // pub effective_date: Option<DateTime<Utc>>,
    #[serde(rename = "expiryDate")]
    pub expiry_date: DateTime<Utc>,
    // #[serde(rename = "startTime")]
    // pub start_time: Option<Value>,
    // #[serde(rename = "termsLink")]
    // pub terms_link: Option<String>,
    // pub subcategory: Option<Value>,
    // #[serde(rename = "socialImageUrl")]
    // pub social_image_url: Option<String>,
    // #[serde(rename = "featuredImageUrl")]
    // pub featured_image_url: Option<Value>,
    // #[serde(rename = "isLive")]
    // pub is_live: Option<bool>,
    // pub events: Option<Value>,
    // pub template: Option<String>,
    // #[serde(rename = "gameId")]
    // pub game_id: Option<Value>,
    // #[serde(rename = "eventTags")]
    // pub event_tags: Option<Value>,
    // pub source: Option<String>,
    // pub settlement: Option<Value>,
    // pub participants: Option<Value>,
}

impl Event {
    pub fn take_contracts(&mut self) -> Vec<Contract> {
        std::mem::take(&mut self.contracts)
    }
}

#[derive(Debug, Deserialize)]
pub struct Contract {
    #[serde(deserialize_with = "gemini_contract_id")]
    pub id: ContractId,
    // pub label: String,
    // #[serde(rename = "abbreviatedName")]
    // pub abbreviated_name: Option<String>,
    // pub description: Value,
    // pub prices: Option<Value>,
    // pub color: Option<String>,
    pub status: String,
    // #[serde(rename = "imageUrl")]
    // pub image_url: Option<String>,
    // #[serde(rename = "priceHistory")]
    // pub price_history: Option<Value>,
    // #[serde(rename = "createdAt")]
    // pub created_at: DateTime<Utc>,
    #[serde(rename = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,
    #[serde(rename = "resolutionSide")]
    pub resolution_side: Option<String>,
    // #[serde(rename = "resolvedAt")]
    // pub resolved_at: Option<DateTime<Utc>>,
    // #[serde(rename = "termsAndConditionsUrl")]
    // pub terms_url: String,
    // pub ticker: String,
    #[serde(rename = "instrumentSymbol")]
    #[serde(deserialize_with = "lowercase")]
    pub instrument_symbol: String,
    // #[serde(rename = "effectiveDate")]
    // pub effective_date: Option<DateTime<Utc>>,
    #[serde(rename = "marketState")]
    pub market_state: Option<String>,
    // #[serde(rename = "sortOrder")]
    // pub sort_order: Option<u64>,
    // #[serde(rename = "sportsSide")]
    // pub sports_side: Option<String>,
    #[serde(rename = "teamId")]
    pub team_id: Option<String>,
    pub strike: Option<Strike>,
    pub source: Option<String>,
    // #[serde(rename = "settlementValue")]
    // pub settlement_value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContractId {
    pub parent: u32,
    pub child: u32,
}

impl PartialEq for Contract {
    fn eq(&self, other: &Self) -> bool {
        self.instrument_symbol == other.instrument_symbol
    }
}

impl Contract {
    pub fn update(&mut self, updated: Self) {
        if self.expiry_date != updated.expiry_date {
            self.expiry_date = updated.expiry_date
        }

        if self.market_state != updated.market_state {
            self.market_state = updated.market_state
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Strike {
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub strike_type: String,
    #[serde(rename = "availableAt")]
    pub available_at: Option<DateTime<Utc>>,
}

impl Strike {
    fn update(&mut self, updated: Self) {
        if self.value != updated.value {
            self.value = updated.value
        }

        if self.available_at != updated.available_at {
            self.available_at = updated.available_at
        }
    }
}

fn lowercase<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    // we need to optimize this
    let s = String::deserialize(deserializer)?;
    Ok(s.to_lowercase())
}

fn string_to_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = <&str>::deserialize(deserializer)?;
    s.parse().map_err(|e| {
        tracing::error!("failed to parse u32 from {:?}: {}", s, e);
        serde::de::Error::custom(e)
    })
}

fn gemini_contract_id<'de, D>(deserializer: D) -> Result<ContractId, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = <&str>::deserialize(deserializer)?;
    let split_idx = s.find("-").unwrap();

    Ok(ContractId {
        parent: s[..split_idx - 1].parse().unwrap(),
        child: s[split_idx + 1..].parse().unwrap(),
    })
}

pub struct BinaryPredictionMarket {
    event: Arc<Event>,
    contract: Contract,
}

impl BinaryPredictionMarket {
    pub fn new(event: Arc<Event>, contract: Contract) -> Self {
        Self { event, contract }
    }
}

impl Metadata for BinaryPredictionMarket {
    fn ticker(&self) -> &str {
        &self.contract.instrument_symbol
    }

    fn category(&self) -> &MarketCategory {
        &self.event.category
    }
}

// polymarket
//

fn deserialize_stringified_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    serde_json::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PolyMarketMarket {
    #[serde(rename = "clobTokenIds")]
    #[serde(deserialize_with = "deserialize_stringified_vec")]
    pub token_ids: Vec<String>,
    #[serde(deserialize_with = "deserialize_stringified_vec")]
    pub outcomes: Vec<String>,
    #[serde(rename = "eventStartTime")]
    pub event_start_time: Option<DateTime<Utc>>,
    #[serde(rename = "endDate")]
    pub end_date: Option<DateTime<Utc>>,
    pub events: Vec<PolyMarketEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PolyMarketEvent {
    pub id: String,
    pub ticker: Option<String>,
    pub slug: Option<String>,
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(rename = "endDate")]
    pub end_date: Option<DateTime<Utc>>,
}
