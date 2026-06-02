use crate::stats::{Observation, observation::Timescale, stratify::Stratified};
use std::sync::Arc;

pub type SharedStr = Arc<str>;
pub type UnscaledMidPrice = u16;

#[derive(Debug)]
pub enum OrderbookUpdate<S, D> {
    Snapshot { key: SharedStr, data: S },
    Diff { key: SharedStr, data: D },
    Terminal(SharedStr),
}

pub enum OrderbookSequence {
    Valid,
    Stale,
    Gap,
}

#[derive(Debug, Clone)]
pub struct OrderbookSnapshot {
    pub id: SharedStr,
    pub mid: Option<u16>,
    pub ts: u64,
}

impl From<OrderbookSnapshot> for Observation<Option<u16>> {
    fn from(value: OrderbookSnapshot) -> Self {
        Self {
            id: value.id,
            ts: value.ts,
            timescale: Timescale::Nano,
            value: value.mid,
            stratified: Default::default(),
        }
    }
}
