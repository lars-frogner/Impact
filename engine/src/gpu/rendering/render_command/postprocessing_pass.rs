//! Generic pass for postprocessing effects.

use super::{STANDARD_FRONT_FACE, StencilValue};
use crate::{
    gpu::rendering::{
        attachment::{
            Blending, RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity, RenderAttachmentTextureManager,
        },
        postprocessing::{PostprocessingShaderTemplate, Postprocessor},
        push_constant::{RenderingPushConstantGroup, RenderingPushConstantVariant},
        render_command::begin_single_render_pass,
        resource::SynchronizedRenderResources,
        surface::RenderingSurface,
    },
    mesh::{self, VertexAttributeSet, VertexPosition, buffer::VertexBufferable},
};
use anyhow::{Result, anyhow};
use impact_camera::buffer::CameraGPUBufferManager;
use impact_gpu::{
    device::GraphicsDevice,
    query::TimestampQueryRegistry,
    resource_group::{GPUResourceGroupID, GPUResourceGroupManager},
    shader::{Shader, ShaderManager},
};
use std::borrow::Cow;

/// Generic pass for postprocessing effects.
#[derive(Debug)]
pub struct PostprocessingRenderPass {
    push_constants: RenderingPushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
    uses_camera: bool,
    gpu_resource_group_id: Option<GPUResourceGroupID>,
    stencil_test: Option<(wgpu::CompareFunction, StencilValue)>,
    writes_to_surface: bool,
    label: Cow<'static, str>,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    pipeline: wgpu::RenderPipeline,
}

