use super::interface::{CurrentTarget, MinerInterface, StopMining};
use super::types::{MinerConfig, Solution};
use crate::ext::DeviceExt;
use crate::krist::address::Address;
use crate::prelude::*;
use ocl::{Buffer, Device, MemFlags, ProQue, Program};
use ocl_extras::full_device_info::FullDeviceInfo;
use sha2::{Digest, Sha256};
use std::cmp::{max, min};
use std::mem::size_of;
use std::time::Instant;

#[derive(Debug)]
pub struct Miner {
    pq: ProQue,
    vecsize: usize,
    rate: f32,
    max_worksize: u32,
    fixed_worksize: Option<u32>,
    address: Address,
}

impl Miner {
    pub fn init(device: Device, cfg: &MinerConfig) -> Fallible<Miner> {
        debug!(
            "Initializing miner on {} with config {:?}",
            device.human_name()?,
            cfg
        );

        let vecsize = match cfg.vector_size {
            Some(v) => v.0 as i32,
            None => device.preferred_vecsize()? as i32,
        };

        let pq = ProQue::builder()
            .platform(device.platform()?.into())
            .device(device)
            .prog_bldr(
                Program::builder()
                    .src(include_str!("kristforge.cl"))
                    .cmplr_def("VECSIZE", vecsize)
                    .clone(),
            )
            .build()?;

        let miner = Miner {
            pq,
            vecsize: vecsize as usize,
            rate: cfg.target_rate,
            max_worksize: cfg.max_worksize,
            address: cfg.address,
            fixed_worksize: cfg.fixed_worksize,
        };

        debug!("Created miner: {:?}", &miner);

        Ok(miner)
    }

    pub fn pq(&self) -> ProQue {
        self.pq.clone()
    }

    pub fn test(&self) -> Fallible<()> {
        debug!("Testing {}", self.pq.device().human_name()?);

        for _ in 0..32 {
            // initialize everything
            let mut inputs = vec![0u8; self.vecsize * 64];
            let mut hashes = vec![0u8; self.vecsize * 32];
            let mut scores = vec![0i64; self.vecsize];

            let mut expected_hashes = Vec::with_capacity(self.vecsize);
            let mut expected_scores = Vec::with_capacity(self.vecsize);

            for i in 0..self.vecsize {
                let input: [u8; 32] = rand::random();

                for (j, &v) in input.iter().enumerate() {
                    inputs[i + (j * self.vecsize)] = v;
                }

                let hash: [u8; 32] = Sha256::digest(&input).into();
                expected_hashes.push(hash);

                let mut score = [0u8; size_of::<i64>()];
                (&mut score[2..]).copy_from_slice(&hash[..6]);
                let score = i64::from_be_bytes(score);
                expected_scores.push(score);
            }

            // create buffers
            let input_buf = Buffer::builder()
                .queue(self.pq.queue().clone())
                .len(inputs.len())
                .copy_host_slice(&inputs)
                .build()?;

            let hash_buf = Buffer::builder()
                .queue(self.pq.queue().clone())
                .len(hashes.len())
                .fill_val(0u8)
                .build()?;

            let score_buf = Buffer::builder()
                .queue(self.pq.queue().clone())
                .len(scores.len())
                .fill_val(0i64)
                .build()?;

            let digest_kernel = self
                .pq
                .kernel_builder("testDigest55")
                .arg(&input_buf)
                .arg(32u32)
                .arg(&hash_buf)
                .build()?;

            let score_kernel = self
                .pq
                .kernel_builder("testScore")
                .arg(&hash_buf)
                .arg(&score_buf)
                .build()?;

            // enqueue kernels
            unsafe {
                digest_kernel.cmd().global_work_size(1).enq()?;
                score_kernel.cmd().global_work_size(1).enq()?;
            }

            // copy data out of buffers
            hash_buf.read(&mut hashes).enq()?;
            score_buf.read(&mut scores).enq()?;

            self.pq.finish()?;

            // check results
            for i in 0..self.vecsize {
                let mut hash = [0u8; 32];
                for j in 0..32 {
                    hash[j] = hashes[i + (j * self.vecsize)];
                }

                ensure!(
                    hash == expected_hashes[i],
                    "hash mismatch: expected {:?}, got {:?}",
                    expected_hashes[i],
                    hash,
                );

                ensure!(
                    scores[i] == expected_scores[i],
                    "score mismatch: expected {:?}, got {:?}",
                    expected_scores[i],
                    scores[i],
                );
            }
        }

        Ok(())
    }

