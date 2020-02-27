//! CPU miner core framework

use crate::krist::address::Address;
use crate::krist::block::ShortHash;
use crossbeam::atomic::AtomicCell;
use crossbeam::channel::Sender;
use std::str;
use std::sync::atomic::{AtomicU64, Ordering};

/// A type that can be used to efficiently feed input to a CPU miner kernel
pub trait KernelInput: Sized {
    /// Create a new instance with the given address and nonce
    fn new(address: Address, nonce: u64) -> Self;

    /// Set the block
    fn set_block(&mut self, block: &[u8; 12]);

    /// Increment the nonce for the next cycle
    fn increment_nonce(&mut self);

    type Score;
    fn get_solution(&mut self, work: u64, score: Self::Score) -> Option<String>;
}

/// A type to manage miner digest input for scalar kernels
#[derive(Clone)]
pub struct ScalarKernelInput {
    data: [u8; 64],
    nonce: u64,
}

impl KernelInput for ScalarKernelInput {
    fn new(address: Address, nonce: u64) -> Self {
        let mut data = [0u8; 64];

        data[..Address::LENGTH].copy_from_slice(address.as_bytes());

        // padding
        data[Self::LENGTH] = 0x80;
        data[62] = (((Self::LENGTH * 8) >> 8) & 0xFF) as u8;
        data[63] = ((Self::LENGTH * 8) & 0xFF) as u8;

        let mut input = Self { data, nonce };
        input.increment_nonce();
        input
    }

    fn set_block(&mut self, block: &[u8; 12]) {
        self.data[Address::LENGTH..Self::LENGTH - Self::NONCE_LENGTH].copy_from_slice(block);
    }

    fn increment_nonce(&mut self) {
        self.nonce = self.nonce.wrapping_add(1);
        let n = self.nonce;

        for (i, v) in self.nonce_mut().iter_mut().enumerate() {
            *v = (((n >> (i * 6)) & 0x3f) + 32) as u8;
        }
    }

    type Score = u64;

    #[inline(always)]
    fn get_solution(&mut self, work: u64, score: u64) -> Option<String> {
        if score <= work {
            Some(self.nonce_str().to_string())
        } else {
            None
        }
    }
}

impl ScalarKernelInput {
    pub const NONCE_LENGTH: usize = 11;
    pub const LENGTH: usize = Address::LENGTH + (ShortHash::LENGTH * 2) + Self::NONCE_LENGTH;

    fn nonce_mut(&mut self) -> &mut [u8] {
        &mut self.data[Self::LENGTH - Self::NONCE_LENGTH..Self::LENGTH]
    }

    /// Get an immutable reference to the expanded nonce (e.g. for submission)
    pub fn nonce(&mut self) -> &[u8] {
        self.nonce_mut() as &_
    }

    pub fn nonce_str(&mut self) -> &str {
        str::from_utf8(self.nonce()).unwrap()
    }

    /// Get the data for this input
    pub fn data(&self) -> &[u8] {
        &self.data[..Self::LENGTH]
    }

    /// Get the full padded data block for this input
    pub fn data_block(&self) -> &[u8; 64] {
        &self.data
    }
}

/// A CPU mining kernel.
pub trait Kernel {
    /// The input type for this kernel
    type Input: KernelInput;

    /// Get the score for a hash with the given input.
    fn score(&self, input: &Self::Input) -> <Self::Input as KernelInput>::Score;
}

pub struct Context<'a> {
    address: Address,
    hashes: &'a AtomicU64,
    target: &'a AtomicCell<Option<([u8; 12], u64)>>,
    nonce: u64,
    sol_tx: &'a Sender<String>,
}

impl<'a> Context<'a> {
    /// Create a new `Context`.
    pub fn new(
        address: Address,
        hashes: &'a AtomicU64,
        target: &'a AtomicCell<Option<([u8; 12], u64)>>,
        nonce: u64,
        sol_tx: &'a Sender<String>,
    ) -> Self {
        Self {
            address,
            hashes,
            target,
            nonce,
            sol_tx,
        }
    }

    /// Mine synchronously using this context and the given kernel.
    pub fn mine<K: Kernel>(self, kernel: K) {
        const BATCH_SIZE: u64 = 10_000;
        let mut input = K::Input::new(self.address, self.nonce);

        while let Some((block, work)) = self.target.load() {
            input.set_block(&block);

            for _ in 0..BATCH_SIZE {
                let score = kernel.score(&input);
                if let Some(solution) = input.get_solution(work, score) {
                    // solution found!
                    if self.sol_tx.send(solution).is_err() {
                        return;
                    }
                }
                input.increment_nonce();
            }

            self.hashes.fetch_add(BATCH_SIZE, Ordering::Relaxed);
        }
    }
}
