pub mod gpu;

use crate::krist::{Address, ShortHash, Solution, Target};
use crate::utils::HashRate;

/// A marker value used to indicate that mining should stop.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("miner stopped")]
pub struct StopMining;

/// An interface for miners, allowing them to communicate with the rest of the
/// program and access necessary configuration values.
pub trait MinerInterface {
    /// Get the address to mine for.
    fn address(&self) -> Address;

    /// Get the prefix for the calling miner.
    ///
    /// The prefix is used to prevent multiple miners for the same address from
    /// evaluating the same hashes, and should be different for every miner.
    ///
    /// This value may change between calls, and miners should store it to
    /// ensure correct behavior. The default implementation simply generates a
    /// random number, which is generally good enough.
    fn prefix(&self) -> u16 {
        rand::random()
    }

    /// Get the current mining target.
    fn target(&mut self) -> Result<Target, StopMining>;

    /// Attempt to submit the given solution.
    fn submit(&mut self, solution: Solution) -> Result<(), StopMining>;

    /// Report the current hash rate.
    fn hashes_completed(&mut self, speed: HashRate);
}

/// A miner interface that uses a fake address and target, ignores submissions,
/// and calls a given function with reported speeds.
///
/// This has little use beyond debugging and benchmarking.
#[derive(Debug, Clone)]
pub struct BenchmarkInterface<F>(pub F);

impl<F: FnMut(HashRate)> MinerInterface for BenchmarkInterface<F> {
    fn address(&self) -> Address {
        Address(*b"k123456789")
    }

    fn prefix(&self) -> u16 {
        0
    }

    fn target(&mut self) -> Result<Target, StopMining> {
        Ok(Target {
            work: 0,
            block: ShortHash([0; ShortHash::LENGTH]),
        })
    }

    fn submit(&mut self, _solution: Solution) -> Result<(), StopMining> {
        Ok(())
    }

    fn hashes_completed(&mut self, hashes: HashRate) {
        self.0(hashes)
    }
}
