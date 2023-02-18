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
    VertexBufferable,
};
pub use material::MaterialRenderResourceManager;
pub use render_pass::{RenderPassManager, SyncRenderPasses};
pub use resource::SyncRenderResources;
pub use shader::{
    BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput, CameraShaderInput,
    FixedColorFeatureShaderInput, FixedTextureShaderInput, InstanceFeatureShaderInput,
    LightShaderInput, MaterialShaderInput, MeshShaderInput, ModelViewTransformShaderInput, Shader,
    ShaderGenerator,
};
pub use tasks::{Render, RenderingTag};
pub use texture::{DepthTexture, ImageTexture};

use self::resource::RenderResourceManager;
use crate::window::ControlFlow;
use anyhow::{Error, Result};
use std::sync::RwLock;

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
}

/// Global rendering configuration options.
#[derive(Clone, Debug)]
pub struct RenderingConfig {
    /// The face culling mode.
    pub cull_mode: Option<wgpu::Face>,
    /// Controls the way each polygon is rasterized.
    pub polygon_mode: wgpu::PolygonMode,
}

impl RenderingSystem {
    /// Creates a new rendering system consisting of the given
    /// core system and rendering pipelines.
    pub async fn new(core_system: CoreRenderingSystem, assets: Assets) -> Result<Self> {
        let depth_texture = DepthTexture::new(&core_system, "Depth texture");

        Ok(Self {
            core_system,
            config: RenderingConfig::default(),
            assets: RwLock::new(assets),
            render_resource_manager: RwLock::new(RenderResourceManager::new()),
            render_pass_manager: RwLock::new(RenderPassManager::new(wgpu::Color::BLACK, 1.0)),
            depth_texture,
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
        let view = Self::create_surface_texture_view(&surface_texture);

        let mut command_encoder = Self::create_render_command_encoder(self.core_system.device());

        {
            let render_resources_guard = self.render_resource_manager.read().unwrap();
            for render_pass_recorder in self.render_pass_manager.read().unwrap().recorders() {
                render_pass_recorder.record_render_pass(
                    render_resources_guard.synchronized(),
                    &view,
                    &self.depth_texture,
                    &mut command_encoder,
                )?;
            }
        } // <- Lock on `self.render_resource_manager` is released here

        self.core_system
            .queue()
            .submit(std::iter::once(command_encoder.finish()));
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

    fn handle_render_error(&self, error: Error, control_flow: &mut ControlFlow<'_>) {
        match error.downcast_ref() {
            // Recreate swap chain if lost
            Some(wgpu::SurfaceError::Lost) => self.initialize_surface(),
            // Quit if GPU is out of memory
            Some(wgpu::SurfaceError::OutOfMemory) => {
                control_flow.exit();
            }
            // Other errors should be resolved by the next frame, so we just log the error and continue
            _ => log::error!("{:?}", error),
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
        }
    }
}
