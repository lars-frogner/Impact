//! Graphics rendering.

pub mod command;
pub mod postprocessing;
pub mod render_command;
pub mod resource;
pub mod screen_capture;

use crate::{scene::Scene, tasks::Render, ui::UserInterface};
use anyhow::Result;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice,
    query::TimestampQueryManager, resource_group::GPUResourceGroupManager, shader::ShaderManager,
    storage::StorageGPUBufferManager, texture::mipmap::MipmapperGenerator, wgpu,
};
use impact_light::shadow_map::ShadowMappingConfig;
use impact_rendering::{
    BasicRenderingConfig,
    attachment::RenderAttachmentTextureManager,
    postprocessing::{
        Postprocessor, ambient_occlusion::AmbientOcclusionConfig, capturing::CapturingCameraConfig,
        temporal_anti_aliasing::TemporalAntiAliasingConfig,
    },
    surface::RenderingSurface,
};
use impact_scheduling::Task;
use impact_thread::ThreadPoolTaskErrors;
use render_command::RenderCommandManager;
use resource::RenderResourceManager;
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroU32,
    sync::{Arc, RwLock},
};

/// Container for data and systems required for rendering.
#[derive(Debug)]
pub struct RenderingSystem {
    graphics_device: Arc<GraphicsDevice>,
    rendering_surface: RenderingSurface,
    surface_texture_to_present: Option<wgpu::SurfaceTexture>,
    mipmapper_generator: Arc<MipmapperGenerator>,
    shader_manager: RwLock<ShaderManager>,
    bind_group_layout_registry: BindGroupLayoutRegistry,
    render_resource_manager: RwLock<RenderResourceManager>,
    render_command_manager: RwLock<RenderCommandManager>,
    render_attachment_texture_manager: RwLock<RenderAttachmentTextureManager>,
    gpu_resource_group_manager: RwLock<GPUResourceGroupManager>,
    storage_gpu_buffer_manager: RwLock<StorageGPUBufferManager>,
    postprocessor: RwLock<Postprocessor>,
    frame_counter: u32,
    timestamp_query_manager: TimestampQueryManager,
    basic_config: BasicRenderingConfig,
    shadow_mapping_config: ShadowMappingConfig,
}

/// Rendering configuration options.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RenderingConfig {
    #[serde(default)]
    pub basic: BasicRenderingConfig,
    #[serde(default)]
    pub shadow_mapping: ShadowMappingConfig,
    #[serde(default)]
    pub ambient_occlusion: AmbientOcclusionConfig,
    #[serde(default)]
    pub temporal_anti_aliasing: TemporalAntiAliasingConfig,
    #[serde(default)]
    pub capturing_camera: CapturingCameraConfig,
}

