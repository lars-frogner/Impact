//! GPU compute passes.

use crate::gpu::{
    push_constant::PushConstantGroup,
    push_constant::PushConstantVariant,
    rendering::{postprocessing::Postprocessor, surface::RenderingSurface},
    resource_group::{GPUResourceGroupID, GPUResourceGroupManager},
    shader::{template::ComputeShaderTemplate, Shader, ShaderManager},
    texture::attachment::{RenderAttachmentInputDescriptionSet, RenderAttachmentTextureManager},
    GraphicsDevice,
};
use anyhow::{anyhow, Result};
use std::borrow::Cow;

/// Helper for invoking a compute shader with specific resources.
#[derive(Debug)]
pub struct ComputePass {
    shader_template: Box<dyn ComputeShaderTemplate>,
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    gpu_resource_group_id: GPUResourceGroupID,
    label: Cow<'static, str>,
    pipeline: wgpu::ComputePipeline,
}

impl ComputePass {
    /// Creates a new compute pass for the given compute shader template.
    ///
    /// # Errors
    /// Returns an error if the GPU resource group for the compute pass is
    /// unavailable.
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        shader_template: impl ComputeShaderTemplate + 'static,
        label: Cow<'static, str>,
    ) -> Result<Self> {
        let push_constants = shader_template.push_constants();
        let input_render_attachments = shader_template.input_render_attachments();
        let gpu_resource_group_id = shader_template.gpu_resource_group_id();

        let (_, shader) = shader_manager
            .get_or_create_compute_shader_from_template(graphics_device, &shader_template);

        let mut bind_group_layouts = Vec::with_capacity(4);

        if !input_render_attachments.is_empty() {
            bind_group_layouts.extend(
                render_attachment_texture_manager
                    .create_and_get_render_attachment_texture_bind_group_layouts(
                        graphics_device,
                        &input_render_attachments,
                    ),
            );
        }

        let gpu_resource_group = gpu_resource_group_manager
            .get_resource_group(gpu_resource_group_id)
            .ok_or_else(|| {
                anyhow!(
                    "Missing GPU resource group for compute pass: {}",
                    gpu_resource_group_id
                )
            })?;

        bind_group_layouts.push(gpu_resource_group.bind_group_layout());

        let pipeline_layout = create_compute_pipeline_layout(
            graphics_device.device(),
            &bind_group_layouts,
            &push_constants.create_ranges(),
            &format!("Compute pipeline layout ({})", label),
        );

        let pipeline = create_compute_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &format!("Compute pipeline ({})", label),
        );

        Ok(Self {
            shader_template: Box::new(shader_template),
            push_constants,
            input_render_attachments,
            gpu_resource_group_id,
            label,
            pipeline,
        })
    }

    /// Records the compute pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if the GPU resource group for the compute pass is
    /// unavailable.
    pub fn record(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            timestamp_writes: None,
            label: Some(&format!("Compute pass ({})", self.label)),
        });

        compute_pass.set_pipeline(&self.pipeline);

        self.set_push_constants(&mut compute_pass, rendering_surface, postprocessor);

        let mut bind_group_index = 0;

        if !self.input_render_attachments.is_empty() {
            for bind_group in render_attachment_texture_manager
                .get_render_attachment_texture_bind_groups(&self.input_render_attachments)
            {
                compute_pass.set_bind_group(bind_group_index, bind_group, &[]);
                bind_group_index += 1;
            }
        }

        let gpu_resource_group = gpu_resource_group_manager
            .get_resource_group(self.gpu_resource_group_id)
            .ok_or_else(|| anyhow!("Missing GPU resource group {}", self.gpu_resource_group_id))?;

        compute_pass.set_bind_group(bind_group_index, gpu_resource_group.bind_group(), &[]);

        let [x, y, z] = self
            .shader_template
            .determine_workgroup_counts(rendering_surface);
        compute_pass.dispatch_workgroups(x, y, z);

        Ok(())
    }

    fn set_push_constants(
        &self,
        compute_pass: &mut wgpu::ComputePass<'_>,
        rendering_surface: &RenderingSurface,
        postprocessor: &Postprocessor,
    ) {
        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                PushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                PushConstantVariant::PixelCount,
                || rendering_surface.pixel_count_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                PushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                PushConstantVariant::InverseExposure,
                || {
                    postprocessor
                        .capturing_camera()
                        .inverse_exposure_push_constant()
                },
            );
    }
}

fn create_compute_pipeline_layout(
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

fn create_compute_pipeline(
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
