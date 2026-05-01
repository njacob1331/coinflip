use anyhow::Result;
use async_trait::async_trait;

use crate::session::Priority;

pub trait Parser<T>: Send + Sync
where
    T: Send + 'static,
{
    fn parse(&self, bytes: &[u8]) -> Result<T>;
}

#[async_trait]
pub trait Router<T>: Send + Sync
where
    T: Send + 'static,
{
    async fn route(&self, msg: T) -> Result<()>;
}

pub trait HasPriority {
    fn priority(&self) -> Priority;
}
