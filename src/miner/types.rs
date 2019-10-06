use super::selector::Selector;
use crate::krist::address::Address;
use crate::krist::block::ShortHash;
use failure::Fail;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug, Clone, Copy)]
pub struct VectorSize(pub u8);

#[derive(Debug, Fail, Clone, Copy)]
#[fail(display = "{} is not a valid vector size", _0)]
pub struct InvalidVectorSize(pub u8);

impl From<VectorSize> for u8 {
    fn from(size: VectorSize) -> Self {
        size.0
    }
}

impl FromStr for VectorSize {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match u8::from_str(s) {
            Ok(v) if [1, 2, 4, 8, 16].contains(&v) => Ok(VectorSize(v)),
            Ok(v) => Err(InvalidVectorSize(v).into()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug, Clone, StructOpt)]
pub struct MinerConfig {
    /// The address to mine for
    pub address: Address,

    /// Select devices to mine with, using selectors from the "info" command
    #[structopt(short, long = "device", name = "selector", default_value = "all")]
    pub devices: Vec<Selector>,

    /// Force use of a given vector size for mining. Must be power of two
    /// between 1 and 16
    #[structopt(short, long, name = "size")]
    pub vector_size: Option<VectorSize>,

    /// Target number of seconds per kernel execution - lower this value if
    /// mining causes unacceptable issues with system performance
    #[structopt(short, long, name = "seconds", default_value = "0.1")]
    pub target_rate: f32,

    /// Maximum kernel worksize to use - will automatically be adjusted to a
    /// maximum of this value based on target rate
    #[structopt(short, long, name = "worksize", default_value = "268435456")]
    pub max_worksize: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub work: u64,
    pub block: ShortHash,
}

#[derive(Debug, Clone, Copy)]
pub struct Solution {
    pub address: Address,
    pub nonce: [u8; 15],
}
