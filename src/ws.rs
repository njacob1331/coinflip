use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use kanal::AsyncReceiver;
use serde::Serialize;
use tokio::{net::TcpStream, task::JoinHandle};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use url::Url;

use crate::{
    session::Request,
    traits::{Parser, Router},
};

pub type Writer = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>;
pub type Reader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
pub type WsMessage = tokio_tungstenite::tungstenite::Message;

pub fn is_array(bytes: &[u8]) -> bool {
    matches!(bytes, [b'[', .., b']'])
}

pub struct Ws;

impl Ws {
    pub async fn connect(url: &str) -> Result<(Writer, Reader)> {
        let url = Url::parse(url).expect("invalid ws url: {url}");
        let (stream, _) = connect_async(url).await?;

        Ok(stream.split())
    }

    pub fn spawn_writer<T: Serialize + Send + 'static>(
        mut writer: Writer,
        rx: AsyncReceiver<Request<T>>,
    ) -> JoinHandle<Result<()>> {
        tokio::spawn(async move {
            while let Ok(request) = rx.recv().await {
                let string = sonic_rs::to_string(&request.into_msg())?;
                println!("{:#?}", string);
                writer.send(WsMessage::Text(string)).await?
            }

            Ok(())
        })
    }

    pub fn spawn_reader<P, M, R>(
        mut reader: Reader,
        parser: Arc<P>,
        router: Arc<R>,
    ) -> JoinHandle<Result<()>>
    where
        P: Parser<M> + Send + 'static,
        R: Router<M> + Send + 'static,
        M: Send + 'static,
    {
        tokio::spawn(async move {
            while let Some(msg) = reader.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => match parser.parse(text.as_bytes()) {
                        Ok(msg) => {
                            if let Err(e) = router.route(msg).await {
                                return Err(anyhow!("ws reader send error: {e}"));
                            }
                        }
                        Err(e) => tracing::error!("failed to parse ws message: {e}"),
                    },
                    Ok(WsMessage::Binary(bytes)) => match parser.parse(&bytes) {
                        Ok(msg) => {
                            if let Err(e) = router.route(msg).await {
                                return Err(anyhow!("ws reader send error: {e}"));
                            }
                        }
                        Err(e) => tracing::error!("failed to parse ws message: {e}"),
                    },
                    Ok(WsMessage::Close(frame)) => {
                        return Err(anyhow!("ws closed by server: {frame:?}"));
                    }
                    Err(e) => {
                        return Err(anyhow!("ws reader error: {e}"));
                    }
                    _ => {}
                }
            }

            Ok(())
        })
    }
}
