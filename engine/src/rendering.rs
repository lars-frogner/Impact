//! Graphics rendering.

pub mod render_command;
pub mod screen_capture;

use crate::{
    gpu::GraphicsSystem, lock_order::OrderedRwLock, resource::ResourceManager, ui::UserInterface,
};
use anyhow::Result;
use impact_gpu::{device::GraphicsDevice, wgpu};
use impact_rendering::{
    BasicRenderingConfig,
    attachment::RenderAttachmentTextureManager,
    postprocessing::{
        Postprocessor, ambient_occlusion::AmbientOcclusionConfig, capturing::CapturingCameraConfig,
        temporal_anti_aliasing::TemporalAntiAliasingConfig,
    },
    surface::{RenderingSurface, SurfaceAcquisition},
};
use parking_lot::RwLock;
use render_command::RenderCommandManager;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, sync::Arc};

/// Container for data and systems required for rendering.
#[derive(Debug)]
pub struct RenderingSystem {
    graphics_device: Arc<GraphicsDevice>,
    rendering_surface: RenderingSurface,
    surface_texture_to_present: Option<wgpu::SurfaceTexture>,
    render_attachment_texture_manager: RwLock<RenderAttachmentTextureManager>,
    postprocessor: RwLock<Postprocessor>,
    render_command_manager: RenderCommandManager,
    frame_counter: u64,
    basic_config: BasicRenderingConfig,
}

