use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use dashmap::DashMap;
use rust_decimal::Decimal;
use tokio::sync::mpsc::Receiver;

type EventTicker = String;
type ContractTicker = String;

#[derive(Debug)]
struct OrderbookSnapshot {
    symbol: String,
    bid: Decimal,
    ask: Decimal,
}

struct CorrelationMatrix {
    matrix: BTreeMap<ContractTicker, Decimal>,
}

impl CorrelationMatrix {
    fn update(update: &OrderbookSnapshot) {}
}

struct StatsEngine {
    correlation_matrices: HashMap<ContractTicker, CorrelationMatrix>,
}

impl StatsEngine {
    async fn compute(&self, mut rx: Receiver<OrderbookSnapshot>) {
        while let Some(update) = rx.recv().await {
            self.correlation_matrices.get(&update.symbol);
        }
    }
}
