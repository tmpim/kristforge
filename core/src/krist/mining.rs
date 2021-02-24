use super::{Address, Block, ShortHash};
use color_eyre::eyre;
use color_eyre::eyre::WrapErr;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A mining target, specifying the current work and previous block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub work: u64,
    pub block: ShortHash,
}

/// A solution to be submitted to the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Solution {
    pub address: Address,
    pub nonce: [u8; 8],
}

impl Solution {
    /// Encode this solution to JSON that can be sent to the server.
    pub(crate) fn to_json(self) -> String {
        static SUBMISSION_ID: AtomicUsize = AtomicUsize::new(1);
        json!({
            "id": SUBMISSION_ID.fetch_add(1, Ordering::AcqRel),
            "type": "submit",
            "address": self.address,
            "nonce": self.nonce,
        })
        .to_string()
    }
}

/// A message from the server containing target information.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ServerMessage {
    /// A message containing a mining target.
    Target {
        #[serde(alias = "last_block")]
        block: Block,

        #[serde(alias = "new_work")]
        work: u64,

        /// Any extra fields this message contained.
        #[serde(flatten)]
        extra_fields: HashMap<String, Value>,
    },

    /// Any other message type.
    Other {
        #[serde(flatten)]
        fields: HashMap<String, Value>,
    },
}

impl ServerMessage {
    pub fn as_target(&self) -> Option<Target> {
        match self {
            ServerMessage::Target { block, work, .. } => Some(Target {
                work: *work,
                block: block.short_hash,
            }),
            ServerMessage::Other { .. } => None,
        }
    }

    #[tracing::instrument(err)]
    pub(crate) fn from_json(json: impl AsRef<str> + Debug) -> eyre::Result<Self> {
        serde_json::from_str(json.as_ref()).wrap_err("invalid json for target message")
    }
}
