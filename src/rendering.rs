//! Graphics rendering.

mod assets;
mod brdf;
mod buffer;
mod camera;
mod core;
mod instance;
mod light;
mod material;
mod mesh;
mod render_pass;
mod resource;
mod shader;
mod tasks;
mod texture;
mod uniform;

pub use self::core::CoreRenderingSystem;
pub use assets::{Assets, TextureID};
pub use brdf::create_specular_ggx_reflectance_lookup_tables;
pub use buffer::{
    create_uniform_buffer_bind_group_layout_entry, create_vertex_buffer_layout_for_instance,
    create_vertex_buffer_layout_for_vertex, UniformBufferable, VertexBufferable,
};
pub use material::{MaterialPropertyTextureManager, MaterialRenderResourceManager};
pub use render_pass::{RenderPassHints, RenderPassManager, SyncRenderPasses};
pub use resource::SyncRenderResources;
pub use shader::{
    AmbientLightShaderInput, AmbientOcclusionCalculationShaderInput, AmbientOcclusionShaderInput,
    BlinnPhongTextureShaderInput, BumpMappingTextureShaderInput, CameraShaderInput,
    DiffuseMicrofacetShadingModel, FixedColorFeatureShaderInput, FixedTextureShaderInput,
    InstanceFeatureShaderInput, LightMaterialFeatureShaderInput, LightShaderInput,
    MaterialShaderInput, MeshShaderInput, MicrofacetShadingModel, MicrofacetTextureShaderInput,
    ModelViewTransformShaderInput, NormalMappingShaderInput, OmnidirectionalLightShaderInput,
    ParallaxMappingShaderInput, PrepassTextureShaderInput, Shader, ShaderGenerator,
    SkyboxTextureShaderInput, SpecularMicrofacetShadingModel, UnidirectionalLightShaderInput,
};
pub use tasks::{Render, RenderingTag};
pub use texture::{
    CascadeIdx, ColorSpace, DepthOrArrayLayers, RenderAttachmentQuantity,
    RenderAttachmentQuantitySet, RenderAttachmentTextureManager, TexelDescription, TexelType,
    Texture, TextureAddressingConfig, TextureConfig, TextureFilteringConfig, TextureLookupTable,
    RENDER_ATTACHMENT_BINDINGS, RENDER_ATTACHMENT_CLEAR_COLORS, RENDER_ATTACHMENT_FLAGS,
    RENDER_ATTACHMENT_FORMATS,
};

use self::{render_pass::RenderPassOutcome, resource::RenderResourceManager};
use crate::{geometry::CubemapFace, scene::MAX_SHADOW_MAP_CASCADES, window::EventLoopController};
use anyhow::{Error, Result};
use chrono::Utc;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    RwLock,
};

/// Floating point type used for rendering.
///
/// # Note
/// Changing this would also require additional
/// code changes where the type is hardcoded.
#[allow(non_camel_case_types)]
pub type fre = f32;

/// Container for all data and logic required for rendering.
#[derive(Debug)]
pub struct RenderingSystem {
    core_system: CoreRenderingSystem,
    config: RenderingConfig,
    assets: RwLock<Assets>,
    render_resource_manager: RwLock<RenderResourceManager>,
    render_pass_manager: RwLock<RenderPassManager>,
    render_attachment_texture_manager: RenderAttachmentTextureManager,
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
    /// Whether ambient occlusion is enabled.
    pub ambient_occlusion_enabled: bool,
}

/// Helper for capturing screenshots and related textures.
#[derive(Debug)]
pub struct ScreenCapturer {
    screenshot_width: u32,
    screenshot_save_requested: AtomicBool,
    render_attachment_save_requested: AtomicBool,
    render_attachment_quantity: AtomicU8,
    omnidirectional_light_shadow_map_save_requested: AtomicBool,
    unidirectional_light_shadow_map_save_requested: AtomicBool,
}

impl RenderingSystem {
    /// Creates a new rendering system consisting of the given core system and
    /// assets.
    pub async fn new(core_system: CoreRenderingSystem, mut assets: Assets) -> Result<Self> {
        let config = RenderingConfig::default();

        let render_attachment_texture_manager = RenderAttachmentTextureManager::new(
            &core_system,
            RenderAttachmentQuantitySet::DEPTH
                | RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::TEXTURE_COORDS
                | RenderAttachmentQuantitySet::COLOR
                | RenderAttachmentQuantitySet::OCCLUSION,
        );

        assets.load_default_lookup_table_textures(&core_system)?;

        Ok(Self {
            core_system,
            config,
            assets: RwLock::new(assets),
            render_resource_manager: RwLock::new(RenderResourceManager::new()),
            render_pass_manager: RwLock::new(RenderPassManager::new(wgpu::Color::BLACK)),
            render_attachment_texture_manager,
        })
    }

