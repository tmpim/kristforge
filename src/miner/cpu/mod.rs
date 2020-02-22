mod framework;
mod kernels;
mod thread_priority;

use crate::miner::cpu::framework::Context;
use crate::miner::interface::{CurrentTarget, MinerInterface};
use crate::miner::{Miner, MinerConfig, MinerError};
use crossbeam::atomic::AtomicCell;
use crossbeam::channel::RecvTimeoutError;
use enumset::{EnumSet, EnumSetType};
use itertools::Itertools;
use std::fmt::{self, Display, Formatter};
use std::num::Wrapping;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Select a CPU mining kernel to use
#[derive(Debug, EnumSetType, PartialOrd, Ord)]
pub enum KernelType {
    /// CPU mining kernel with no hardware-specific optimizations.
    Unoptimized,

    /// CPU mining kernel using x86/x86_64 SHA instructions
    SHA,
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid kernel type: {0}")]
pub struct InvalidKernelType(String);

impl FromStr for KernelType {
    type Err = InvalidKernelType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_ref() {
            "unoptimized" => Self::Unoptimized,
            "sha" => Self::SHA,
            s => return Err(InvalidKernelType(s.to_string())),
        })
    }
}

impl KernelType {
    pub fn mine_with(self, context: Context) {
        match self {
            Self::Unoptimized => context.mine(kernels::Unoptimized),
            Self::SHA => context.mine(kernels::SHA),
        }
    }
}

impl Default for KernelType {
    fn default() -> Self {
        KernelType::Unoptimized
    }
}

impl Display for KernelType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let name = match self {
            Self::Unoptimized => "unoptimized",
            Self::SHA => "SHA",
        };

        write!(f, "{}", name)
    }
}

#[derive(Debug)]
pub struct CpuInfo {
    threads: usize,
    supported: EnumSet<KernelType>,
}

impl Display for CpuInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "CPU:\n\
            \tThreads: {threads}\n\
            \tSupported kernels: {supported}\n\
            \tUnsupported kernels: {available}",
            threads = self.threads,
            supported = self.supported.iter().join(", "),
            available = (!self.supported).iter().join(", "),
        )
    }
}

fn get_supported_kernels() -> EnumSet<KernelType> {
    let mut supported = EnumSet::only(KernelType::Unoptimized);

    if is_x86_feature_detected!("sha") {
        supported |= KernelType::SHA;
    }

    supported
}

fn get_best_kernel() -> KernelType {
    Iterator::max(get_supported_kernels().iter()).unwrap_or_default()
}

pub fn get_cpu_info() -> CpuInfo {
    let threads = num_cpus::get();
    let supported = get_supported_kernels();

    CpuInfo { threads, supported }
}

pub struct CpuMiner {
    kernel_type: KernelType,
    threads: usize,
}

impl CpuMiner {
    pub fn new(
        &MinerConfig {
            cpu_threads,
            cpu_kernel,
            ..
        }: &MinerConfig,
    ) -> CpuMiner {
        CpuMiner {
            threads: cpu_threads.unwrap_or_else(num_cpus::get),
            kernel_type: cpu_kernel.unwrap_or_else(get_best_kernel),
        }
    }
}

impl Miner for CpuMiner {
    fn describe(&self) -> String {
        format!("CPU [{}x {}]", self.threads, self.kernel_type)
    }

    fn mine(self: Box<Self>, mut interface: MinerInterface) -> Result<(), MinerError> {
        let Self {
            threads,
            kernel_type,
        } = *self;
        // todo: investigate using evc to avoid locks, or parking_lot for better locks?
        let hashes = AtomicU64::new(0);
        let target = AtomicCell::new(interface.current_target().into_raw());
        let (sol_tx, sol_rx) = crossbeam::channel::bounded(1);

        // convert bindings to references to avoid lifetime/ownership complications
        let hashes = &hashes;
        let target = &target;
        let sol_tx = &sol_tx;

        crossbeam::scope(|s| {
            let address = interface.address();
            let mut offset = Wrapping(rand::random());

            for i in 0..threads {
                offset += Wrapping(std::u64::MAX / (threads as u64));
                let ctx = Context::new(address, hashes, target, offset.0, sol_tx);
                s.builder()
                    .name(format!("CPU miner {}", i))
                    .spawn(move |_| {
                        thread_priority::set_low_priority();
                        kernel_type.mine_with(ctx);
                    })
                    .unwrap();
            }

            // management thread
            s.builder()
                .name("CPU miner dispatch".to_string())
                .spawn(|_| {
                    let mut cycle_start = Instant::now();

                    loop {
                        match sol_rx.recv_timeout(Duration::from_millis(1000)) {
                            Ok(s) => {
                                if interface.report_solution(s).is_err() {
                                    target.store(None);
                                    break;
                                }
                            }
                            Err(RecvTimeoutError::Disconnected) => break,
                            Err(RecvTimeoutError::Timeout) => {}
                        }

                        match interface.current_target() {
                            CurrentTarget::Unchanged(_) => {}
                            t => target.store(t.into_raw()),
                        }

                        let cycle_time =
                            std::mem::replace(&mut cycle_start, Instant::now()).elapsed();
                        let cycle_hashes = hashes.swap(0, Ordering::Relaxed);
                        interface.report_speed(cycle_hashes, cycle_time);
                    }
                })
                .unwrap();
        })
        .unwrap();

        Ok(())
    }
}
