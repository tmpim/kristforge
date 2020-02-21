pub mod cpu;
pub mod gpu;
pub mod interface;

use crate::krist::address::Address;
use crate::krist::block::ShortHash;
use crate::miner::cpu::{CpuMiner, KernelType};
use crate::miner::gpu::OclMiner;
use crate::miner::interface::MinerInterface;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct MinerConfig {
    /// Don't use OpenCL for mining.
    #[structopt(long)]
    no_gpu: bool,
    // TODO: allow selecting individual devices
    /// OpenCL miner target kernel execution time, in seconds.
    #[structopt(long, default_value = "0.1")]
    gpu_rate: f32,

    /// OpenCL miner max work size (default 2^31).
    #[structopt(long, default_value = "2147483648")]
    gpu_max_worksize: usize,

    /// Don't use the CPU for mining.
    #[structopt(long)]
    no_cpu: bool,

    /// CPU miner threads, defaulting to the processor's thread count.
    #[structopt(long)]
    cpu_threads: Option<usize>,

    /// Select a specific CPU mining kernel.
    #[structopt(long)]
    cpu_kernel: Option<KernelType>,
}

#[derive(Debug, thiserror::Error)]
pub enum MinerError {
    #[error("OpenCL error: {0}")]
    OclError(#[from] dynamic_ocl::Error),
}

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
    let mut miners = Vec::<Box<dyn Miner + Send>>::new();

    if !opts.no_gpu {
        for device in gpu::get_opencl_devices()? {
            miners.push(Box::new(OclMiner::new(device, &opts)?));
        }
    }

    if !opts.no_cpu {
        miners.push(Box::new(CpuMiner::new(&opts)));
    }

    Ok(miners)
}
