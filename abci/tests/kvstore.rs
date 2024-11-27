mod common;

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
    sync::{RwLock, RwLockWriteGuard},
};

use bincode::{Decode, Encode};
use blake2::{
    digest::{consts::U32, FixedOutput},
    Blake2b, Digest,
};
use lazy_static::lazy_static;
use proto::abci::{self, ResponseException};
use tenderdash_abci::{check_version, proto, Application, CancellationToken};
use tracing::error;
use tracing_subscriber::filter::LevelFilter;

const SOCKET: &str = "/tmp/abci.sock";

lazy_static! {
    static ref CANCEL_TOKEN: CancellationToken = CancellationToken::new();
}

#[cfg(feature = "docker-tests")]
#[cfg(feature = "unix")]
#[test]
fn test_kvstore() {
    use std::{fs, os::unix::prelude::PermissionsExt};

    use tenderdash_abci::ServerBuilder;
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    let kvstore = RwLock::new(KVStore::new());
    let abci_app = KVStoreABCI::new(&kvstore);

    // Alter kvstore's state to make some work for Tenderdash:
    kvstore
        .write()
        .expect("kvstore lock is poisoned")
        .pending_operations = [Operation::Insert {
        key: "ayy".to_owned(),
        value: "lmao".to_owned(),
    }]
    .into_iter()
    .collect();

    let mut state_reference = BTreeMap::new();
    state_reference.insert("ayy".to_owned(), "lmao".to_owned());

    let bind_address = format!("unix://{}", SOCKET);

    let cancel = CANCEL_TOKEN.clone();
    let server = ServerBuilder::new(abci_app, &bind_address)
        .with_cancel_token(cancel)
        .build()
        .expect("server failed");

    let perms = fs::Permissions::from_mode(0o777);
    fs::set_permissions(SOCKET, perms).expect("set perms");

    let socket_uri = bind_address.to_string();
    let _td = common::docker::TenderdashDocker::new("tenderdash", None, &socket_uri);

    let next_client = server.next_client();
    tracing::debug!(?next_client, "next client");
    assert!(matches!(
        next_client,
        Err(tenderdash_abci::Error::Cancelled())
    ));
    drop(server);

    let kvstore_app = kvstore.into_inner().expect("kvstore lock is poisoned");
    assert_eq!(kvstore_app.persisted_state, state_reference);
    assert_eq!(kvstore_app.last_block_height, 1);
}

/// An example storage.
///
/// For clarity it separates commited data (application data with associated
/// block height) and uncommited data. Tenderdash interaction details are
/// factored out as much as possible.
#[derive(Debug, Default)]
pub(crate) struct KVStore {
    persisted_state: BTreeMap<String, String>,
    last_block_height: u32,
    pub(crate) pending_operations: BTreeSet<Operation>,
}

impl KVStore {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn commit(&mut self) {
        let pending_operations = std::mem::take(&mut self.pending_operations);
        pending_operations
            .into_iter()
            .for_each(|op| op.apply(&mut self.persisted_state));
        self.last_block_height += 1;
    }

    pub(crate) fn calculate_uncommited_state_hash(&self) -> [u8; 32] {
        let mut temp_state = self.persisted_state.clone();
        self.pending_operations
            .iter()
            .cloned()
            .for_each(|op| op.apply(&mut temp_state));

        simple_map_hash(&temp_state)
    }

    pub(crate) fn calculate_persisted_state_hash(&self) -> [u8; 32] {
        simple_map_hash(&self.persisted_state)
    }

    pub(crate) fn last_block_height(&self) -> u32 {
        self.last_block_height
    }
}

fn simple_map_hash<'a, K, V>(map: impl IntoIterator<Item = (&'a K, &'a V)>) -> [u8; 32]
where
    K: AsRef<[u8]> + 'a,
    V: AsRef<[u8]> + 'a,
{
    let mut hasher: Blake2b<U32> = Digest::new();
    map.into_iter()
        .map(|(k, v)| (k.as_ref(), v.as_ref()))
        .for_each(|(k, v)| {
            hasher.update(k.len().to_ne_bytes());
            hasher.update(k);
            hasher.update(v.len().to_ne_bytes());
            hasher.update(v);
        });
    hasher.finalize_fixed().into()
}