    /// Returns a reference to the core rendering system.
    pub fn core_system(&self) -> &CoreRenderingSystem {
        &self.core_system
    }

    /// Returns a reference to the global rendering configuration.
    pub fn config(&self) -> &RenderingConfig {
        &self.config
    }

    /// Returns a reference to the rendering assets, guarded by a [`RwLock`].
    pub fn assets(&self) -> &RwLock<Assets> {
        &self.assets
    }

    /// Returns a reference to the [`RenderResourceManager`], guarded
    /// by a [`RwLock`].
    pub fn render_resource_manager(&self) -> &RwLock<RenderResourceManager> {
        &self.render_resource_manager
    }

    /// Returns a reference to the [`RenderPassManager`], guarded
    /// by a [`RwLock`].
    pub fn render_pass_manager(&self) -> &RwLock<RenderPassManager> {
        &self.render_pass_manager
    }

    /// Returns a reference to the [`RenderAttachmentTextureManager`].
    pub fn render_attachment_texture_manager(&self) -> &RenderAttachmentTextureManager {
        &self.render_attachment_texture_manager
    }

    /// Returns the width and height of the rendering surface in pixels.
    pub fn surface_dimensions(&self) -> (u32, u32) {
        let surface_config = self.core_system.surface_config();
        (surface_config.width, surface_config.height)
    }

    /// Creates and presents a rendering using the current synchronized render
    /// resources.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The surface texture to render to can not be obtained.
    /// - Recording a render pass fails.
    pub fn render(&self) -> Result<()> {
        with_timing_info_logging!("Rendering"; {
            let surface_texture = self.render_surface()?;
            surface_texture.present();
        });
        Ok(())
    }

    /// Sets a new size for the rendering surface and assocated textures.
    pub fn resize_surface(&mut self, new_size: (u32, u32)) {
        self.core_system.resize_surface(new_size);
        self.render_attachment_texture_manager
            .recreate_textures(&self.core_system);
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
        self.render_pass_manager
            .write()
            .unwrap()
            .clear_model_render_pass_recorders();
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
        self.render_pass_manager
            .write()
            .unwrap()
            .clear_model_render_pass_recorders();
    }

    /// Toggles ambient occlusion.
    pub fn toggle_ambient_occlusion(&mut self) {
        self.config.ambient_occlusion_enabled = !self.config.ambient_occlusion_enabled;
    }

    /// Marks the render resources as being out of sync with the source data.
    pub fn declare_render_resources_desynchronized(&self) {
        self.render_resource_manager
            .write()
            .unwrap()
            .declare_desynchronized();
    }

    /// Initializes the surface for presentation using the
    /// current surface configuration.
    fn initialize_surface(&self) {
        self.core_system.initialize_surface();
    }

    fn render_surface(&self) -> Result<wgpu::SurfaceTexture> {
        let surface_texture = self.core_system.surface().get_current_texture()?;

        let mut command_encoder = Self::create_render_command_encoder(self.core_system.device());

        let mut n_recorded_passes = 0;

        {
            let render_resources_guard = self.render_resource_manager.read().unwrap();
            for render_pass_recorder in self.render_pass_manager.read().unwrap().recorders() {
                let outcome = render_pass_recorder.record_render_pass(
                    &self.core_system,
                    render_resources_guard.synchronized(),
                    &surface_texture,
                    &self.render_attachment_texture_manager,
                    &mut command_encoder,
                )?;
                if outcome == RenderPassOutcome::Recorded {
                    n_recorded_passes += 1;
                }
            }
        } // <- Lock on `self.render_resource_manager` is released here

        log::info!("Performing {} render passes", n_recorded_passes);

        self.core_system
            .queue()
            .submit(std::iter::once(command_encoder.finish()));

        Ok(surface_texture)
    }

    fn handle_render_error(&self, error: Error, _event_loop_controller: &EventLoopController<'_>) {
        if let Some(wgpu::SurfaceError::Lost) = error.downcast_ref() {
            // Recreate swap chain if lost
            self.initialize_surface();
        }
    }

