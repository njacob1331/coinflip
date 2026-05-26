use std::sync::Arc;

pub type SharedStr = Arc<str>;

#[derive(Debug)]
pub enum OrderbookUpdate<K, S, D> {
    Snapshot { key: K, data: S },
    Diff { key: K, data: D },
    Terminal(K),
}

impl<K, S, D> OrderbookUpdate<K, S, D>
where
    K: Clone,
{
    pub fn key(&self) -> &K {
        match self {
            Self::Snapshot { key, .. } => key,
            Self::Diff { key, .. } => key,
            Self::Terminal(key) => key,
        }
    }

    pub fn take_key(&self) -> K {
        match self {
            Self::Snapshot { key, .. } => key.clone(),
            Self::Diff { key, .. } => key.clone(),
            Self::Terminal(key) => key.clone(),
        }
    }
}

pub enum OrderbookSequence {
    Valid,
    Stale,
    Gap,
}
