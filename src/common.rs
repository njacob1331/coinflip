use std::sync::Arc;

pub type SharedStr = Arc<str>;

#[derive(Debug)]
pub enum OrderbookUpdate<K, S, D> {
    Snapshot { key: K, data: S },
    Diff { key: K, data: D },
    Terminal(K),
}

pub enum OrderbookSequence {
    Valid,
    Stale,
    Gap,
}
