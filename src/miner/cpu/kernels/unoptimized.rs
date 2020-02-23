use super::{score_output, Kernel, ScalarKernelInput};
use ring::digest::{digest, SHA256};

pub struct Unoptimized;
impl Kernel for Unoptimized {
    type Input = ScalarKernelInput;

    #[inline(always)]
    fn score(&self, input: &ScalarKernelInput) -> u64 {
        score_output(digest(&SHA256, input.data()).as_ref())
    }
}
