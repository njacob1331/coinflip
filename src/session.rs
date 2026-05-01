use crate::{
    traits::{HasPriority, Parser, Router},
    ws::Ws,
};
use anyhow::Result;
use kanal::AsyncSender;
use serde::Serialize;
use std::{collections::BinaryHeap, sync::Arc, time::Duration};
use tokio::{
    sync::{
        Notify,
        mpsc::{Receiver, channel},
    },
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum Payload<T> {
    Single(T),
    Batch(Vec<T>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critial,
}

#[derive(Debug)]
pub struct Request<T> {
    pub priority: Priority,
    pub payload: Payload<T>,
}

impl<T> From<Payload<T>> for Request<T>
where
    T: HasPriority,
{
    fn from(payload: Payload<T>) -> Self {
        let priority = match &payload {
            Payload::Single(item) => item.priority(),

            // Batch rule: highest priority wins
            Payload::Batch(items) => items
                .iter()
                .map(|i| i.priority())
                .max()
                .unwrap_or(Priority::Low),
        };

        Request { priority, payload }
    }
}

impl<T> Ord for Request<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl<T> PartialOrd for Request<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq for Request<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<T> Eq for Request<T> {}

struct Session<T> {
    session_tx: AsyncSender<Request<T>>,
    pq: BinaryHeap<Request<T>>,
    writer_handle: JoinHandle<Result<()>>,
    reader_handle: JoinHandle<Result<()>>,
}

impl<T> Session<T>
where
    T: HasPriority + Serialize + Send + 'static,
{
    async fn new<P, M, R>(url: &str, parser: Arc<P>, router: Arc<R>) -> Result<Self>
    where
        P: Parser<M> + Send + 'static,
        R: Router<M> + Send + 'static,
        M: Send + 'static,
    {
        let (writer, reader) = Ws::connect(url).await?;
        let (session_tx, session_rx) = kanal::bounded_async::<Request<T>>(8);

        Ok(Self {
            session_tx,
            pq: BinaryHeap::with_capacity(100),
            writer_handle: Ws::spawn_writer(writer, session_rx),
            reader_handle: Ws::spawn_reader(reader, parser, router),
        })
    }

    async fn forward_from_queue(&mut self) -> Result<()> {
        if let Some(req) = self.pq.pop() {
            self.session_tx.send(req).await?;
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.writer_handle.abort();
        self.reader_handle.abort();
    }

    async fn run(&mut self, request_rx: &mut Receiver<Payload<T>>, cancel: CancellationToken) {
        loop {
            tokio::select! {
                biased;

                _ = cancel.cancelled() => {
                    println!("session cancelling");
                    break
                },

                res = &mut self.writer_handle => {
                    match res {
                        Ok(res) => println!("ws session ended with result: {res:?}"),
                        Err(e) => println!("ws writer join error: {e}")
                    }
                    break
                }

                res = &mut self.reader_handle => {
                    match res {
                        Ok(res) => println!("ws session ended with result: {res:?}"),
                        Err(e) => println!("ws reader join error: {e}")
                    }
                    break
                }

                request = request_rx.recv() => {
                    match request {
                        Some(req) => {
                            self.pq.push(req.into());
                            if let Err(e) = self.forward_from_queue().await {
                                println!("failed to forward ws message, channel closed: {e}");
                                break;
                            }
                        },
                        None => break
                    }
                }
            }
        }

        println!("session exiting");
        self.exit();
    }
}

pub struct SessionManager {
    connection_reset: Arc<Notify>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            connection_reset: Arc::new(Notify::new()),
        }
    }

    pub fn connection_listener(&self) -> Arc<Notify> {
        self.connection_reset.clone()
    }

    pub async fn run<P, R, M, T>(
        &mut self,
        url: &str,
        parser: P,
        router: R,
        mut request_rx: Receiver<Payload<T>>,
        cancel: CancellationToken,
    ) where
        P: Parser<M> + Send + 'static,
        R: Router<M> + Send + 'static,
        M: Send + 'static,
        T: HasPriority + Serialize + Send + 'static,
    {
        let parser = Arc::new(parser);
        let router = Arc::new(router);
        let retry_interval = Duration::from_secs(1);

        loop {
            tracing::info!("starting ws connection @ {url}");
            match Session::new(url, parser.clone(), router.clone()).await {
                Ok(mut session) => session.run(&mut request_rx, cancel.clone()).await,
                Err(e) => println!("failed to start ws session: {e}"),
            }

            if cancel.is_cancelled() {
                break;
            }

            // notify that the connection died
            self.connection_reset.notify_waiters();

            tracing::info!(
                "ws @ {url} disconnected, reconnecting in {} secs...",
                retry_interval.as_secs()
            );

            tokio::time::sleep(retry_interval).await;
        }

        println!("session manager exiting")
    }
}
