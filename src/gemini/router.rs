use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::{
    common::OrderbookUpdate,
    gemini::messages::{
        BalanceUpdate, ContractStatus, L2DifferentialDepth, Message, SubscriptionError,
    },
    traits::Router,
};

pub struct GeminiRouter {
    orderbook_tx: Sender<OrderbookUpdate<L2DifferentialDepth, L2DifferentialDepth>>,
    orderbook_rx: Option<Receiver<OrderbookUpdate<L2DifferentialDepth, L2DifferentialDepth>>>,
    balance_tx: Sender<BalanceUpdate>,
    balance_rx: Option<Receiver<BalanceUpdate>>,
    subscription_err_tx: Sender<SubscriptionError>,
    subscription_err_rx: Option<Receiver<SubscriptionError>>,
    contract_status_tx: Sender<Arc<ContractStatus>>,
    contract_status_rx: Option<Receiver<Arc<ContractStatus>>>,
    contract_metadata_tx: Sender<Arc<ContractStatus>>,
    contract_metadata_rx: Option<Receiver<Arc<ContractStatus>>>,
}

impl GeminiRouter {
    pub fn new() -> Self {
        let (orderbook_tx, orderbook_rx) = channel(256);
        let (balance_tx, balance_rx) = channel(32);
        let (subscription_err_tx, subscription_err_rx) = channel(32);
        let (contract_status_tx, contract_status_rx) = channel(128);
        let (contract_metadata_tx, contract_metadata_rx) = channel(128);

        Self {
            orderbook_tx,
            orderbook_rx: Some(orderbook_rx),
            balance_tx,
            balance_rx: Some(balance_rx),
            subscription_err_tx,
            subscription_err_rx: Some(subscription_err_rx),
            contract_status_tx,
            contract_status_rx: Some(contract_status_rx),
            contract_metadata_tx,
            contract_metadata_rx: Some(contract_metadata_rx),
        }
    }

    pub fn orderbook_rx(
        &mut self,
    ) -> Receiver<OrderbookUpdate<L2DifferentialDepth, L2DifferentialDepth>> {
        self.orderbook_rx.take().unwrap()
    }

    pub fn balance_rx(&mut self) -> Receiver<BalanceUpdate> {
        self.balance_rx.take().unwrap()
    }

    pub fn subscription_err_rx(&mut self) -> Receiver<SubscriptionError> {
        self.subscription_err_rx.take().unwrap()
    }

    pub fn contract_status_rx(&mut self) -> Receiver<Arc<ContractStatus>> {
        self.contract_status_rx.take().unwrap()
    }

    pub fn contract_metadata_rx(&mut self) -> Receiver<Arc<ContractStatus>> {
        self.contract_metadata_rx.take().unwrap()
    }
}

impl Router<Message> for GeminiRouter {
    #[inline]
    async fn route(&self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::L2DifferentialDepth(update) => self.orderbook_tx.send(update.into()).await?,
            Message::BalanceUpdate(update) => self.balance_tx.send(update).await?,
            Message::SubscriptionError(error) => self.subscription_err_tx.send(error).await?,
            Message::ContractStatus(status) => {
                let status = Arc::new(status);
                if status.new_status == "Settled" {
                    self.orderbook_tx
                        .send(OrderbookUpdate::Terminal(status.symbol.clone()))
                        .await?;
                }
                self.contract_status_tx.send(status.clone()).await?;
                self.contract_metadata_tx.send(status).await?
            }
            _ => {}
        }

        Ok(())
    }

    #[inline]
    fn route_sync(&self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::L2DifferentialDepth(update) => self.orderbook_tx.try_send(update.into())?,
            Message::BalanceUpdate(update) => self.balance_tx.try_send(update)?,
            Message::SubscriptionError(error) => self.subscription_err_tx.try_send(error)?,
            Message::ContractStatus(status) => {
                let status = Arc::new(status);
                if status.new_status == "Settled" {
                    self.orderbook_tx
                        .try_send(OrderbookUpdate::Terminal(status.symbol.clone()))?;
                }
                self.contract_status_tx.try_send(status.clone())?;
                self.contract_metadata_tx.try_send(status)?
            }
            _ => {}
        }

        Ok(())
    }
}
