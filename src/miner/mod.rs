pub mod interface;

use crate::krist::address::Address;
use crate::krist::block::ShortHash;
use crate::miner::interface::MinerInterface;
use structopt::StructOpt;
use uuid::Uuid;

#[derive(Debug, Clone, StructOpt)]
pub struct MinerConfig {
    /// Don't use OpenCL for mining.
    #[structopt(long)]
    no_gpu: bool,

    /// Select one or more specific OpenCL devices to use. If not set, all
    /// compatible devices will be used.
    #[structopt(short, long)]
    gpu: Option<Vec<Uuid>>,
}

#[derive(Debug, thiserror::Error)]
pub enum MinerError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub work: u64,
    pub block: ShortHash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Solution {
    pub address: Address,
    pub nonce: [u8; 12],
}

pub trait Miner {
    /// Get a human-readable description of this miner
    fn describe(&self) -> String;

    /// Start a long-lived mining operation, blocking the thread and using the
    /// given interface for state operations.
    fn mine(self: Box<Self>, interface: MinerInterface) -> Result<(), MinerError>;
}

pub fn create_miners(opts: MinerConfig) -> Result<Vec<Box<dyn Miner + Send>>, MinerError> {
    let mut miners = vec![];

    if !opts.no_gpu {}

    Ok(miners)
}
