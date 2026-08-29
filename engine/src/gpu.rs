//! GPU management.

pub mod resource;

use crate::lock_order::OrderedRwLock;
use anyhow::Result;
use impact_camera::CameraManager;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    immediate::MAX_TOTAL_IMMEDIATE_SIZE,
    resource_group::GPUResourceGroupManager,
    shader::ShaderManager,
    storage::StorageGPUBufferManager,
    texture::mipmap::MipmapperGenerator,
    timestamp_query::{
        TimestampQueryManager,
        external::{ExternalGPUProfiler, TracyGPUProfiler},
    },
    vertex_attribute_ranges, wgpu,
};
use impact_light::LightManager;
use impact_particle::{ParticleEmitterManager, material::ParticleMaterialRegistry};
use impact_rendering::surface::RenderingSurface;
use impact_scene::model::ModelInstanceManager;
use impact_voxel::VoxelObjectManager;
use parking_lot::RwLock;
use resource::GPUResourceManager;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, sync::Arc};

/// Interface to a graphics device and a surface that can be rendered to.
#[derive(Debug)]
pub struct GraphicsContext {
    pub device: GraphicsDevice,
    pub surface: RenderingSurface,
}

/// Manages resources for generic GPU operations.
#[derive(Debug)]
pub struct GraphicsSystem {
    graphics_device: Arc<GraphicsDevice>,
    timestamp_query_manager: RwLock<TimestampQueryManager>,
    shader_manager: RwLock<ShaderManager>,
    gpu_resource_manager: RwLock<GPUResourceManager>,
    gpu_resource_group_manager: RwLock<GPUResourceGroupManager>,
    storage_gpu_buffer_manager: RwLock<StorageGPUBufferManager>,
    bind_group_layout_registry: BindGroupLayoutRegistry,
    mipmapper_generator: Arc<MipmapperGenerator>,
    config: GraphicsConfig,
}

/// Graphics configuration options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsConfig {
    pub timings_enabled: bool,
}

impl GraphicsSystem {
    /// Creates managers for all generic GPU resources.
    pub fn new(graphics_device: Arc<GraphicsDevice>, mut config: GraphicsConfig) -> Result<Self> {
        config.make_compatible_with_device(&graphics_device);

        let mut timestamp_query_manager = TimestampQueryManager::new(
            &graphics_device,
            NonZeroU32::new(128).unwrap(),
            config.timings_enabled,
        );

        timestamp_query_manager.set_external_profiler(if cfg!(feature = "tracy") {
            ExternalGPUProfiler::Tracy(TracyGPUProfiler::new(&graphics_device, Some("Engine"))?)
        } else {
            ExternalGPUProfiler::None
        });

        let shader_manager = ShaderManager::new();

        let gpu_resource_manager = GPUResourceManager::new(&graphics_device);

        let gpu_resource_group_manager = GPUResourceGroupManager::new();

        let storage_gpu_buffer_manager = StorageGPUBufferManager::new();

        let bind_group_layout_registry = BindGroupLayoutRegistry::new();

        let mipmapper_generator = Arc::new(MipmapperGenerator::new(
            &graphics_device,
            MipmapperGenerator::DEFAULT_SUPPORTED_FORMATS,
        ));

        Ok(Self {
            graphics_device,
            timestamp_query_manager: RwLock::new(timestamp_query_manager),
            shader_manager: RwLock::new(shader_manager),
            gpu_resource_manager: RwLock::new(gpu_resource_manager),
            gpu_resource_group_manager: RwLock::new(gpu_resource_group_manager),
            storage_gpu_buffer_manager: RwLock::new(storage_gpu_buffer_manager),
            bind_group_layout_registry,
            mipmapper_generator,
            config,
        })
    }

    /// Returns a reference to the graphics device.
    pub fn graphics_device(&self) -> &GraphicsDevice {
        &self.graphics_device
    }

    /// Returns a reference to the [`TimestampQueryManager`], guarded by a
    /// [`RwLock`].
    pub fn timestamp_query_manager(&self) -> &RwLock<TimestampQueryManager> {
        &self.timestamp_query_manager
    }

    /// Returns a reference to the [`ShaderManager`], guarded by a [`RwLock`].
    pub fn shader_manager(&self) -> &RwLock<ShaderManager> {
        &self.shader_manager
    }

