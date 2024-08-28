//! Graphics rendering.

pub mod brdf;
pub mod postprocessing;
pub mod render_command;
pub mod resource;
pub mod surface;
pub mod tasks;

use crate::{
    geometry::CubemapFace,
    gpu::{
        query::{self, TimestampQueryManager},
        resource_group::GPUResourceGroupManager,
        shader::ShaderManager,
        storage::StorageGPUBufferManager,
        texture::{self, attachment::RenderAttachmentTextureManager, mipmap::MipmapperGenerator},
        GraphicsDevice,
    },
    light::MAX_SHADOW_MAP_CASCADES,
    material::MaterialLibrary,
    window::EventLoopController,
};
use anyhow::{anyhow, Error, Result};
use chrono::Utc;
use postprocessing::{
    ambient_occlusion::AmbientOcclusionConfig, capturing::CapturingCameraConfig,
    temporal_anti_aliasing::TemporalAntiAliasingConfig, Postprocessor,
};
use render_command::RenderCommandManager;
use resource::RenderResourceManager;
use std::{
    mem,
    num::NonZeroU32,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};
use surface::RenderingSurface;

/// Floating point type used for rendering.
///
/// # Note
/// Changing this would also require additional code changes where the type is
/// hardcoded.
#[allow(non_camel_case_types)]
pub type fre = f32;

/// Container for data and systems required for rendering.
#[derive(Debug)]
pub struct RenderingSystem {
    config: RenderingConfig,
    graphics_device: Arc<GraphicsDevice>,
    rendering_surface: RenderingSurface,
    surface_texture_to_present: Option<wgpu::SurfaceTexture>,
    mipmapper_generator: Arc<MipmapperGenerator>,
    shader_manager: RwLock<ShaderManager>,
    render_resource_manager: RwLock<RenderResourceManager>,
    render_command_manager: RwLock<RenderCommandManager>,
    render_attachment_texture_manager: RwLock<RenderAttachmentTextureManager>,
    gpu_resource_group_manager: RwLock<GPUResourceGroupManager>,
    storage_gpu_buffer_manager: RwLock<StorageGPUBufferManager>,
    postprocessor: RwLock<Postprocessor>,
    frame_counter: u32,
    timestamp_query_manager: TimestampQueryManager,
}

/// Global rendering configuration options.
#[derive(Clone, Debug)]
pub struct RenderingConfig {
    /// The width and height of each face of the omnidirectional light shadow
    /// cubemap in number of texels.
    pub omnidirectional_light_shadow_map_resolution: u32,
    /// The width and height of the unidirectional light shadow map in number of
    /// texels.
    pub unidirectional_light_shadow_map_resolution: u32,
    /// Whether shadow mapping is enabled.
    pub shadow_mapping_enabled: bool,
    pub ambient_occlusion: AmbientOcclusionConfig,
    pub temporal_anti_aliasing: TemporalAntiAliasingConfig,
    pub capturing_camera: CapturingCameraConfig,
    pub timings_enabled: bool,
}

/// Helper for capturing screenshots and related textures.
#[derive(Debug)]
pub struct ScreenCapturer {
    screenshot_save_requested: AtomicBool,
    omnidirectional_light_shadow_map_save_requested: AtomicBool,
    unidirectional_light_shadow_map_save_requested: AtomicBool,
}

