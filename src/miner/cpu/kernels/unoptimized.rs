use super::{score_output, HashInput, Kernel};
use ring::digest::{digest, SHA256};

pub struct Unoptimized;
impl Kernel for Unoptimized {
    #[inline(always)]
    fn score(&self, input: &HashInput) -> u64 {
        score_output(digest(&SHA256, input.data()).as_ref())
    }
}
