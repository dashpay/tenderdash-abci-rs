//! Generic ABCI server
#[cfg(feature = "tcp")]
use std::net::ToSocketAddrs;
use std::{fmt::Debug, sync::Arc};
#[cfg(feature = "unix")]
use std::{fs, path::Path};

#[cfg(feature = "tcp")]
use tokio::net::TcpListener;
#[cfg(feature = "unix")]
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tokio_util::net::Listener;
use tracing::info;

use super::{codec::Codec, Server, ServerRuntime};
use crate::{CancellationToken, Error, RequestDispatcher};

/// A TCP-based server for serving a specific ABCI application.
///
/// Only one incoming connection is handled at a time.
pub(super) struct GenericServer<App: RequestDispatcher, L: Listener> {
    app: App,
    listener: Arc<Mutex<L>>,
    cancel: CancellationToken,
    runtime: ServerRuntime,
}

impl<App: RequestDispatcher, L: Listener> GenericServer<App, L> {
    fn new(app: App, listener: L, cancel: CancellationToken, runtime: ServerRuntime) -> Self {
        Self {
            app,
            listener: Arc::new(Mutex::new(listener)),
            cancel,
            runtime,
        }
    }
}

#[cfg(feature = "tcp")]
impl<App: RequestDispatcher> GenericServer<App, TcpListener> {
    pub(super) fn bind<Addr>(
        app: App,
        addr: Addr,
        cancel: CancellationToken,
        runtime: ServerRuntime,
    ) -> Result<Self, Error>
    where
        Addr: ToSocketAddrs,
    {
        let std_listener = std::net::TcpListener::bind(addr)?;
        std_listener.set_nonblocking(true)?;
        let listener = TcpListener::from_std(std_listener)?;

        // let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        info!(
            "ABCI TCP server  {} with proto {} running at {}",
            env!("CARGO_PKG_VERSION"),
            tenderdash_proto::ABCI_VERSION,
            local_addr
        );

        let server = Self::new(app, listener, cancel, runtime);
        Ok(server)
    }
}

#[cfg(feature = "unix")]
impl<App: RequestDispatcher> GenericServer<App, UnixListener> {
    pub(super) fn bind<Addr>(
        app: App,
        addr: Addr,
        cancel: CancellationToken,
        runtime: ServerRuntime,
    ) -> Result<Self, Error>
    where
        Addr: AsRef<Path>,
    {
        let socket_file = addr.as_ref();
        fs::remove_file(socket_file).ok();

        let listener = UnixListener::bind(addr)?;

        // let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        info!(
            "ABCI Unix server {} with proto {} running at {:?}",
            env!("CARGO_PKG_VERSION"),
            tenderdash_proto::ABCI_VERSION,
            local_addr
        );

        let server = Self::new(app, listener, cancel, runtime);
        Ok(server)
    }
}

impl<App: RequestDispatcher, L: Listener> Server for GenericServer<App, L>
where
    L: Listener + Send + Sync + 'static,
    L::Addr: Send + Debug,
    L::Io: Send,
{
    fn next_client(&self) -> Result<(), Error> {
        // we create child token to stop the codec but not kill the app
        let cancel_token = self.cancel.child_token();
        let listener = Arc::clone(&self.listener);

        let mut codec = Codec::new(listener, cancel_token.clone(), &self.runtime);
        while !cancel_token.is_cancelled() {
            let Some(request) = codec.next() else {
                tracing::error!("client terminated stream");
                return Ok(());
            };

            let Some(response) = self.app.handle(request.clone()) else {
                // `RequestDispatcher` decided to stop receiving new requests:
                info!("ABCI Application is shutting down");
                return Ok(());
            };

            if let Some(crate::proto::abci::response::Value::Exception(ex)) = response.value.clone()
            {
                tracing::error!(error = ex.error, ?request, "error processing request")
            };

            codec.send(response)?;
        }

        Err(Error::Cancelled())
    }
}

impl<App: RequestDispatcher, L: Listener> Drop for GenericServer<App, L> {
    fn drop(&mut self) {
        tracing::debug!("ABCI server shut down")
    }
}