impl RenderingSystem {
    /// Creates a new rendering system using the given configuration, graphics
    /// device and rendering surface.
    pub fn new(
        mut config: RenderingConfig,
        graphics_device: Arc<GraphicsDevice>,
        rendering_surface: RenderingSurface,
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

        let timestamp_query_manager = TimestampQueryManager::new(
            &graphics_device,
            NonZeroU32::new(128).unwrap(),
            config.basic.timings_enabled,
        );

        Ok(Self {
            graphics_device,
            rendering_surface,
            surface_texture_to_present: None,
            mipmapper_generator,
            shader_manager: RwLock::new(shader_manager),
            bind_group_layout_registry,
            render_resource_manager: RwLock::new(RenderResourceManager::new()),
            render_command_manager: RwLock::new(render_command_manager),
            render_attachment_texture_manager: RwLock::new(render_attachment_texture_manager),
            gpu_resource_group_manager: RwLock::new(gpu_resource_group_manager),
            storage_gpu_buffer_manager: RwLock::new(storage_gpu_buffer_manager),
            postprocessor: RwLock::new(postprocessor),
            frame_counter: 1,
            timestamp_query_manager,
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

    /// Returns a reference to the [`ShaderManager`], guarded by a [`RwLock`].
    pub fn shader_manager(&self) -> &RwLock<ShaderManager> {
        &self.shader_manager
    }

    /// Returns a reference to the [`BindGroupLayoutRegistry`].
    pub fn bind_group_layout_registry(&self) -> &BindGroupLayoutRegistry {
        &self.bind_group_layout_registry
    }

    /// Returns a reference to the [`RenderResourceManager`], guarded
    /// by a [`RwLock`].
    pub fn render_resource_manager(&self) -> &RwLock<RenderResourceManager> {
        &self.render_resource_manager
    }

    /// Returns a reference to the [`RenderCommandManager`], guarded by a
    /// [`RwLock`].
    pub fn render_command_manager(&self) -> &RwLock<RenderCommandManager> {
        &self.render_command_manager
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

    /// Returns a reference to the [`TimestampQueryManager`].
    pub fn timestamp_query_manager(&self) -> &TimestampQueryManager {
        &self.timestamp_query_manager
    }

    /// The frame count wraps around after [`u32::MAX`].
    pub fn current_frame_count(&self) -> u32 {
        self.frame_counter
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
            self.frame_counter = self.frame_counter.wrapping_add(1);
        }
    }

    /// Renders to the surface using the current synchronized render resources.
    /// The surface texture to present (if any) is stored for later presentation
    /// by calling [`Self::present`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The surface texture to render to can not be obtained.
    /// - Recording a render pass fails.
    pub fn render_to_surface(
        &mut self,
        scene: &Scene,
        user_interface: &dyn UserInterface,
    ) -> Result<()> {
        impact_log::with_timing_info_logging!("Rendering"; {
            self.surface_texture_to_present = self.render_surface(scene, user_interface)?;
        });
        Ok(())
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

    /// Marks the render resources as being out of sync with the source data.
    pub fn declare_render_resources_desynchronized(&self) {
        self.render_resource_manager
            .write()
            .unwrap()
            .declare_desynchronized();
    }

    fn render_surface(
        &mut self,
        scene: &Scene,
        user_interface: &dyn UserInterface,
    ) -> Result<Option<wgpu::SurfaceTexture>> {
        self.render_attachment_texture_manager
            .write()
            .unwrap()
            .swap_previous_and_current_attachment_variants(&self.graphics_device);

        let (surface_texture_view, surface_texture) = self
            .rendering_surface
            .get_texture_view_with_presentable_texture()?;

        let mut timestamp_recorder = self
            .timestamp_query_manager
            .create_timestamp_query_registry();

        let mut command_encoder =
            Self::create_render_command_encoder(self.graphics_device.device());

        let light_storage = scene.light_storage().read().unwrap();
        let material_library = scene.material_library().read().unwrap();
        let instance_feature_manager = scene.instance_feature_manager().read().unwrap();
        let scene_camera = scene.scene_camera().read().unwrap();

        self.render_command_manager.read().unwrap().record(
            &self.rendering_surface,
            &surface_texture_view,
            &light_storage,
            &material_library,
            &instance_feature_manager,
            scene_camera.as_ref(),
            self.render_resource_manager.read().unwrap().synchronized(),
            &self.render_attachment_texture_manager.read().unwrap(),
            &self.gpu_resource_group_manager.read().unwrap(),
            &self.storage_gpu_buffer_manager.read().unwrap(),
            &self.postprocessor.read().unwrap(),
            &self.shadow_mapping_config,
            self.frame_counter,
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

        self.timestamp_query_manager
            .load_recorded_timing_results(&self.graphics_device)?;

        self.postprocessor
            .write()
            .unwrap()
            .capturing_camera_mut()
            .update_exposure(
                &self.graphics_device,
                &self.storage_gpu_buffer_manager.read().unwrap(),
            )?;

        Ok(surface_texture)
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

    fn recreate_render_attachment_textures(&mut self) {
        self.render_attachment_texture_manager
            .write()
            .unwrap()
            .recreate_textures(&self.graphics_device, &self.rendering_surface);
    }

    /// Identifies rendering-related errors that need special handling in the
    /// given set of task errors and handles them.
    pub fn handle_task_errors(&self, task_errors: &mut ThreadPoolTaskErrors) {
        if let Some(render_error) = task_errors.get_error_of(Render.id()) {
            if let Some(wgpu::SurfaceError::Lost) = render_error.downcast_ref() {
                self.handle_surface_lost();
                task_errors.clear_error_of(Render.id());
            }
        }
    }
}
