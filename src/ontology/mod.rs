use std::collections::HashMap;

use anyhow::Result;

use crate::{
    common::SharedStr,
    ontology::client::{Item, SearchResult, WikiDataClient},
};

pub mod client;

pub struct TopResult {
    id: String,
    match_type: String,
    match_value: String,
}

pub struct SearchEngine {
    client: WikiDataClient,
    cache: HashMap<SharedStr, Item>,
}

impl SearchEngine {
    pub async fn search(&mut self, query: SharedStr) -> Result<Vec<Item>> {
        if let Some(cached) = self.cache.get(&query) {}

        self.client.search(&query).await.map(|s| s.results)
    }
}
