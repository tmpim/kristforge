use ring::digest::{digest, SHA256};
use std::convert::TryInto;

pub struct MinerCore;
impl super::MinerCore for MinerCore {
    fn score_hash(data: [u8; 64], len: usize) -> u64 {
        let hash: [u8; 32] = digest(&SHA256, &data[..len]).as_ref().try_into().unwrap();

        hash[..6]
            .iter()
            .enumerate()
            .map(|(i, &v)| (v as u64) << (40 - (8 * i)))
            .sum()
    }
}
