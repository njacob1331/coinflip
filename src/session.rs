use crate::{
    traits::{Parser, Router},
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

#[derive(Debug)]
pub enum Request<T> {
    LowPriority { batch: bool, inner: T },
    HighPriority { batch: bool, inner: T },
}

impl<T> Request<T> {
    pub fn split(self) -> (bool, T) {
        match self {
            Request::HighPriority { batch, inner } => (batch, inner),
            Request::LowPriority { batch, inner } => (batch, inner),
        }
    }
}

impl<T> Ord for Request<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use Request::*;
        use std::cmp::Ordering::*;

        match (self, other) {
            (HighPriority { .. }, LowPriority { .. }) => Greater,
            (LowPriority { .. }, HighPriority { .. }) => Less,
            _ => Equal,
        }
    }
}

impl<T> PartialOrd for Request<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq for Request<T> {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Request::HighPriority { .. }, Request::HighPriority { .. })
                | (Request::LowPriority { .. }, Request::LowPriority { .. })
        )
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
    T: Serialize + Send + 'static,
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

    fn teardown(&mut self) {
        self.writer_handle.abort();
        self.reader_handle.abort();
    }

    async fn run(&mut self, request_rx: &mut Receiver<Request<T>>)
    where
        T: Serialize,
    {
        loop {
            tokio::select! {
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
                        Some(req) => self.pq.push(req),
                        None => break
                    }
                }
            }

            if let Err(e) = self.forward_from_queue().await {
                println!("failed to forward ws message, channel closed: {e}");
                break;
            }
        }

        self.teardown();
    }
}

pub struct SessionManager {
    connection_reset: Arc<Notify>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { 
            connection_reset: Arc::new(Notify::new())
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
        mut request_tx: Receiver<Request<T>>,
    ) where
        P: Parser<M> + Send + 'static,
        R: Router<M> + Send + 'static,
        M: Send + 'static,
        T: Serialize + Send + 'static,
    {
        let parser = Arc::new(parser);
        let router = Arc::new(router);
        let retry_interval = Duration::from_secs(1);

        loop {
            tracing::info!("starting ws connection @ {url}");
            match Session::new(url, parser.clone(), router.clone()).await {
                Ok(mut session) => session.run(&mut request_tx).await,
                Err(e) => println!("failed to start ws session: {e}"),
            }

            // notify that the connection died
            self.connection_reset.notify_waiters();

            tracing::info!(
                "ws @ {url} disconnected, reconnecting in {} secs...",
                retry_interval.as_secs()
            );

            tokio::time::sleep(retry_interval).await;
        }
    }
}
