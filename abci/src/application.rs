//! ABCI application interface.

use tenderdash_proto::abci::{ExecTxResult, ValidatorSetUpdate};
use tracing::{debug, error};

use crate::proto::{
    abci,
    abci::{request, response},
};

/// An ABCI application.
pub trait Application {
    /// Echo back the same message as provided in the request.
    fn echo(
        &self,
        request: abci::RequestEcho,
    ) -> Result<abci::ResponseEcho, abci::ResponseException> {
        Ok(abci::ResponseEcho {
            message: request.message,
        })
    }

    /// Signals that messages queued on the client should be flushed to the
    /// server.
    fn flush(
        &self,
        _request: abci::RequestFlush,
    ) -> Result<abci::ResponseFlush, abci::ResponseException> {
        Ok(Default::default())
    }

    /// Provide information about the ABCI application.
    fn info(
        &self,
        request: abci::RequestInfo,
    ) -> Result<abci::ResponseInfo, abci::ResponseException> {
        if !check_version(&request.abci_version) {
            return Err(abci::ResponseException {
                error: format!(
                    "version mismatch: tenderdash {} vs our {}",
                    request.version,
                    crate::proto::ABCI_VERSION
                ),
            });
        }

        Ok(Default::default())
    }

    /// Called once upon genesis.
    fn init_chain(
        &self,
        _request: abci::RequestInitChain,
    ) -> Result<abci::ResponseInitChain, abci::ResponseException> {
        Ok(Default::default())
    }

    /// Query the application for data at the current or past height.
    fn query(
        &self,
        _request: abci::RequestQuery,
    ) -> Result<abci::ResponseQuery, abci::ResponseException> {
        Ok(Default::default())
    }

    /// Check the given transaction before putting it into the local mempool.
    fn check_tx(
        &self,
        _request: abci::RequestCheckTx,
    ) -> Result<abci::ResponseCheckTx, abci::ResponseException> {
        Ok(Default::default())
    }

    /// Used during state sync to discover available snapshots on peers.
    fn list_snapshots(
        &self,
        _request: abci::RequestListSnapshots,
    ) -> Result<abci::ResponseListSnapshots, abci::ResponseException> {
        Ok(Default::default())
    }

    /// Called when bootstrapping the node using state sync.
    fn offer_snapshot(
        &self,
        _request: abci::RequestOfferSnapshot,
    ) -> Result<abci::ResponseOfferSnapshot, abci::ResponseException> {
        Ok(Default::default())
    }

    /// Used during state sync to retrieve chunks of snapshots from peers.
    fn load_snapshot_chunk(
        &self,
        _request: abci::RequestLoadSnapshotChunk,
    ) -> Result<abci::ResponseLoadSnapshotChunk, abci::ResponseException> {
        Ok(Default::default())
    }

    /// Apply the given snapshot chunk to the application's state.
    fn apply_snapshot_chunk(
        &self,
        _request: abci::RequestApplySnapshotChunk,
    ) -> Result<abci::ResponseApplySnapshotChunk, abci::ResponseException> {
        Ok(Default::default())
    }

    fn finalize_snapshot(
        &self,
        _request: abci::RequestFinalizeSnapshot,
    ) -> Result<abci::ResponseFinalizeSnapshot, abci::ResponseException> {
        Ok(Default::default())
    }

    fn extend_vote(
        &self,
        _request: abci::RequestExtendVote,
    ) -> Result<abci::ResponseExtendVote, abci::ResponseException> {
        Ok(Default::default())
    }

    fn finalize_block(
        &self,
        _request: abci::RequestFinalizeBlock,
    ) -> Result<abci::ResponseFinalizeBlock, abci::ResponseException> {
        Ok(Default::default())
    }

    fn prepare_proposal(
        &self,
        _request: abci::RequestPrepareProposal,
    ) -> Result<abci::ResponsePrepareProposal, abci::ResponseException> {
        Ok(Default::default())
    }

    fn process_proposal(
        &self,
        _request: abci::RequestProcessProposal,
    ) -> Result<abci::ResponseProcessProposal, abci::ResponseException> {
        Ok(Default::default())
    }

    fn verify_vote_extension(
        &self,
        _request: abci::RequestVerifyVoteExtension,
    ) -> Result<abci::ResponseVerifyVoteExtension, abci::ResponseException> {
        Ok(Default::default())
    }
}

