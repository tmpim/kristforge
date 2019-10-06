use super::types::Target;
use crate::miner::types::Solution;
use crossbeam::channel::{Receiver, RecvError, TryRecvError};
use indicatif::ProgressBar;
use std::cmp::min;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

pub struct StopMining;

pub struct MinerInterface {
    prefix: u8,
    target_rx: Receiver<Target>,
    target: Option<Target>,
    pb: ProgressBar,
    solution_tx: UnboundedSender<Solution>,
}

pub enum CurrentTarget {
    New(Target),
    Unchanged(Target),
    StopMining,
}

impl MinerInterface {
    pub fn new(
        prefix: u8,
        target_rx: Receiver<Target>,
        pb: ProgressBar,
        solution_tx: UnboundedSender<Solution>,
    ) -> Self {
        Self {
            prefix,
            target_rx,
            target: None,
            pb,
            solution_tx,
        }
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    /// Get the current target
    /// This may block the thread (i.e. on the first invocation)
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
                Err(RecvError) => CurrentTarget::StopMining,
            }
        }
    }

    /// Report a solution
    pub fn report_solution(&mut self, solution: Solution) -> Result<(), StopMining> {
        let target = self.target.unwrap();
        self.pb.println(format!(
            "Submitting solution for block {} (nonce {})",
            target.block.into_hex(),
            String::from_utf8_lossy(&solution.nonce)
        ));

        self.solution_tx.try_send(solution).map_err(|_| StopMining)
    }

    pub fn report_speed(&mut self, hashes: u32, duration: Duration) {
        let per_second = hashes as f64 / duration.as_secs_f64();

        const PREFIXES: [&str; 5] = ["", "k", "M", "G", "T"];
        let magnitude = min(PREFIXES.len() - 1, per_second.log(1000.).floor() as usize);
        let value = per_second / 1000f64.powf(magnitude as f64);

        self.pb.set_message(&format!(
            "Mining at {:.1} {}h/s",
            value, PREFIXES[magnitude]
        ));
    }
}

impl Drop for MinerInterface {
    fn drop(&mut self) {
        self.pb.finish();
    }
}