/// Rendering configuration options.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderingConfig {
    pub basic: BasicRenderingConfig,
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
        graphics_system: &GraphicsSystem,
    ) -> Result<Self> {
        config.basic.make_compatible_with_device(&graphics_device);

        let mut render_attachment_texture_manager =
            RenderAttachmentTextureManager::new(&graphics_device, &rendering_surface);

        let mut shader_manager = graphics_system.shader_manager().owrite();
        let mut gpu_resource_group_manager = graphics_system.gpu_resource_group_manager().owrite();
        let mut storage_gpu_buffer_manager = graphics_system.storage_gpu_buffer_manager().owrite();

        let render_command_manager = RenderCommandManager::new(
            &graphics_device,
            &rendering_surface,
            &mut shader_manager,
            &mut render_attachment_texture_manager,
            resource_manager,
            graphics_system.bind_group_layout_registry(),
            &config.basic,
        );

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
            graphics_system.bind_group_layout_registry(),
        )?;

        Ok(Self {
            graphics_device,
            rendering_surface,
            surface_texture_to_present: None,
            render_attachment_texture_manager: RwLock::new(render_attachment_texture_manager),
            postprocessor: RwLock::new(postprocessor),
            render_command_manager,
            frame_counter: 0,
            basic_config: config.basic,
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

    /// Returns a reference to the [`RenderAttachmentTextureManager`], guarded
    /// by a [`RwLock`].
    pub fn render_attachment_texture_manager(&self) -> &RwLock<RenderAttachmentTextureManager> {
        &self.render_attachment_texture_manager
    }

    /// Returns a reference to the [`Postprocessor`], guarded by a [`RwLock`].
    pub fn postprocessor(&self) -> &RwLock<Postprocessor> {
        &self.postprocessor
    }

    /// Returns a reference to the [`RenderCommandManager`].
    pub fn render_command_manager(&self) -> &RenderCommandManager {
        &self.render_command_manager
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

    /// Presents the last surface texture that was rendered to. Does nothing if
    /// there is no texture to present.
    pub fn present(&mut self) {
        if let Some(surface_texture) = self.surface_texture_to_present.take() {
            self.graphics_device.queue().present(surface_texture);
        }
    }

    /// Synchronizes the render commands with the current GPU resources.
    ///
    /// # Errors
    /// Returns an error if synchronization fails due to missing resources.
    pub fn synchronize_render_commands(&mut self, graphics_system: &GraphicsSystem) -> Result<()> {
        let mut shader_manager = graphics_system.shader_manager().owrite();
        let gpu_resource_manager = graphics_system.gpu_resource_manager().oread();
        self.render_command_manager.sync_with_gpu_resources(
            &self.graphics_device,
            &mut shader_manager,
            &gpu_resource_manager,
            graphics_system.bind_group_layout_registry(),
        )
    }

    /// Records and submits render commands that do not require the surface
    /// texture to be available.
    ///
    /// # Errors
    /// Returns an error if recording a render commands fails.
    pub fn render_before_surface(&mut self, graphics_system: &GraphicsSystem) -> Result<()> {
        self.render_attachment_texture_manager
            .owrite()
            .swap_previous_and_current_attachment_variants(&self.graphics_device);

        let mut command_encoder =
            Self::create_render_command_encoder(self.graphics_device.device());

        {
            let mut timestamp_query_manager = graphics_system.timestamp_query_manager().owrite();
            let mut timestamp_recorder = timestamp_query_manager.create_timestamp_query_registry();

            let gpu_resource_manager = graphics_system.gpu_resource_manager().oread();
            let gpu_resource_group_manager = graphics_system.gpu_resource_group_manager().oread();
            let storage_gpu_buffer_manager = graphics_system.storage_gpu_buffer_manager().oread();

            let render_attachment_texture_manager = self.render_attachment_texture_manager.oread();
            let postprocessor = self.postprocessor.oread();

            self.render_command_manager.record_before_surface(
                &self.rendering_surface,
                &gpu_resource_manager,
                &render_attachment_texture_manager,
                &gpu_resource_group_manager,
                &storage_gpu_buffer_manager,
                &postprocessor,
                self.frame_counter as u32,
                &mut timestamp_recorder,
                &mut command_encoder,
            )?;

            timestamp_recorder.finish(&mut command_encoder);
        }

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
    /// Returns [`None`] if the surface is not available for this frame, in
    /// which case the frame should be skipped.
    ///
    /// # Errors
    /// Return an error if obtaining the surface texture fails.
    pub fn obtain_surface(
        &mut self,
    ) -> Result<Option<(wgpu::TextureView, Option<wgpu::SurfaceTexture>)>> {
        match self
            .rendering_surface
            .acquire_surface(&self.graphics_device)?
        {
            SurfaceAcquisition::Acquired {
                texture_view,
                presentable_texture,
            } => Ok(Some((texture_view, presentable_texture))),
            SurfaceAcquisition::Skip => Ok(None),
            SurfaceAcquisition::Invalidated => {
                self.recreate_render_attachment_textures();
                Ok(None)
            }
        }
    }

    /// Records and submits the final render commands that write into the
    /// surface texture. The surface texture to present (if any) is stored for
    /// later presentation by calling [`Self::present`].
    ///
    /// # Errors
    /// Returns an error if recording a render commands fails.
    pub fn render_to_surface(
        &mut self,
        graphics_system: &GraphicsSystem,
        surface_texture_view: wgpu::TextureView,
        surface_texture: Option<wgpu::SurfaceTexture>,
        user_interface: &dyn UserInterface,
    ) -> Result<()> {
        let mut command_encoder =
            Self::create_render_command_encoder(self.graphics_device.device());

        {
            let mut timestamp_query_manager = graphics_system.timestamp_query_manager().owrite();
            let mut timestamp_recorder = timestamp_query_manager.create_timestamp_query_registry();

            let gpu_resource_manager = graphics_system.gpu_resource_manager().oread();
            let gpu_resource_group_manager = graphics_system.gpu_resource_group_manager().oread();

            let render_attachment_texture_manager = self.render_attachment_texture_manager.oread();
            let postprocessor = self.postprocessor.oread();

            self.render_command_manager.record_with_surface(
                &self.rendering_surface,
                &surface_texture_view,
                &gpu_resource_manager,
                &render_attachment_texture_manager,
                &gpu_resource_group_manager,
                &postprocessor,
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
        }

        self.graphics_device
            .queue()
            .submit(std::iter::once(command_encoder.finish()));

        self.surface_texture_to_present = surface_texture;
        self.frame_counter = self.frame_counter.wrapping_add(1);

        Ok(())
    }

    /// Updates the exposure based on the current settings and potentially the
    /// average incident luminance. Call after [`Self::render_to_surface`]. This
    /// method will wait for the GPU to finish rendering if it needs the average
    /// incident luminance.
    pub fn update_exposure(&self, graphics_system: &GraphicsSystem) -> Result<()> {
        let storage_gpu_buffer_manager = graphics_system.storage_gpu_buffer_manager().oread();
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

    /// Recreates the rendering surface and render attachment textures if the
    /// surface was lost when it was last acquired. Returns whether the surface
    /// was recreated.
    ///
    /// Must be called on the thread owning the window if the surface is attached
    /// to a window.
    pub fn recreate_rendering_surface_if_lost(&mut self) -> Result<bool> {
        if !self
            .rendering_surface
            .recreate_if_lost(&self.graphics_device)?
        {
            return Ok(false);
        }

        log::info!("Recreated lost rendering surface");

        self.recreate_render_attachment_textures();

        Ok(true)
    }

    pub fn set_wireframe_mode_enabled(&mut self, graphics_system: &GraphicsSystem, enabled: bool) {
        if enabled
            && !self
                .graphics_device
                .supports_features(wgpu::Features::POLYGON_MODE_LINE)
        {
            log::warn!("Not enabling wireframe mode due to missing graphics device features");
            return;
        }

        let was_enabled = self.basic_config.wireframe_mode_on;

        if enabled != was_enabled {
            self.basic_config.wireframe_mode_on = enabled;
            self.sync_render_command_manager_with_basic_config(graphics_system);
        }
    }

    fn create_render_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render command encoder"),
        })
    }

    fn sync_render_command_manager_with_basic_config(&mut self, graphics_system: &GraphicsSystem) {
        self.render_command_manager.sync_with_config(
            &self.graphics_device,
            &graphics_system.shader_manager().oread(),
            &self.basic_config,
        );
    }

    fn recreate_render_attachment_textures(&mut self) {
        self.render_attachment_texture_manager
            .owrite()
            .recreate_textures(&self.graphics_device, &self.rendering_surface);
    }
}
