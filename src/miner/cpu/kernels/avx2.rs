use super::{Kernel, KernelInput, ScalarKernelInput};
use crate::krist::address::Address;
use std::arch::x86_64::*;

const NONCE_LENGTH: usize = ScalarKernelInput::NONCE_LENGTH;
const LENGTH: usize = ScalarKernelInput::LENGTH;

struct Avx2KernelInput {
    data: [__m256i; 64],
    nonce: u64,
}

impl Avx2KernelInput {
    fn nonce_mut(&mut self) -> &mut [__m256i] {
        &mut self.data[LENGTH - NONCE_LENGTH..LENGTH]
    }
}

impl KernelInput for Avx2KernelInput {
    #[target_feature(enable = "avx2")]
    fn new(address: Address, nonce: u64) -> Self {
        unsafe {
            let mut data = [_mm256_setzero_si256(); 64];

            // address
            for (d, &a) in data.iter_mut().zip(address.as_bytes().iter()) {
                *d = _mm256_set1_epi32(a as i32);
            }

            // padding
            data[LENGTH] = _mm256_set1_epi32(0x80);
            data[62] = _mm256_set1_epi32((((LENGTH * 8) >> 8) & 0xFF) as i32);
            data[63] = _mm256_set1_epi32(((LENGTH * 8) & 0xFF) as i32);

            let mut input = Self { data, nonce };
            input.increment_nonce();
            input
        }
    }

    #[target_feature(enable = "avx2")]
    fn set_block(&mut self, block: &[u8; 12]) {
        for (d, &b) in self.data[Address::LENGTH..].iter_mut().zip(block.iter()) {
            unsafe {
                *d = _mm256_set1_epi32(b as i32);
            }
        }
    }

    #[target_feature(enable = "avx2")]
    fn increment_nonce(&mut self) {
        unsafe {
            self.nonce = self.nonce.wrapping_add(8);

            // NOTE: this can result in overlaps when the lower component
            //  overflows, but this case is rare and inconsequential enough
            //  that it's acceptable
            let u = _mm256_set1_epi32((self.nonce >> 32) as i32);
            let l = _mm256_add_epi32(
                _mm256_set1_epi32(self.nonce as i32),
                _mm256_set_epi32(0, 1, 2, 3, 4, 5, 6, 7),
            );

            for (i, v) in self.nonce_mut().iter_mut().enumerate() {
                let shift = 
            }
        }
    }

    type Score = ();

    fn get_solution(&mut self, work: u64, score: Self::Score) -> Option<String> {
        unimplemented!()
    }
}
