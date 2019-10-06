use crate::krist::address::Address;
use crate::krist::block::Block;
use crate::miner::types::Solution;
use crate::prelude::*;
use failure::_core::num::NonZeroU64;
use serde::{Deserializer, Serializer};
use std::collections::HashMap;
use structopt::StructOpt;
use url::Url;

#[derive(Debug, StructOpt)]
pub struct NetConfig {
    /// The krist node to connect to
    #[structopt(short, long, default_value = "https://krist.ceriat.net/ws/start")]
    pub node: Url,
}

#[derive(Debug, Clone, Copy)]
pub struct KeepAliveType;

impl<'de> Deserialize<'de> for KeepAliveType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "keepalive" => Ok(KeepAliveType),
            _ => Err(D::Error::custom("Message type is not keepalive")),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ServerMessage {
    Target {
        #[serde(alias = "type")]
        msg_type: String,

        #[serde(alias = "last_block")]
        block: Block,

        #[serde(alias = "new_work")]
        work: u64,
    },

    KeepAlive {
        #[serde(alias = "type")]
        msg_type: KeepAliveType,
    },

    Unknown {
        #[serde(alias = "type")]
        msg_type: Option<String>,

        #[serde(flatten)]
        fields: HashMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SubmitBlockType;

impl Serialize for SubmitBlockType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("submit_block")
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ClientMessage {
    SubmitBlock {
        id: NonZeroU64,
        #[serde(rename = "type")]
        msg_type: SubmitBlockType,
        address: Address,
        nonce: String,
    },
}

impl From<Solution> for ClientMessage {
    fn from(solution: Solution) -> Self {
        ClientMessage::SubmitBlock {
            id: rand::random(),
            msg_type: SubmitBlockType,
            address: solution.address,
            nonce: String::from_utf8(solution.nonce.to_vec()).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_value};

    #[test]
    fn test_client_message() {
        let json = json!({
            "id": 5,
            "type": "submit_block",
            "address": "k5ztameslf",
            "nonce": "aaaaaaaaaaaaaaa"
        });

        let msg = ClientMessage::SubmitBlock {
            id: NonZeroU64::new(5).unwrap(),
            msg_type: SubmitBlockType,
            address: Address::from_str("k5ztameslf").unwrap(),
            nonce: "aaaaaaaaaaaaaaa".to_string(),
        };

        assert_eq!(json, to_value(&msg).unwrap());
    }
}
