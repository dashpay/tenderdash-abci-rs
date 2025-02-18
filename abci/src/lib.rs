//! Tenderdash ABCI Application library.
//!
//! ABCI Application is responsible for storage and implementation of
//! application state.
//!
//! Use [[start_server]] to create a new server that will accept connections
//! from Tenderdash.
//!
//! Run [Server::handle_connection()] in a loop to handle incoming server
//! connections.
//!
//! Implement the [application::Application] trait with custom logic for
//! blockchain processing. Expect messages defined in [proto::abci] crate.

mod application;
#[cfg(feature = "server")]
mod server;

use std::io;

pub use application::{check_version, Application, RequestDispatcher};
#[allow(deprecated)]
#[cfg(feature = "server")]
pub use server::{start_server, CancellationToken, Server, ServerBuilder, ServerRuntime};
pub use tenderdash_proto as proto;
use tenderdash_proto::prost::{DecodeError, EncodeError};

#[cfg(feature = "crypto")]
mod merkle;
#[cfg(feature = "crypto")]
pub mod signatures;

#[cfg(feature = "tracing-span")]
/// Create tracing::Span for better logging
pub mod tracing_span;

/// Errors that may happen during protobuf communication
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("connection error")]
    Connection(#[from] io::Error),
    #[error("cannot decode protobuf message")]
    Decode(DecodeError),
    #[error("cannot encode protobuf message")]
    Encode(EncodeError),
    #[error("cannot create canonical message: {0}")]
    Canonical(String),
    #[error("server terminated")]
    Cancelled(),
    #[error("async runtime error")]
    Async(String),
}

// manually implemented due to no_std compatibility
impl From<EncodeError> for Error {
    fn from(error: EncodeError) -> Error {
        Error::Encode(error)
    }
}

// manually implemented due to no_std compatibility
impl From<DecodeError> for Error {
    fn from(error: DecodeError) -> Error {
        Error::Decode(error)
    }
}