    /// Returns a reference to the [`GPUResourceManager`], guarded by a
    /// [`RwLock`].
    pub fn gpu_resource_manager(&self) -> &RwLock<GPUResourceManager> {
        &self.gpu_resource_manager
    }

    /// Returns a reference to the [`GPUResourceGroupManager`], guarded by a
    /// [`RwLock`].
    pub fn gpu_resource_group_manager(&self) -> &RwLock<GPUResourceGroupManager> {
        &self.gpu_resource_group_manager
    }

    /// Returns a reference to the [`StorageGPUBufferManager`], guarded by a
    /// [`RwLock`].
    pub fn storage_gpu_buffer_manager(&self) -> &RwLock<StorageGPUBufferManager> {
        &self.storage_gpu_buffer_manager
    }

    /// Returns a reference to the [`BindGroupLayoutRegistry`].
    pub fn bind_group_layout_registry(&self) -> &BindGroupLayoutRegistry {
        &self.bind_group_layout_registry
    }

    /// Returns a reference counting pointer to the [`MipmapperGenerator`].
    pub fn mipmapper_generator(&self) -> &Arc<MipmapperGenerator> {
        &self.mipmapper_generator
    }

    /// Returns the graphics configuration parameters.
    pub fn config(&self) -> &GraphicsConfig {
        &self.config
    }

    /// Records and submits commands for synchronizing dynamic GPU resources
    /// (resources that benefit from a staging belt).
    pub fn sync_dynamic_gpu_resources(
        &self,
        camera_manager: &CameraManager,
        light_manager: &LightManager,
        particle_material_registry: &ParticleMaterialRegistry,
        particle_emitter_manager: &ParticleEmitterManager,
        voxel_object_manager: &mut VoxelObjectManager,
        model_instance_manager: &mut ModelInstanceManager,
        effective_time_step_duration: f32,
    ) {
        let mut command_encoder =
            Self::create_resource_sync_command_encoder(self.graphics_device.device());

        let mut gpu_resource_manager_lock = self.gpu_resource_manager.owrite();
        let gpu_resource_manager = &mut **gpu_resource_manager_lock;

        camera_manager.sync_gpu_resources(
            &self.graphics_device,
            &mut gpu_resource_manager.staging_belt,
            &mut command_encoder,
            &self.bind_group_layout_registry,
            &mut gpu_resource_manager.camera,
        );

        // TODO: All light uniform GPU buffers are updated every frame in
        // practice because the primitive change detection in the uniform
        // buffers gets triggered even when the contents have not changed.
        light_manager.sync_gpu_resources(
            &self.graphics_device,
            &mut gpu_resource_manager.staging_belt,
            &mut command_encoder,
            &self.bind_group_layout_registry,
            &mut gpu_resource_manager.lights,
        );

        particle_emitter_manager.sync_simulation_gpu_resources(
            particle_material_registry,
            &self.graphics_device,
            &mut gpu_resource_manager.staging_belt,
            &mut command_encoder,
            &self.bind_group_layout_registry,
            effective_time_step_duration,
            &mut gpu_resource_manager.particle_simulations,
        );

        voxel_object_manager.sync_gpu_resources(
            &self.graphics_device,
            &mut gpu_resource_manager.staging_belt,
            &mut command_encoder,
            &self.bind_group_layout_registry,
            model_instance_manager,
            &mut gpu_resource_manager.voxel_objects,
        );

        // TODO: Most instance feature GPU buffers are also updated every frame
        // because the instance feature buffers are cleared and repopulated
        // every frame.
        model_instance_manager.sync_gpu_buffers(
            &self.graphics_device,
            &mut gpu_resource_manager.staging_belt,
            &mut command_encoder,
            &mut gpu_resource_manager.model_instance_buffers,
        );

        gpu_resource_manager
            .staging_belt
            .finish_and_recall_on_submit(&command_encoder);

        drop(gpu_resource_manager_lock);

        self.graphics_device
            .queue()
            .submit([command_encoder.finish()]);
    }

    /// Loads the timestamps recorded during graphics operations. This method
    /// will wait for the GPU to finish its work.
    pub fn load_recorded_timing_results(&self) -> Result<()> {
        let mut timestamp_query_manager = self.timestamp_query_manager.owrite();
        timestamp_query_manager.load_recorded_timing_results(&self.graphics_device)?;
        timestamp_query_manager.reset();
        Ok(())
    }

