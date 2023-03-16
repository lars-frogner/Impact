//! Graphics rendering.

mod assets;
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
pub use buffer::{
    create_uniform_buffer_bind_group_layout_entry, create_vertex_buffer_layout_for_instance,
    create_vertex_buffer_layout_for_vertex, UniformBufferable, VertexBufferable,
};
pub use material::{MaterialPropertyTextureManager, MaterialRenderResourceManager};
pub use render_pass::{RenderPassManager, SyncRenderPasses};
pub use resource::SyncRenderResources;
pub use shader::{
    BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput, CameraShaderInput,
    FixedColorFeatureShaderInput, FixedTextureShaderInput, GlobalAmbientColorShaderInput,
    InstanceFeatureShaderInput, LightShaderInput, MaterialShaderInput, MeshShaderInput,
    ModelViewTransformShaderInput, PointLightShaderInput, Shader, ShaderGenerator,
    UnidirectionalLightShaderInput,
};
pub use tasks::{Render, RenderingTag};
pub use texture::{CascadeIdx, DepthTexture, ImageTexture, MultisampledRenderTargetTexture};

use self::resource::RenderResourceManager;
use crate::{geometry::CubemapFace, scene::MAX_SHADOW_MAP_CASCADES, window::ControlFlow};
use anyhow::{Error, Result};
use chrono::Utc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
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
    depth_texture: DepthTexture,
    multisampled_render_target_texture: Option<MultisampledRenderTargetTexture>,
    screenshotter: Screenshotter,
}

/// Global rendering configuration options.
#[derive(Clone, Debug)]
pub struct RenderingConfig {
    /// The face culling mode.
    pub cull_mode: Option<wgpu::Face>,
    /// Controls the way each polygon is rasterized.
    pub polygon_mode: wgpu::PolygonMode,
    /// The number of samples to use for multisampling anti-aliasing.
    pub multisampling_sample_count: u32,
    /// The width and height of each face of the point light shadow cubemap in
    /// number of texels.
    pub point_light_shadow_map_resolution: u32,
    /// The width and height of the unidirectional light shadow map in number of
    /// texels.
    pub unidirectional_light_shadow_map_resolution: u32,
}

#[derive(Debug)]
struct Screenshotter {
    screenshot_save_requested: AtomicBool,
    depth_map_save_requested: AtomicBool,
    point_light_shadow_map_save_requested: AtomicBool,
    unidirectional_light_shadow_map_save_requested: AtomicBool,
}

impl RenderingSystem {
    /// Creates a new rendering system consisting of the given core system and
    /// assets.
    pub async fn new(core_system: CoreRenderingSystem, assets: Assets) -> Result<Self> {
        let config = RenderingConfig::default();

        let depth_texture = DepthTexture::new(&core_system, config.multisampling_sample_count);

        let mut renderer = Self {
            core_system,
            config,
            assets: RwLock::new(assets),
            render_resource_manager: RwLock::new(RenderResourceManager::new()),
            render_pass_manager: RwLock::new(RenderPassManager::new(wgpu::Color::BLACK)),
            depth_texture,
            multisampled_render_target_texture: None,
            screenshotter: Screenshotter::new(),
        };

        renderer.recreate_multisampled_render_target_texture();

        Ok(renderer)
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

    /// Returns a reference to the [`DepthTexture`].
    pub fn depth_texture(&self) -> &DepthTexture {
        &self.depth_texture
    }

    /// Creates and presents a rendering of the current data in the pipelines.
    ///
    /// # Errors
    /// Returns an error if:
    /// - If the surface texture to render to can not be obtained.
    /// - If recording a render pass fails.
    pub fn render(&self) -> Result<()> {
        let surface_texture = self.core_system.surface().get_current_texture()?;
        let surface_texture_view = Self::create_surface_texture_view(&surface_texture);

        let (color_attachment_texture_view, color_attachment_resolve_target) =
            if let Some(multisampled_render_target_texture) =
                self.multisampled_render_target_texture.as_ref()
            {
                // If multisampling, use multisampled texture as render target
                // and surface texture as resolve target
                (
                    multisampled_render_target_texture.view(),
                    Some(&surface_texture_view),
                )
            } else {
                (&surface_texture_view, None)
            };

        let mut command_encoder = Self::create_render_command_encoder(self.core_system.device());

        {
            let render_resources_guard = self.render_resource_manager.read().unwrap();
            for render_pass_recorder in self.render_pass_manager.read().unwrap().recorders() {
                render_pass_recorder.record_render_pass(
                    render_resources_guard.synchronized(),
                    color_attachment_texture_view,
                    color_attachment_resolve_target,
                    self.depth_texture.view(),
                    &mut command_encoder,
                )?;
            }
        } // <- Lock on `self.render_resource_manager` is released here

        self.core_system
            .queue()
            .submit(std::iter::once(command_encoder.finish()));

        // Screenshots must be saved before the surface is presented
        self.screenshotter
            .save_screenshot_if_requested(&self.core_system, &surface_texture)?;

        self.screenshotter
            .save_depth_map_if_requested(&self.core_system, &self.depth_texture)?;

        self.screenshotter
            .save_point_light_shadow_map_if_requested(
                &self.core_system,
                &self.render_resource_manager,
            )?;

        self.screenshotter
            .save_unidirectional_light_shadow_map_if_requested(
                &self.core_system,
                &self.render_resource_manager,
            )?;

        surface_texture.present();

        Ok(())
    }

    /// Sets a new size for the rendering surface.
    pub fn resize_surface(&mut self, new_size: (u32, u32)) {
        self.core_system.resize_surface(new_size);
        self.recreate_depth_texture();
        self.recreate_multisampled_render_target_texture();
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

    pub fn toggle_4x_msaa(&mut self) {
        let sample_count = if self.multisampled_render_target_texture.is_none() {
            4
        } else {
            1
        };
        self.set_multisampling_sample_count(sample_count);
    }

    pub fn request_screenshot_save(&self) {
        self.screenshotter.request_screenshot_save();
    }

    pub fn request_depth_map_save(&self) {
        self.screenshotter.request_depth_map_save();
    }

    pub fn request_point_light_shadow_map_save(&self) {
        self.screenshotter.request_point_light_shadow_map_save();
    }

    pub fn request_unidirectional_light_shadow_map_save(&self) {
        self.screenshotter
            .request_unidirectional_light_shadow_map_save();
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

    fn handle_render_error(&self, error: Error, _control_flow: &mut ControlFlow<'_>) {
        if let Some(wgpu::SurfaceError::Lost) = error.downcast_ref() {
            // Recreate swap chain if lost
            self.initialize_surface();
        }
    }

    fn create_surface_texture_view(surface_texture: &wgpu::SurfaceTexture) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
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

            self.recreate_depth_texture();
            self.recreate_multisampled_render_target_texture();

            // Remove all render pass recorders so that they will be recreated with
            // the updated configuration
            self.render_pass_manager
                .write()
                .unwrap()
                .clear_model_render_pass_recorders();
        }
    }

    fn recreate_depth_texture(&mut self) {
        self.depth_texture =
            DepthTexture::new(&self.core_system, self.config.multisampling_sample_count);
    }

    fn recreate_multisampled_render_target_texture(&mut self) {
        self.multisampled_render_target_texture.take();

        if self.config.multisampling_sample_count > 1 {
            self.multisampled_render_target_texture = Some(MultisampledRenderTargetTexture::new(
                &self.core_system,
                self.config.multisampling_sample_count,
            ));
        }
    }
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            multisampling_sample_count: 1,
            point_light_shadow_map_resolution: 1024,
            unidirectional_light_shadow_map_resolution: 1024,
        }
    }
}

