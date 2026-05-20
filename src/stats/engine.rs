use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use rust_decimal::Decimal;
use tokio::sync::mpsc::Receiver;

#[derive(Debug)]
struct PredictionMarketDetails {
    ticker: String,
    category: String,
    expiry: String,
}

#[derive(Debug)]
struct StockMarketDetails {
    ticker: String,
}

#[derive(Debug)]
struct OptionsMarketDetails {
    ticker: String,
    expiry: String,
    strike: String,
}

#[derive(Debug)]
enum MarketInfo {
    Prediction(PredictionMarketDetails),
    Stock(StockMarketDetails),
    Options(OptionsMarketDetails),
}

struct MarketCorrelationMatrix {
    matrix: BTreeMap<String, (String, Decimal)>,
}

struct MarketMatcher {
    correlation_map: HashMap<String, MarketCorrelationMatrix>,
}

impl MarketMatcher {
    async fn compute(&self, mut rx: Receiver<MarketInfo>) {
        while let Some(info) = rx.recv().await {
            tracing::info!("{info:#?}")
        }
    }
}