impl RenderingSystem {
    /// Creates a new rendering system using the given configuration, graphics
    /// device and rendering surface.
    pub fn new(
        config: RenderingConfig,
        graphics_device: Arc<GraphicsDevice>,
        rendering_surface: RenderingSurface,
    ) -> Result<Self> {
        let mipmapper_generator = Arc::new(MipmapperGenerator::new(
            &graphics_device,
            MipmapperGenerator::DEFAULT_SUPPORTED_FORMATS,
        ));

        let mut shader_manager = ShaderManager::new();

        let mut render_attachment_texture_manager = RenderAttachmentTextureManager::new(
            &graphics_device,
            &rendering_surface,
            &mipmapper_generator,
        );

        let render_command_manager = RenderCommandManager::new(
            &graphics_device,
            &mut shader_manager,
            &mut render_attachment_texture_manager,
        );

        let mut gpu_resource_group_manager = GPUResourceGroupManager::new();

        let mut storage_gpu_buffer_manager = StorageGPUBufferManager::new();

        let postprocessor = Postprocessor::new(
            &graphics_device,
            &rendering_surface,
            &mut shader_manager,
            &mut render_attachment_texture_manager,
            &mut gpu_resource_group_manager,
            &mut storage_gpu_buffer_manager,
            &config.ambient_occlusion,
            &config.temporal_anti_aliasing,
            &config.capturing_camera,
        )?;

        let timestamp_query_manager = TimestampQueryManager::new(
            &graphics_device,
            NonZeroU32::new(64).unwrap(),
            config.timings_enabled,
        );

        Ok(Self {
            config,
            graphics_device,
            rendering_surface,
            surface_texture_to_present: None,
            mipmapper_generator,
            shader_manager: RwLock::new(shader_manager),
            render_resource_manager: RwLock::new(RenderResourceManager::new()),
            render_command_manager: RwLock::new(render_command_manager),
            render_attachment_texture_manager: RwLock::new(render_attachment_texture_manager),
            gpu_resource_group_manager: RwLock::new(gpu_resource_group_manager),
            storage_gpu_buffer_manager: RwLock::new(storage_gpu_buffer_manager),
            postprocessor: RwLock::new(postprocessor),
            frame_counter: 1,
            timestamp_query_manager,
        })
    }