    pub fn set_gpu_pass_timings_enabled(&mut self, enabled: bool) {
        if enabled
            && !self
                .graphics_device()
                .supports_features(wgpu::Features::TIMESTAMP_QUERY)
        {
            log::warn!("Not enabling timestamp queries due to missing graphics device features");
            return;
        }

        let was_enabled = self.config.timings_enabled;

        if enabled != was_enabled {
            self.config.timings_enabled = enabled;
            self.timestamp_query_manager.owrite().set_enabled(enabled);
        }
    }

    fn create_resource_sync_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Resource synchronization command encoder"),
        })
    }
}

impl GraphicsConfig {
    /// Adjusts the configuration parameters to avoid using features not
    /// supported by the given graphics device.
    pub fn make_compatible_with_device(&mut self, graphics_device: &GraphicsDevice) {
        if self.timings_enabled
            && !graphics_device.supports_features(wgpu::Features::TIMESTAMP_QUERY)
        {
            log::warn!("Disabling timestamp queries due to missing device features");
            self.timings_enabled = false;
        }
    }
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            timings_enabled: false,
        }
    }
}

/// Connects to a graphics device and creates a headless rendering surface
/// (backed by an ordinary texture instead of a window) with the given
/// dimensions in physical pixels.
///
/// # Errors
/// See [`GraphicsDevice::connect`].
pub fn initialize_for_headless_rendering(
    width: NonZeroU32,
    height: NonZeroU32,
) -> Result<GraphicsContext> {
    let wgpu_instance = create_wgpu_instance(None);

    let mut rendering_surface = RenderingSurface::new_headless(width, height);

    let graphics_device =
        connect_to_graphics_device_for_rendering(&wgpu_instance, &mut rendering_surface)?;

    Ok(GraphicsContext {
        device: graphics_device,
        surface: rendering_surface,
    })
}

/// Creates a rendering surface for the given window, connects to a graphics
/// device compatible with the surface and initializes the surface for
/// presentation through the connected graphics device.
///
/// # Errors
/// See [`RenderingSurface::new_for_window`] and [`GraphicsDevice::connect`].
#[cfg(feature = "window")]
pub fn initialize_for_window_rendering(window: &crate::window::Window) -> Result<GraphicsContext> {
    let display_handle = window.display_handle().clone();
    let wgpu_instance = create_wgpu_instance(Some(Box::new(display_handle)));

    let mut rendering_surface =
        RenderingSurface::new_for_window(&wgpu_instance, std::sync::Arc::new(window.clone()))?;

    let graphics_device =
        connect_to_graphics_device_for_rendering(&wgpu_instance, &mut rendering_surface)?;

    Ok(GraphicsContext {
        device: graphics_device,
        surface: rendering_surface,
    })
}

/// Creates a new instance of `wgpu`.
pub fn create_wgpu_instance(
    display: Option<Box<dyn wgpu::wgt::WgpuHasDisplayHandle + 'static>>,
) -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        display,
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        backend_options: wgpu::BackendOptions::default(),
    })
}

/// Connects to a graphics device compatible with the given surface and
/// initializes the surface for presentation through the connected graphics
/// device.
///
/// # Errors
/// See [`GraphicsDevice::connect`].
pub fn connect_to_graphics_device_for_rendering(
    wgpu_instance: &wgpu::Instance,
    rendering_surface: &mut RenderingSurface,
) -> Result<GraphicsDevice> {
    let graphics_device = pollster::block_on(GraphicsDevice::connect(
        wgpu_instance,
        wgpu::Features::IMMEDIATES
            | wgpu::Features::DEPTH32FLOAT_STENCIL8
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::FLOAT32_FILTERABLE
            | wgpu::Features::INDIRECT_FIRST_INSTANCE
            | wgpu::Features::DEPTH_CLIP_CONTROL,
        wgpu::Features::POLYGON_MODE_LINE
            | wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
        wgpu::Limits {
            max_bind_groups: 7,
            max_immediate_size: MAX_TOTAL_IMMEDIATE_SIZE as u32,
            max_color_attachment_bytes_per_sample: 64,
            max_vertex_attributes: vertex_attribute_ranges::TOTAL_LOCATIONS,
            ..wgpu::Limits::default()
        },
        wgpu::MemoryHints::Performance,
        rendering_surface.presentable_surface(),
    ))?;

    rendering_surface.initialize_for_device(&graphics_device)?;

    Ok(graphics_device)
}
