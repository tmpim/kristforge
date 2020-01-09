use crate::krist::address::Address;
use crate::miner::interface::{CurrentTarget, MinerInterface};
use crate::miner::{Miner, MinerConfig, MinerError};
use crossbeam::atomic::AtomicCell;
use crossbeam::channel::{RecvTimeoutError, Sender};
use enumset::{EnumSet, EnumSetType};
use itertools::Itertools;
use multiversion::target_clones;
use std::fmt::{self, Display, Formatter};
use std::num::Wrapping;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

fn mine_core(input: [u8; 22], work: u64, nonce: Wrapping<u64>) -> Option<[u8; 11]> {
    // initialize hash inputs
    let mut text = [0u8; 64];

    // fill first 22 bytes of hash input
    (&mut text[..22]).copy_from_slice(&input);

    // TODO: implement this lol

    None
}

fn mine(
    address: Address,
    hashes: &AtomicU64,
    target: &AtomicCell<Option<([u8; 12], u64)>>,
    mut nonce: Wrapping<u64>,
    sol_tx: &Sender<String>,
) {
    use self::mine_core;

    const HASHES_BATCH_SIZE: u64 = 1000;

    let mut input = [0u8; 22];
    (&mut input[..10]).copy_from_slice(address.as_bytes());

    while let Some((block, work)) = target.load() {
        (&mut input[10..]).copy_from_slice(&block);

        for _ in 0..HASHES_BATCH_SIZE {
            nonce += Wrapping(1);
            if let Some(s) = mine_core(input, work, nonce) {
                sol_tx.send(String::from_utf8(s.to_vec()).unwrap()).unwrap();
            }
        }

        hashes.fetch_add(HASHES_BATCH_SIZE, Ordering::Relaxed);
    }
}

#[derive(EnumSetType, Debug)]
pub enum InstructionSets {
    AVX,
    AVX2,
    SHA,
}

impl Display for InstructionSets {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let name = match self {
            Self::AVX => "AVX",
            Self::AVX2 => "AVX2",
            Self::SHA => "SHA",
        };

        write!(f, "{}", name)
    }
}

pub struct CpuInfo {
    threads: usize,
    instructions: EnumSet<InstructionSets>,
}

impl Display for CpuInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "CPU:\n\
            \tThreads: {threads}\n\
            \tInstruction sets: {instructions}",
            threads = self.threads,
            instructions = self.instructions.iter().join(", ")
        )
    }
}

pub fn get_cpu_info() -> CpuInfo {
    let threads = num_cpus::get();

    let mut instructions = EnumSet::new();

    if is_x86_feature_detected!("avx") {
        instructions |= InstructionSets::AVX;
    }

    if is_x86_feature_detected!("avx2") {
        instructions |= InstructionSets::AVX2;
    }

    if is_x86_feature_detected!("sha") {
        instructions |= InstructionSets::SHA;
    }

    CpuInfo {
        threads,
        instructions,
    }
}

pub struct CpuMiner {
    threads: usize,
}

impl CpuMiner {
    pub fn new(&MinerConfig { cpu_threads, .. }: &MinerConfig) -> CpuMiner {
        CpuMiner {
            threads: cpu_threads.unwrap_or_else(num_cpus::get),
        }
    }
}

impl Miner for CpuMiner {
    fn describe(&self) -> &str {
        "CPU"
    }

    fn mine(self: Box<Self>, mut interface: MinerInterface) -> Result<(), MinerError> {
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

            for i in 0..self.threads {
                offset += Wrapping(std::u64::MAX / (self.threads as u64));
                s.builder()
                    .name(format!("CPU miner {}", i))
                    .spawn(move |_| {
                        mine(address, hashes, target, offset, sol_tx);
                    })
                    .unwrap();
            }

            // management thread
            s.builder()
                .name("CPU miner dispatch".to_string())
                .spawn(|_| {
                    let mut cycle_start = Instant::now();

                    loop {
                        match sol_rx.recv_timeout(Duration::from_millis(250)) {
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