    fn create_render_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render encoder"),
        })
    }
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            omnidirectional_light_shadow_map_resolution: 1024,
            unidirectional_light_shadow_map_resolution: 1024,
            ambient_occlusion_enabled: true,
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
    pub fn new(screenshot_width: u32) -> Self {
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
    /// [`save_screenshot_if_requested`] call.
    pub fn request_screenshot_save(&self) {
        self.screenshot_save_requested
            .store(true, Ordering::Release);
    }

    /// Schedule a capture of the render attachment texture for the given
    /// quantity for the next [`save_render_attachment_quantity_if_requested`]
    /// call.
    pub fn request_render_attachment_quantity_save(&self, quantity: RenderAttachmentQuantity) {
        self.render_attachment_save_requested
            .store(true, Ordering::Release);
        self.render_attachment_quantity
            .store(quantity as u8, Ordering::Release);
    }

    /// Schedule a capture of the omnidirectional light shadow map texture for
    /// the next [`save_omnidirectional_light_shadow_map_if_requested`] call.
    pub fn request_omnidirectional_light_shadow_map_save(&self) {
        self.omnidirectional_light_shadow_map_save_requested
            .store(true, Ordering::Release);
    }

    /// Schedule a capture of the unidirectional light shadow map texture for
    /// the next [`save_unidirectional_light_shadow_map_if_requested`] call.
    pub fn request_unidirectional_light_shadow_map_save(&self) {
        self.unidirectional_light_shadow_map_save_requested
            .store(true, Ordering::Release);
    }

    /// Checks if a screenshot capture was scheduled with
    /// [`request_screenshot_save`], and if so, captures a screenshot and saves
    /// it as a timestamped PNG file in the current directory.
    pub fn save_screenshot_if_requested(&self, renderer: &RwLock<RenderingSystem>) -> Result<()> {
        if self
            .screenshot_save_requested
            .swap(false, Ordering::Acquire)
        {
            let mut renderer = renderer.write().unwrap();

            let original_dimensions = renderer.surface_dimensions();

            renderer.resize_surface((
                self.screenshot_width,
                self.determine_screenshot_height(original_dimensions),
            ));
            {
                // Re-render the surface at the screenshot resolution.
                let surface_texture = renderer.render_surface()?;

                texture::save_texture_as_image_file(
                    renderer.core_system(),
                    &surface_texture.texture,
                    0,
                    format!("screenshot_{}.png", Utc::now().to_rfc3339()),
                )?;
            }
            renderer.resize_surface(original_dimensions);
        }
        Ok(())
    }

    /// Checks if a render attachment capture was scheduled with
    /// [`request_render_attachment_quantity_save`], and if so, captures the
    /// requested render attachment texture and saves it as a timestamped PNG
    /// file in the current directory.
    pub fn save_render_attachment_quantity_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
    ) -> Result<()> {
        if self
            .render_attachment_save_requested
            .swap(false, Ordering::Acquire)
        {
            let quantity = RenderAttachmentQuantity::from_u8(
                self.render_attachment_quantity.load(Ordering::Acquire),
            )
            .unwrap();

            let mut renderer = renderer.write().unwrap();

            let original_dimensions = renderer.surface_dimensions();

            renderer.resize_surface((
                self.screenshot_width,
                self.determine_screenshot_height(original_dimensions),
            ));
            {
                // Re-render the surface at the screenshot resolution.
                renderer.render_surface()?;

                renderer
                    .render_attachment_texture_manager()
                    .save_render_attachment_texture_as_image_file(
                        renderer.core_system(),
                        quantity,
                        format!("{}_{}.png", quantity, Utc::now().to_rfc3339()),
                    )?;
            }
            renderer.resize_surface(original_dimensions);
        }
        Ok(())
    }

    /// Checks if a omnidirectional light shadow map capture was scheduled with
    /// [`request_omnidirectional_light_shadow_map_save`], and if so, captures
    /// the texture and saves it as a timestamped PNG file in the current
    /// directory.
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
                            renderer.core_system(),
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
    /// [`request_unidirectional_light_shadow_map_save`], and if so, captures
    /// the texture and saves it as a timestamped PNG file in the current
    /// directory.
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
                            renderer.core_system(),
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

    fn determine_screenshot_height(&self, surface_dimensions: (u32, u32)) -> u32 {
        let aspect_ratio = (surface_dimensions.1 as f32) / (surface_dimensions.0 as f32);
        f32::round((self.screenshot_width as f32) * aspect_ratio) as u32
    }
}
