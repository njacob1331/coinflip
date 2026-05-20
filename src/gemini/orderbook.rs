use crate::{gemini::messages::L2DifferentialDepth, traits::OrderBook};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct GeminiOrderbook {
    pub bids: BTreeMap<u8, i32>,
    pub asks: BTreeMap<u8, i32>,
    pub last_update_id: u64,
}

impl OrderBook<L2DifferentialDepth> for GeminiOrderbook {
    fn from_snapshot(snapshot: L2DifferentialDepth) -> Self {
        Self {
            bids: snapshot
                .bids
                .into_iter()
                .map(|l| (l.price, l.qty))
                .collect(),
            asks: snapshot
                .asks
                .into_iter()
                .map(|l| (l.price, l.qty))
                .collect(),
            last_update_id: snapshot.last_update_id,
        }
    }

    fn valid_sequence(&self, update: &L2DifferentialDepth) -> bool {
        if self.last_update_id == 0 {
            return true;
        }
        update.first_update_id == self.last_update_id
    }

    fn corrupted(&self, update: L2DifferentialDepth) -> bool {
        false
    }

    fn update(&mut self, update: L2DifferentialDepth) {
        self.last_update_id = update.last_update_id;

        for level in update.bids {
            if level.qty == 0 {
                self.bids.remove(&level.price);
            } else {
                self.bids.insert(level.price, level.qty);
            }
        }

        for level in update.asks {
            if level.qty == 0 {
                self.asks.remove(&level.price);
            } else {
                self.asks.insert(level.price, level.qty);
            }
        }
    }
}
