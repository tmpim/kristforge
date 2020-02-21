pub mod naive;
pub mod x86_64_sha;

pub trait MinerCore {
    fn score_hash(data: [u8; 64], len: usize) -> u64;
}
