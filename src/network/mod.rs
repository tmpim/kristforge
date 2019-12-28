//! Networking code for interacting with a krist node

mod http;
mod ws;

use crate::krist::address::Address;
use crate::krist::block::Block;
use futures::{Sink, TryStream};
use isahc::http::Uri;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::num::NonZeroU64;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct NetConfig {
    /// The krist node to connect to
    #[structopt(short, long, default_value = "https://krist.ceriat.net/ws/start")]
    pub node: Uri,
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] isahc::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Websocket error: {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),
}

#[derive(Debug, Clone, Copy)]
pub struct KeepAliveType;

impl<'de> Deserialize<'de> for KeepAliveType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match <&str>::deserialize(deserializer)? {
            "keepalive" => Ok(KeepAliveType),
            _ => Err(D::Error::custom("Message type is not keepalive")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SubmitBlockType;

impl Serialize for SubmitBlockType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("submit_block")
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
        fields: HashMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ClientMessage {
    SubmitBlock {
        #[serde(rename = "type")]
        msg_type: SubmitBlockType,
        id: NonZeroU64,
        address: Address,
        nonce: String,
    },
}

impl ClientMessage {
    pub fn new_solution(address: Address, nonce: [u8; 12]) -> Self {
        ClientMessage::SubmitBlock {
            msg_type: SubmitBlockType,
            id: rand::random(),
            address,
            nonce: std::str::from_utf8(&nonce)
                .expect("invalid nonce")
                .to_string(),
        }
    }
}

pub async fn connect(
    cfg: NetConfig,
) -> Result<
    (
        impl Sink<ClientMessage, Error = NetworkError>,
        impl TryStream<Ok = ServerMessage, Error = NetworkError>,
    ),
    NetworkError,
> {
    ws::ws_connect(http::ws_start(cfg.node).await?.url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_value};
    use std::str::FromStr;

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
