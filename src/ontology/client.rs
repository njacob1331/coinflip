use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub results: Vec<Item>,
}

#[derive(Debug, Deserialize)]
pub struct Item {
    pub id: String,
    pub display_label: DisplayLabel,
    pub description: Description,
    #[serde(rename = "match")]
    pub match_info: Match,
}

#[derive(Debug, Deserialize)]
pub struct DisplayLabel {
    pub language: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Description {
    pub language: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Match {
    #[serde(rename = "type")]
    pub match_type: String,
    pub language: String,
    pub text: String,
}

#[derive(Debug)]
pub struct WikiDataClient {
    inner: reqwest::Client,
}

impl WikiDataClient {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
        }
    }

    pub async fn search(&self, query: &str) -> Result<SearchResult> {
        let url = "https://www.wikidata.org/w/rest.php/wikibase/v1/search/items";

        let res = self
            .inner
            .get(url)
            .query(&[("q", query), ("language", "en")])
            .send()
            .await
            .with_context(|| format!("request to {} failed", url))?;

        let parsed: SearchResult = res.json().await?;

        Ok(parsed)
    }
}