pub trait RequestDispatcher {
    /// Executes the relevant application method based on the type of the
    /// request, and produces the corresponding response.
    ///
    /// `RequestDispatcher` can indicate that it will no longer process new
    /// requests by returning `None` variant.
    fn handle(&self, request: abci::Request) -> Option<abci::Response>;
}

// Implement `RequestDispatcher` for all `Application`s.
impl<A: Application> RequestDispatcher for A {
    fn handle(&self, request: abci::Request) -> Option<abci::Response> {
        #[cfg(feature = "tracing-span")]
        let _span = super::tracing_span::span(request.clone().value?);
        tracing::trace!(?request, "received ABCI request");

        let response: response::Value = match request.value? {
            request::Value::Echo(req) => self.echo(req).map(|v| v.into()),
            request::Value::Flush(req) => self.flush(req).map(|v| v.into()),
            request::Value::Info(req) => self.info(req).map(|v| v.into()),
            request::Value::InitChain(req) => self.init_chain(req).map(|v| v.into()),
            request::Value::Query(req) => self.query(req).map(|v| v.into()),
            request::Value::CheckTx(req) => self.check_tx(req).map(|v| v.into()),
            request::Value::OfferSnapshot(req) => self.offer_snapshot(req).map(|v| v.into()),
            request::Value::LoadSnapshotChunk(req) => {
                self.load_snapshot_chunk(req).map(|v| v.into())
            },
            request::Value::ApplySnapshotChunk(req) => {
                self.apply_snapshot_chunk(req).map(|v| v.into())
            },
            request::Value::FinalizeSnapshot(req) => self.finalize_snapshot(req).map(|v| v.into()),
            request::Value::ListSnapshots(req) => self.list_snapshots(req).map(|v| v.into()),
            request::Value::PrepareProposal(req) => self.prepare_proposal(req).map(|v| v.into()),
            request::Value::ProcessProposal(req) => self.process_proposal(req).map(|v| v.into()),
            request::Value::FinalizeBlock(req) => self.finalize_block(req).map(|v| v.into()),
            request::Value::ExtendVote(req) => self.extend_vote(req).map(|v| v.into()),
            request::Value::VerifyVoteExtension(req) => {
                self.verify_vote_extension(req).map(|v| v.into())
            },
        }
        .unwrap_or_else(|e| e.into());

        if let response::Value::Exception(_) = response {
            tracing::error!(?response, "sending ABCI exception");
        } else {
            let response_log = serialize_response_for_logging(&response);
            tracing::trace!(?response_log, "sending ABCI response");
        };

        Some(abci::Response {
            value: Some(response),
        })
    }
}

/// Serialize message for logging.
///
/// This macro is used to serialize the message for logging.
/// When `serde` feature is enabled, it uses `serde_json`, otherwise, it uses
/// `format!` macro.
macro_rules! serialize {
    ($($key:expr => $value:expr),* $(,)?) => {
        {
            #[cfg(feature = "serde")]
            {
                serde_json::json!({ $($key: $value),* }).to_string()
            }

            #[cfg(not(feature = "serde"))]
            {
                format!(stringify!($($key " {:?}",)*), $($value,)*)
            }
        }
    };
}

fn serialize_response_for_logging(response: &response::Value) -> String {
    match response {
        response::Value::PrepareProposal(response) => {
            let tx_records_hex: Vec<String> = response
                .tx_records
                .iter()
                .map(|tx_record| {
                    // Convert each byte array in tx_record to hex string
                    let tx_hex = hex::encode(&tx_record.tx);
                    serialize!(
                        "action" => tx_record.action, // Adjust according to actual fields
                        "tx" => tx_hex,
                    )
                    .to_string()
                })
                .collect();

            let app_hash_hex = hex::encode(&response.app_hash);

            let tx_results_hex: Vec<String> = exec_tx_results_to_string(&response.tx_results);

            let consensus_params = format!("{:?}", response.consensus_param_updates);

            let validator_set_update =
                validator_set_update_to_string(response.validator_set_update.as_ref());

            serialize!(
                "tx_records" => tx_records_hex,
                "app_hash" => app_hash_hex,
                "tx_results" => tx_results_hex,
                "consensus_param_updates" => consensus_params,
                "core_chain_lock_update" => response.core_chain_lock_update,
                "validator_set_update" => validator_set_update,
            )
            .to_string()
        },
        response::Value::ProcessProposal(response) => {
            let status_string = match response.status {
                0 => "Unknown",
                1 => "Accepted",
                2 => "Rejected",
                _ => "Unknown(too high)",
            };

            let app_hash_hex = hex::encode(&response.app_hash);

            let tx_results_hex: Vec<String> = exec_tx_results_to_string(&response.tx_results);

            let consensus_params = format!("{:?}", response.consensus_param_updates);

            let validator_set_update =
                validator_set_update_to_string(response.validator_set_update.as_ref());

            serialize!(
                "status" => status_string,
                "app_hash" => app_hash_hex,
                "tx_results" => tx_results_hex,
                "consensus_param_updates" => consensus_params,
                "validator_set_update" => validator_set_update,
            )
            .to_string()
        },

        value => format!("{:?}", value),
    }
}

