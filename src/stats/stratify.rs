use crate::{data_structures::Cache, stats::Observation};

#[derive(Debug, Default, Clone, Copy)]
pub enum Stratified {
    Time {
        bucket: u32,
    },
    #[default]
    None,
}

pub struct Stratifier {
    cache: Cache<u64>,
}

impl Stratifier {
    pub fn new() -> Self {
        Self {
            cache: Cache::new(),
        }
    }

    pub fn stratify_many<T>(
        &mut self,
        matrix: &mut [Vec<Observation<T>>],
        window_timeframe_ms: u64,
    ) {
        if let Some(batch_min) = matrix.iter().flatten().map(|o| o.ts).min() {
            self.cache
                .write_if(batch_min, |current, value| value < current);
        }
        for obs in matrix.iter_mut().flatten() {
            self.stratify(obs, window_timeframe_ms);
        }
    }

    pub fn stratify<T>(&mut self, obs: &mut Observation<T>, window_timeframe_ms: u64) {
        let min = match self.cache.read() {
            Some(&cached) if cached <= obs.ts => cached,
            _ => obs.ts,
        };
        self.cache.write_if(min, |current, new| new < current);

        obs.stratify(Stratified::Time {
            bucket: ((obs.ts - min) / window_timeframe_ms) as u32,
        });
    }
}
