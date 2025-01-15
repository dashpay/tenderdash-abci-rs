use tenderdash_proto::abci::request::Value;
use tracing::Level;

const SPAN_NAME: &str = "abci";
const LEVEL: Level = Level::ERROR;

macro_rules! block_span {
    ($request: expr, $endpoint:expr, $request_id:expr) => {
        tracing::span!(
            LEVEL,
            SPAN_NAME,
            endpoint = $endpoint,
            request_id = $request_id,
            height = $request.height,
            round = $request.round
        )
    };
}
/// Creates a new span for tracing.
///
/// This function creates a new `tracing::span::EnteredSpan` based on the
/// provided request. It uses the request to determine the endpoint and includes
/// a unique request ID in the span.
///
/// The level of the span is set to ERROR, so it will be included on all log
/// levels.
///
/// # Arguments
///
/// * `request` - A value that can be converted into a `Value`. Depending on the
///   specific variant of `Value`, additional information like height, round, or
///   path might be included in the span.
///
/// # Returns
///
/// An entered span which represents an active or entered span state.
///
/// # Examples
///
/// ```
/// # use tenderdash_proto::abci::{RequestInfo, request};
/// # use tenderdash_abci::tracing_span::span;
///
/// let request = request::Value::Info(RequestInfo::default());
/// let span = span(request);
/// ```
pub fn span<T>(request: T) -> tracing::span::EnteredSpan
where
    T: Into<Value>,
{
    let value = request.into();

    let endpoint = abci_method_name(&value);
    let request_id = uuid::Uuid::new_v4().to_string();

    let span = match value {
        Value::Info(_r) => tracing::span!(LEVEL, SPAN_NAME, endpoint, request_id),
        Value::InitChain(_r) => {
            tracing::span!(LEVEL, SPAN_NAME, endpoint, request_id)
        },
        Value::PrepareProposal(r) => block_span!(r, endpoint, request_id),
        Value::ProcessProposal(r) => block_span!(r, endpoint, request_id),
        Value::ExtendVote(r) => block_span!(r, endpoint, request_id),
        Value::VerifyVoteExtension(r) => block_span!(r, endpoint, request_id),
        Value::FinalizeBlock(r) => block_span!(r, endpoint, request_id),
        Value::CheckTx(_r) => {
            tracing::span!(LEVEL, SPAN_NAME, endpoint, request_id)
        },
        Value::Query(r) => {
            tracing::span!(LEVEL, SPAN_NAME, endpoint, request_id, path = r.path)
        },
        _ => tracing::span!(LEVEL, SPAN_NAME, endpoint, request_id),
    };

    span.entered()
}

fn abci_method_name(request: &Value) -> String {
    match request {
        Value::ApplySnapshotChunk(_) => "ApplySnapshotChunk",
        Value::CheckTx(_) => "CheckTx",
        Value::Echo(_) => "Echo",
        Value::ExtendVote(_) => "ExtendVote",
        Value::FinalizeBlock(_) => "FinalizeBlock",
        Value::Flush(_) => "Flush",
        Value::Info(_) => "Info",
        Value::InitChain(_) => "InitChain",
        Value::ListSnapshots(_) => "ListSnapshots",
        Value::LoadSnapshotChunk(_) => "LoadSnapshotChunk",
        Value::FinalizeSnapshot(_) => "FinalizeSnapshot",
        Value::OfferSnapshot(_) => "OfferSnapshot",
        Value::PrepareProposal(_) => "PrepareProposal",
        Value::ProcessProposal(_) => "ProcessProposal",
        Value::Query(_) => "Query",
        Value::VerifyVoteExtension(_) => "VerifyVoteExtension",
    }
    .to_string()
}