#[derive(Debug, Clone, Encode, Decode, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) enum Operation {
    Insert { key: String, value: String },
    Delete(String),
}

impl Operation {
    fn apply(self, map: &mut BTreeMap<String, String>) {
        match self {
            Operation::Insert { key, value } => {
                map.insert(key, value);
            },
            Operation::Delete(key) => {
                map.remove(&key);
            },
        }
    }
}

#[derive(Debug)]
pub(crate) struct KVStoreABCI<'a> {
    kvstore: &'a RwLock<KVStore>,
}

impl<'a> KVStoreABCI<'a> {
    pub(crate) fn new(kvstore: &'a RwLock<KVStore>) -> Self {
        KVStoreABCI { kvstore }
    }

    fn lock_kvstore(&self) -> RwLockWriteGuard<KVStore> {
        self.kvstore.write().expect("kvstore lock is poisoned")
    }
}

impl Application for KVStoreABCI<'_> {
    fn info(
        &self,
        request: proto::abci::RequestInfo,
    ) -> Result<abci::ResponseInfo, abci::ResponseException> {
        let kvstore_lock = self.lock_kvstore();

        if !check_version(&request.abci_version) {
            return Err(abci::ResponseException {
                error: format!(
                    "version mismatch: tenderdash {} vs our {}",
                    request.version,
                    crate::proto::ABCI_VERSION
                ),
            });
        }

        Ok(abci::ResponseInfo {
            data: "kvstore-rs".to_string(),
            version: "0.1.0".to_string(),
            app_version: 1,
            last_block_height: kvstore_lock.last_block_height() as i64,
            last_block_app_hash: kvstore_lock.calculate_persisted_state_hash().to_vec(),
        })
    }

    fn init_chain(
        &self,
        _request: proto::abci::RequestInitChain,
    ) -> Result<abci::ResponseInitChain, abci::ResponseException> {
        // Do nothing special as we're working with a simple example
        Ok(abci::ResponseInitChain {
            app_hash: self
                .lock_kvstore()
                .calculate_persisted_state_hash()
                .to_vec(),
            ..Default::default()
        })
    }

    fn prepare_proposal(
        &self,
        request: proto::abci::RequestPrepareProposal,
    ) -> Result<abci::ResponsePrepareProposal, abci::ResponseException> {
        let mut kvstore_lock = self.lock_kvstore();
        assert_block_height(request.height, &kvstore_lock);

        // Decode proposed transactions
        let Some(td_proposed_transactions) = request
            .txs
            .into_iter()
            .map(decode_transaction)
            .collect::<Option<BTreeSet<Operation>>>()
        else {
            error!("Cannot decode transactions");
            return Err(abci::ResponseException {
                error: "cannot decode transactions".to_string(),
            });
        };

        // Mark transactions that should be added to the proposed transactions
        let pending_local_transactions = &kvstore_lock.pending_operations;
        let node_proposed_transactions =
            pending_local_transactions.difference(&td_proposed_transactions);

        let tx_records_encoded: Option<Vec<proto::abci::TxRecord>> = td_proposed_transactions
            .iter()
            .map(|tx| {
                Some(proto::abci::TxRecord {
                    tx: bincode::encode_to_vec(tx, bincode::config::standard()).ok()?,
                    action: proto::abci::tx_record::TxAction::Unmodified.into(),
                })
            })
            .chain(node_proposed_transactions.map(|tx| {
                Some(proto::abci::TxRecord {
                    tx: bincode::encode_to_vec(tx, bincode::config::standard()).ok()?,
                    action: proto::abci::tx_record::TxAction::Added.into(),
                })
            }))
            .collect();

        let Some(tx_records) = tx_records_encoded else {
            error!("cannot encode transactions");
            return Err(ResponseException {
                error: "cannot encode transactions".to_string(),
            });
        };

        // Put both local and proposed transactions into staging area
        let joined_transactions = pending_local_transactions.union(&td_proposed_transactions);

        let tx_results = tx_results_accept(joined_transactions.clone().count());

        kvstore_lock.pending_operations = joined_transactions.cloned().collect();

        Ok(abci::ResponsePrepareProposal {
            tx_records,
            tx_results,
            app_hash: kvstore_lock.calculate_uncommited_state_hash().to_vec(),
            app_version: 1,
            ..Default::default()
        })
    }

    fn process_proposal(
        &self,
        request: proto::abci::RequestProcessProposal,
    ) -> Result<abci::ResponseProcessProposal, abci::ResponseException> {
        let mut kvstore_lock = self.lock_kvstore();

        assert_block_height(request.height, &kvstore_lock);

        // Decode proposed transactions
        let Some(td_proposed_transactions) = request
            .txs
            .into_iter()
            .map(decode_transaction)
            .collect::<Option<BTreeSet<Operation>>>()
        else {
            return Err(ResponseException {
                error: "cannot decode transactions".to_string(),
            });
        };

        let tx_results = tx_results_accept(td_proposed_transactions.len());

        // For simplicity just agree with proposed transactions:
        kvstore_lock.pending_operations = td_proposed_transactions;

        let app_hash = kvstore_lock.calculate_uncommited_state_hash().to_vec();

        Ok(abci::ResponseProcessProposal {
            status: abci::response_process_proposal::ProposalStatus::Accept.into(),
            tx_results,
            app_hash,
            ..Default::default()
        })
    }

    fn extend_vote(
        &self,
        request: proto::abci::RequestExtendVote,
    ) -> Result<abci::ResponseExtendVote, abci::ResponseException> {
        // request.height
        let height = request.height.to_be_bytes().to_vec();

        Ok(abci::ResponseExtendVote {
            vote_extensions: vec![proto::abci::ExtendVoteExtension {
                r#type: proto::types::VoteExtensionType::ThresholdRecover as i32,
                extension: height,
                sign_request_id: None,
            }],
        })
    }

    fn verify_vote_extension(
        &self,
        request: proto::abci::RequestVerifyVoteExtension,
    ) -> Result<abci::ResponseVerifyVoteExtension, abci::ResponseException> {
        let height = request.height.to_be_bytes().to_vec();
        let ext = request
            .vote_extensions
            .first()
            .expect("missing vote extension");

        let status = match ext.extension == height {
            true => abci::response_verify_vote_extension::VerifyStatus::Accept as i32,
            false => abci::response_verify_vote_extension::VerifyStatus::Reject as i32,
        };

        Ok(abci::ResponseVerifyVoteExtension { status })
    }

    fn finalize_block(
        &self,
        request: proto::abci::RequestFinalizeBlock,
    ) -> Result<abci::ResponseFinalizeBlock, abci::ResponseException> {
        let mut kvstore_lock = self.lock_kvstore();

        assert_block_height(request.height, &kvstore_lock);

        kvstore_lock.commit();

        // we want to end the test and shutdown the server
        let cancel = CANCEL_TOKEN.clone();
        cancel.cancel();

        Ok(Default::default())
    }
}

fn decode_transaction(bytes: impl AsRef<[u8]>) -> Option<Operation> {
    bincode::decode_from_slice(bytes.as_ref(), bincode::config::standard())
        .map(|decoded| decoded.0)
        .ok()
}

fn tx_results_accept(len: usize) -> Vec<proto::abci::ExecTxResult> {
    let mut tx_results = Vec::<proto::abci::ExecTxResult>::new();

    for _ in 0..len {
        tx_results.push(proto::abci::ExecTxResult {
            code: 0,
            ..Default::default()
        });
    }

    tx_results
}

/// Check if the node is up to date and ready for the next block
fn assert_block_height(height: i64, kvstore: &impl Deref<Target = KVStore>) {
    if height != (kvstore.last_block_height() + 1) as i64 {
        error!(
            "Proposed block height is {} when kvstore is on {}",
            height,
            kvstore.last_block_height()
        );
        panic!("non-recoverable, aborting");
    }
}
