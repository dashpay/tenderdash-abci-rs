//! Tenderdash protobuf implementation

use std::fmt::Display;

// Requirements
pub const DEP_PROTOC_VERSION: f32 = 25.0;

/// Tenderdash repository URL.
pub const TENDERDASH_REPO: &str = "https://github.com/dashpay/tenderdash";

/// How to generate the protobuf files.
pub enum GenerationMode {
    /// Generate the files using `tonic` and put them into `tenderdash_grpc`
    /// module.
    GrpcServer,
    /// Generate minimal gRPC client using tonic, without transport
    /// implementation. Put them into `tenderdash_grpc_client` module.
    GrpcClient,
    /// Generate the files without `std` and put them into `tenderdash_nostd`
    /// module.
    NoStd,
}

impl GenerationMode {
    pub fn module_name(&self) -> String {
        match self {
            GenerationMode::GrpcServer => "tenderdash_grpc".to_string(),
            GenerationMode::GrpcClient => "tenderdash_grpc_client".to_string(),
            GenerationMode::NoStd => "tenderdash_nostd".to_string(),
        }
    }
}

impl Display for GenerationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode = match self {
            GenerationMode::GrpcServer => "grpc-client-server".to_string(),
            GenerationMode::GrpcClient => "grpc-client".to_string(),
            GenerationMode::NoStd => "nostd".to_string(),
        };
        write!(f, "{}", mode)
    }
}

// Commitish formats:
// Tag: v0.34.0-rc4
// Branch: master
// Commit ID (full length): d7d0ffea13c60c98b812d243ba5a2c375f341c15
// Set env variable TENDERDASH_COMMITISH to override the default.
pub(crate) const DEFAULT_TENDERDASH_COMMITISH: &str = "v0.10-dev";

/// Predefined custom attributes for message annotations
const PRIMITIVE_ENUM: &str = r#"#[derive(::num_derive::FromPrimitive, ::num_derive::ToPrimitive)]"#;
pub(crate) const SERIALIZED: &str =
    r#"#[cfg_attr(feature = "serde", derive(::serde::Deserialize, ::serde::Serialize))]"#;
const TYPE_TAG: &str = r#"#[cfg_attr(feature = "serde", serde(tag = "type", content = "value"))]"#;
/// Predefined custom attributes for field annotations
const QUOTED: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::from_str"))]"#;
const QUOTED_WITH_DEFAULT: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::from_str", default))]"#;
const DEFAULT: &str = r#"#[cfg_attr(feature = "serde", serde(default))]"#;
const HEXSTRING: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::bytes::hexstring"))]"#;
const BASE64STRING: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::bytes::base64string"))]"#;
const VEC_BASE64STRING: &str = r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::bytes::vec_base64string"))]"#;
const OPTIONAL: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::optional"))]"#;
// const BYTES_SKIP_IF_EMPTY: &str = r#"#[serde(skip_serializing_if =
// "bytes::Bytes::is_empty")]"#;
const DERIVE_FROM_FORWARD: &str = r#"#[from(forward)]"#;
const NULLABLEVECARRAY: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::txs"))]"#;
const NULLABLE: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::nullable"))]"#;
const ALIAS_POWER_QUOTED: &str = r#"#[cfg_attr(feature = "serde", serde(alias = "power", with = "crate::serializers::from_str"))]"#;
const PART_SET_HEADER_TOTAL: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::serializers::part_set_header_total"))]"#;
const RENAME_EDPUBKEY: &str = r#"#[cfg_attr(feature = "serde", serde(rename = "tenderdash/PubKeyEd25519", with = "crate::serializers::bytes::base64string"))]"#;
const RENAME_SECPPUBKEY: &str = r#"#[cfg_attr(feature = "serde", serde(rename = "tenderdash/PubKeySecp256k1", with = "crate::serializers::bytes::base64string"))]"#;
const RENAME_SRPUBKEY: &str = r#"#[cfg_attr(feature = "serde", serde(rename = "tenderdash/PubKeySr25519", with = "crate::serializers::bytes::base64string"))]"#;
const RENAME_DUPLICATEVOTE: &str =
    r#"#[cfg_attr(feature = "serde", serde(rename = "tenderdash/DuplicateVoteEvidence"))]"#;
