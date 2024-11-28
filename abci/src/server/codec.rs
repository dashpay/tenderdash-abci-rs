//! Encoding/decoding mechanisms for ABCI requests and responses.
//!
//! Implements the [Tendermint Socket Protocol][tsp].
//!
//! [tsp]: https://github.com/tendermint/tendermint/blob/v0.34.x/spec/abci/client-server.md#tsp

use std::{fmt::Debug, sync::Arc};

use bytes::{Buf, BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use proto::abci::{Request, Response};
use tenderdash_proto::prost::{
    encoding::{decode_varint, encode_varint},
    Message,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
};
use tokio_util::{
    codec::{Decoder, Encoder, Framed},
    net::Listener,
};

use super::ServerRuntime;
use crate::{proto, CancellationToken, Error};

/// The maximum number of bytes we expect in a varint. We use this to check if
/// we're encountering a decoding error for a varint.
pub const MAX_VARINT_LENGTH: usize = 16;

pub struct Codec {
    request_rx: Receiver<Request>,
    response_tx: Sender<Response>,
}

impl<'a> Codec {
    pub(crate) fn new<L>(
        listener: Arc<Mutex<L>>,
        cancel: CancellationToken,
        runtime: &ServerRuntime,
    ) -> Self
    where
        L: Listener + Send + Sync + 'static,
        L::Addr: Send + Debug,
        L::Io: Send,
    {
        let (request_tx, request_rx) = mpsc::channel::<proto::abci::Request>(1);
        let (response_tx, response_rx) = mpsc::channel::<proto::abci::Response>(1);

        runtime
            .handle
            .spawn(Self::worker(listener, request_tx, response_rx, cancel));

        Self {
            request_rx,
            response_tx,
        }
    }

    /// Worker that bridges data between async streams and sync processing code.
    ///
    /// ## Error handling
    ///
    /// Any error will cause disconnect
    async fn worker<L>(
        listener: Arc<Mutex<L>>,
        request_tx: Sender<proto::abci::Request>,
        response_rx: Receiver<proto::abci::Response>,
        cancel: CancellationToken,
    ) where
        L: Listener + Send + Sync,
        L::Addr: Debug,
    {
        let mut listener = listener.lock().await;
        tracing::trace!("listening for new connection");

        let (stream, address) = tokio::select! {
            conn = listener.accept() => match conn {
                Ok(r) => r,
                Err(error) => {
                    tracing::error!(?error, "cannot accept connection");
                    cancel.cancel();
                    return;
                },
            },
            _ = cancel.cancelled() => return,
        };

        tracing::info!(?address, "accepted connection");

        let stream = Box::pin(stream);
        let codec = Framed::new(stream, Coder {});

        Self::process_worker_queues(codec, request_tx, response_rx, cancel).await;
    }

    /// Worker that moves messages between codec and requests and responses
    /// queues.
    ///
    /// Reads messages from ABCI Client from `codec`, sends them to Tenderdash
    /// via `request_tx`, receives Tenderdash responses from `response_rx`
    /// and forwards to
    ///
    /// ## Error handling
    ///
    /// On error, it cancels the `cancel` [CancellationToken] and exits.
    /// It causes `response_rx` to be closed.
    async fn process_worker_queues<L: AsyncRead + AsyncWrite + Unpin>(
        mut codec: Framed<L, Coder>,
        request_tx: Sender<proto::abci::Request>,
        mut response_rx: Receiver<proto::abci::Response>,
        cancel: CancellationToken,
    ) {
        loop {
            tokio::select! {
                // Only read next message if we have capacity in request_tx to process it.
                // Otherwise, we might block the codec worker on request_tx.send() and never
                // process the next message from the response_rx stream.
                request = codec.next(), if request_tx.capacity() > 0 => match request {
                    Some(Ok(i)) => {
                        if let Err(error) = request_tx.try_send(i) {
                            tracing::error!(?error, "unable to forward request for processing");
                            cancel.cancel();
                        }
                    },
                    Some(Err(error)) => {
                        tracing::error!(?error, "unable to parse request");
                        cancel.cancel();
                    },
                    None => {
                        tracing::warn!("client connection terminated");
                        cancel.cancel();
                    },
                },
                response = response_rx.recv() => match response{
                    Some(msg) => {
                        if let Err(error) =   codec.send(msg).await {
                            tracing::error!(?error, "unable to send response to tenderdash");
                            cancel.cancel();
                        }
                    },
                    None => {
                        tracing::warn!("client connection terminated");
                        cancel.cancel();
                    }
                },
                _ = cancel.cancelled() => {
                    tracing::debug!("codec worker shutting down");
                    return; // stop processing
                }
            }
        }
    }

    pub fn next(&mut self) -> Option<Request> {
        self.request_rx.blocking_recv()
    }

    pub fn send(&self, value: Response) -> Result<(), Error> {
        self.response_tx.blocking_send(value).map_err(|_| {
            // channel closed, `process_worker_queues` either errored or cancelled
            Error::Cancelled()
        })
    }
}