impl Screenshotter {
    fn new() -> Self {
        Self {
            screenshot_save_requested: AtomicBool::new(false),
            depth_map_save_requested: AtomicBool::new(false),
            point_light_shadow_map_save_requested: AtomicBool::new(false),
            unidirectional_light_shadow_map_save_requested: AtomicBool::new(false),
        }
    }

    fn request_screenshot_save(&self) {
        self.screenshot_save_requested
            .store(true, Ordering::Release);
    }

    fn request_depth_map_save(&self) {
        self.depth_map_save_requested.store(true, Ordering::Release);
    }

    fn request_point_light_shadow_map_save(&self) {
        self.point_light_shadow_map_save_requested
            .store(true, Ordering::Release);
    }

    fn request_unidirectional_light_shadow_map_save(&self) {
        self.unidirectional_light_shadow_map_save_requested
            .store(true, Ordering::Release);
    }

    fn save_screenshot_if_requested(
        &self,
        core_system: &CoreRenderingSystem,
        surface_texture: &wgpu::SurfaceTexture,
    ) -> Result<()> {
        if self
            .screenshot_save_requested
            .swap(false, Ordering::Acquire)
        {
            texture::save_color_texture_as_image_file(
                core_system,
                &surface_texture.texture,
                format!("screenshot_{}.png", Utc::now().to_rfc3339()),
            )
        } else {
            Ok(())
        }
    }

    fn save_depth_map_if_requested(
        &self,
        core_system: &CoreRenderingSystem,
        depth_texture: &DepthTexture,
    ) -> Result<()> {
        if self.depth_map_save_requested.swap(false, Ordering::Acquire) {
            depth_texture.save_as_image_file(
                core_system,
                format!("depth_map_{}.png", Utc::now().to_rfc3339()),
            )
        } else {
            Ok(())
        }
    }

    fn save_point_light_shadow_map_if_requested(
        &self,
        core_system: &CoreRenderingSystem,
        render_resource_manager: &RwLock<RenderResourceManager>,
    ) -> Result<()> {
        if self
            .point_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            if let Some(light_buffer_manager) = render_resource_manager
                .read()
                .unwrap()
                .synchronized()
                .get_light_buffer_manager()
            {
                for face in CubemapFace::all() {
                    light_buffer_manager
                        .point_light_shadow_map_texture()
                        .save_face_as_image_file(
                            core_system,
                            face,
                            format!(
                                "point_light_shadow_map_{}_{:?}.png",
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

    fn save_unidirectional_light_shadow_map_if_requested(
        &self,
        core_system: &CoreRenderingSystem,
        render_resource_manager: &RwLock<RenderResourceManager>,
    ) -> Result<()> {
        if self
            .unidirectional_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            if let Some(light_buffer_manager) = render_resource_manager
                .read()
                .unwrap()
                .synchronized()
                .get_light_buffer_manager()
            {
                for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                    light_buffer_manager
                        .unidirectional_light_shadow_map_texture()
                        .save_cascade_as_image_file(
                            core_system,
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
}
