use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use kanal::AsyncReceiver;
use tokio::{net::TcpStream, task::JoinHandle};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use url::Url;

use crate::traits::{Parser, Router};

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

    pub fn spawn_writer(mut writer: Writer, rx: AsyncReceiver<String>) -> JoinHandle<Result<()>> {
        // add resusable request string buffer
        // final serialization to String should occur right before sending the message
        // ex
        // let mut buf = String::with_capacity(capacity);
        // sonic_rs::to_writer(&mut buf, &request)?;
        
        tokio::spawn(async move {
            while let Ok(request) = rx.recv().await {
                writer.send(WsMessage::Text(request)).await?
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
