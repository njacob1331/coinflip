use rust_decimal::Decimal;
use std::collections::BTreeMap;

use crate::gemini::messages::OrderbookUpdate;

#[derive(Debug, Clone)]
pub struct Orderbook {
    pub bids: BTreeMap<Decimal, Decimal>,
    pub asks: BTreeMap<Decimal, Decimal>,
    pub last_update_id: u64,
}

impl Orderbook {
    pub fn from_snapshot(snapshot: OrderbookUpdate) -> Self {
        Self {
            bids: snapshot.bids.into_iter().collect(),
            asks: snapshot.asks.into_iter().collect(),
            last_update_id: snapshot.last_update_id,
        }
    }

    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update_id: 0,
        }
    }

    pub fn is_valid_sequence(&self, update: &OrderbookUpdate) -> bool {
        if self.last_update_id == 0 {
            return true;
        }
        update.first_update_id == self.last_update_id
    }

    pub fn update(&mut self, update: OrderbookUpdate) {
        self.last_update_id = update.last_update_id;

        // 2. Apply Bids
        for (price, qty) in update.bids {
            Self::apply_level(&mut self.bids, price, qty);
        }

        // 3. Apply Asks
        for (price, qty) in update.asks {
            Self::apply_level(&mut self.asks, price, qty);
        }
    }

    // Helper to consolidate logic and ensure negative quantities aren't stored
    fn apply_level(levels: &mut BTreeMap<Decimal, Decimal>, price: Decimal, qty: Decimal) {
        if qty.is_zero() || qty.is_sign_negative() {
            levels.remove(&price);
        } else {
            levels.insert(price, qty);
        }
    }

    pub fn is_crossed(&self) -> bool {
        if let (Some(bid), Some(ask)) = (self.yes_bid(), self.yes_ask()) {
            return bid >= ask;
        }

        false
    }

    // --- Price Accessors ---

    pub fn yes_bid(&self) -> Option<Decimal> {
        self.bids.last_key_value().map(|(p, _)| *p)
    }

    pub fn yes_ask(&self) -> Option<Decimal> {
        self.asks.first_key_value().map(|(p, _)| *p)
    }

    /// The "No" bid price is (1 - Best Yes Ask)
    pub fn no_bid(&self) -> Option<Decimal> {
        self.yes_ask().map(|p| Decimal::from(1) - p)
    }

    /// The "No" ask price is (1 - Best Yes Bid)
    pub fn no_ask(&self) -> Option<Decimal> {
        self.yes_bid().map(|p| Decimal::from(1) - p)
    }

    pub fn spread(&self) -> Option<Decimal> {
        match (self.yes_bid(), self.yes_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }
}
