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
        resource_group::GPUResourceGroupManager,
        shader::ShaderManager,
        storage::StorageGPUBufferManager,
        texture::{
            self,
            attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
            mipmap::MipmapperGenerator,
        },
        GraphicsDevice,
    },
    light::MAX_SHADOW_MAP_CASCADES,
    material::MaterialLibrary,
    scene::Scene,
    window::EventLoopController,
};
use anyhow::{Error, Result};
use chrono::Utc;
use postprocessing::{
    ambient_occlusion::AmbientOcclusionConfig, capturing::CapturingCameraConfig, Postprocessor,
};
use render_command::RenderCommandManager;
use render_command::RenderCommandOutcome;
use resource::RenderResourceManager;
use std::{
    num::NonZeroU32,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
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
}

/// Global rendering configuration options.
#[derive(Clone, Debug)]
pub struct RenderingConfig {
    /// The face culling mode.
    pub cull_mode: Option<wgpu::Face>,
    /// Controls the way each polygon is rasterized.
    pub polygon_mode: wgpu::PolygonMode,
    /// The width and height of each face of the omnidirectional light shadow
    /// cubemap in number of texels.
    pub omnidirectional_light_shadow_map_resolution: u32,
    /// The width and height of the unidirectional light shadow map in number of
    /// texels.
    pub unidirectional_light_shadow_map_resolution: u32,
    /// Whether shadow mapping is enabled.
    pub shadow_mapping_enabled: bool,
    /// The number of samples to use for multisampling anti-aliasing.
    pub multisampling_sample_count: u32,
    pub ambient_occlusion: AmbientOcclusionConfig,
    pub capturing_camera: CapturingCameraConfig,
}

/// Helper for capturing screenshots and related textures.
#[derive(Debug)]
pub struct ScreenCapturer {
    screenshot_width: NonZeroU32,
    screenshot_save_requested: AtomicBool,
    render_attachment_save_requested: AtomicBool,
    render_attachment_quantity: AtomicU8,
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

        let render_attachment_texture_manager = RenderAttachmentTextureManager::new(
            &graphics_device,
            &rendering_surface,
            &mipmapper_generator,
            config.multisampling_sample_count,
        );

        let mut gpu_resource_group_manager = GPUResourceGroupManager::new();

        let mut storage_gpu_buffer_manager = StorageGPUBufferManager::new();

        let postprocessor = Postprocessor::new(
            &graphics_device,
            &mut shader_manager,
            &render_attachment_texture_manager,
            &mut gpu_resource_group_manager,
            &mut storage_gpu_buffer_manager,
            &config.ambient_occlusion,
            &config.capturing_camera,
        );

