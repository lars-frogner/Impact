//! Graphics rendering.

pub mod render_command;
pub mod resource;
pub mod screen_capture;

use crate::{
    lock_order::OrderedRwLock, resource::ResourceManager, tasks::RenderToSurface, ui::UserInterface,
};
use anyhow::Result;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    resource_group::GPUResourceGroupManager,
    shader::ShaderManager,
    storage::StorageGPUBufferManager,
    texture::mipmap::MipmapperGenerator,
    timestamp_query::{
        TimestampQueryManager,
        external::{ExternalGPUProfiler, TracyGPUProfiler},
    },
    wgpu,
};
use impact_light::{LightManager, shadow_map::ShadowMappingConfig};
use impact_rendering::{
    BasicRenderingConfig,
    attachment::RenderAttachmentTextureManager,
    postprocessing::{
        Postprocessor, ambient_occlusion::AmbientOcclusionConfig, capturing::CapturingCameraConfig,
        temporal_anti_aliasing::TemporalAntiAliasingConfig,
    },
    surface::RenderingSurface,
};
use impact_scene::{camera::CameraManager, model::ModelInstanceManager};
use impact_scheduling::{Task, TaskErrors};
use impact_voxel::VoxelObjectManager;
use parking_lot::RwLock;
use render_command::RenderCommandManager;
use resource::RenderResourceManager;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, sync::Arc};

/// Container for data and systems required for rendering.
#[derive(Debug)]
pub struct RenderingSystem {
    graphics_device: Arc<GraphicsDevice>,
    rendering_surface: RenderingSurface,
    surface_texture_to_present: Option<wgpu::SurfaceTexture>,
    mipmapper_generator: Arc<MipmapperGenerator>,
    render_resource_manager: RwLock<RenderResourceManager>,
    shader_manager: RwLock<ShaderManager>,
    render_attachment_texture_manager: RwLock<RenderAttachmentTextureManager>,
    gpu_resource_group_manager: RwLock<GPUResourceGroupManager>,
    storage_gpu_buffer_manager: RwLock<StorageGPUBufferManager>,
    postprocessor: RwLock<Postprocessor>,
    bind_group_layout_registry: BindGroupLayoutRegistry,
    staging_belt: wgpu::util::StagingBelt,
    render_command_manager: RenderCommandManager,
    timestamp_query_manager: TimestampQueryManager,
    frame_counter: u64,
    basic_config: BasicRenderingConfig,
    shadow_mapping_config: ShadowMappingConfig,
}

/// Rendering configuration options.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderingConfig {
    pub basic: BasicRenderingConfig,
    pub shadow_mapping: ShadowMappingConfig,
    pub ambient_occlusion: AmbientOcclusionConfig,
    pub temporal_anti_aliasing: TemporalAntiAliasingConfig,
    pub capturing_camera: CapturingCameraConfig,
}

impl RenderingSystem {
    /// Creates a new rendering system using the given configuration, graphics
    /// device and rendering surface.
    pub fn new(
        mut config: RenderingConfig,
        graphics_device: Arc<GraphicsDevice>,
        rendering_surface: RenderingSurface,
        resource_manager: &ResourceManager,
    ) -> Result<Self> {
        config.basic.make_compatible_with_device(&graphics_device);

        let mipmapper_generator = Arc::new(MipmapperGenerator::new(
            &graphics_device,
            MipmapperGenerator::DEFAULT_SUPPORTED_FORMATS,
        ));

        let mut shader_manager = ShaderManager::new();

        let mut render_attachment_texture_manager =
            RenderAttachmentTextureManager::new(&graphics_device, &rendering_surface);

        let bind_group_layout_registry = BindGroupLayoutRegistry::new();

        let render_command_manager = RenderCommandManager::new(
            &graphics_device,
            &rendering_surface,
            &mut shader_manager,
            &mut render_attachment_texture_manager,
            resource_manager,
            &bind_group_layout_registry,
            &config.basic,
        );

        let mut gpu_resource_group_manager = GPUResourceGroupManager::new();

        let mut storage_gpu_buffer_manager = StorageGPUBufferManager::new();

        let postprocessor = Postprocessor::new(
            config.ambient_occlusion,
            config.temporal_anti_aliasing,
            config.capturing_camera,
            &graphics_device,
            &rendering_surface,
            &mut shader_manager,
            &mut render_attachment_texture_manager,
            &mut gpu_resource_group_manager,
            &mut storage_gpu_buffer_manager,
            &bind_group_layout_registry,
        )?;

        let mut timestamp_query_manager = TimestampQueryManager::new(
            &graphics_device,
            NonZeroU32::new(128).unwrap(),
            config.basic.timings_enabled,
        );

        timestamp_query_manager.set_external_profiler(if cfg!(feature = "tracy") {
            ExternalGPUProfiler::Tracy(TracyGPUProfiler::new(&graphics_device, Some("Rendering"))?)
        } else {
            ExternalGPUProfiler::None
        });

        Ok(Self {
            graphics_device,
            rendering_surface,
            surface_texture_to_present: None,
            mipmapper_generator,
            render_resource_manager: RwLock::new(RenderResourceManager::new()),
            shader_manager: RwLock::new(shader_manager),
            render_attachment_texture_manager: RwLock::new(render_attachment_texture_manager),
            gpu_resource_group_manager: RwLock::new(gpu_resource_group_manager),
            storage_gpu_buffer_manager: RwLock::new(storage_gpu_buffer_manager),
            postprocessor: RwLock::new(postprocessor),
            bind_group_layout_registry,
            staging_belt: wgpu::util::StagingBelt::new(1024 * 1024),
            render_command_manager,
            timestamp_query_manager,
            frame_counter: 0,
            basic_config: config.basic,
            shadow_mapping_config: config.shadow_mapping,
        })
    }

