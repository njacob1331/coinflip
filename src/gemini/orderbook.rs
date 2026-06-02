use crate::{
    common::{OrderbookSequence, OrderbookSnapshot, SharedStr},
    gemini::messages::L2DifferentialDepth,
    traits::OrderBook,
};
use std::{collections::BTreeMap, time::Instant};

#[derive(Debug, Clone)]
pub struct GeminiOrderbook {
    pub bids: BTreeMap<u8, i32>,
    pub asks: BTreeMap<u8, i32>,
    pub last_update_id: u64,
    pub last_update_ts_ns: u64,
}

impl OrderBook<L2DifferentialDepth, L2DifferentialDepth> for GeminiOrderbook {
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
            last_update_ts_ns: snapshot.event_time_ns,
        }
    }

    fn sequence(&self, update: &L2DifferentialDepth) -> OrderbookSequence {
        let last = self.last_update_id;

        // 1. Ignore outdated updates
        if update.last_update_id <= last {
            return OrderbookSequence::Stale;
        }

        // 2. First valid update after snapshot
        //
        // U <= lastUpdateId + 1 <= u
        if update.first_update_id <= last + 1 && last + 1 <= update.last_update_id {
            return OrderbookSequence::Valid;
        }

        // 3. Sequential update
        //
        // U == lastUpdateId + 1
        if update.first_update_id == last + 1 {
            return OrderbookSequence::Valid;
        }

        // 4. Gap → corruption → must resync
        OrderbookSequence::Gap
    }

    fn corrupted(&self, update: L2DifferentialDepth) -> bool {
        matches!(self.sequence(&update), OrderbookSequence::Gap)
    }

    fn update(&mut self, update: L2DifferentialDepth) {
        self.last_update_id = update.last_update_id;
        self.last_update_ts_ns = update.event_time_ns;

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

    fn snapshot(&self, id: SharedStr) -> OrderbookSnapshot {
        OrderbookSnapshot {
            id,
            mid: self.mid(),
            ts: self.last_update_ts_ns,
        }
    }

    fn mid(&self) -> Option<u16> {
        let best_bid = *self.bids.keys().next_back()?;
        let best_ask = *self.asks.keys().next()?;
        let mid = (best_bid + best_ask / 2) as u16;

        Some(mid * 100)
    }
}
