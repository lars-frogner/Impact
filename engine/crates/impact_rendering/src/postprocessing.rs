//! Application of postprocessing.

pub mod ambient_occlusion;
pub mod capturing;
pub mod gaussian_blur;
pub mod render_attachment_visualization;
pub mod temporal_anti_aliasing;

use crate::{
    attachment::{
        RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
        RenderAttachmentQuantity, RenderAttachmentTextureManager,
    },
    push_constant::BasicPushConstantGroup,
    render_command::StencilValue,
    resource::{BasicGPUResources, BasicResourceRegistries},
    surface::RenderingSurface,
};
use ambient_occlusion::{AmbientOcclusionConfig, AmbientOcclusionRenderCommands};
use anyhow::Result;
use capturing::{CapturingCamera, CapturingCameraConfig};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    query::TimestampQueryRegistry,
    resource_group::{GPUResourceGroupID, GPUResourceGroupManager},
    shader::{ShaderManager, template::SpecificShaderTemplate},
    storage::StorageGPUBufferManager,
    wgpu,
};
use render_attachment_visualization::RenderAttachmentVisualizationPasses;
use temporal_anti_aliasing::{TemporalAntiAliasingConfig, TemporalAntiAliasingRenderCommands};

/// Specific shader template that can be resolved to generate a postprocessing
/// shader.
pub trait PostprocessingShaderTemplate: SpecificShaderTemplate {
    /// Returns the group of push constants used by the shader.
    fn push_constants(&self) -> BasicPushConstantGroup;

    /// Returns the set of render attachments used as input by the shader.
    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet;

    /// Returns the descriptions of the render attachments that the shader will
    /// write to.
    fn output_render_attachments(&self) -> RenderAttachmentOutputDescriptionSet;

    /// Whether the shader uses the camera projection uniform.
    fn uses_camera(&self) -> bool {
        false
    }

    /// Returns the ID of the GPU resource group used by the shader, or [`None`]
    /// if the shader does not use a GPU resource group.
    fn gpu_resource_group_id(&self) -> Option<GPUResourceGroupID> {
        None
    }

    /// Returns the comparison function and stencil value to use for stencil
    /// testing when using the shader, or [`None`] if stencil testing should not
    /// be used.
    fn stencil_test(&self) -> Option<(wgpu::CompareFunction, StencilValue)> {
        None
    }

    /// Whether the shader writes to the actual surface texture that will be
    /// displayed.
    fn writes_to_surface(&self) -> bool {
        false
    }
}

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
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<Self> {
        let ambient_occlusion_commands = AmbientOcclusionRenderCommands::new(
            ambient_occlusion_config,
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
        )?;

        let temporal_anti_aliasing_commands = TemporalAntiAliasingRenderCommands::new(
            temporal_anti_aliasing_config,
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
        )?;

        let capturing_camera = CapturingCamera::new(
            capturing_camera_config,
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            bind_group_layout_registry,
        )?;

        let render_attachment_visualization_passes = RenderAttachmentVisualizationPasses::new(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
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
        resource_registries: &impl BasicResourceRegistries,
        gpu_resources: &impl BasicGPUResources,
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
            resource_registries,
            gpu_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            self,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;
        self.capturing_camera
            .record_commands_before_dynamic_range_compression(
                rendering_surface,
                resource_registries,
                gpu_resources,
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
            resource_registries,
            gpu_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            self,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;
        self.capturing_camera
            .record_dynamic_range_compression_render_commands(
                rendering_surface,
                surface_texture_view,
                resource_registries,
                gpu_resources,
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
            resource_registries,
            gpu_resources,
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

    pub fn ambient_occlusion_commands_mut(&mut self) -> &mut AmbientOcclusionRenderCommands {
        &mut self.ambient_occlusion_commands
    }

    pub fn temporal_anti_aliasing_commands_mut(
        &mut self,
    ) -> &mut TemporalAntiAliasingRenderCommands {
        &mut self.temporal_anti_aliasing_commands
    }

    pub fn capturing_camera_mut(&mut self) -> &mut CapturingCamera {
        &mut self.capturing_camera
    }

    pub fn render_attachment_visualization_passes_mut(
        &mut self,
    ) -> &mut RenderAttachmentVisualizationPasses {
        &mut self.render_attachment_visualization_passes
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
