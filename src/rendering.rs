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
    create_vertex_buffer_layout_for_instance, create_vertex_buffer_layout_for_vertex,
    UniformBufferable, VertexBufferable,
};
pub use material::{MaterialPropertyTextureManager, MaterialRenderResourceManager};
pub use render_pass::{RenderPassManager, SyncRenderPasses};
pub use resource::SyncRenderResources;
pub use shader::{
    BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput, CameraShaderInput,
    DirectionalLightShaderInput, FixedColorFeatureShaderInput, FixedTextureShaderInput,
    InstanceFeatureShaderInput, LightShaderInput, MaterialShaderInput, MeshShaderInput,
    ModelViewTransformShaderInput, PointLightShaderInput, Shader, ShaderGenerator,
};
pub use tasks::{Render, RenderingTag};
pub use texture::{DepthTexture, ImageTexture};

use self::resource::RenderResourceManager;
use crate::window::ControlFlow;
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
    screenshotter: Screenshotter,
}

/// Global rendering configuration options.
#[derive(Clone, Debug)]
pub struct RenderingConfig {
    /// The face culling mode.
    pub cull_mode: Option<wgpu::Face>,
    /// Controls the way each polygon is rasterized.
    pub polygon_mode: wgpu::PolygonMode,
    pub directional_light_shadow_map_resolution: u32,
}

#[derive(Debug)]
struct Screenshotter {
    screenshot_save_requested: AtomicBool,
    depth_map_save_requested: AtomicBool,
    directional_light_shadow_map_save_requested: AtomicBool,
}

impl RenderingSystem {
    /// Creates a new rendering system consisting of the given core system and
    /// assets.
    pub async fn new(core_system: CoreRenderingSystem, assets: Assets) -> Result<Self> {
        let config = RenderingConfig::default();

        let depth_texture = DepthTexture::new(&core_system, "Depth texture");

        Ok(Self {
            core_system,
            config,
            assets: RwLock::new(assets),
            render_resource_manager: RwLock::new(RenderResourceManager::new()),
            render_pass_manager: RwLock::new(RenderPassManager::new(wgpu::Color::BLACK)),
            depth_texture,
            screenshotter: Screenshotter::new(),
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

        let mut command_encoder = Self::create_render_command_encoder(self.core_system.device());

        {
            let render_resources_guard = self.render_resource_manager.read().unwrap();
            for render_pass_recorder in self.render_pass_manager.read().unwrap().recorders() {
                render_pass_recorder.record_render_pass(
                    render_resources_guard.synchronized(),
                    &surface_texture_view,
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
            .save_directional_light_shadow_map_if_requested(
                &self.core_system,
                &self.render_resource_manager,
            )?;

        surface_texture.present();

        Ok(())
    }

    /// Sets a new size for the rendering surface.
    pub fn resize_surface(&mut self, new_size: (u32, u32)) {
        self.core_system.resize_surface(new_size);
        self.depth_texture = DepthTexture::new(&self.core_system, "Depth texture");
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

    pub fn request_screenshot_save(&self) {
        self.screenshotter.request_screenshot_save();
    }

    pub fn request_depth_map_save(&self) {
        self.screenshotter.request_depth_map_save();
    }

    pub fn request_directional_light_shadow_map_save(&self) {
        self.screenshotter
            .request_directional_light_shadow_map_save();
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
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            directional_light_shadow_map_resolution: 1024,
        }
    }
}

impl Screenshotter {
    fn new() -> Self {
        Self {
            screenshot_save_requested: AtomicBool::new(false),
            depth_map_save_requested: AtomicBool::new(false),
            directional_light_shadow_map_save_requested: AtomicBool::new(false),
        }
    }

    fn request_screenshot_save(&self) {
        self.screenshot_save_requested
            .store(true, Ordering::Release);
    }

    fn request_depth_map_save(&self) {
        self.depth_map_save_requested.store(true, Ordering::Release);
    }

    fn request_directional_light_shadow_map_save(&self) {
        self.directional_light_shadow_map_save_requested
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

    fn save_directional_light_shadow_map_if_requested(
        &self,
        core_system: &CoreRenderingSystem,
        render_resource_manager: &RwLock<RenderResourceManager>,
    ) -> Result<()> {
        if self
            .directional_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            if let Some(light_buffer_manager) = render_resource_manager
                .read()
                .unwrap()
                .synchronized()
                .get_light_buffer_manager()
            {
                light_buffer_manager
                    .directional_light_shadow_map_texture()
                    .save_as_image_file(
                        core_system,
                        format!(
                            "directional_light_shadow_map_{}.png",
                            Utc::now().to_rfc3339()
                        ),
                    )
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}