pub struct Coder;

impl Decoder for Coder {
    type Error = Error;
    type Item = proto::abci::Request;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let src_len = src.len();

        let mut tmp = src.clone().freeze();
        let encoded_len = match decode_varint(&mut tmp) {
            Ok(len) => len,
            // We've potentially only received a partial length delimiter
            Err(_) if src_len <= MAX_VARINT_LENGTH => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let remaining = tmp.remaining() as u64;
        if remaining < encoded_len {
            // We don't have enough data yet to decode the entire message
            Ok(None)
        } else {
            let delim_len = src_len - tmp.remaining();
            // We only advance the source buffer once we're sure we have enough
            // data to try to decode the result.
            src.advance(delim_len + (encoded_len as usize));

            let mut result_bytes = BytesMut::from(tmp.split_to(encoded_len as usize).as_ref());
            let res = proto::abci::Request::decode(&mut result_bytes)?;

            Ok(Some(res))
        }
    }
}

impl Encoder<proto::abci::Response> for Coder {
    type Error = Error;

    fn encode(
        &mut self,
        message: proto::abci::Response,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        let mut buf = BytesMut::new();
        message.encode(&mut buf)?;

        let buf = buf.freeze();
        encode_varint(buf.len() as u64, dst);
        dst.put(buf);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use tenderdash_proto::{abci, prost::Message};
    use tokio::{io::AsyncWriteExt, sync::mpsc};
    use tokio_util::sync::CancellationToken;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    /// Test if a bug in the codec receiving 2 requests without a response in
    /// between is fixed.
    async fn test_codec_msg_msg_resp() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_ansi(true)
            .try_init()
            .ok();

        let (request_tx, mut request_rx) = mpsc::channel::<abci::Request>(1);
        let (response_tx, response_rx) = mpsc::channel::<abci::Response>(1);
        let cancel = CancellationToken::new();

        let (mut client, server) = tokio::io::duplex(10240);

        let codec = tokio_util::codec::Framed::new(server, super::Coder {});

        let worker_cancel = cancel.clone();
        let hdl = tokio::spawn(super::Codec::process_worker_queues(
            codec,
            request_tx,
            response_rx,
            worker_cancel,
        ));

        // We send 2 requests over the wire
        for n_requests in 0..5 {
            let encoded = abci::Request {
                value: Some(abci::request::Value::Echo(abci::RequestEcho {
                    message: format!("hello {}", n_requests),
                })),
            }
            .encode_length_delimited_to_vec();

            client.write_all(&encoded).await.unwrap();
        }

        // Now, wait till the codec has processed the requests
        // The bug we fixed was that the codec would not process the second request
        // until a response was sent.
        // If the bug is still present, the test should report error here.
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Then, we read one request
        tracing::debug!("MAIN THREAD: reading request 1");
        request_rx.recv().await.expect("dequeue request 1");
        tracing::debug!("MAIN THREAD: dequeued request 1");

        //  Then, we send a response
        tracing::debug!("MAIN THREAD: sending response 1");
        response_tx
            .send(abci::Response {
                value: Some(abci::response::Value::Echo(abci::ResponseEcho {
                    message: "hello".to_string(),
                })),
            })
            .await
            .expect("enqueue response 1");
        tracing::debug!("MAIN THREAD: enqueued response 1");

        // Then, we read second request
        tracing::debug!("MAIN THREAD: reading request 2");
        request_rx.recv().await.expect("dequeue request 2");
        tracing::debug!("MAIN THREAD: dequeued request 2");

        // Success :)

        // Cleanup
        cancel.cancel();
        hdl.await.unwrap();
    }
}
