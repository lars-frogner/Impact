//! Application of postprocessing.

pub mod ambient_occlusion;
pub mod capturing;
pub mod gaussian_blur;
pub mod render_attachment_visualization;
pub mod temporal_anti_aliasing;

use crate::gpu::{
    query::TimestampQueryRegistry,
    rendering::{resource::SynchronizedRenderResources, surface::RenderingSurface},
    resource_group::GPUResourceGroupManager,
    shader::ShaderManager,
    storage::StorageGPUBufferManager,
    texture::attachment::RenderAttachmentTextureManager,
    GraphicsDevice,
};
use ambient_occlusion::{AmbientOcclusionConfig, AmbientOcclusionRenderCommands};
use anyhow::Result;
use capturing::{CapturingCamera, CapturingCameraConfig};
use render_attachment_visualization::RenderAttachmentVisualizationPasses;
use temporal_anti_aliasing::{TemporalAntiAliasingConfig, TemporalAntiAliasingRenderCommands};

/// Manager of GPU resources and render commands for postprocessing effects.
#[derive(Debug)]
pub struct Postprocessor {
    ambient_occlusion_enabled: bool,
    ambient_occlusion_commands: AmbientOcclusionRenderCommands,
    temporal_anti_aliasing_enabled: bool,
    temporal_anti_aliasing_commands: TemporalAntiAliasingRenderCommands,
    capturing_camera: CapturingCamera,
    render_attachment_visualization_passes: RenderAttachmentVisualizationPasses,
}

impl Postprocessor {
    /// Creates a new postprocessor along with the associated render commands
    /// according to the given configuration.
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
        ambient_occlusion_config: &AmbientOcclusionConfig,
        temporal_anti_aliasing_config: &TemporalAntiAliasingConfig,
        capturing_camera_settings: &CapturingCameraConfig,
    ) -> Result<Self> {
        let ambient_occlusion_commands = AmbientOcclusionRenderCommands::new(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            ambient_occlusion_config,
        )?;

        let temporal_anti_aliasing_commands = TemporalAntiAliasingRenderCommands::new(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            temporal_anti_aliasing_config,
        )?;

        let capturing_camera = CapturingCamera::new(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            capturing_camera_settings,
        )?;

        let render_attachment_visualization_passes = RenderAttachmentVisualizationPasses::new(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
        )?;

        Ok(Self {
            ambient_occlusion_enabled: ambient_occlusion_config.initially_enabled,
            ambient_occlusion_commands,
            temporal_anti_aliasing_enabled: temporal_anti_aliasing_config.initially_enabled,
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
            self.ambient_occlusion_enabled,
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
            self.temporal_anti_aliasing_enabled,
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

    /// Returns a reference to the capturing camera.
    pub fn capturing_camera(&self) -> &CapturingCamera {
        &self.capturing_camera
    }

    /// Whether ambient occlusion is enabled.
    pub fn ambient_occlusion_enabled(&self) -> bool {
        self.ambient_occlusion_enabled
    }

    /// Whether temporal anti-aliasing is enabled.
    pub fn temporal_anti_aliasing_enabled(&self) -> bool {
        self.temporal_anti_aliasing_enabled
    }

    /// Returns a mutable reference to the capturing camera.
    pub fn capturing_camera_mut(&mut self) -> &mut CapturingCamera {
        &mut self.capturing_camera
    }

    /// Toggles ambient occlusion.
    pub fn toggle_ambient_occlusion(&mut self) {
        self.ambient_occlusion_enabled = !self.ambient_occlusion_enabled;
    }

    /// Toggles temporal anti-aliasing.
    pub fn toggle_temporal_anti_aliasing(&mut self) {
        self.temporal_anti_aliasing_enabled = !self.temporal_anti_aliasing_enabled;
    }

    /// Toggles visualization of render attachments.
    pub fn toggle_render_attachment_visualization(&mut self) {
        self.render_attachment_visualization_passes.toggle_enabled();
    }

    /// Changes the visualized render attachment quantity to the next quantity
    /// in the list, or wraps around.
    pub fn cycle_visualized_render_attachment_quantity_forward(&mut self) {
        self.render_attachment_visualization_passes
            .cycle_quantity_forward();
    }

    /// Changes the visualized render attachment quantity to the previous
    /// quantity in the list, or wraps around.
    pub fn cycle_visualized_render_attachment_quantity_backward(&mut self) {
        self.render_attachment_visualization_passes
            .cycle_quantity_backward();
    }
}