        Ok(Self {
            config,
            graphics_device,
            rendering_surface,
            surface_texture_to_present: None,
            mipmapper_generator,
            shader_manager: RwLock::new(shader_manager),
            render_resource_manager: RwLock::new(RenderResourceManager::new()),
            render_command_manager: RwLock::new(RenderCommandManager::new()),
            render_attachment_texture_manager: RwLock::new(render_attachment_texture_manager),
            gpu_resource_group_manager: RwLock::new(gpu_resource_group_manager),
            storage_gpu_buffer_manager: RwLock::new(storage_gpu_buffer_manager),
            postprocessor: RwLock::new(postprocessor),
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

    /// Returns a reference to the [`RenderAttachmentTextureManager`], guarded by a
    /// [`RwLock`].
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

    /// Toggles culling of triangle back faces in all render passes.
    pub fn toggle_back_face_culling(&mut self) {
        if self.config.cull_mode.is_some() {
            self.config.cull_mode = None;
        } else {
            self.config.cull_mode = Some(wgpu::Face::Back);
        }
        // Remove all render pass recorders so that they will be recreated with
        // the updated configuration
        self.render_command_manager
            .write()
            .unwrap()
            .clear_recorders();
    }

    /// Toggles rendering of triangle fill in all render passes. Only triangle
    /// edges will be rendered if fill is turned off.
    pub fn toggle_triangle_fill(&mut self) {
        if self.config.polygon_mode != wgpu::PolygonMode::Fill {
            self.config.polygon_mode = wgpu::PolygonMode::Fill;
        } else {
            self.config.polygon_mode = wgpu::PolygonMode::Line;
        }
        // Remove all render pass recorders so that they will be recreated with
        // the updated configuration
        self.render_command_manager
            .write()
            .unwrap()
            .clear_recorders();
    }

    /// Toggles shadow mapping.
    pub fn toggle_shadow_mapping(&mut self) {
        self.config.shadow_mapping_enabled = !self.config.shadow_mapping_enabled;
    }

    pub fn cycle_msaa(&mut self) {
        let sample_count = match self.config.multisampling_sample_count {
            1 => 4,
            _ => 1,
        };
        self.set_multisampling_sample_count(sample_count);
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

        let mut command_encoder =
            Self::create_render_command_encoder(self.graphics_device.device());

        let mut n_recorded_passes = 0;

        {
            let render_resources_guard = self.render_resource_manager.read().unwrap();
            for render_pass_recorder in self.render_command_manager.read().unwrap().recorders() {
                let outcome = render_pass_recorder.record(
                    &self.rendering_surface,
                    &surface_texture_view,
                    material_library,
                    render_resources_guard.synchronized(),
                    &self.render_attachment_texture_manager.read().unwrap(),
                    &self.gpu_resource_group_manager.read().unwrap(),
                    &self.storage_gpu_buffer_manager.read().unwrap(),
                    &self.postprocessor.read().unwrap(),
                    &mut command_encoder,
                )?;
                if outcome == RenderCommandOutcome::Recorded {
                    n_recorded_passes += 1;
                }
            }
        } // <- Render resource guard is released here

        log::info!("Performing {} render passes", n_recorded_passes);

        self.graphics_device
            .queue()
            .submit(std::iter::once(command_encoder.finish()));

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

    fn set_multisampling_sample_count(&mut self, sample_count: u32) {
        if self.config.multisampling_sample_count != sample_count {
            log::info!("MSAA sample count changed to {}", sample_count);

            self.config.multisampling_sample_count = sample_count;

            self.recreate_render_attachment_textures();

            // Remove all render command recorders so that they will be
            // recreated with the updated configuration
            self.render_command_manager
                .write()
                .unwrap()
                .clear_recorders();
        }
    }

    fn recreate_render_attachment_textures(&mut self) {
        self.render_attachment_texture_manager
            .write()
            .unwrap()
            .recreate_textures(
                &self.graphics_device,
                &self.rendering_surface,
                &self.mipmapper_generator,
                self.config.multisampling_sample_count,
            );

        self.postprocessor
            .write()
            .unwrap()
            .capturing_camera_mut()
            .handle_new_render_attachment_textures(
                &self.graphics_device,
                &mut self.shader_manager.write().unwrap(),
                &self.render_attachment_texture_manager.read().unwrap(),
                &mut self.gpu_resource_group_manager.write().unwrap(),
                &mut self.storage_gpu_buffer_manager.write().unwrap(),
            );

        self.render_command_manager
            .write()
            .unwrap()
            .clear_recorders();
    }
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            omnidirectional_light_shadow_map_resolution: 1024,
            unidirectional_light_shadow_map_resolution: 1024,
            shadow_mapping_enabled: true,
            multisampling_sample_count: 1,
            ambient_occlusion: AmbientOcclusionConfig::default(),
            capturing_camera: CapturingCameraConfig::default(),
        }
    }
}

impl ScreenCapturer {
    /// Creates a new screen capturer that will use the given width when saving
    /// screenshots. The height will be determined automatically to match the
    /// aspect ratio of the rendering surface.
    ///
    /// # Panics
    /// When a screenshot is captured, a panic will occur if the width times the
    /// number of bytes per pixel is not a multiple of 256.
    pub fn new(screenshot_width: NonZeroU32) -> Self {
        Self {
            screenshot_width,
            screenshot_save_requested: AtomicBool::new(false),
            render_attachment_save_requested: AtomicBool::new(false),
            render_attachment_quantity: AtomicU8::new(0),
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

    /// Schedule a capture of the render attachment texture for the given
    /// quantity for the next
    /// [`Self::save_render_attachment_quantity_if_requested`]
    /// call.
    pub fn request_render_attachment_quantity_save(&self, quantity: RenderAttachmentQuantity) {
        self.render_attachment_save_requested
            .store(true, Ordering::Release);
        self.render_attachment_quantity
            .store(quantity as u8, Ordering::Release);
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
    pub fn save_screenshot_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
        scene: &RwLock<Scene>,
    ) -> Result<()> {
        if self
            .screenshot_save_requested
            .swap(false, Ordering::Acquire)
        {
            let mut renderer = renderer.write().unwrap();
            let scene = scene.read().unwrap();
            let material_library = scene.material_library().read().unwrap();

            let (original_width, original_height) =
                renderer.rendering_surface().surface_dimensions();

            renderer.resize_rendering_surface(
                self.screenshot_width,
                self.determine_screenshot_height(original_width, original_height),
            );
            {
                // Re-render the surface at the screenshot resolution.
                let surface_texture = renderer.render_surface(&material_library)?;

                texture::save_texture_as_image_file(
                    renderer.graphics_device(),
                    &surface_texture.texture,
                    0,
                    0,
                    format!("screenshot_{}.png", Utc::now().to_rfc3339()),
                )?;
            }
            renderer.resize_rendering_surface(original_width, original_height);
        }
        Ok(())
    }

    /// Checks if a render attachment capture was scheduled with
    /// [`Self::request_render_attachment_quantity_save`], and if so, captures
    /// the requested render attachment texture and saves it as a timestamped
    /// PNG file in the current directory.
    pub fn save_render_attachment_quantity_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
        scene: &RwLock<Scene>,
    ) -> Result<()> {
        if self
            .render_attachment_save_requested
            .swap(false, Ordering::Acquire)
        {
            let quantity = RenderAttachmentQuantity::from_index(
                self.render_attachment_quantity.load(Ordering::Acquire),
            )
            .unwrap();

            let mut renderer = renderer.write().unwrap();
            let scene = scene.read().unwrap();
            let material_library = scene.material_library().read().unwrap();

            let (original_width, original_height) =
                renderer.rendering_surface().surface_dimensions();

            renderer.resize_rendering_surface(
                self.screenshot_width,
                self.determine_screenshot_height(original_width, original_height),
            );
            {
                // Re-render the surface at the screenshot resolution.
                renderer.render_surface(&material_library)?;

                renderer
                    .render_attachment_texture_manager
                    .read()
                    .unwrap()
                    .save_render_attachment_texture_as_image_file(
                        renderer.graphics_device(),
                        quantity,
                        0,
                        format!("{}_{}.png", quantity, Utc::now().to_rfc3339()),
                    )?;
            }
            renderer.resize_rendering_surface(original_width, original_height);
        }
        Ok(())
    }

    /// Checks if a omnidirectional light shadow map capture was scheduled with
    /// [`Self::request_omnidirectional_light_shadow_map_save`], and if so,
    /// captures the texture and saves it as a timestamped PNG file in the
    /// current directory.
    pub fn save_omnidirectional_light_shadow_map_if_requested(
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
                for face in CubemapFace::all() {
                    light_buffer_manager
                        .omnidirectional_light_shadow_map_texture()
                        .save_face_as_image_file(
                            renderer.graphics_device(),
                            face,
                            format!(
                                "omnidirectional_light_shadow_map_{}_{:?}.png",
                                Utc::now().to_rfc3339(),
                                face
                            ),
                        )?;
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
    /// captures the texture and saves it as a timestamped PNG file in the
    /// current directory.
    pub fn save_unidirectional_light_shadow_map_if_requested(
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
                for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                    light_buffer_manager
                        .unidirectional_light_shadow_map_texture()
                        .save_cascade_as_image_file(
                            renderer.graphics_device(),
                            cascade_idx,
                            format!(
                                "unidirectional_light_shadow_map_{}_{}.png",
                                Utc::now().to_rfc3339(),
                                cascade_idx
                            ),
                        )?;
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn determine_screenshot_height(
        &self,
        surface_width: NonZeroU32,
        surface_height: NonZeroU32,
    ) -> NonZeroU32 {
        let aspect_ratio = (u32::from(surface_height) as f32) / (u32::from(surface_width) as f32);
        let screenshot_height =
            f32::round((u32::from(self.screenshot_width) as f32) * aspect_ratio) as u32;
        NonZeroU32::new(u32::max(1, screenshot_height)).unwrap()
    }
}
