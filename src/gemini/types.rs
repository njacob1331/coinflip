use std::sync::Arc;

use crate::{
    gemini::responses::{Contract, Event},
    metadata::{Category, TimeFrame},
    traits::Metadata,
};

pub struct BinaryPredictionMarket {
    event: Arc<Event>,
    contract: Contract,
}

impl BinaryPredictionMarket {
    pub fn new(event: Arc<Event>, contract: Contract) -> Self {
        Self { event, contract }
    }

    pub fn timeframe(&self) -> TimeFrame {
        let start = self.event.start_time;
        let end = self.event.expiry_date;

        match start {
            Some(start) => TimeFrame::Discrete { start, end },
            None => TimeFrame::ExpiresAt { end },
        }
    }
}

impl Metadata for BinaryPredictionMarket {
    fn id(&self) -> &str {
        &self.contract.instrument_symbol
    }

    fn category(&self) -> Category {
        self.event.category.clone()
    }

    fn context(&self) -> Option<String> {
        let tags = self.event.tags.as_ref()?;
        Some(tags.join(" "))
    }

    fn timeframe(&self) -> TimeFrame {
        self.timeframe()
    }
}
