use super::{Address, Block, ShortHash};
use color_eyre::eyre;
use color_eyre::eyre::WrapErr;
use serde::Deserialize;
use serde_json::json;
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
pub struct ServerMessage {
    pub block: Block,
    pub work: u64,
}

impl ServerMessage {
    pub fn as_target(&self) -> Target {
        Target {
            work: self.work,
            block: self.block.short_hash,
        }
    }

    #[tracing::instrument(err)]
    pub(crate) fn from_json(json: impl AsRef<str> + Debug) -> eyre::Result<Self> {
        serde_json::from_str(json.as_ref()).wrap_err("invalid json for target message")
    }
}
