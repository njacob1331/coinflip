use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

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
    pub id: String,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub category: String,
    pub series: Option<String>,
    pub ticker: String,
    pub status: String,
    #[serde(rename = "resolvedAt")]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[serde(default)]
    pub contracts: Vec<Contract>,

    #[serde(rename = "contractOrderbooks")]
    pub contract_orderbooks: Option<Value>,
    pub volume: Option<String>,
    #[serde(rename = "volume24h")]
    pub volume_24h: Option<String>,
    pub liquidity: Option<String>,
    pub tags: Option<Value>,
    #[serde(rename = "effectiveDate")]
    pub effective_date: Option<DateTime<Utc>>,
    #[serde(rename = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,
    #[serde(rename = "startTime")]
    pub start_time: Option<Value>,
    #[serde(rename = "termsLink")]
    pub terms_link: Option<String>,
    pub subcategory: Option<Value>,
    #[serde(rename = "socialImageUrl")]
    pub social_image_url: Option<String>,
    #[serde(rename = "featuredImageUrl")]
    pub featured_image_url: Option<Value>,
    #[serde(rename = "isLive")]
    pub is_live: Option<bool>,
    pub events: Option<Value>,
    pub template: Option<String>,
    #[serde(rename = "gameId")]
    pub game_id: Option<Value>,
    #[serde(rename = "eventTags")]
    pub event_tags: Option<Value>,
    pub source: Option<String>,
    pub settlement: Option<Value>,
    pub participants: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct Contract {
    pub id: String,
    pub label: String,
    #[serde(rename = "abbreviatedName")]
    pub abbreviated_name: Option<String>,

    pub description: Value,
    pub prices: Option<Value>,

    pub color: Option<String>,
    pub status: String,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "priceHistory")]
    pub price_history: Option<Value>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,
    #[serde(rename = "resolutionSide")]
    pub resolution_side: Option<String>,
    #[serde(rename = "resolvedAt")]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(rename = "termsAndConditionsUrl")]
    pub terms_url: String,
    pub ticker: String,
    #[serde(rename = "instrumentSymbol")]
    pub instrument_symbol: String,
    #[serde(rename = "effectiveDate")]
    pub effective_date: Option<DateTime<Utc>>,
    #[serde(rename = "marketState")]
    pub market_state: Option<String>,
    #[serde(rename = "sortOrder")]
    pub sort_order: Option<u64>,
    #[serde(rename = "sportsSide")]
    pub sports_side: Option<String>,
    #[serde(rename = "teamId")]
    pub team_id: Option<String>,
    pub strike: Option<Strike>,
    pub source: Option<String>,
    #[serde(rename = "settlementValue")]
    pub settlement_value: Option<String>,
}

impl PartialEq for Contract {
    fn eq(&self, other: &Self) -> bool {
        self.instrument_symbol == other.instrument_symbol
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
