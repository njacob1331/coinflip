use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::mpsc::Receiver;

use crate::{stats::ml::EmbeddingModel, traits::Metadata};

#[derive(Debug, Clone)]
struct StructuralCorrelation {
    categorical_similarity: Option<f32>,
    strike_similarity: Option<f32>,
    expiration_similarity: Option<f32>,
    agg_similarity: f32,
}

impl StructuralCorrelation {
    fn compute<M: Metadata>(model: &mut EmbeddingModel, x: &M, y: &M) -> Self {
        let categorical_similarity = {
            if x.category() == y.category() {
                Some(1.0)
            } else {
                model
                    .call(x.category().as_str(), y.category().as_str())
                    .ok()
            }
        };

        let agg_similarity = (categorical_similarity.unwrap_or(0.0)) / 4.0;

        Self {
            categorical_similarity,
            strike_similarity: None,
            expiration_similarity: None,
            agg_similarity,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub target: u32,
    pub correlation: StructuralCorrelation,
}

pub struct StructuralCorrelationGraph<M> {
    model: EmbeddingModel,
    /// ticker -> node id
    entries: HashMap<String, u32>,

    /// node id -> ticker
    reverse: Vec<String>,

    /// node id -> market
    markets: Vec<M>,

    /// adjacency list
    graph: Vec<Vec<Edge>>,

    _phanton: std::marker::PhantomData<M>,
}

impl<M> StructuralCorrelationGraph<M>
where
    M: Metadata,
{
    pub fn new() -> Self {
        Self {
            model: EmbeddingModel::try_new().expect("embedding model failed to init"),
            entries: HashMap::new(),
            reverse: Vec::new(),
            markets: Vec::new(),
            graph: Vec::new(),
            _phanton: std::marker::PhantomData,
        }
    }

    pub fn insert(&mut self, entry: M) {
        let ticker = entry.ticker().to_string();

        if self.entries.contains_key(&ticker) {
            return;
        }

        let new_id = self.graph.len() as u32;

        // compute correlations BEFORE moving entry
        let mut edges = Vec::new();

        for (other_id, other_market) in self.markets.iter().enumerate() {
            let correlation = StructuralCorrelation::compute(&mut self.model, &entry, other_market);

            edges.push(Edge {
                target: other_id as u32,
                correlation: correlation.clone(),
            });

            self.graph[other_id].push(Edge {
                target: new_id,
                correlation,
            });
        }

        self.entries.insert(ticker.clone(), new_id);
        self.reverse.push(ticker);
        self.markets.push(entry);
        self.graph.push(edges);
    }

    pub fn similarities(&self, ticker: &str) -> Option<Vec<(&str, &StructuralCorrelation)>> {
        let id = *self.entries.get(ticker)?;

        let edges = &self.graph[id as usize];

        Some(
            edges
                .iter()
                .map(|edge| {
                    (
                        self.reverse[edge.target as usize].as_str(),
                        &edge.correlation,
                    )
                })
                .collect(),
        )
    }

    pub async fn run(&mut self, rx: &mut Receiver<MetadataTransportMsg<M>>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                MetadataTransportMsg::Insert(metadata) => self.insert(metadata),
                MetadataTransportMsg::Remove(key) => {
                    tracing::info!("GLOBAL: request to remove {key}")
                }
            }

            tracing::info!("{:#?}", self.graph);
        }

        tracing::info!("market matcher exiting")
    }
}

pub enum MetadataTransportMsg<M> {
    Insert(M),
    Remove(String),
}

impl<M> From<M> for MetadataTransportMsg<M>
where
    M: Metadata,
{
    fn from(value: M) -> Self {
        Self::Insert(value)
    }
}

// ["btc5m" -> []]