    /// Returns a reference to the graphics device used for rendering.
    pub fn graphics_device(&self) -> &GraphicsDevice {
        &self.graphics_device
    }

    /// Returns a reference to the rendering surface.
    pub fn rendering_surface(&self) -> &RenderingSurface {
        &self.rendering_surface
    }

    /// Returns a reference counting pointer to the [`MipmapperGenerator`].
    pub fn mipmapper_generator(&self) -> &Arc<MipmapperGenerator> {
        &self.mipmapper_generator
    }

    /// Returns a reference to the [`RenderResourceManager`], guarded
    /// by a [`RwLock`].
    pub fn render_resource_manager(&self) -> &RwLock<RenderResourceManager> {
        &self.render_resource_manager
    }

    /// Returns a reference to the [`ShaderManager`], guarded by a [`RwLock`].
    pub fn shader_manager(&self) -> &RwLock<ShaderManager> {
        &self.shader_manager
    }

    /// Returns a reference to the [`RenderAttachmentTextureManager`], guarded
    /// by a [`RwLock`].
    pub fn render_attachment_texture_manager(&self) -> &RwLock<RenderAttachmentTextureManager> {
        &self.render_attachment_texture_manager
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

    /// Returns a reference to the [`Postprocessor`], guarded by a [`RwLock`].
    pub fn postprocessor(&self) -> &RwLock<Postprocessor> {
        &self.postprocessor
    }

    /// Returns a reference to the [`BindGroupLayoutRegistry`].
    pub fn bind_group_layout_registry(&self) -> &BindGroupLayoutRegistry {
        &self.bind_group_layout_registry
    }

    /// Returns a reference to the [`RenderCommandManager`].
    pub fn render_command_manager(&self) -> &RenderCommandManager {
        &self.render_command_manager
    }

    /// Returns a reference to the [`TimestampQueryManager`].
    pub fn timestamp_query_manager(&self) -> &TimestampQueryManager {
        &self.timestamp_query_manager
    }

    pub fn is_initial_frame(&self) -> bool {
        self.frame_counter == 0
    }

    pub fn mark_initial_frame_over(&mut self) {
        if self.is_initial_frame() {
            self.frame_counter = 1;
        }
    }

    pub fn basic_config(&self) -> &BasicRenderingConfig {
        &self.basic_config
    }

    pub fn shadow_mapping_config(&self) -> &ShadowMappingConfig {
        &self.shadow_mapping_config
    }

    pub fn shadow_mapping_enabled_mut(&mut self) -> &mut bool {
        &mut self.shadow_mapping_config.enabled
    }

    /// Presents the last surface texture that was rendered to. Does nothing if
    /// there is no texture to present.
    pub fn present(&mut self) {
        if let Some(surface_texture) = self.surface_texture_to_present.take() {
            surface_texture.present();
        }
    }

    /// Records and submits commands for synchronizing dynamic GPU resources
    /// (resources that benefit from a staging belt).
    pub fn sync_dynamic_gpu_resources(
        &mut self,
        camera_manager: &CameraManager,
        light_manager: &LightManager,
        voxel_object_manager: &mut VoxelObjectManager,
        model_instance_manager: &mut ModelInstanceManager,
    ) {
        let mut command_encoder =
            Self::create_render_command_encoder(self.graphics_device.device());

        let mut render_resource_manager = self.render_resource_manager.owrite();

        camera_manager.sync_gpu_resources(
            &self.graphics_device,
            &mut self.staging_belt,
            &mut command_encoder,
            &self.bind_group_layout_registry,
            &mut render_resource_manager.camera,
        );

        // TODO: All light uniform GPU buffers are updated every frame in
        // practice because the primitive change detection in the uniform
        // buffers gets triggered even when the contents have not changed.
        light_manager.sync_gpu_resources(
            &self.graphics_device,
            &mut self.staging_belt,
            &mut command_encoder,
            &self.bind_group_layout_registry,
            &mut render_resource_manager.lights,
            &self.shadow_mapping_config,
        );

        voxel_object_manager.sync_gpu_resources(
            &self.graphics_device,
            &mut self.staging_belt,
            &mut command_encoder,
            &self.bind_group_layout_registry,
            model_instance_manager,
            &mut render_resource_manager.voxel_objects,
        );

        // TODO: Most instance feature GPU buffers are also updated every frame
        // because the instance feature buffers are cleared and repopulated
        // every frame.
        model_instance_manager.sync_gpu_buffers(
            &self.graphics_device,
            &mut self.staging_belt,
            &mut command_encoder,
            &mut render_resource_manager.model_instance_buffers,
        );

        drop(render_resource_manager);

        self.staging_belt.finish();

        self.graphics_device
            .queue()
            .submit(std::iter::once(command_encoder.finish()));
    }

    /// Synchronizes the render commands with the current render resources.
    ///
    /// # Errors
    /// Returns an error if synchronization fails due to missing resources.
    pub fn synchronize_render_commands(&mut self) -> Result<()> {
        let render_resource_manager = self.render_resource_manager.oread();
        let mut shader_manager = self.shader_manager.owrite();
        self.render_command_manager.sync_with_render_resources(
            &self.graphics_device,
            &mut shader_manager,
            &**render_resource_manager,
            &self.bind_group_layout_registry,
        )
    }

    /// Records and submits render commands that do not require the surface
    /// texture to be available.
    ///
    /// # Errors
    /// Returns an error if recording a render commands fails.
    pub fn render_before_surface(&mut self) -> Result<()> {
        self.render_attachment_texture_manager
            .owrite()
            .swap_previous_and_current_attachment_variants(&self.graphics_device);

        let mut timestamp_recorder = self
            .timestamp_query_manager
            .create_timestamp_query_registry();

        let mut command_encoder =
            Self::create_render_command_encoder(self.graphics_device.device());

        self.render_command_manager.record_before_surface(
            &self.rendering_surface,
            &**self.render_resource_manager.oread(),
            &self.render_attachment_texture_manager.oread(),
            &self.gpu_resource_group_manager.oread(),
            &self.storage_gpu_buffer_manager.oread(),
            &self.postprocessor.oread(),
            &self.shadow_mapping_config,
            self.frame_counter as u32,
            &mut timestamp_recorder,
            &mut command_encoder,
        )?;

        timestamp_recorder.finish(&mut command_encoder);

        self.graphics_device
            .queue()
            .submit(std::iter::once(command_encoder.finish()));

        Ok(())
    }

    /// Obtains a view of the surface texture, along with the
    /// [`wgpu::SurfaceTexture`] to present after rendering if the surface is
    /// attached to a window. This function may have to wait for the next
    /// surface texture from the swap chain to be ready.
    ///
    /// # Errors
    /// Return an error if obtaining the surface texture fails.
    pub fn obtain_surface(&mut self) -> Result<(wgpu::TextureView, Option<wgpu::SurfaceTexture>)> {
        self.rendering_surface
            .get_texture_view_with_presentable_texture()
    }

    /// Records and submits the final render commands that write into the
    /// surface texture. The surface texture to present (if any) is stored for
    /// later presentation by calling [`Self::present`].
    ///
    /// # Errors
    /// Returns an error if recording a render commands fails.
    pub fn render_to_surface(
        &mut self,
        surface_texture_view: wgpu::TextureView,
        surface_texture: Option<wgpu::SurfaceTexture>,
        user_interface: &dyn UserInterface,
    ) -> Result<()> {
        let mut timestamp_recorder = self
            .timestamp_query_manager
            .create_timestamp_query_registry();

        let mut command_encoder =
            Self::create_render_command_encoder(self.graphics_device.device());

        self.render_command_manager.record_with_surface(
            &self.rendering_surface,
            &surface_texture_view,
            &**self.render_resource_manager.oread(),
            &self.render_attachment_texture_manager.oread(),
            &self.gpu_resource_group_manager.oread(),
            &self.postprocessor.oread(),
            self.frame_counter as u32,
            &mut timestamp_recorder,
            &mut command_encoder,
        )?;

        user_interface.render(
            &self.graphics_device,
            &self.rendering_surface,
            &surface_texture_view,
            &mut timestamp_recorder,
            &mut command_encoder,
        )?;

        timestamp_recorder.finish(&mut command_encoder);

        self.graphics_device
            .queue()
            .submit(std::iter::once(command_encoder.finish()));

        self.staging_belt.recall();

        self.surface_texture_to_present = surface_texture;
        self.frame_counter = self.frame_counter.wrapping_add(1);

        Ok(())
    }

    /// Loads the timestamps recorded during rendering. Call after
    /// [`Self::render_to_surface`]. This method will wait for the GPU to finish
    /// rendering.
    pub fn load_recorded_timing_results(&mut self) -> Result<()> {
        self.timestamp_query_manager
            .load_recorded_timing_results(&self.graphics_device)?;
        self.timestamp_query_manager.reset();
        Ok(())
    }

    /// Updates the exposure based on the current settings and potentially the
    /// average incident luminance. Call after [`Self::render_to_surface`]. This
    /// method will wait for the GPU to finish rendering if it needs the average
    /// incident luminance.
    pub fn update_exposure(&self) -> Result<()> {
        let storage_gpu_buffer_manager = self.storage_gpu_buffer_manager.oread();
        let mut postprocessor = self.postprocessor.owrite();
        postprocessor
            .capturing_camera_mut()
            .update_exposure(&self.graphics_device, &storage_gpu_buffer_manager)
    }

    /// Sets a new width and height for the rendering surface and any textures
    /// that need to have the same dimensions as the surface.
    pub fn resize_rendering_surface(&mut self, new_width: NonZeroU32, new_height: NonZeroU32) {
        self.rendering_surface
            .resize(&self.graphics_device, new_width, new_height);

        self.recreate_render_attachment_textures();
    }

    pub fn update_pixels_per_point(&mut self, pixels_per_point: f64) {
        self.rendering_surface
            .update_pixels_per_point(pixels_per_point);
    }

    pub fn set_wireframe_mode_enabled(&mut self, enabled: bool) {
        if enabled
            && !self
                .graphics_device()
                .supports_features(wgpu::Features::POLYGON_MODE_LINE)
        {
            impact_log::warn!(
                "Not enabling wireframe mode due to missing graphics device features"
            );
            return;
        }

        let was_enabled = self.basic_config.wireframe_mode_on;

        if enabled != was_enabled {
            self.basic_config.wireframe_mode_on = enabled;
            self.sync_render_command_manager_with_basic_config();
        }
    }

    pub fn set_render_pass_timings_enabled(&mut self, enabled: bool) {
        if enabled
            && !self
                .graphics_device()
                .supports_features(wgpu::Features::TIMESTAMP_QUERY)
        {
            impact_log::warn!(
                "Not enabling timestamp queries due to missing graphics device features"
            );
            return;
        }

        let was_enabled = self.basic_config.timings_enabled;

        if enabled != was_enabled {
            self.basic_config.timings_enabled = enabled;
            self.timestamp_query_manager.set_enabled(enabled);
        }
    }

    fn handle_surface_lost(&self) {
        self.rendering_surface
            .reinitialize_lost_surface(self.graphics_device());
    }

    fn create_render_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render encoder"),
        })
    }

    fn sync_render_command_manager_with_basic_config(&mut self) {
        self.render_command_manager.sync_with_config(
            &self.graphics_device,
            &self.shader_manager.oread(),
            &self.basic_config,
        );
    }

    fn recreate_render_attachment_textures(&mut self) {
        self.render_attachment_texture_manager
            .owrite()
            .recreate_textures(&self.graphics_device, &self.rendering_surface);
    }

    /// Identifies rendering-related errors that need special handling in the
    /// given set of task errors and handles them.
    pub fn handle_task_errors(&self, task_errors: &mut TaskErrors) {
        if let Some(render_error) = task_errors.get_error_of(RenderToSurface.id())
            && let Some(wgpu::SurfaceError::Lost) = render_error.downcast_ref()
        {
            self.handle_surface_lost();
            task_errors.clear_error_of(RenderToSurface.id());
        }
    }
}