    pub fn start_miner(self, mut interface: MinerInterface) -> Fallible<()> {
        // initialize all the buffers
        let address_buf = Buffer::builder()
            .queue(self.pq.queue().clone())
            .len(10)
            .copy_host_slice(self.address.as_bytes())
            .flags(MemFlags::new().read_only().host_no_access())
            .build()?;

        let block_buf = Buffer::builder()
            .queue(self.pq.queue().clone())
            .len(12)
            .fill_val(0u8)
            .flags(MemFlags::new().read_only().host_write_only())
            .build()?;

        let prefix: [u8; 2] = hex::encode(&[interface.prefix()])
            .as_bytes()
            .try_into()
            .unwrap();

        let prefix_buf = Buffer::builder()
            .queue(self.pq.queue().clone())
            .len(2)
            .copy_host_slice(&prefix)
            .flags(MemFlags::new().read_only().host_no_access())
            .build()?;

        let solution_buf = Buffer::builder()
            .queue(self.pq.queue().clone())
            .len(15)
            .fill_val(0u8)
            .flags(MemFlags::new().read_write())
            .build()?;

        // construct kernel
        let kernel = self
            .pq
            .kernel_builder("kristMiner")
            .arg_named("kristAddress", address_buf)
            .arg_named("block", &block_buf)
            .arg_named("prefix", prefix_buf)
            .arg_named("offset", 0i64)
            .arg_named("work", 0i64)
            .arg_named("solution", &solution_buf)
            .build()?;

        let mut offset = 0i64;
        let mut worksize = match self.fixed_worksize {
            Some(w) => w,
            None => 32,
        };

        loop {
            // check for new target
            match interface.current_target() {
                CurrentTarget::New(target) => {
                    // update state
                    offset = 0;
                    let block_hex = target.block.into_hex();
                    block_buf.write(block_hex.as_bytes()).enq()?;
                    kernel.set_arg("work", target.work as i64)?;
                }
                CurrentTarget::Unchanged(_) => {}
                CurrentTarget::StopMining => break,
            }

            let cycle_start = Instant::now();

            // execute kernel
            kernel.set_arg("offset", offset)?;
            unsafe { kernel.cmd().global_work_size(worksize).enq()? };

            // check if a solution was found
            let mut nonce = [0u8; 15];
            solution_buf.read(&mut nonce[..]).enq()?;

            if nonce != [0u8; 15] {
                // submit solution
                let solution = Solution {
                    address: self.address,
                    nonce,
                };

                match interface.report_solution(solution) {
                    Ok(_) => {}
                    Err(StopMining) => break,
                }

                // clear solution buffer
                solution_buf.cmd().fill(0u8, None).enq()?;
            }

            let cycle_time = cycle_start.elapsed();

            // report speed
            interface.report_speed(worksize, cycle_time);

            // bump offset
            offset += worksize as i64 * self.vecsize as i64;

            // choose new worksize based on time taken

            if self.fixed_worksize.is_none() {
                let mut ratio = self.rate / cycle_time.as_secs_f32();
                if ratio < 0.25 {
                    ratio = 0.25;
                } else if ratio > 4. {
                    ratio = 4.;
                }
                worksize = min(self.max_worksize, (worksize as f32 * ratio) as u32);
                worksize -= worksize % 32;
                worksize = max(worksize, 32);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Miner;
    use crate::ext::DeviceExt;
    use crate::krist::address::Address;
    use crate::miner::selector::Selector;
    use crate::miner::types::MinerConfig;
    use std::str::FromStr;

    #[test]
    #[ignore]
    fn test_miners() {
        let selectors = vec![Selector::All];
        let devices = Selector::select_all(&selectors).unwrap();
        let cfg = MinerConfig {
            address: Address::from_str("kaaaaaaaaa").unwrap(),
            devices: selectors,
            vector_size: None,
            target_rate: 0.1,
            max_worksize: 32,
            fixed_worksize: None,
        };

        for device in devices {
            eprintln!("Testing device {}", device.human_name().unwrap());
            let miner = Miner::init(device, &cfg).unwrap();
            miner.test().unwrap();
        }
    }
}
