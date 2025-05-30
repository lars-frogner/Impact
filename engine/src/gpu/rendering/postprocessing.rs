//! Application of postprocessing.

pub mod ambient_occlusion;
pub mod capturing;
pub mod command;
pub mod gaussian_blur;
pub mod render_attachment_visualization;
pub mod temporal_anti_aliasing;

use crate::gpu::{
    GraphicsDevice,
    query::TimestampQueryRegistry,
    rendering::{resource::SynchronizedRenderResources, surface::RenderingSurface},
    resource_group::GPUResourceGroupManager,
    shader::ShaderManager,
    storage::StorageGPUBufferManager,
    texture::attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
};
use ambient_occlusion::{AmbientOcclusionConfig, AmbientOcclusionRenderCommands};
use anyhow::Result;
use capturing::{CapturingCamera, CapturingCameraConfig};
use render_attachment_visualization::RenderAttachmentVisualizationPasses;
use temporal_anti_aliasing::{TemporalAntiAliasingConfig, TemporalAntiAliasingRenderCommands};

/// Manager of GPU resources and render commands for postprocessing effects.
#[derive(Debug)]
pub struct Postprocessor {
    ambient_occlusion_commands: AmbientOcclusionRenderCommands,
    temporal_anti_aliasing_commands: TemporalAntiAliasingRenderCommands,
    capturing_camera: CapturingCamera,
    render_attachment_visualization_passes: RenderAttachmentVisualizationPasses,
}

impl Postprocessor {
    /// Creates a new postprocessor along with the associated render commands
    /// according to the given configuration.
    pub fn new(
        ambient_occlusion_config: AmbientOcclusionConfig,
        temporal_anti_aliasing_config: TemporalAntiAliasingConfig,
        capturing_camera_config: CapturingCameraConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    ) -> Result<Self> {
        let ambient_occlusion_commands = AmbientOcclusionRenderCommands::new(
            ambient_occlusion_config,
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
        )?;

        let temporal_anti_aliasing_commands = TemporalAntiAliasingRenderCommands::new(
            temporal_anti_aliasing_config,
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
        )?;

        let capturing_camera = CapturingCamera::new(
            capturing_camera_config,
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
        )?;

        let render_attachment_visualization_passes = RenderAttachmentVisualizationPasses::new(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
        )?;

        Ok(Self {
            ambient_occlusion_commands,
            temporal_anti_aliasing_commands,
            capturing_camera,
            render_attachment_visualization_passes,
        })
    }

    /// Records all postprocessing render commands into the given command
    /// encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record_commands(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.ambient_occlusion_commands.record(
            rendering_surface,
            surface_texture_view,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            self,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;
        self.capturing_camera.record_commands_before_tone_mapping(
            rendering_surface,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            self,
            timestamp_recorder,
            command_encoder,
        )?;
        self.temporal_anti_aliasing_commands.record(
            rendering_surface,
            surface_texture_view,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            self,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;
        self.capturing_camera.record_tone_mapping_render_commands(
            rendering_surface,
            surface_texture_view,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            self,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;
        self.render_attachment_visualization_passes.record(
            rendering_surface,
            surface_texture_view,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            self,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;
        Ok(())
    }

    pub fn capturing_camera(&self) -> &CapturingCamera {
        &self.capturing_camera
    }

    pub fn ambient_occlusion_config(&self) -> &AmbientOcclusionConfig {
        self.ambient_occlusion_commands.config()
    }

    /// Sets the given ambient occlusion configuration parameters and updates
    /// the appropriate render resources.
    pub fn set_ambient_occlusion_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        config: AmbientOcclusionConfig,
    ) {
        self.ambient_occlusion_commands.set_config(
            graphics_device,
            gpu_resource_group_manager,
            config,
        );
    }

    pub fn ambient_occlusion_enabled_mut(&mut self) -> &mut bool {
        self.ambient_occlusion_commands.enabled_mut()
    }

    pub fn temporal_anti_aliasing_config(&self) -> &TemporalAntiAliasingConfig {
        self.temporal_anti_aliasing_commands.config()
    }

    /// Sets the given temporal anti-aliasing configuration parameters and
    /// updates the appropriate render resources.
    pub fn set_temporal_anti_aliasing_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        config: TemporalAntiAliasingConfig,
    ) {
        self.temporal_anti_aliasing_commands.set_config(
            graphics_device,
            gpu_resource_group_manager,
            config,
        );
    }

    pub fn temporal_anti_aliasing_enabled_mut(&mut self) -> &mut bool {
        self.temporal_anti_aliasing_commands.enabled_mut()
    }

    pub fn capturing_camera_mut(&mut self) -> &mut CapturingCamera {
        &mut self.capturing_camera
    }

    pub fn visualized_render_attachment_quantity(&self) -> Option<RenderAttachmentQuantity> {
        if self.render_attachment_visualization_passes.enabled() {
            Some(self.render_attachment_visualization_passes.quantity())
        } else {
            None
        }
    }

    pub fn visualize_render_attachment_quantity(
        &mut self,
        quantity: Option<RenderAttachmentQuantity>,
    ) -> Result<()> {
        if let Some(quantity) = quantity {
            *self.render_attachment_visualization_passes.enabled_mut() = true;
            self.render_attachment_visualization_passes
                .set_quantity(quantity)?;
        } else {
            *self.render_attachment_visualization_passes.enabled_mut() = false;
        }
        Ok(())
    }
}
