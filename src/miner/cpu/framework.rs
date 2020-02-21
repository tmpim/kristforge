//! CPU miner core framework

use crate::krist::address::Address;
use crate::krist::block::ShortHash;
use crossbeam::atomic::AtomicCell;
use crossbeam::channel::Sender;
use std::str;
use std::sync::atomic::{AtomicU64, Ordering};

/// A type to manage miner digest input
#[derive(Clone)]
pub struct HashInput {
    data: [u8; 64],
    nonce: u64,
}

impl HashInput {
    const NONCE_LENGTH: usize = 11;
    pub const LENGTH: usize = Address::LENGTH + (ShortHash::LENGTH * 2) + Self::NONCE_LENGTH;

    /// Initialize a new `HashInput`. Should not be used until a target is
    /// set.
    pub fn new(address: Address, nonce: u64) -> Self {
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

    /// Increment the nonce value and update the array accordingly.
    pub fn increment_nonce(&mut self) {
        self.nonce = self.nonce.wrapping_add(1);
        let n = self.nonce;

        for (i, v) in self.nonce_mut().iter_mut().enumerate() {
            *v = (((n >> (i * 6)) & 0x3f) + 32) as u8;
        }
    }

    /// Update the block of this input
    pub fn set_block(&mut self, block: &[u8; 12]) {
        self.data[Address::LENGTH..Self::LENGTH - Self::NONCE_LENGTH].copy_from_slice(block);
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

pub struct Context<'a> {
    input: HashInput,
    hashes: &'a AtomicU64,
    target: &'a AtomicCell<Option<([u8; 12], u64)>>,
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
        let input = HashInput::new(address, nonce);
        Self {
            input,
            hashes,
            target,
            sol_tx,
        }
    }

    /// Mine synchronously using this context and the given kernel.
    pub fn mine(mut self, kernel: impl Kernel) {
        const BATCH_SIZE: u64 = 10_000;

        while let Some((block, work)) = self.target.load() {
            self.input.set_block(&block);

            for _ in 0..BATCH_SIZE {
                if kernel.score(&self.input) <= work {
                    // solution found!
                    if self
                        .sol_tx
                        .send(self.input.nonce_str().to_string())
                        .is_err()
                    {
                        return;
                    }
                }
                self.input.increment_nonce();
            }

            self.hashes.fetch_add(BATCH_SIZE, Ordering::Relaxed);
        }
    }
}

/// A CPU mining kernel.
pub trait Kernel {
    /// Get the score for a hash with the given input.
    fn score(&self, input: &HashInput) -> u64;
}
