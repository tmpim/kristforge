mod sha;
mod unoptimized;

use super::framework::{Kernel, ScalarKernelInput};
pub use sha::SHA;
pub use unoptimized::Unoptimized;

/// Calculate the score from the raw a/b variables of the hash state
#[inline(always)]
fn score_ab(a: u32, b: u32) -> u64 {
    (a.to_le() as u64) << 16 | (b as u64) >> 16
}

/// Calculate the score from the hash output (len must be >= 6)
#[inline(always)]
fn score_output(h: &[u8]) -> u64 {
    h[..6]
        .iter()
        .enumerate()
        .map(|(i, &v)| (v as u64) << (40 - (8 * i)))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::super::framework::KernelInput;
    use super::*;
    use crate::krist::address::Address;
    use ring::digest::{digest, SHA256};
    use std::str::FromStr;

    fn test_scalar_kernel(kernel: impl Kernel<Input = ScalarKernelInput>) {
        let mut input = ScalarKernelInput::new(Address::from_str("k5ztameslf").unwrap(), 0);
        input.set_block(&*b"abce8f03b1d2");

        let expected_hex = hex::encode(digest(&SHA256, input.data()).as_ref());
        let expected = u64::from_str_radix(&expected_hex[..12], 16).unwrap();

        let actual = kernel.score(&input);

        assert_eq!(
            expected,
            actual,
            "hash score mismatch for input '{}'",
            String::from_utf8_lossy(input.data())
        )
    }

    #[test]
    fn test_unoptimized_kernel() {
        test_scalar_kernel(Unoptimized);
    }

    #[test]
    fn test_sha_kernel() {
        if is_x86_feature_detected!("sha") {
            test_scalar_kernel(SHA);
        }
    }
}
