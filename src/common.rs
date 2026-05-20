use serde::{Deserialize, Deserializer};

#[derive(Debug, PartialEq)]
pub enum MarketCategory {
    Sports,
    Crypto,
    Commodities,
    Weather,
    Other(String),
}

impl MarketCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Sports => "sports prediction market",
            Self::Crypto => "crypto prediction market",
            Self::Commodities => "commodity prediction market",
            Self::Weather => "weather prediction market",
            Self::Other(inner) => inner.as_str(),
        }
    }
}

impl<'de> Deserialize<'de> for MarketCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;

        let normalized = raw.to_lowercase();

        Ok(match normalized.as_str() {
            "sports" | "sport" => Self::Sports,

            "crypto" => Self::Crypto,

            "commodities" | "commodity" => Self::Commodities,

            "weather" => Self::Weather,

            _ => Self::Other(normalized),
        })
    }
}

// pub struct Market {
//     ticker: String,
//     category: MarketCategory,
//     expiration: Option<DateTime<Utc>>,
//     underlying_asset: String,
//     strike: Option<String>,
// }

// impl Market {
//     pub fn ticker(&self) -> &str {
//         &self.ticker
//     }
//     pub fn category(&self) -> &MarketCategory {
//         &self.category
//     }
//     pub fn expiration(&self) -> Option<&DateTime<Utc>> {
//         self.expiration.as_ref()
//     }
//     pub fn underlying_asset(&self) -> &str {
//         &self.underlying_asset
//     }
//     pub fn strike(&self) -> Option<&String> {
//         self.strike.as_ref()
//     }
// }