    /// Returns a reference to the global rendering configuration.
    pub fn config(&self) -> &RenderingConfig {
        &self.config
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

    /// Presents the last surface texture that was rendered to.
    pub fn present(&mut self) {
        if let Some(surface_texture) = self.surface_texture_to_present.take() {
            surface_texture.present();
            self.frame_counter = self.frame_counter.wrapping_add(1);
        }
    }

    /// Renders to the surface using the current synchronized render resources.
    /// The surface texture to present is stored for later presentation by
    /// calling [`Self::present`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The surface texture to render to can not be obtained.
    /// - Recording a render pass fails.
    pub fn render_to_surface(&mut self, material_library: &MaterialLibrary) -> Result<()> {
        with_timing_info_logging!("Rendering"; {
            self.surface_texture_to_present = Some(self.render_surface(material_library)?);
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

    /// Marks the render resources as being out of sync with the source data.
    pub fn declare_render_resources_desynchronized(&self) {
        self.render_resource_manager
            .write()
            .unwrap()
            .declare_desynchronized();
    }

    /// Toggles shadow mapping.
    pub fn toggle_shadow_mapping(&mut self) {
        self.config.shadow_mapping_enabled = !self.config.shadow_mapping_enabled;
    }

    /// Toggles ambient occlusion.
    pub fn toggle_ambient_occlusion(&self) {
        self.postprocessor
            .write()
            .unwrap()
            .toggle_ambient_occlusion();
    }

    /// Toggles bloom.
    pub fn toggle_bloom(&self) {
        self.postprocessor
            .write()
            .unwrap()
            .capturing_camera_mut()
            .toggle_bloom();
    }

    /// Cycle tone mapping.
    pub fn cycle_tone_mapping(&self) {
        self.postprocessor
            .write()
            .unwrap()
            .capturing_camera_mut()
            .cycle_tone_mapping();
    }

    /// Toggles visualization of render attachments.
    pub fn toggle_render_attachment_visualization(&self) {
        self.postprocessor
            .write()
            .unwrap()
            .toggle_render_attachment_visualization();
    }

    /// Changes the visualized render attachment quantity to the next quantity
    /// in the list, or wraps around.
    pub fn cycle_visualized_render_attachment_quantity_forward(&self) {
        self.postprocessor
            .write()
            .unwrap()
            .cycle_visualized_render_attachment_quantity_forward();
    }

    /// Changes the visualized render attachment quantity to the previous
    /// quantity in the list, or wraps around.
    pub fn cycle_visualized_render_attachment_quantity_backward(&self) {
        self.postprocessor
            .write()
            .unwrap()
            .cycle_visualized_render_attachment_quantity_backward();
    }

    /// Toggle render pass timings.
    pub fn toggle_timings(&mut self) {
        self.config.timings_enabled = !self.config.timings_enabled;
        self.timestamp_query_manager
            .set_enabled(self.config.timings_enabled);
    }

    /// Returns the size of the push constant containing `self.frame_counter`.
    pub const fn frame_counter_push_constant_size() -> u32 {
        mem::size_of::<u32>() as u32
    }

    fn render_surface(
        &mut self,
        material_library: &MaterialLibrary,
    ) -> Result<wgpu::SurfaceTexture> {
        self.render_attachment_texture_manager
            .write()
            .unwrap()
            .swap_previous_and_current_attachment_variants(&self.graphics_device);

        let surface_texture = self.rendering_surface.surface().get_current_texture()?;
        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut timestamp_recorder = self
            .timestamp_query_manager
            .create_timestamp_query_registry();

        let mut command_encoder =
            Self::create_render_command_encoder(self.graphics_device.device());

        self.render_command_manager.read().unwrap().record(
            &self.rendering_surface,
            &surface_texture_view,
            material_library,
            self.render_resource_manager.read().unwrap().synchronized(),
            &self.render_attachment_texture_manager.read().unwrap(),
            &self.gpu_resource_group_manager.read().unwrap(),
            &self.storage_gpu_buffer_manager.read().unwrap(),
            &self.postprocessor.read().unwrap(),
            &self.config,
            self.frame_counter,
            &mut timestamp_recorder,
            &mut command_encoder,
        )?;

        timestamp_recorder.finish(&mut command_encoder);

        self.graphics_device
            .queue()
            .submit(std::iter::once(command_encoder.finish()));

        let timing_results = self
            .timestamp_query_manager
            .load_recorded_timing_results(&self.graphics_device)?;

        query::print_timing_results(&timing_results);

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

    fn handle_render_error(&self, error: Error, _event_loop_controller: &EventLoopController<'_>) {
        if let Some(wgpu::SurfaceError::Lost) = error.downcast_ref() {
            // Reconfigure surface if lost
            self.rendering_surface
                .configure_surface_for_device(self.graphics_device());
        }
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
            .recreate_textures(
                &self.graphics_device,
                &self.rendering_surface,
                &self.mipmapper_generator,
            );
    }
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            omnidirectional_light_shadow_map_resolution: 1024,
            unidirectional_light_shadow_map_resolution: 1024,
            shadow_mapping_enabled: true,
            ambient_occlusion: AmbientOcclusionConfig::default(),
            temporal_anti_aliasing: TemporalAntiAliasingConfig::default(),
            capturing_camera: CapturingCameraConfig::default(),
            timings_enabled: false,
        }
    }
}

impl ScreenCapturer {
    /// Creates a new screen capturer.
    ///
    /// # Panics
    /// When a screenshot is captured, a panic will occur if the width times the
    /// number of bytes per pixel is not a multiple of 256.
    pub fn new() -> Self {
        Self {
            screenshot_save_requested: AtomicBool::new(false),
            omnidirectional_light_shadow_map_save_requested: AtomicBool::new(false),
            unidirectional_light_shadow_map_save_requested: AtomicBool::new(false),
        }
    }

    /// Schedule a screenshot capture for the next
    /// [`Self::save_screenshot_if_requested`] call.
    pub fn request_screenshot_save(&self) {
        self.screenshot_save_requested
            .store(true, Ordering::Release);
    }

    /// Schedule a capture of the omnidirectional light shadow map texture for
    /// the next [`Self::save_omnidirectional_light_shadow_map_if_requested`]
    /// call.
    pub fn request_omnidirectional_light_shadow_map_save(&self) {
        self.omnidirectional_light_shadow_map_save_requested
            .store(true, Ordering::Release);
    }

    /// Schedule a capture of the unidirectional light shadow map texture for
    /// the next [`Self::save_unidirectional_light_shadow_map_if_requested`]
    /// call.
    pub fn request_unidirectional_light_shadow_map_save(&self) {
        self.unidirectional_light_shadow_map_save_requested
            .store(true, Ordering::Release);
    }

    /// Checks if a screenshot capture was scheduled with
    /// [`Self::request_screenshot_save`], and if so, captures a screenshot and
    /// saves it as a timestamped PNG file in the current directory.
    pub fn save_screenshot_if_requested(&self, renderer: &RwLock<RenderingSystem>) -> Result<()> {
        if self
            .screenshot_save_requested
            .swap(false, Ordering::Acquire)
        {
            let renderer = renderer.read().unwrap();

            let surface_texture = renderer
                .surface_texture_to_present
                .as_ref()
                .ok_or_else(|| anyhow!("No unpresented surface to save as screenshot"))?;

            texture::save_texture_as_image_file(
                renderer.graphics_device(),
                &surface_texture.texture,
                0,
                0,
                format!("screenshot_{}.png", Utc::now().to_rfc3339()),
            )?;
        }

        Ok(())
    }

    /// Checks if a omnidirectional light shadow map capture was scheduled with
    /// [`Self::request_omnidirectional_light_shadow_map_save`], and if so,
    /// captures the textures and saves them as timestamped PNG files in the
    /// current directory.
    pub fn save_omnidirectional_light_shadow_maps_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
    ) -> Result<()> {
        if self
            .omnidirectional_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            let renderer = renderer.read().unwrap();

            let render_resource_manager = renderer.render_resource_manager().read().unwrap();

            if let Some(light_buffer_manager) = render_resource_manager
                .synchronized()
                .get_light_buffer_manager()
            {
                for (light_idx, texture) in light_buffer_manager
                    .omnidirectional_light_shadow_map_manager()
                    .textures()
                    .iter()
                    .enumerate()
                {
                    for face in CubemapFace::all() {
                        texture.save_face_as_image_file(
                            renderer.graphics_device(),
                            face,
                            format!(
                                "omnidirectional_light_{}_shadow_map_{:?}_{}.png",
                                light_idx,
                                face,
                                Utc::now().to_rfc3339(),
                            ),
                        )?;
                    }
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    /// Checks if a unidirectional light shadow map capture was scheduled with
    /// [`Self::request_unidirectional_light_shadow_map_save`], and if so,
    /// captures the textures and saves them as timestamped PNG files in the
    /// current directory.
    pub fn save_unidirectional_light_shadow_maps_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
    ) -> Result<()> {
        if self
            .unidirectional_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            let renderer = renderer.read().unwrap();

            let render_resource_manager = renderer.render_resource_manager().read().unwrap();

            if let Some(light_buffer_manager) = render_resource_manager
                .synchronized()
                .get_light_buffer_manager()
            {
                for (light_idx, texture) in light_buffer_manager
                    .unidirectional_light_shadow_map_manager()
                    .textures()
                    .iter()
                    .enumerate()
                {
                    for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                        texture.save_cascade_as_image_file(
                            renderer.graphics_device(),
                            cascade_idx,
                            format!(
                                "unidirectional_light_{}_shadow_map_{}_{}.png",
                                light_idx,
                                cascade_idx,
                                Utc::now().to_rfc3339(),
                            ),
                        )?;
                    }
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

impl Default for ScreenCapturer {
    fn default() -> Self {
        Self::new()
    }
}
