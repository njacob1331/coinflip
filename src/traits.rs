use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::{future::Future, sync::Arc};

use crate::{
    common::OrderbookSequence,
    metadata::{Category, TimeFrame},
    session::Priority,
};

pub trait Parser<T>: Send + Sync
where
    T: Send + 'static,
{
    fn parse(&self, bytes: &[u8]) -> Result<T>;
}

// #[async_trait]
// pub trait Router<T>: Send + Sync
// where
//    T: Send + 'static,
// {
//     async fn route(&self, msg: T) -> Result<()>;
// }

pub trait Router<T>: Send + Sync
where
    T: Send + 'static,
{
    fn route(&self, msg: T) -> impl Future<Output = Result<()>> + Send;
}

pub trait Prioritize {
    fn priority(&self) -> Priority;
}

pub trait OrderBook<S, D> {
    fn from_snapshot(snapshot: S) -> Self;
    fn update(&mut self, update: D);
    // fn bid(&self) -> Option<Decimal>;
    // fn ask(&self) -> Option<Decimal>;
    // fn mid(&self) -> Option<Decimal>;
    // fn spread(&self) -> Option<Decimal>;
    fn sequence(&self, update: &D) -> OrderbookSequence;
    fn corrupted(&self, update: S) -> bool;
}

pub trait OrderbookData {
    fn key(&self) -> &str;
    fn take_key(&mut self) -> Arc<str>;
    fn is_snapshot(&self) -> bool;
}

pub trait Metadata {
    fn id(&self) -> &str;
    fn category(&self) -> Category;
    fn context(&self) -> Option<String>;
    fn timeframe(&self) -> TimeFrame;
}
