use anyhow::{Context, Result};

use crate::gemini::responses::{Event, ListMarketEvents};

#[derive(Debug)]
pub struct GeminiClient {
    inner: reqwest::Client,
}

impl GeminiClient {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
        }
    }

    pub async fn list_prediction_market_events(&self) -> Result<Vec<Event>> {
        let url = "https://api.gemini.com/v1/prediction-markets/events";

        let mut all_events = Vec::with_capacity(250);
        let mut offset = 0;
        let limit = 50;

        loop {
            let res = self
                .inner
                .get(url)
                .query(&[
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ])
                .send()
                .await
                .with_context(|| format!("request to {} failed", url))?;

            // Deserialize directly into ApiResponse
            let parsed: ListMarketEvents = res
                .json()
                .await
                .with_context(|| format!("failed to deserialize response from {}", url))?;

            all_events.extend(parsed.data);

            // Check if we've fetched all events
            offset += limit;
            if offset >= parsed.pagination.total as usize {
                break;
            }
        }

        Ok(all_events)
    }
}
