use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

use crate::common::SharedStr;

pub enum ObservationType {
    Measurement,
    Outcome,
    Derivative,
    Forecast,
    // ex report released, touchdown scored, etc
    Signal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeFrame {
    Continuous,
    Discrete {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    OpenEnded {
        start: DateTime<Utc>,
        est_end: Option<DateTime<Utc>>,
    },
    ExpiresAt {
        end: DateTime<Utc>,
    },
    Sampled {
        interval: Duration,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Category {
    Sports,
    Crypto,
    Commodities,
    Weather,
    Other(String),
}

pub struct CryptoMetadata {
    asset: String,
}

impl AsRef<str> for Category {
    fn as_ref(&self) -> &str {
        match self {
            Self::Sports => "sports",
            Self::Crypto => "crypto",
            Self::Commodities => "commodity",
            Self::Weather => "weather",
            Self::Other(inner) => inner.as_str(),
        }
    }
}

impl<'de> Deserialize<'de> for Category {
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

struct Metadata {
    id: SharedStr,
    category: Category,
    subject: SharedStr,
}
