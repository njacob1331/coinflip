use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

use crate::{stats::ml::EmbeddingModel, traits::Metadata};
use slotmap::{SlotMap, new_key_type};

// ── Correlation ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct StructuralCorrelation {
    categorical_similarity: Option<f32>,
    semantic_similarity: Option<f32>,
    agg_similarity: f32,
}

impl StructuralCorrelation {
    fn compare<S>(x: &S, y: &S) -> Option<f32>
    where
        S: PartialEq + AsRef<str>,
    {
        if x == y {
            return Some(1.0);
        }

        Some(0.0)
    }

    fn compute<M>(x: &M, y: &M) -> Self
    where
        M: Metadata,
    {
        let categorical_similarity = Self::compare(&x.category(), &y.category());

        let semantic_similarity = match (x.context(), y.context()) {
            (Some(x), Some(y)) => Self::compare(&x, &y),
            _ => None,
        };

        let agg_similarity =
            (categorical_similarity.unwrap_or(0.0) + semantic_similarity.unwrap_or(0.0)) / 2.0;

        Self {
            categorical_similarity,
            semantic_similarity,
            agg_similarity,
        }
    }
}

// ── Graph types ───────────────────────────────────────────────────────────────

new_key_type! { pub struct NodeKey; }

#[derive(Debug, Clone)]
pub struct Edge {
    pub target: NodeKey,
    pub correlation: StructuralCorrelation,
}

/// Bundles metadata and adjacency together so they can never get out of sync.
#[derive(Debug)]
struct Node<M> {
    data: M,
    edges: Vec<Edge>,
}

// ── Graph ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct StructuralCorrelationGraph<M> {
    entries: HashMap<String, NodeKey>, // ticker → stable key
    nodes: SlotMap<NodeKey, Node<M>>,  // key → data + edges
}

impl<M: Metadata> StructuralCorrelationGraph<M> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            nodes: SlotMap::with_key(),
        }
    }

    pub fn insert(&mut self, entry: M) {
        if self.entries.contains_key(entry.id()) {
            return;
        }

        let ticker = entry.id().to_string();

        // Collect (key, correlation) pairs before inserting — lets us hold
        // &self.nodes and &mut self.model simultaneously without a borrow conflict.
        let edge_data: Vec<(NodeKey, StructuralCorrelation)> = self
            .nodes
            .iter()
            .map(|(key, node)| {
                let corr = StructuralCorrelation::compute(&entry, &node.data);
                (key, corr)
            })
            .collect();

        // Insert with empty edges; we'll populate them right after.
        let key = self.nodes.insert(Node {
            data: entry,
            edges: vec![],
        });

        // Wire forward edges from the new node and back-edges from each neighbor.
        // Both operations are safe now that `key` is known.
        let forward: Vec<Edge> = edge_data
            .iter()
            .map(|&(target, correlation)| {
                self.nodes[target].edges.push(Edge {
                    target: key,
                    correlation,
                });
                Edge {
                    target,
                    correlation,
                }
            })
            .collect();

        self.nodes[key].edges = forward;
        self.entries.insert(ticker, key);
    }

    pub fn remove(&mut self, ticker: &str) {
        let Some(key) = self.entries.remove(ticker) else {
            return;
        };

        // Collect neighbors before mutating so we don't borrow `nodes` twice.
        let neighbors: Vec<NodeKey> = self.nodes[key].edges.iter().map(|e| e.target).collect();

        for neighbor in neighbors {
            if let Some(node) = self.nodes.get_mut(neighbor) {
                node.edges.retain(|e| e.target != key);
            }
        }

        // Drops both data and edges atomically — nothing left to clean up.
        self.nodes.remove(key);
    }

    pub fn similarities(&self, ticker: &str) -> Option<Vec<(&str, &StructuralCorrelation)>> {
        let key = *self.entries.get(ticker)?;

        Some(
            self.nodes[key]
                .edges
                .iter()
                .filter_map(|edge| {
                    let node = self.nodes.get(edge.target)?;
                    Some((node.data.id(), &edge.correlation))
                })
                .collect(),
        )
    }

    pub async fn run(&mut self, rx: &mut Receiver<MetadataTransportMsg<M>>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                MetadataTransportMsg::Insert(m) => self.insert(m),
                MetadataTransportMsg::Update(m) => {
                    // Remove old entry and reinsert — edges recomputed fresh.
                    self.remove(m.id());
                    self.insert(m);
                }
                MetadataTransportMsg::Remove(key) => self.remove(&key),
            }

            // tracing::info!("{:#?}", self.entries.keys())
        }
    }
}

// ── Transport ─────────────────────────────────────────────────────────────────

pub enum MetadataTransportMsg<O> {
    Insert(O),
    Update(O),
    Remove(String),
}

impl<M: Metadata> From<M> for MetadataTransportMsg<M> {
    fn from(value: M) -> Self {
        Self::Insert(value)
    }
}