const RENAME_LIGHTCLIENTATTACK: &str =
    r#"#[cfg_attr(feature = "serde", serde(rename = "tenderdash/LightClientAttackEvidence"))]"#;
// const EVIDENCE_VARIANT: &str = r#"#[serde(from =
// "crate::serializers::evidence::EvidenceVariant",
// into = "crate::serializers::evidence::EvidenceVariant")]"#;
const ALIAS_VALIDATOR_POWER_QUOTED: &str = r#"#[cfg_attr(feature = "serde", serde(alias = "ValidatorPower", with = "crate::serializers::from_str"))]"#;
const ALIAS_TOTAL_VOTING_POWER_QUOTED: &str = r#"#[cfg_attr(feature = "serde", serde(alias = "TotalVotingPower", with = "crate::serializers::from_str"))]"#;
const ALIAS_TIMESTAMP: &str = r#"#[cfg_attr(feature = "serde", serde(alias = "Timestamp"))]"#;
const ALIAS_PARTS: &str = r#"#[cfg_attr(feature = "serde", serde(alias = "parts"))]"#;
const DERIVE_FROM: &str = r#"#[derive(derive_more::From)]"#;
const DERIVE_FROM_STR: &str = r#"#[derive(derive_more::FromStr)]"#;
/// Custom type attributes applied on top of protobuf structs
/// The first item in the tuple defines the message where the annotation should
/// apply and the second item is the string that should be added as annotation.
/// The first item is a path as defined in the prost_build::Config::btree_map
/// here: <https://docs.rs/prost-build/0.6.1/prost_build/struct.Config.html#method.btree_map>
pub static CUSTOM_TYPE_ATTRIBUTES: &[(&str, &str)] = &[
    (".tendermint.crypto.PublicKey.sum", TYPE_TAG),
    (".tendermint.types.BlockIDFlag", PRIMITIVE_ENUM),
    (".tendermint.types.Evidence.sum", TYPE_TAG),
    (".tendermint.abci.Request.value", DERIVE_FROM),
    (".tendermint.abci.Response.value", DERIVE_FROM),
    (".tendermint.abci.ResponseException", DERIVE_FROM),
    (".tendermint.abci.ResponseException", DERIVE_FROM_STR),
];

