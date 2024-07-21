//! GPU compute passes.

use crate::gpu::{
    push_constant::PushConstantGroup,
    push_constant::PushConstantVariant,
    rendering::{
        postprocessing::Postprocessor,
        render_command::{RenderCommandOutcome, RenderCommandState},
        surface::RenderingSurface,
    },
    resource_group::{GPUResourceGroupID, GPUResourceGroupManager},
    shader::{Shader, ShaderID, ShaderManager},
    texture::attachment::{RenderAttachmentInputDescriptionSet, RenderAttachmentTextureManager},
    GraphicsDevice,
};
use anyhow::{anyhow, Result};

/// Holds the information describing a specific compute pass.
#[derive(Clone, Debug)]
pub struct ComputePassSpecification {
    pub shader_id: ShaderID,
    pub workgroup_counts: [u32; 3],
    pub push_constants: PushConstantGroup,
    pub resource_group_id: Option<GPUResourceGroupID>,
    pub input_render_attachments: RenderAttachmentInputDescriptionSet,
    pub label: String,
}

/// Recorder for a specific compute pass.
#[derive(Debug)]
pub struct ComputePassRecorder {
    specification: ComputePassSpecification,
    pipeline: wgpu::ComputePipeline,
    state: RenderCommandState,
}

impl ComputePassRecorder {
    /// Creates a new recorder for the compute pass defined by the given
    /// specification.
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        specification: ComputePassSpecification,
        state: RenderCommandState,
    ) -> Result<Self> {
        let resource_group = if let Some(resource_group_id) = specification.resource_group_id {
            Some(
                gpu_resource_group_manager
                    .get_resource_group(resource_group_id)
                    .ok_or_else(|| anyhow!("Missing GPU resource group {}", resource_group_id))?,
            )
        } else {
            None
        };

        let mut bind_group_layouts = resource_group
            .map(|resource_group| vec![resource_group.bind_group_layout()])
            .unwrap_or_default();

        if !specification.input_render_attachments.is_empty() {
            bind_group_layouts.extend(
                render_attachment_texture_manager
                    .create_and_get_render_attachment_texture_bind_group_layouts(
                        graphics_device,
                        &specification.input_render_attachments,
                    ),
            );
        }

        let push_constant_ranges = specification.push_constants.create_ranges();

        let shader = shader_manager
            .compute_shaders
            .get(&specification.shader_id)
            .ok_or_else(|| {
                anyhow!(
                    "Missing compute shader for compute pass: {}",
                    specification.label
                )
            })?;

        let pipeline_layout = Self::create_pipeline_layout(
            graphics_device.device(),
            &bind_group_layouts,
            &push_constant_ranges,
            &format!("{} compute pipeline layout", &specification.label),
        );

        let pipeline = Self::create_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &format!("{} compute pipeline", &specification.label),
        );

        Ok(Self {
            specification,
            pipeline,
            state,
        })
    }

    /// Records the compute pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the resources used in this compute pass are
    /// no longer available.
    pub fn record_pass(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<RenderCommandOutcome> {
        if self.state().is_disabled() {
            log::debug!("Skipping compute pass: {}", &self.specification.label);
            return Ok(RenderCommandOutcome::Skipped);
        }

        log::debug!("Recording compute pass: {}", &self.specification.label);

        let resource_group = if let Some(resource_group_id) = self.specification.resource_group_id {
            Some(
                gpu_resource_group_manager
                    .get_resource_group(resource_group_id)
                    .ok_or_else(|| anyhow!("Missing GPU resource group {}", resource_group_id))?,
            )
        } else {
            None
        };

        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            timestamp_writes: None,
            label: Some(&self.specification.label),
        });

        compute_pass.set_pipeline(&self.pipeline);

        Self::set_push_constants(
            &self.specification.push_constants,
            &mut compute_pass,
            rendering_surface,
            postprocessor,
        );

        let mut bind_groups = resource_group
            .map(|resource_group| vec![resource_group.bind_group()])
            .unwrap_or_default();

        if !self.specification.input_render_attachments.is_empty() {
            bind_groups.extend(
                render_attachment_texture_manager.get_render_attachment_texture_bind_groups(
                    &self.specification.input_render_attachments,
                ),
            );
        }

        for (index, bind_group) in bind_groups.iter().enumerate() {
            compute_pass.set_bind_group(index as u32, bind_group, &[]);
        }

        let [x, y, z] = self.specification.workgroup_counts;
        compute_pass.dispatch_workgroups(x, y, z);

        Ok(RenderCommandOutcome::Recorded)
    }

    /// Returns the state of the compute pass.
    pub fn state(&self) -> RenderCommandState {
        self.state
    }

    /// Sets the state of the compute pass.
    pub fn set_state(&mut self, state: RenderCommandState) {
        self.state = state;
    }

    /// Set whether the compute pass should be skipped.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.state = RenderCommandState::disabled_if(disabled);
    }

    fn set_push_constants(
        push_constants: &PushConstantGroup,
        compute_pass: &mut wgpu::ComputePass<'_>,
        rendering_surface: &RenderingSurface,
        postprocessor: &Postprocessor,
    ) {
        push_constants.set_push_constant_for_compute_pass_if_present(
            compute_pass,
            PushConstantVariant::InverseWindowDimensions,
            || rendering_surface.inverse_window_dimensions_push_constant(),
        );

        push_constants.set_push_constant_for_compute_pass_if_present(
            compute_pass,
            PushConstantVariant::PixelCount,
            || rendering_surface.pixel_count_push_constant(),
        );

        push_constants.set_push_constant_for_compute_pass_if_present(
            compute_pass,
            PushConstantVariant::Exposure,
            || postprocessor.capturing_camera().exposure_push_constant(),
        );

        push_constants.set_push_constant_for_compute_pass_if_present(
            compute_pass,
            PushConstantVariant::InverseExposure,
            || {
                postprocessor
                    .capturing_camera()
                    .inverse_exposure_push_constant()
            },
        );
    }

    fn create_pipeline_layout(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        push_constant_ranges: &[wgpu::PushConstantRange],
        label: &str,
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts,
            push_constant_ranges,
            label: Some(label),
        })
    }

    fn create_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &Shader,
        label: &str,
    ) -> wgpu::ComputePipeline {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: Some(layout),
            module: shader.compute_module(),
            entry_point: shader.compute_entry_point_name().unwrap(),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            label: Some(label),
        })
    }
}
