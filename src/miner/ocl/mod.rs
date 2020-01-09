use super::MinerError;
use crate::miner::interface::{CurrentTarget, MinerInterface};
use crate::miner::{Miner, MinerConfig};
use dynamic_ocl::buffer::flags::{DeviceReadOnly, DeviceWriteOnly, HostReadOnly, HostWriteOnly};
use dynamic_ocl::buffer::Buffer;
use dynamic_ocl::device::{Device, DeviceType};
use dynamic_ocl::kernel::Kernel;
use dynamic_ocl::platform::Platform;
use dynamic_ocl::program::ProgramBuilder;
use dynamic_ocl::queue::Queue;
use dynamic_ocl::raw::{
    cl_device_info, cl_uchar, cl_uint, cl_ulong, OpenCLVersion, CL_DEVICE_NOT_FOUND,
};
use dynamic_ocl::util::OclInfo;
use dynamic_ocl::{load_opencl, Error as OclError};
use std::cmp::{max, min};
use std::collections::HashSet;
use std::ffi::CString;
use std::fmt::{self, Display, Formatter};
use std::time::Instant;

/// OpenCL kernel source
const OCL_SRC: &str = include_str!("kristforge.cl");

/// An OpenCL device that can be used for mining
#[derive(Debug)]
pub struct MiningDevice {
    device: Device,
    name: String,
    compute_units: cl_uint,
    clock_freq: cl_uint,
}

impl Display for MiningDevice {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "OpenCL device \"{name}\":\n\
            \tCompute units: {compute_units}\n\
            \tClock frequency: {clock_freq}",
            name = self.name,
            compute_units = self.compute_units,
            clock_freq = self.clock_freq
        )
    }
}

fn device_human_name(device: Device) -> Result<CString, OclError> {
    const CL_DEVICE_BOARD_NAME_AMD: cl_device_info = 0x4038;
    if device
        .extensions()?
        .to_string_lossy()
        .contains("cl_amd_device_attribute_query")
    {
        device.get_info(CL_DEVICE_BOARD_NAME_AMD)
    } else {
        device.name()
    }
}

/// Get compatible OpenCL devices
pub fn get_opencl_devices() -> Result<Vec<MiningDevice>, MinerError> {
    match load_opencl() {
        Err(e) => {
            eprintln!(
                "Failed to load OpenCL library; GPU support disabled. Check logs for details.",
            );
            log::error!("OpenCL load error: {:?}", e);
            Ok(vec![])
        }
        Ok(v) if v < OpenCLVersion::CL12 => {
            eprintln!(
                "OpenCL 1.2 is required but system only supports {}; GPU support disabled.",
                v
            );
            log::error!("Unsupported OpenCL version {:?}", v);
            Ok(vec![])
        }
        Ok(_) => {
            let mut devices = HashSet::new();

            for platform in Platform::get_platforms()? {
                let platform_devices = match platform.get_devices(DeviceType::GPU) {
                    Err(OclError::ApiError(e)) if e.code() == CL_DEVICE_NOT_FOUND => vec![],
                    e => e?,
                };
                devices.extend(platform_devices.into_iter());
            }

            let mut wrapped = vec![];

            for device in devices {
                let name = device_human_name(device)?.to_string_lossy().into_owned();
                let compute_units = device.max_compute_units()?;
                let clock_freq = device.max_clock_frequency()?;

                wrapped.push(MiningDevice {
                    device,
                    name,
                    compute_units,
                    clock_freq,
                });
            }

            Ok(wrapped)
        }
    }
}

type MinerKernel = Kernel<(
    Buffer<'static, HostWriteOnly, cl_uchar>,
    cl_ulong,
    cl_ulong,
    Buffer<'static, HostReadOnly, cl_uchar>,
)>;

pub struct OclMiner {
    name: String,
    queue: Queue,
    kernel: MinerKernel,
    max_work_size: usize,
    target_rate: f32,
}

impl OclMiner {
    pub fn new(
        MiningDevice { device, name, .. }: MiningDevice,
        &MinerConfig {
            gpu_rate: target_rate,
            gpu_max_worksize: max_work_size,
            ..
        }: &MinerConfig,
    ) -> Result<Self, MinerError> {
        let ctx = device.create_context()?;
        let queue = ctx.create_queue(device)?;
        let program = ProgramBuilder::with_source(&ctx, &OCL_SRC).build()?;
        let kernel = program.create_kernel(&CString::new("mine").unwrap())?;

        let input_buf = ctx
            .buffer_builder()
            .host_access::<HostWriteOnly>()
            .device_access::<DeviceReadOnly>()
            .build_with_size(22)?;

        let output_buf = ctx
            .buffer_builder()
            .host_access::<HostReadOnly>()
            .device_access::<DeviceWriteOnly>()
            .alloc_host_ptr()
            .build_copying_slice(&[0u8; 11])?;

        let kernel = kernel.bind_arguments((input_buf, 0, 0, output_buf))?;

        Ok(Self {
            name,
            queue,
            kernel,
            max_work_size,
            target_rate,
        })
    }
}

impl Miner for OclMiner {
    fn describe(&self) -> &str {
        &self.name
    }

    fn mine(mut self: Box<Self>, mut interface: MinerInterface) -> Result<(), MinerError> {
        // write the address into first part of input buffer
        self.queue
            .buffer_cmd(&mut self.kernel.arguments().0)
            .write(&interface.address().as_bytes()[..])?;

        let mut work_size = 1usize;
        let mut offset = rand::random();

        let mut cycle_start = Instant::now();

        loop {
            // update miner target
            match interface.current_target() {
                CurrentTarget::StopMining => break,
                CurrentTarget::New(t) => {
                    let (mut input, work, _, _) = self.kernel.arguments();
                    work.set(t.work)?;
                    self.queue
                        .buffer_cmd(&mut input)
                        .offset(10)
                        .write(t.block.into_hex().as_bytes())?;
                }
                CurrentTarget::Unchanged(_) => {}
            };

            // execute kernel
            self.queue
                .kernel_cmd(&mut self.kernel)
                .exec_ndrange(work_size)?;

            // read output and check for solution
            let mut solution = [0u8; 11];
            self.queue
                .buffer_cmd(&mut self.kernel.arguments().3)
                .read(&mut solution)?;

            if solution != [0u8; 11] {
                // solution found!
                let solution = String::from_utf8(Vec::from(&solution[..])).expect("invalid nonce");

                if interface.report_solution(solution).is_err() {
                    break;
                }

                // zero out solution buffer
                self.queue
                    .buffer_cmd(&mut self.kernel.arguments().3)
                    .fill(&0)?;
            }

            let cycle_time = std::mem::replace(&mut cycle_start, Instant::now()).elapsed();

            offset += work_size as u64;
            self.kernel.arguments().2.set(offset)?;
            interface.report_speed(work_size as u64, cycle_time);

            // adjust work size for next execution
            if cycle_time.as_secs_f32() < self.target_rate / 2.0 {
                work_size = min(self.max_work_size, work_size * 2);
            } else if cycle_time.as_secs_f32() > self.target_rate * 2.0 {
                work_size = max(1, work_size / 2);
            }

            //            println!("work size: {} / {}", work_size, self.max_work_size);
        }

        Ok(())
    }
}
