use crate::krist::{Address, Target};
use crate::miner::MinerInterface;
use crate::utils::HashRate;
use color_eyre::eyre;
use color_eyre::eyre::WrapErr;
use instant::Instant;
use std::fmt::{self, Debug, Display, Formatter};
use std::mem::{size_of, transmute};
use std::sync::Arc;
use tracing::{debug, info, trace, warn};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Adapter, BackendBit, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferDescriptor, BufferUsage,
    CommandEncoderDescriptor, ComputePassDescriptor, ComputePipelineDescriptor, Device,
    DeviceDescriptor, Features, Limits, Maintain, MapMode, PipelineLayoutDescriptor,
    PushConstantRange, ShaderStage,
};
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

pub struct GpuMiner {
    adapter: Adapter,
}

impl Display for GpuMiner {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let info = self.adapter.get_info();

        write!(
            f,
            "GPU device: {name}\n\tVendor ID: 0x{vendor:x}\n\tBackend: {backend:?}\n\tCompatible: {compatible}",
            name = info.name,
            vendor = info.vendor,
            backend = info.backend,
            compatible = self.is_compatible(),
        )
    }
}

impl Debug for GpuMiner {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("GpuMiner")
            .field("adapter.info", &self.adapter.get_info())
            .field("adapter.features", &self.adapter.features())
            .field("adapter.limits", &self.adapter.limits())
            .finish()
    }
}

// NOTE: must match shader
#[derive(Debug, Clone, AsBytes)]
#[repr(C)]
struct MinerInputs {
    netstate: [u8; 22],
    prefix: [u8; 2],
    work: u64,
}

impl MinerInputs {
    fn new(address: Address, prefix: [u8; 2], target: Target) -> Self {
        let mut netstate = [0u8; 22];
        netstate[..10].copy_from_slice(address.as_bytes());
        let mut inputs = Self {
            netstate,
            prefix,
            work: 0,
        };
        inputs.update_target(target);
        inputs
    }

    fn update_target(&mut self, Target { work, block }: Target) {
        self.work = work;
        self.netstate[10..].copy_from_slice(block.into_hex().as_bytes())
    }
}

// NOTE: must match shader
#[derive(Debug, Clone, FromBytes, AsBytes)]
#[repr(C)]
struct MinerState {
    offset: u32,
    solution: u32,
}

impl MinerState {
    const fn with_offset(offset: u32) -> Self {
        Self {
            offset,
            solution: 0,
        }
    }
}

impl GpuMiner {
    /// Create all compatible vulkan miners.
    #[tracing::instrument(err)]
    pub async fn get_miners() -> eyre::Result<Vec<GpuMiner>> {
        let instance = wgpu::Instance::new(BackendBit::PRIMARY);
        info!(?instance, "Enumerating adapters");

        #[cfg(not(target_arch = "wasm32"))]
        let adapters = instance.enumerate_adapters(BackendBit::PRIMARY);

        #[cfg(target_arch = "wasm32")]
        let adapters = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: None,
                power_preference: wgpu::PowerPreference::HighPerformance,
            })
            .await;

        Ok(adapters
            .into_iter()
            .map(|a| GpuMiner { adapter: a })
            .collect())
    }

    fn required_features() -> Features {
        Features::empty()
    }

    fn required_limits() -> Limits {
        Limits {
            max_bind_groups: 1,
            max_dynamic_uniform_buffers_per_pipeline_layout: 0,
            max_dynamic_storage_buffers_per_pipeline_layout: 0,
            max_sampled_textures_per_shader_stage: 0,
            max_samplers_per_shader_stage: 0,
            max_storage_buffers_per_shader_stage: 1,
            max_storage_textures_per_shader_stage: 0,
            max_uniform_buffers_per_shader_stage: 1,
            max_uniform_buffer_binding_size: size_of::<MinerInputs>() as _,
            max_push_constant_size: 0,
        }
    }

    /// Check whether this device is compatible with kristforge.
    ///
    /// If `false` is returned, mining may not work.
    pub fn is_compatible(&self) -> bool {
        // TODO: also check limits
        self.adapter.features().contains(Self::required_features())
    }

    /// Mine for krist using this device and the given interface.
    #[tracing::instrument(skip(interface), err)]
    pub async fn mine(self, mut interface: impl MinerInterface) -> eyre::Result<()> {
        if !self.is_compatible() {
            warn!("attempting to mine with incompatible device");
        }

        let (device, queue) = self
            .adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Self::required_features(),
                    limits: Self::required_limits(),
                },
                None,
            )
            .await
            .wrap_err("error initializing mining device")?;

        let device = Arc::new(device);

        let cs_module = device.create_shader_module(&wgpu::include_spirv!("kristforge.comp.spv"));

        let input_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("input buffer"),
            size: size_of::<MinerInputs>() as _,
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("state staging buffer"),
            size: size_of::<MinerState>() as _,
            usage: BufferUsage::MAP_READ | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let state_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("state buffer"),
            size: size_of::<MinerState>() as _,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None, // TODO: ???
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: state_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &cs_module,
            entry_point: "main",
        });

        // start task to poll device events - not required on wasm
        // TODO: this may be inefficient, creating a busy loop?
        #[cfg(not(target_arch = "wasm32"))]
        {
            let device = Arc::downgrade(&device);
            tokio::spawn(async move {
                while let Some(device) = device.upgrade() {
                    device.poll(Maintain::Poll);
                    tokio::task::yield_now().await;
                }
            });
        }

        let mut inputs = MinerInputs::new(
            interface.address(),
            interface.prefix().to_be_bytes(),
            interface.target()?,
        );
        let mut state = MinerState::with_offset(0);
        let mut work_size = 64;
        let mut cycle_start = Instant::now();

        loop {
            info!(work_size, "CYCLE START");
            // TODO: only update input buffer when needed
            queue.write_buffer(&input_buffer, 0, inputs.as_bytes());
            queue.write_buffer(&state_buffer, 0, state.as_bytes());

            let mut enc = device.create_command_encoder(&CommandEncoderDescriptor { label: None });
            {
                let mut pass = enc.begin_compute_pass(&ComputePassDescriptor { label: None });
                pass.set_pipeline(&compute_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch(work_size / 8, 8, 1); // TODO
            }

            enc.copy_buffer_to_buffer(
                &state_buffer,
                0,
                &staging_buffer,
                0,
                size_of::<MinerState>() as _,
            );

            queue.submit(Some(enc.finish()));

            let buf_slice = staging_buffer.slice(..);
            let buf_fut = buf_slice.map_async(MapMode::Read);

            buf_fut.await.wrap_err("mapping state buffer")?;
            let data = buf_slice.get_mapped_range();
            state.as_bytes_mut().copy_from_slice(&data);
            drop(data);
            staging_buffer.unmap();

            let time = std::mem::replace(&mut cycle_start, Instant::now()).elapsed();

            interface.hashes_completed(HashRate {
                hashes: work_size as usize,
                elapsed: time,
            });

            let secs = time.as_secs_f32();
            if secs < 0.05 {
                work_size *= 2;
            } else if secs > 0.2 {
                work_size /= 2;
            }
            work_size = work_size.clamp(64, 2u32.pow(30));
            info!(work_size, "CYCLE FINISH");
        }

        Ok(())
    }
}
