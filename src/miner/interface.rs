use super::Target;
use crate::krist::address::Address;
use crate::miner::calculate_work;
use crossbeam_channel::{Receiver, TryRecvError};
use futures::channel::mpsc::UnboundedSender;
use indicatif::ProgressBar;
use sha2::{Digest, Sha256};
use std::cmp::min;
use std::convert::TryInto;
use std::time::Duration;

pub struct MinerInterface {
    address: Address,
    pb: ProgressBar,
    target_rx: Receiver<Target>,
    target: Option<Target>,
    solution_tx: UnboundedSender<Vec<u8>>,
}

pub struct StopMining;

#[derive(Debug, Clone, Copy)]
pub enum CurrentTarget {
    New(Target),
    Unchanged(Target),
    StopMining,
}

impl MinerInterface {
    pub fn new(
        address: Address,
        pb: ProgressBar,
        target_rx: Receiver<Target>,
        solution_tx: UnboundedSender<Vec<u8>>,
    ) -> Self {
        Self {
            address,
            pb,
            target_rx,
            target: None,
            solution_tx,
        }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    /// Get the current target, blocking the thread if necessary
    pub fn current_target(&mut self) -> CurrentTarget {
        if let Some(old) = self.target {
            match self.target_rx.try_recv() {
                Ok(target) => {
                    self.target = Some(target);
                    CurrentTarget::New(target)
                }
                Err(TryRecvError::Empty) => CurrentTarget::Unchanged(old),
                Err(TryRecvError::Disconnected) => CurrentTarget::StopMining,
            }
        } else {
            match self.target_rx.recv() {
                Ok(target) => {
                    self.target = Some(target);
                    CurrentTarget::New(target)
                }
                Err(_) => CurrentTarget::StopMining,
            }
        }
    }

    pub fn report_speed(&mut self, hashes: u64, time: Duration) {
        let per_second = hashes as f64 / time.as_secs_f64();

        const PREFIXES: [&str; 5] = ["", "k", "M", "G", "T"];
        let magnitude = min(PREFIXES.len() - 1, per_second.log(1000.).floor() as usize);
        let value = per_second / 1000f64.powf(magnitude as f64);

        self.pb.set_message(&format!(
            "Mining at {:.1} {}h/s",
            value, PREFIXES[magnitude]
        ));
    }

    pub fn report_solution(&mut self, solution: Vec<u8>) -> Result<(), StopMining> {
        // calculate the score for the reported solution
        let input = self.address.to_string() + &self.target.unwrap().block.into_hex();
        let mut input = input.as_bytes().to_vec();
        input.extend(&solution);

        let hash = Sha256::digest(&input);
        let score = calculate_work(hash[..6].try_into().unwrap());

        // TODO: reject invalid solutions here

        log::info!(
            "Solution reported:\n\
            \tAddress: {}\n\
            \tTarget: {:?}\n\
            \tRaw solution nonce (len={}): `{:?}`\n\
            \tHex solution nonce: {}\n\
            \tRaw hash input (len={}): `{:?}`\n\
            \tHex hash input: {}\n\
            \tHex hash output: {}\n\
            \tCalculated score: {}",
            self.address,
            self.target,
            solution.len(),
            solution,
            hex::encode(&solution),
            input.len(),
            input,
            hex::encode(&input),
            hex::encode(hash),
            score,
        );

        self.pb.println(format!(
            "Submitting solution for block {} (hex nonce {})",
            self.target.unwrap().block.into_hex(),
            hex::encode(&solution),
        ));

        self.solution_tx
            .unbounded_send(solution)
            .map_err(|_| StopMining)
    }
}

impl Drop for MinerInterface {
    fn drop(&mut self) {
        self.pb.finish();
    }
}

#[cfg(test)]
mod tests {}