fn exec_tx_results_to_string(tx_results: &[ExecTxResult]) -> Vec<String> {
    tx_results
        .iter()
        .map(|tx_result| {
            let data_hex = hex::encode(&tx_result.data);

            // Assuming `Event` is another complex type, you would serialize it similarly.
            // Here, we'll just represent events as an array of placeholders. You should
            // replace this with the actual serialization of `Event`.
            let events_serialized = format!("{:?}", tx_result.events);

            serialize!(
                "code" => tx_result.code,
                "data" =>data_hex,
                "log" => tx_result.log,
                "info" => tx_result.info,
                "gas_used" => tx_result.gas_used,
                "events" => events_serialized,
                "codespace" => tx_result.codespace,
            )
            .to_string()
        })
        .collect()
}

/// Serialize `ValidatorSetUpdate` to string for logging.
fn validator_set_update_to_string(validator_set_update: Option<&ValidatorSetUpdate>) -> String {
    validator_set_update
        .as_ref()
        .map(|validator_set_update| {
            let quorum_hash_hex = hex::encode(&validator_set_update.quorum_hash);

            let validator_updates_string: Vec<String> = validator_set_update
                .validator_updates
                .iter()
                .map(|validator_update| {
                    let pro_tx_hash_hex = hex::encode(&validator_update.pro_tx_hash);
                    serialize!(
                        "pub_key" => validator_update.pub_key,
                        "power" => validator_update.power,
                        "pro_tx_hash" => pro_tx_hash_hex,
                        "node_address" => validator_update.node_address,
                    )
                    .to_string()
                })
                .collect();
            serialize!(
                "validator_updates" => validator_updates_string,
                "threshold_public_key" => validator_set_update.threshold_public_key,
                "quorum_hash" => quorum_hash_hex,
            )
            .to_string()
        })
        .unwrap_or("None".to_string())
}

/// Check if ABCI version sent by Tenderdash matches version of linked protobuf
/// data objects.
///
/// You should use this function inside [Application::info()] handler, to ensure
/// that the protocol versions match. Match is determined based on Semantic
/// Versioning rules, as defined for '^' operator.
///
/// ## Examples
///
/// ### Using `check_version` in `Application::info` handler
///
/// ```should_panic
/// use tenderdash_abci::{check_version, Application};
/// use tenderdash_abci::proto::abci::{RequestInfo, ResponseInfo, ResponseException};
///
/// # let request = RequestInfo{
/// #  abci_version: String::from("108.234.356"),
/// #  ..Default::default()
/// # };
/// struct AbciApp{}
///
/// impl tenderdash_abci::Application for AbciApp {
///   fn info(&self, request: RequestInfo) -> Result<ResponseInfo, ResponseException> {
///     if !check_version(&request.abci_version) {
///       panic!("abci version mismatch");
///     }
///     Ok(Default::default())
///   }
/// }
///
/// # let app = AbciApp{};
/// # app.info(request);
/// ```
pub fn check_version(tenderdash_version: &str) -> bool {
    match_versions(tenderdash_version, tenderdash_proto::ABCI_VERSION)
}

