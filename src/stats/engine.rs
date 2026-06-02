use kanal::AsyncReceiver;

use crate::{
    common::{OrderbookSnapshot, SharedStr},
    data_structures::IndexedMatrix,
    stats::{Observation, TransportMsg, observation::Timescale, stratify::Stratifier},
};

pub struct StatsEngine {
    matrix: IndexedMatrix<SharedStr, Observation<Option<u16>>>,
    stratifier: Stratifier,
}

impl StatsEngine {
    pub fn new() -> Self {
        Self {
            matrix: IndexedMatrix::new(100),
            stratifier: Stratifier::new(),
        }
    }

    fn preprocess(&mut self, data: OrderbookSnapshot) -> Observation<Option<u16>> {
        let mut obs: Observation<Option<u16>> = data.into();

        match obs.timescale {
            Timescale::Nano => obs.ts = obs.ts / 1_000_000,
            _ => {}
        }

        self.stratifier.stratify(&mut obs, 100_000);

        obs
    }

    pub async fn run(&mut self, rx: AsyncReceiver<TransportMsg<SharedStr, OrderbookSnapshot>>) {
        while let Ok(msg) = rx.recv().await {
            match msg {
                TransportMsg::HandleData(data) => {
                    let obs = self.preprocess(data);
                    self.matrix.push(&obs.id.clone(), obs)
                }
                TransportMsg::RemoveData(index) => self.matrix.remove(&index),
            }
        }
    }
}
