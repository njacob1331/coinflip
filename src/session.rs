use crate::{
    common::{Parser, Router},
    ws::Ws,
};
use anyhow::Result;
use kanal::AsyncSender;
use std::{collections::BinaryHeap, sync::Arc, time::Duration};
use tokio::{
    sync::{
        Notify,
        mpsc::{Receiver, channel},
    },
    task::JoinHandle,
};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Request {
    LowPriority(String),
    HighPriority(String),
}

impl Request {
    pub fn into_msg(self) -> String {
        match self {
            Request::HighPriority(msg) => msg,
            Request::LowPriority(msg) => msg,
        }
    }
}

struct Session {
    session_tx: AsyncSender<String>,
    pq: BinaryHeap<Request>,
    writer_handle: JoinHandle<Result<()>>,
    reader_handle: JoinHandle<Result<()>>,
}

impl Session {
    async fn new<P, M, R>(url: &str, parser: Arc<P>, router: Arc<R>) -> Result<Self>
    where
        P: Parser<M> + Send + 'static,
        R: Router<M> + Send + 'static,
        M: Send + 'static,
    {
        let (writer, reader) = Ws::connect(url).await?;
        let (session_tx, session_rx) = kanal::bounded_async::<String>(8);

        Ok(Self {
            session_tx,
            pq: BinaryHeap::with_capacity(100),
            writer_handle: Ws::spawn_writer(writer, session_rx),
            reader_handle: Ws::spawn_reader(reader, parser, router),
        })
    }

    async fn forward_from_queue(&mut self) -> Result<()> {
        if let Some(req) = self.pq.pop() {
            self.session_tx.send(req.into_msg()).await?;
        }
        Ok(())
    }

    fn teardown(&mut self) {
        self.writer_handle.abort();
        self.reader_handle.abort();
    }

    async fn run(&mut self, request_rx: &mut Receiver<Request>) {
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

pub struct SessionManager<P, M, R>
where
    P: Parser<M> + Send + 'static,
    R: Router<M> + Send + 'static,
    M: Send + 'static,
{
    connection_reset: Arc<Notify>,
    request_rx: Receiver<Request>,
    parser: Arc<P>,
    router: Arc<R>,
    _marker: std::marker::PhantomData<M>,
}

impl<P, M, R> SessionManager<P, M, R>
where
    P: Parser<M> + Send + 'static,
    R: Router<M> + Send + 'static,
    M: Send + 'static,
{
    pub fn new(
        connection_reset: Arc<Notify>,
        request_rx: Receiver<Request>,
        parser: P,
        router: R,
    ) -> Self {
        Self {
            connection_reset,
            request_rx,
            parser: Arc::new(parser),
            router: Arc::new(router),
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn run(&mut self, url: &str) {
        let retry_interval = Duration::from_secs(1);

        loop {
            match Session::new(url, self.parser.clone(), self.router.clone()).await {
                Ok(mut session) => session.run(&mut self.request_rx).await,
                Err(e) => println!("failed to start ws session: {e}"),
            }

            // notify that the connection died
            self.connection_reset.notify_waiters();

            println!(
                "ws disconnected, reconnecting in {} secs...",
                retry_interval.as_secs()
            );

            tokio::time::sleep(retry_interval).await;
        }
    }
}
