#[cfg(not(feature = "grpc"))]
use core::convert::TryFrom;

use tenderdash_proto::{
    abci::ResponseException,
    types::{BlockId as RawBlockId, PartSetHeader as RawPartSetHeader},
    Protobuf,
};

impl Protobuf<RawBlockId> for BlockId {}

// Example implementation of a protobuf struct using Protobuf.
#[derive(Clone, Debug)]
pub struct BlockId {
    hash: String,
    state_id: String,
    part_set_header_exists: bool,
}

// Domain types MUST have the TryFrom trait to convert from Protobuf messages.
impl TryFrom<RawBlockId> for BlockId {
    type Error = &'static str;

    fn try_from(value: RawBlockId) -> Result<Self, Self::Error> {
        Ok(BlockId {
            hash: String::from_utf8(value.hash)
                .map_err(|_| "Could not convert vector to string")?,
            state_id: String::from_utf8(value.state_id)
                .map_err(|_| "Could not convert state ID to string")?,
            part_set_header_exists: value.part_set_header.is_some(),
        })
    }
}

// Domain types MUST be able to convert to Protobuf messages without errors
// using the From trait.
impl From<BlockId> for RawBlockId {
    fn from(value: BlockId) -> Self {
        RawBlockId {
            hash: value.hash.into_bytes(),
            state_id: value.state_id.into_bytes(),
            part_set_header: match value.part_set_header_exists {
                true => Some(RawPartSetHeader {
                    total: 0,
                    hash: vec![],
                }),
                false => None,
            },
        }
    }
}

// Do any custom implementation for your type
impl PartialEq for BlockId {
    fn eq(&self, other: &Self) -> bool {
        self.part_set_header_exists == other.part_set_header_exists && self.hash == other.hash
    }
}

#[test]
pub fn protobuf_struct_example() {
    let my_domain_type = BlockId {
        hash: "Hello world!".to_string(),
        state_id: Default::default(),
        part_set_header_exists: false,
    };

    let mut wire = vec![];
    my_domain_type.encode(&mut wire).unwrap();
    assert_eq!(
        wire,
        vec![10, 12, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33]
    );
    let new_domain_type = BlockId::decode(wire.as_ref()).unwrap();
    assert_eq!(new_domain_type.hash, "Hello world!".to_string());
    assert!(!new_domain_type.part_set_header_exists);
    assert_eq!(my_domain_type.encoded_len(), 14);
}

#[test]
pub fn protobuf_struct_length_delimited_example() {
    let my_domain_type = BlockId {
        hash: "Hello world!".to_string(),
        state_id: Default::default(),
        part_set_header_exists: false,
    };

    let mut wire = vec![];
    my_domain_type.encode_length_delimited(&mut wire).unwrap();
    assert_eq!(
        wire,
        vec![14, 10, 12, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33]
    );

    let new_domain_type = BlockId::decode_length_delimited(wire.as_ref()).unwrap();
    assert_eq!(new_domain_type.hash, "Hello world!".to_string());
    assert!(!new_domain_type.part_set_header_exists);
}

#[test]
pub fn protobuf_struct_conveniences_example() {
    let my_domain_type = BlockId {
        hash: "Hello world!".to_string(),
        state_id: Default::default(),
        part_set_header_exists: false,
    };

    let wire = my_domain_type.encode_vec().unwrap();
    assert_eq!(
        wire,
        vec![10, 12, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33]
    );
    let new_domain_type = BlockId::decode_vec(&wire).unwrap();
    assert_eq!(my_domain_type, new_domain_type);

    let wire = my_domain_type.encode_length_delimited_vec().unwrap();
    assert_eq!(
        wire,
        vec![14, 10, 12, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33]
    );
    let new_domain_type = BlockId::decode_length_delimited_vec(&wire).unwrap();
    assert_eq!(my_domain_type, new_domain_type);
}

#[test]
pub fn test_response_exception_from() {
    assert_eq!(
        ResponseException::from(String::from("string")).error,
        "string"
    );
    assert_eq!(
        ResponseException::from(String::from("string")).error,
        "string"
    );
}

#[test]
#[cfg(feature = "serde")]
pub fn test_consensus_params_serde() {
    use tenderdash_proto::types::ConsensusParams;

    let json = r#"
    {
        "block": {
          "max_bytes": "2097152",
          "max_gas": "500000000"
        },
        "evidence": {
          "max_age_num_blocks": "10000",
          "max_age_duration": "172800000000000",
          "max_bytes": "0"
        },
        "validator": {
          "pub_key_types": [
            "bls12381"
          ]
        },
        "version": {
          "consensus_version": "1",
          "app_version": "1"
        },
        "synchrony": {
          "precision": "500000000",
          "message_delay": "60000000000"
        },
        "timeout": {
          "propose": "40000000000",
          "propose_delta": "5000000000",
          "vote": "40000000000",
          "vote_delta": "5000000000",
          "bypass_commit_timeout": true
        },
        "abci": {
          "recheck_tx": true
        }
    }
    "#;

    let new_params: ConsensusParams = serde_json::from_str(json).unwrap();
    assert_eq!(new_params.version.unwrap().consensus_version, 1)
}
