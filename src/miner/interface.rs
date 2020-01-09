use super::Target;
use crate::krist::address::Address;
use crossbeam::channel::{Receiver, TryRecvError};
use futures::channel::mpsc::UnboundedSender;
use indicatif::ProgressBar;
use std::cmp::min;
use std::convert::TryInto;
use std::time::Duration;

pub struct MinerInterface {
    address: Address,
    pb: ProgressBar,
    target_rx: Receiver<Target>,
    target: Option<Target>,
    solution_tx: UnboundedSender<String>,
}

pub struct StopMining;

#[derive(Debug, Clone, Copy)]
pub enum CurrentTarget {
    New(Target),
    Unchanged(Target),
    StopMining,
}

impl CurrentTarget {
    pub fn into_raw(self) -> Option<([u8; 12], u64)> {
        match self {
            CurrentTarget::New(t) | CurrentTarget::Unchanged(t) => {
                Some((t.block.into_hex().as_bytes().try_into().unwrap(), t.work))
            }
            CurrentTarget::StopMining => None,
        }
    }
}

impl MinerInterface {
    pub fn new(
        address: Address,
        pb: ProgressBar,
        target_rx: Receiver<Target>,
        solution_tx: UnboundedSender<String>,
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

    pub fn report_solution(&self, solution: String) -> Result<(), StopMining> {
        log::info!(
            "Solution reported for address {} and target {:?}: nonce {} (hex: {:x?})",
            self.address,
            self.target,
            solution,
            solution,
        );

        self.pb.println(format!(
            "Submitting solution for block {} (nonce {})",
            self.target.unwrap().block.into_hex(),
            solution
        ));

        // TODO: validate solution

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