/// Custom field attributes applied on top of protobuf fields in (a) struct(s)
/// The first item in the tuple defines the field where the annotation should
/// apply and the second item is the string that should be added as annotation.
/// The first item is a path as defined in the prost_build::Config::btree_map
/// here: <https://docs.rs/prost-build/0.6.1/prost_build/struct.Config.html#method.btree_map>
pub static CUSTOM_FIELD_ATTRIBUTES: &[(&str, &str)] = &[
    (".tendermint.version.Consensus.block", QUOTED),
    (".tendermint.version.Consensus.app", QUOTED_WITH_DEFAULT),
    (".tendermint.abci.ResponseInfo.data", DEFAULT),
    (".tendermint.abci.ResponseInfo.version", DEFAULT),
    (
        ".tendermint.abci.ResponseInfo.app_version",
        QUOTED_WITH_DEFAULT,
    ),
    (
        ".tendermint.abci.ResponseInfo.last_block_height",
        QUOTED_WITH_DEFAULT,
    ),
    (".tendermint.abci.ResponseInfo.last_block_app_hash", DEFAULT),
    (
        ".tendermint.abci.ResponseInfo.last_block_app_hash",
        HEXSTRING,
    ),
    (
        ".tendermint.abci.ResponseException.error",
        DERIVE_FROM_FORWARD,
    ),
    (".tendermint.types.BlockID.hash", HEXSTRING),
    (".tendermint.types.BlockID.part_set_header", ALIAS_PARTS),
    (
        ".tendermint.types.CanonicalBlockID.part_set_header",
        ALIAS_PARTS,
    ),
    (
        ".tendermint.types.PartSetHeader.total",
        PART_SET_HEADER_TOTAL,
    ),
    (".tendermint.types.PartSetHeader.hash", HEXSTRING),
    (".tendermint.types.Header.height", QUOTED),
    (".tendermint.types.Header.time", OPTIONAL),
    (".tendermint.types.Header.last_commit_hash", HEXSTRING),
    (".tendermint.types.Header.data_hash", HEXSTRING),
    (".tendermint.types.Header.validators_hash", HEXSTRING),
    (".tendermint.types.Header.next_validators_hash", HEXSTRING),
    (".tendermint.types.Header.consensus_hash", HEXSTRING),
    (".tendermint.types.Header.app_hash", HEXSTRING),
    (".tendermint.types.Header.last_results_hash", HEXSTRING),
    (".tendermint.types.Header.evidence_hash", HEXSTRING),
    (".tendermint.types.Header.proposer_address", HEXSTRING),
    (".tendermint.types.Data.txs", NULLABLEVECARRAY),
    (".tendermint.types.EvidenceList.evidence", NULLABLE),
    (".tendermint.types.Commit.height", QUOTED),
    (".tendermint.types.Commit.signatures", NULLABLE),
    (".tendermint.types.CommitSig.validator_address", HEXSTRING),
    (".tendermint.types.CommitSig.timestamp", OPTIONAL),
    (".tendermint.types.CommitSig.signature", BASE64STRING),
    (
        ".tendermint.types.DuplicateVoteEvidence.total_voting_power",
        ALIAS_TOTAL_VOTING_POWER_QUOTED,
    ),
    (
        ".tendermint.types.DuplicateVoteEvidence.validator_power",
        ALIAS_VALIDATOR_POWER_QUOTED,
    ),
    (
        ".tendermint.types.DuplicateVoteEvidence.timestamp",
        ALIAS_TIMESTAMP,
    ),
    (".tendermint.types.Vote.height", QUOTED),
    (".tendermint.types.Vote.validator_address", HEXSTRING),
    (".tendermint.types.Vote.signature", BASE64STRING),
    (".tendermint.types.Vote.timestamp", OPTIONAL),
    (".tendermint.types.Validator.address", HEXSTRING),
    (
        ".tendermint.types.Validator.voting_power",
        ALIAS_POWER_QUOTED,
    ), // https://github.com/tendermint/tendermint/issues/5549
    (
        ".tendermint.types.Validator.proposer_priority",
        QUOTED_WITH_DEFAULT,
    ), // Default is for /genesis deserialization
    (".tendermint.types.BlockMeta.block_size", QUOTED),
    (".tendermint.types.BlockMeta.num_txs", QUOTED),
    (".tendermint.crypto.PublicKey.sum.ed25519", RENAME_EDPUBKEY),
    (
        ".tendermint.crypto.PublicKey.sum.secp256k1",
        RENAME_SECPPUBKEY,
    ),
    (".tendermint.crypto.PublicKey.sum.sr25519", RENAME_SRPUBKEY),
    (
        ".tendermint.types.Evidence.sum.duplicate_vote_evidence",
        RENAME_DUPLICATEVOTE,
    ),
    (
        ".tendermint.types.Evidence.sum.light_client_attack_evidence",
        RENAME_LIGHTCLIENTATTACK,
    ),
    (".tendermint.types.TxProof.data", BASE64STRING),
    (".tendermint.types.TxProof.root_hash", HEXSTRING),
    (".tendermint.crypto.Proof.index", QUOTED),
    (".tendermint.crypto.Proof.total", QUOTED),
    (".tendermint.crypto.Proof.aunts", VEC_BASE64STRING),
    (".tendermint.crypto.Proof.leaf_hash", BASE64STRING),
    // Consensus params
    (
        ".tendermint.types.BlockParams.max_bytes",
        QUOTED_WITH_DEFAULT,
    ),
    (".tendermint.types.BlockParams.max_gas", QUOTED_WITH_DEFAULT),
    (
        ".tendermint.types.EvidenceParams.max_age_num_blocks",
        QUOTED_WITH_DEFAULT,
    ),
    (
        ".tendermint.types.EvidenceParams.max_bytes",
        QUOTED_WITH_DEFAULT,
    ),
    (
        ".tendermint.types.VersionParams.app_version",
        QUOTED_WITH_DEFAULT,
    ),
    (
        ".tendermint.types.VersionParams.consensus_version",
        QUOTED_WITH_DEFAULT,
    ),
];