impl PostprocessingRenderPass {
    /// Creates a new postprocessing render pass based on the given shader
    /// template.
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        shader_template: &impl PostprocessingShaderTemplate,
        label: Cow<'static, str>,
    ) -> Result<Self> {
        let push_constants = shader_template.push_constants();
        let input_render_attachments = shader_template.input_render_attachments();
        let output_render_attachments = shader_template.output_render_attachments();
        let uses_camera = shader_template.uses_camera();
        let gpu_resource_group_id = shader_template.gpu_resource_group_id();
        let stencil_test = shader_template.stencil_test();
        let writes_to_surface = shader_template.writes_to_surface();

        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, shader_template);

        let mut bind_group_layouts = Vec::with_capacity(8);

        if uses_camera {
            bind_group_layouts.push(CameraGPUBufferManager::get_or_create_bind_group_layout(
                graphics_device,
            ));
        }

        if !input_render_attachments.is_empty() {
            bind_group_layouts.extend(
                render_attachment_texture_manager
                    .create_and_get_render_attachment_texture_bind_group_layouts(
                        graphics_device,
                        &input_render_attachments,
                    ),
            );
        }

        if let Some(gpu_resource_group_id) = gpu_resource_group_id {
            let gpu_resource_group = gpu_resource_group_manager
                .get_resource_group(gpu_resource_group_id)
                .ok_or_else(|| {
                    anyhow!(
                        "Missing GPU resource group for postprocessing pass: {}",
                        gpu_resource_group_id
                    )
                })?;

            bind_group_layouts.push(gpu_resource_group.bind_group_layout());
        }

        let pipeline_layout = super::create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layouts,
            &push_constants.create_ranges(),
            &format!("Postprocessing pass render pipeline layout ({})", label),
        );

        let color_target_states = Self::color_target_states(
            rendering_surface,
            &output_render_attachments,
            writes_to_surface,
        );

        let depth_stencil_state = stencil_test
            .map(|(compare, _)| super::depth_stencil_state_for_stencil_testing(compare));

        let pipeline = super::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            &color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state.clone(),
            &format!("Postprocessing pass render pipeline ({})", label),
        );

        Ok(Self {
            push_constants,
            input_render_attachments,
            output_render_attachments,
            uses_camera,
            gpu_resource_group_id,
            stencil_test,
            writes_to_surface,
            label,
            color_target_states,
            depth_stencil_state,
            pipeline,
        })
    }

    fn color_target_states(
        rendering_surface: &RenderingSurface,
        output_render_attachments: &RenderAttachmentOutputDescriptionSet,
        writes_to_surface: bool,
    ) -> Vec<Option<wgpu::ColorTargetState>> {
        let mut color_target_states: Vec<_> = RenderAttachmentQuantity::all()
            .iter()
            .filter_map(|quantity| {
                if output_render_attachments
                    .quantities()
                    .contains(quantity.flag())
                {
                    let description = output_render_attachments
                        .only_description_for_quantity(*quantity)
                        .unwrap();

                    let blend_state = match description.blending() {
                        Blending::Replace => wgpu::BlendState::REPLACE,
                        Blending::Additive => super::additive_blend_state(),
                    };

                    Some(Some(wgpu::ColorTargetState {
                        format: quantity.texture_format(),
                        blend: Some(blend_state),
                        write_mask: description.write_mask(),
                    }))
                } else {
                    None
                }
            })
            .collect();

        if writes_to_surface {
            color_target_states.push(Some(wgpu::ColorTargetState {
                format: rendering_surface.texture_format(),
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::all(),
            }));
        }

        color_target_states
    }

    fn color_attachments<'a, 'b: 'a>(
        &self,
        surface_texture_view: &'b wgpu::TextureView,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'a>>> {
        let mut color_attachments = Vec::with_capacity(self.color_target_states.len());

        color_attachments.extend(
            render_attachment_texture_manager
                .request_render_attachment_textures(self.output_render_attachments.quantities())
                .map(|texture| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: texture.base_texture_view(),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })
                }),
        );

        if self.writes_to_surface {
            color_attachments.push(Some(wgpu::RenderPassColorAttachment {
                view: surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }));
        }

        color_attachments
    }

    fn depth_stencil_attachment<'a>(
        &self,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
    ) -> Option<wgpu::RenderPassDepthStencilAttachment<'a>> {
        if self.depth_stencil_state.is_some() {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: render_attachment_texture_manager
                    .render_attachment_texture(RenderAttachmentQuantity::DepthStencil)
                    .base_texture_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            })
        } else {
            None
        }
    }

    fn set_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        rendering_surface: &RenderingSurface,
        postprocessor: &Postprocessor,
        frame_counter: u32,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                RenderingPushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                RenderingPushConstantVariant::PixelCount,
                || rendering_surface.pixel_count_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                RenderingPushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                RenderingPushConstantVariant::InverseExposure,
                || {
                    postprocessor
                        .capturing_camera()
                        .inverse_exposure_push_constant()
                },
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                RenderingPushConstantVariant::FrameCounter,
                || frame_counter,
            );
    }

    /// Records the render pass into the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let color_attachments =
            self.color_attachments(surface_texture_view, render_attachment_texture_manager);

        let depth_stencil_attachment =
            self.depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &color_attachments,
            depth_stencil_attachment,
            self.label.clone(),
        );

        render_pass.set_pipeline(&self.pipeline);

        if let Some((_, stencil_value)) = self.stencil_test {
            render_pass.set_stencil_reference(stencil_value as u32);
        }

        self.set_push_constants(
            &mut render_pass,
            rendering_surface,
            postprocessor,
            frame_counter,
        );

        let mut bind_group_index = 0;

        if self.uses_camera {
            let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() else {
                return Ok(());
            };

            render_pass.set_bind_group(bind_group_index, camera_buffer_manager.bind_group(), &[]);
            bind_group_index += 1;
        }

        for bind_group in render_attachment_texture_manager
            .get_render_attachment_texture_bind_groups(&self.input_render_attachments)
        {
            render_pass.set_bind_group(bind_group_index, bind_group, &[]);
            bind_group_index += 1;
        }

        #[allow(unused_assignments)]
        if let Some(gpu_resource_group_id) = self.gpu_resource_group_id {
            let gpu_resource_group = gpu_resource_group_manager
                .get_resource_group(gpu_resource_group_id)
                .ok_or_else(|| {
                    anyhow!(
                        "Missing GPU resource group for postprocessing pass: {}",
                        gpu_resource_group_id
                    )
                })?;
            render_pass.set_bind_group(bind_group_index, gpu_resource_group.bind_group(), &[]);
            bind_group_index += 1;
        }

        let mesh_id = mesh::screen_filling_quad_mesh_id();

        let mesh_buffer_manager = render_resources
            .get_triangle_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        let position_buffer = mesh_buffer_manager
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_buffer_manager
                .triangle_mesh_index_gpu_buffer()
                .valid_buffer_slice(),
            mesh_buffer_manager.triangle_mesh_index_format(),
        );

        render_pass.draw_indexed(
            0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
            0,
            0..1,
        );

        log::trace!("Recorded postprocessing pass: {}", &self.label);

        Ok(())
    }
}

pub fn create_postprocessing_render_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    push_constant_ranges: &[wgpu::PushConstantRange],
    label: &str,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts,
        push_constant_ranges,
        label: Some(&format!(
            "Postprocessing pass render pipeline layout ({})",
            label
        )),
    })
}

pub fn create_postprocessing_render_pipeline(
    graphics_device: &GraphicsDevice,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &Shader,
    color_target_states: &[Option<wgpu::ColorTargetState>],
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    label: &str,
) -> wgpu::RenderPipeline {
    super::create_render_pipeline(
        graphics_device.device(),
        pipeline_layout,
        shader,
        &[VertexPosition::BUFFER_LAYOUT],
        color_target_states,
        STANDARD_FRONT_FACE,
        Some(wgpu::Face::Back),
        wgpu::PolygonMode::Fill,
        depth_stencil_state,
        &format!("Postprocessing pass render pipeline ({})", label),
    )
}
