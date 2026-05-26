use std::sync::Arc;

use crate::{
    common::{OrderbookUpdate, SharedStr},
    gemini::{
        messages::{ContractStatus, L2DifferentialDepth},
        responses::{Contract, Event},
    },
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

    pub fn contract_id(&self) -> Arc<str> {
        self.contract.instrument_symbol.clone()
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
        let start = self.event.start_time;
        let end = self.event.expiry_date;

        match start {
            Some(start) => TimeFrame::Discrete { start, end },
            None => TimeFrame::ExpiresAt { end },
        }
    }
}

impl From<L2DifferentialDepth>
    for OrderbookUpdate<SharedStr, L2DifferentialDepth, L2DifferentialDepth>
{
    fn from(value: L2DifferentialDepth) -> Self {
        if value.first_update_id == value.last_update_id {
            return OrderbookUpdate::Snapshot {
                key: value.symbol.clone(),
                data: value,
            };
        }

        OrderbookUpdate::Diff {
            key: value.symbol.clone(),
            data: value,
        }
    }
}

impl From<ContractStatus> for OrderbookUpdate<SharedStr, L2DifferentialDepth, L2DifferentialDepth> {
    fn from(value: ContractStatus) -> Self {
        OrderbookUpdate::Terminal(value.symbol.clone())
    }
}
