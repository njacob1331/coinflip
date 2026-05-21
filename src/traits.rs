use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::future::Future;

use crate::{
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

pub trait OrderBook<T>
where
    T: OrderbookData,
{
    fn from_snapshot(snapshot: T) -> Self;
    fn update(&mut self, update: T);
    // fn bid(&self) -> Option<Decimal>;
    // fn ask(&self) -> Option<Decimal>;
    // fn mid(&self) -> Option<Decimal>;
    // fn spread(&self) -> Option<Decimal>;
    fn valid_sequence(&self, update: &T) -> bool;
    fn corrupted(&self, update: T) -> bool;
}

pub trait OrderbookData {
    fn key(&self) -> &str;
    fn take_key(&mut self) -> String;
    fn is_snapshot(&self) -> bool;
}

pub trait Metadata {
    fn id(&self) -> &str;
    fn category(&self) -> Category;
    fn context(&self) -> Option<String>;
    fn timeframe(&self) -> TimeFrame;
}