/// Check if Tenderdash provides ABCI interface compatible with our library.
///
/// Tenderdash is compatible if its abci version matches the abci version of
/// linked protobuf data objects, eg. version provided in
/// `rs_tenderdash_abci_version` argument. The PATCH level can be ignored, as is
/// should be backwards-compatible.
///
/// For example, Tenderdash abci version `1.23.2` should work with
/// rs-tenderdash-abci linked with abci version `1.23.1` and `1.22.1`, but not
/// with `1.24.1` or `0.23.1`.
fn match_versions(tenderdash_version: &str, rs_tenderdash_abci_version: &str) -> bool {
    let rs_tenderdash_abci_version = semver::Version::parse(rs_tenderdash_abci_version)
        .expect("cannot parse protobuf library version");
    let tenderdash_version =
        semver::Version::parse(tenderdash_version).expect("cannot parse tenderdash version");

    let requirement = match rs_tenderdash_abci_version.pre.as_str() {
        "" => format!(
            "^{}.{}",
            rs_tenderdash_abci_version.major, rs_tenderdash_abci_version.minor
        ),
        pre => format!(
            "^{}.{}.0-{}",
            rs_tenderdash_abci_version.major, rs_tenderdash_abci_version.minor, pre
        ),
    };

    let matcher = semver::VersionReq::parse(&requirement).expect("cannot parse tenderdash version");

    match matcher.matches(&tenderdash_version) {
        true => {
            debug!(
                "version match(rs-tenderdash-abci proto version: {}), tenderdash server proto version {} = {}",
                rs_tenderdash_abci_version, tenderdash_version, requirement
            );
            true
        },
        false => {
            error!(
                "version mismatch(rs-tenderdash-abci proto version: {}), tenderdash server proto version {} != {}",
                rs_tenderdash_abci_version, tenderdash_version, requirement
            );
            false
        },
    }
}

#[cfg(test)]
mod tests {
    use super::match_versions;

    fn setup_logs() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new("trace"))
            .try_init()
            .ok();
    }

    /// test_versions! {} (td_version, our_version, expected); }
    // Test if various combinations of versions match
    //
    // ## Arguments
    //
    // * `td_version` - Tenderdash version, as returned by the Tenderdash
    // * `our_version` - our version - version of rs-tenderdash-abci library
    // * `expected` - expected result - true or false
    //
    macro_rules! test_versions {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                setup_logs();
                let (td, our, expect) = $value;
                assert_eq!(match_versions(td, our),expect,
                    "tenderdash version: {}, rs-tenderdash-abci version: {}, expect: {}", td, our,expect);
            }
        )*
        }
    }

    test_versions! {
        // rs-tenderdash-abci should be able to connect to any Tenderdash that is backwards-compatible
        // It means that:
        // * MAJOR of Tenderdash must match MAJOR of rs-tenderdash-abci
        // * MINOR of Tenderdash must be greater or equal to MINOR of rs-tenderdash-abci
        // * PATCH of Tenderdash can be anything

        // MAJOR 0

        // vesions match
        test_major_0: ("0.23.1", "0.23.1", true),
        // tenderdash is newer than our library, but it's backwards-compatible
        test_major_0_old_minor: ("0.23.1", "0.22.1", false),
        // tenderdash patch level is higher than ours; it should not matter
        test_major_0_new_patch: ("0.23.2", "0.23.1", true),
        // tenderdash patch level is lower than ours; it should not matter
        test_major_0_old_patch: ("0.23.0", "0.23.1", true),
        // tenderdash is older than our library, it should not match
        test_major_0_new_minor: ("0.23.1", "0.24.1", false),
        test_major_0_new_major: ("0.23.1", "1.23.1", false),

        // MAJOR 1

        test_major_1: ("1.23.1", "1.23.1", true),
        // tenderdash is newer than our library, but it's backwards-compatible
        test_major_1_old_minor: ("1.23.1", "1.22.1", true),
        // tenderdash patch level is higher than ours; it should not matter
        test_major_1_new_patch: ("1.23.2", "1.23.1", true),
        // tenderdash patch level is lower than ours; it should not matter
        test_major_1_old_patch: ("1.23.0", "1.23.1", true),
        // tenderdash is older than our library, it should not match
        test_major_1_new_minor: ("1.23.1", "1.24.1", false),
        test_major_1_old_major: ("1.23.1", "0.23.1", false),

        test_dev_td_newer: ("0.1.2-dev.1", "0.1.0", false),
        test_dev_equal: ("0.1.0-dev.1","0.1.0-dev.1",true),
        test_dev_our_newer_dev: ("0.1.0-dev.1", "0.1.0-dev.2",false),
    }
}
