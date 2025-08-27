//! Pass for computing reflected luminance due to ambient light.

use crate::{
    attachment::{
        Blending, RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
        RenderAttachmentQuantity, RenderAttachmentTextureManager,
    },
    lookup_tables,
    postprocessing::Postprocessor,
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
    render_command::{self, STANDARD_FRONT_FACE, StencilValue, begin_single_render_pass},
    resource::{BasicGPUResources, BasicResourceRegistries},
    shader_templates::ambient_light::AmbientLightShaderTemplate,
    surface::RenderingSurface,
};
use anyhow::{Result, anyhow};
use impact_camera::gpu_resource::CameraGPUResource;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice,
    query::TimestampQueryRegistry, shader::ShaderManager, wgpu,
};
use impact_light::{LightManager, gpu_resource::LightGPUResources};
use impact_mesh::{VertexAttributeSet, VertexPosition, gpu_resource::VertexBufferable};
use std::borrow::Cow;

/// Pass for computing reflected luminance due to ambient light.
#[derive(Debug)]
pub struct AmbientLightPass {
    push_constants: BasicPushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil_state: wgpu::DepthStencilState,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
}

impl AmbientLightPass {
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        resource_registries: &impl BasicResourceRegistries,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let push_constants = AmbientLightShaderTemplate::push_constants();
        let input_render_attachments = AmbientLightShaderTemplate::input_render_attachments();
        let output_render_attachments = AmbientLightShaderTemplate::output_render_attachments();

        let max_light_count = LightManager::INITIAL_LIGHT_CAPACITY;

        let shader_template = AmbientLightShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let mut bind_group_layouts = vec![CameraGPUResource::get_or_create_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        )];

        bind_group_layouts.extend(
            render_attachment_texture_manager
                .create_and_get_render_attachment_texture_bind_group_layouts(
                    graphics_device,
                    &input_render_attachments,
                )
                .cloned(),
        );

        bind_group_layouts.push(
            LightGPUResources::get_or_create_ambient_light_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            ),
        );

        let specular_ggx_reflectance_table = resource_registries
            .lookup_table()
            .get(lookup_tables::specular_ggx_reflectance::lookup_table_id())
            .expect("Missing specular GGX reflectance lookup table");

        bind_group_layouts.push(
            specular_ggx_reflectance_table
                .get_or_create_bind_group_layout(graphics_device, bind_group_layout_registry),
        );

        let bind_group_layout_refs: Vec<&wgpu::BindGroupLayout> =
            bind_group_layouts.iter().collect();
        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layout_refs,
            &push_constants.create_ranges(),
            "Ambient light pass render pipeline layout",
        );

        let color_target_states = Self::color_target_states(&output_render_attachments);

        let depth_stencil_state = render_command::depth_stencil_state_for_equal_stencil_testing();

        let pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            &color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            Some(depth_stencil_state.clone()),
            "Ambient light pass render pipeline",
        );

        Self {
            push_constants,
            input_render_attachments,
            output_render_attachments,
            color_target_states,
            depth_stencil_state,
            pipeline_layout,
            pipeline,
            max_light_count,
        }
    }

    pub fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl BasicGPUResources,
    ) {
        let Some(light_gpu_resources) = gpu_resources.light() else {
            return;
        };

        let max_light_count = light_gpu_resources.max_ambient_light_count();

        if max_light_count != self.max_light_count {
            let shader_template = AmbientLightShaderTemplate::new(max_light_count);
            let (_, shader) = shader_manager
                .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

            self.pipeline = render_command::create_render_pipeline(
                graphics_device.device(),
                &self.pipeline_layout,
                shader,
                &[VertexPosition::BUFFER_LAYOUT],
                &self.color_target_states,
                STANDARD_FRONT_FACE,
                Some(wgpu::Face::Back),
                wgpu::PolygonMode::Fill,
                Some(self.depth_stencil_state.clone()),
                "Ambient light pass render pipeline",
            );
            self.max_light_count = max_light_count;
        }
    }

    fn color_target_states(
        output_render_attachments: &RenderAttachmentOutputDescriptionSet,
    ) -> Vec<Option<wgpu::ColorTargetState>> {
        RenderAttachmentQuantity::all()
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
                        Blending::Additive => render_command::additive_blend_state(),
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
            .collect()
    }

    fn color_attachments<'a, 'b: 'a>(
        &self,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'a>>> {
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
            })
            .collect()
    }

    fn depth_stencil_attachment(
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
    ) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
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
        }
    }

    fn set_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        rendering_surface: &RenderingSurface,
        postprocessor: &Postprocessor,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );
    }

    pub fn record(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let Some(camera_gpu_resources) = gpu_resources.camera() else {
            return Ok(());
        };
        let Some(light_gpu_resources) = gpu_resources.light() else {
            return Ok(());
        };

        let color_attachments = self.color_attachments(render_attachment_texture_manager);

        let depth_stencil_attachment =
            Self::depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &color_attachments,
            Some(depth_stencil_attachment),
            Cow::Borrowed("Ambient light pass"),
        );

        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_stencil_reference(StencilValue::PhysicalModel as u32);

        self.set_push_constants(&mut render_pass, rendering_surface, postprocessor);

        render_pass.set_bind_group(0, camera_gpu_resources.bind_group(), &[]);

        let mut bind_group_index = 1;
        for bind_group in render_attachment_texture_manager
            .get_render_attachment_texture_bind_groups(&self.input_render_attachments)
        {
            render_pass.set_bind_group(bind_group_index, bind_group, &[]);
            bind_group_index += 1;
        }

        render_pass.set_bind_group(
            bind_group_index,
            light_gpu_resources.ambient_light_bind_group(),
            &[],
        );
        bind_group_index += 1;

        let specular_ggx_reflectance_lookup_table_bind_group = gpu_resources
            .lookup_table_bind_group()
            .get(lookup_tables::specular_ggx_reflectance::lookup_table_id())
            .ok_or_else(|| {
                anyhow!("Missing GPU resource group for specular GGX reflectance lookup table")
            })?;

        render_pass.set_bind_group(
            bind_group_index,
            &specular_ggx_reflectance_lookup_table_bind_group.bind_group,
            &[],
        );

        let mesh_id = AmbientLightShaderTemplate::light_volume_mesh_id();

        let mesh_gpu_resources = gpu_resources
            .triangle_mesh()
            .get(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU resources for mesh {}", mesh_id))?;

        let position_buffer = mesh_gpu_resources
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_gpu_resources
                .triangle_mesh_index_gpu_buffer()
                .valid_buffer_slice(),
            mesh_gpu_resources.triangle_mesh_index_format(),
        );

        render_pass.draw_indexed(
            0..u32::try_from(mesh_gpu_resources.n_indices()).unwrap(),
            0,
            0..1,
        );

        impact_log::trace!("Recorded ambient light pass (1 draw call)");

        Ok(())
    }
}
