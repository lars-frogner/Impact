//! Pass for computing reflected luminance due to directional lights.

use crate::{
    attachment::{
        RenderAttachmentInputDescriptionSet, RenderAttachmentQuantity,
        RenderAttachmentTextureManager,
    },
    postprocessing::Postprocessor,
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
    render_command::{self, STANDARD_FRONT_FACE, StencilValue, begin_single_render_pass},
    resource::BasicGPUResources,
    shader_templates::{
        omnidirectional_light::OmnidirectionalLightShaderTemplate,
        shadowable_omnidirectional_light::ShadowableOmnidirectionalLightShaderTemplate,
        shadowable_unidirectional_light::ShadowableUnidirectionalLightShaderTemplate,
        unidirectional_light::UnidirectionalLightShaderTemplate,
    },
    surface::RenderingSurface,
};
use anyhow::{Result, anyhow};
use impact_camera::buffer::CameraGPUBufferManager;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice,
    query::TimestampQueryRegistry, shader::ShaderManager, wgpu,
};
use impact_light::{
    LightFlags, LightStorage,
    buffer::{
        LightGPUBufferManager, OmnidirectionalLightShadowMapManager,
        UnidirectionalLightShadowMapManager,
    },
};
use impact_mesh::{VertexAttributeSet, VertexPosition, gpu_resource::VertexBufferable};
use std::borrow::Cow;

/// Pass for computing reflected luminance due to directional lights.
#[derive(Debug)]
pub struct DirectionalLightPass {
    push_constants: BasicPushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachment_quantity: RenderAttachmentQuantity,
    color_target_state: wgpu::ColorTargetState,
    depth_stencil_state: wgpu::DepthStencilState,
    omnidirectional_light_pipeline: OmnidirectionalLightPipeline,
    shadowable_omnidirectional_light_pipeline: ShadowableOmnidirectionalLightPipeline,
    unidirectional_light_pipeline: UnidirectionalLightPipeline,
    shadowable_unidirectional_light_pipeline: ShadowableUnidirectionalLightPipeline,
}

#[derive(Debug)]
struct OmnidirectionalLightPipeline {
    layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
}

#[derive(Debug)]
struct ShadowableOmnidirectionalLightPipeline {
    layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
}

#[derive(Debug)]
struct UnidirectionalLightPipeline {
    layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
}

#[derive(Debug)]
struct ShadowableUnidirectionalLightPipeline {
    layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
}

impl DirectionalLightPass {
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let push_constants = OmnidirectionalLightShaderTemplate::push_constants();
        let input_render_attachments =
            OmnidirectionalLightShaderTemplate::input_render_attachments();
        let output_render_attachment_quantity =
            OmnidirectionalLightShaderTemplate::output_render_attachment_quantity();

        assert_eq!(
            &push_constants,
            &ShadowableOmnidirectionalLightShaderTemplate::push_constants()
        );
        assert_eq!(
            &input_render_attachments,
            &ShadowableOmnidirectionalLightShaderTemplate::input_render_attachments()
        );
        assert_eq!(
            &output_render_attachment_quantity,
            &ShadowableOmnidirectionalLightShaderTemplate::output_render_attachment_quantity()
        );
        assert_eq!(
            &push_constants,
            &UnidirectionalLightShaderTemplate::push_constants()
        );
        assert_eq!(
            &input_render_attachments,
            &UnidirectionalLightShaderTemplate::input_render_attachments()
        );
        assert_eq!(
            &output_render_attachment_quantity,
            &UnidirectionalLightShaderTemplate::output_render_attachment_quantity()
        );
        assert_eq!(
            &push_constants,
            &ShadowableUnidirectionalLightShaderTemplate::push_constants()
        );
        assert_eq!(
            &input_render_attachments,
            &ShadowableUnidirectionalLightShaderTemplate::input_render_attachments()
        );
        assert_eq!(
            &output_render_attachment_quantity,
            &ShadowableUnidirectionalLightShaderTemplate::output_render_attachment_quantity()
        );

        let mut bind_group_layouts = vec![CameraGPUBufferManager::get_or_create_bind_group_layout(
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

        let push_constant_ranges = push_constants.create_ranges();

        let color_target_state = Self::color_target_state(output_render_attachment_quantity);

        let depth_stencil_state = render_command::depth_stencil_state_for_equal_stencil_testing();

        let omnidirectional_light_pipeline = OmnidirectionalLightPipeline::new(
            graphics_device,
            shader_manager,
            bind_group_layout_registry,
            bind_group_layouts.clone(),
            &push_constant_ranges,
            &[Some(color_target_state.clone())],
            Some(depth_stencil_state.clone()),
        );

        let shadowable_omnidirectional_light_pipeline = ShadowableOmnidirectionalLightPipeline::new(
            graphics_device,
            shader_manager,
            bind_group_layout_registry,
            bind_group_layouts.clone(),
            &push_constant_ranges,
            &[Some(color_target_state.clone())],
            Some(depth_stencil_state.clone()),
        );

        let unidirectional_light_pipeline = UnidirectionalLightPipeline::new(
            graphics_device,
            shader_manager,
            bind_group_layout_registry,
            bind_group_layouts.clone(),
            &push_constant_ranges,
            &[Some(color_target_state.clone())],
            Some(depth_stencil_state.clone()),
        );

        let shadowable_unidirectional_light_pipeline = ShadowableUnidirectionalLightPipeline::new(
            graphics_device,
            shader_manager,
            bind_group_layout_registry,
            bind_group_layouts,
            &push_constant_ranges,
            &[Some(color_target_state.clone())],
            Some(depth_stencil_state.clone()),
        );

        Self {
            push_constants,
            input_render_attachments,
            output_render_attachment_quantity,
            color_target_state,
            depth_stencil_state,
            omnidirectional_light_pipeline,
            shadowable_omnidirectional_light_pipeline,
            unidirectional_light_pipeline,
            shadowable_unidirectional_light_pipeline,
        }
    }

    pub fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl BasicGPUResources,
    ) -> Result<()> {
        let light_buffer_manager = gpu_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

        if light_buffer_manager.max_omnidirectional_light_count()
            != self.omnidirectional_light_pipeline.max_light_count
        {
            self.omnidirectional_light_pipeline
                .update_shader_with_new_max_light_count(
                    graphics_device,
                    shader_manager,
                    &[Some(self.color_target_state.clone())],
                    Some(self.depth_stencil_state.clone()),
                    light_buffer_manager.max_omnidirectional_light_count(),
                );
        }

        if light_buffer_manager.max_shadowable_omnidirectional_light_count()
            != self
                .shadowable_omnidirectional_light_pipeline
                .max_light_count
        {
            self.shadowable_omnidirectional_light_pipeline
                .update_shader_with_new_max_light_count(
                    graphics_device,
                    shader_manager,
                    &[Some(self.color_target_state.clone())],
                    Some(self.depth_stencil_state.clone()),
                    light_buffer_manager.max_shadowable_omnidirectional_light_count(),
                );
        }

        if light_buffer_manager.max_unidirectional_light_count()
            != self.unidirectional_light_pipeline.max_light_count
        {
            self.unidirectional_light_pipeline
                .update_shader_with_new_max_light_count(
                    graphics_device,
                    shader_manager,
                    &[Some(self.color_target_state.clone())],
                    Some(self.depth_stencil_state.clone()),
                    light_buffer_manager.max_unidirectional_light_count(),
                );
        }

        if light_buffer_manager.max_shadowable_unidirectional_light_count()
            != self
                .shadowable_unidirectional_light_pipeline
                .max_light_count
        {
            self.shadowable_unidirectional_light_pipeline
                .update_shader_with_new_max_light_count(
                    graphics_device,
                    shader_manager,
                    &[Some(self.color_target_state.clone())],
                    Some(self.depth_stencil_state.clone()),
                    light_buffer_manager.max_shadowable_unidirectional_light_count(),
                );
        }

        Ok(())
    }

    fn color_target_state(
        output_render_attachment_quantity: RenderAttachmentQuantity,
    ) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: output_render_attachment_quantity.texture_format(),
            blend: Some(render_command::additive_blend_state()),
            write_mask: wgpu::ColorWrites::COLOR,
        }
    }

    fn color_attachment<'a, 'b: 'a>(
        &self,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
    ) -> wgpu::RenderPassColorAttachment<'a> {
        let texture = render_attachment_texture_manager
            .render_attachment_texture(self.output_render_attachment_quantity);
        wgpu::RenderPassColorAttachment {
            view: texture.base_texture_view(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        }
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

    fn set_constant_push_constants(
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

    fn set_light_idx_push_constant(&self, render_pass: &mut wgpu::RenderPass<'_>, light_idx: u32) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::LightIdx,
                || light_idx,
            );
    }

    pub fn record(
        &self,
        rendering_surface: &RenderingSurface,
        light_storage: &LightStorage,
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let Some(camera_buffer_manager) = gpu_resources.get_camera_buffer_manager() else {
            return Ok(());
        };

        let n_omnidirectional_lights = light_storage.omnidirectional_lights().len();
        let n_shadowable_omnidirectional_lights =
            light_storage.shadowable_omnidirectional_lights().len();
        let n_unidirectional_lights = light_storage.unidirectional_lights().len();
        let n_shadowable_unidirectional_lights =
            light_storage.shadowable_unidirectional_lights().len();

        let n_lights = n_omnidirectional_lights
            + n_shadowable_omnidirectional_lights
            + n_unidirectional_lights
            + n_shadowable_unidirectional_lights;

        if n_lights == 0 {
            return Ok(());
        }

        let light_buffer_manager = gpu_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

        let color_attachment = self.color_attachment(render_attachment_texture_manager);

        let depth_stencil_attachment =
            Self::depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[Some(color_attachment)],
            Some(depth_stencil_attachment),
            Cow::Borrowed("Directional light pass"),
        );

        render_pass.set_stencil_reference(StencilValue::PhysicalModel as u32);

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        let mut bind_group_index = 1;
        for bind_group in render_attachment_texture_manager
            .get_render_attachment_texture_bind_groups(&self.input_render_attachments)
        {
            render_pass.set_bind_group(bind_group_index, bind_group, &[]);
            bind_group_index += 1;
        }

        let light_bind_group_index = bind_group_index;
        let shadow_map_bind_group_index = bind_group_index + 1;

        let mut draw_call_count = 0;

        // **** Omnidirectional lights ****

        if n_omnidirectional_lights > 0 {
            render_pass.set_pipeline(&self.omnidirectional_light_pipeline.pipeline);

            self.set_constant_push_constants(&mut render_pass, rendering_surface, postprocessor);

            render_pass.set_bind_group(
                light_bind_group_index,
                light_buffer_manager.omnidirectional_light_bind_group(),
                &[],
            );

            let mesh_id = OmnidirectionalLightShaderTemplate::light_volume_mesh_id();

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

            let n_indices = u32::try_from(mesh_gpu_resources.n_indices()).unwrap();

            for (light_idx, light) in light_storage.omnidirectional_lights().iter().enumerate() {
                if light.flags().contains(LightFlags::IS_DISABLED) {
                    continue;
                }

                self.set_light_idx_push_constant(
                    &mut render_pass,
                    u32::try_from(light_idx).unwrap(),
                );

                render_pass.draw_indexed(0..n_indices, 0, 0..1);
                draw_call_count += 1;
            }
        }

        // **** Shadowable omnidirectional lights ****
        if n_shadowable_omnidirectional_lights > 0 {
            render_pass.set_pipeline(&self.shadowable_omnidirectional_light_pipeline.pipeline);

            self.set_constant_push_constants(&mut render_pass, rendering_surface, postprocessor);

            render_pass.set_bind_group(
                light_bind_group_index,
                light_buffer_manager.shadowable_omnidirectional_light_bind_group(),
                &[],
            );

            let mesh_id = ShadowableOmnidirectionalLightShaderTemplate::light_volume_mesh_id();

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

            let n_indices = u32::try_from(mesh_gpu_resources.n_indices()).unwrap();

            let omnidirectional_light_shadow_map_manager =
                light_buffer_manager.omnidirectional_light_shadow_map_manager();
            let omnidirectional_light_shadow_map_textures =
                omnidirectional_light_shadow_map_manager.textures();

            assert_eq!(
                omnidirectional_light_shadow_map_textures.len(),
                n_shadowable_omnidirectional_lights
            );

            for (light_idx, (light, shadow_map_texture)) in light_storage
                .shadowable_omnidirectional_lights()
                .iter()
                .zip(omnidirectional_light_shadow_map_textures)
                .enumerate()
            {
                if light.flags().contains(LightFlags::IS_DISABLED) {
                    continue;
                }

                self.set_light_idx_push_constant(
                    &mut render_pass,
                    u32::try_from(light_idx).unwrap(),
                );

                render_pass.set_bind_group(
                    shadow_map_bind_group_index,
                    shadow_map_texture.bind_group(),
                    &[],
                );

                render_pass.draw_indexed(0..n_indices, 0, 0..1);
                draw_call_count += 1;
            }
        }

        // **** Unidirectional lights ****
        if n_unidirectional_lights > 0 {
            render_pass.set_pipeline(&self.unidirectional_light_pipeline.pipeline);

            self.set_constant_push_constants(&mut render_pass, rendering_surface, postprocessor);

            render_pass.set_bind_group(
                light_bind_group_index,
                light_buffer_manager.unidirectional_light_bind_group(),
                &[],
            );

            let mesh_id = UnidirectionalLightShaderTemplate::light_volume_mesh_id();

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

            let n_indices = u32::try_from(mesh_gpu_resources.n_indices()).unwrap();

            for (light_idx, light) in light_storage.unidirectional_lights().iter().enumerate() {
                if light.flags().contains(LightFlags::IS_DISABLED) {
                    continue;
                }

                self.set_light_idx_push_constant(
                    &mut render_pass,
                    u32::try_from(light_idx).unwrap(),
                );

                render_pass.draw_indexed(0..n_indices, 0, 0..1);
                draw_call_count += 1;
            }
        }

        // **** Shadowable unidirectional lights ****
        if n_shadowable_unidirectional_lights > 0 {
            render_pass.set_pipeline(&self.shadowable_unidirectional_light_pipeline.pipeline);

            self.set_constant_push_constants(&mut render_pass, rendering_surface, postprocessor);

            render_pass.set_bind_group(
                light_bind_group_index,
                light_buffer_manager.shadowable_unidirectional_light_bind_group(),
                &[],
            );

            let mesh_id = ShadowableUnidirectionalLightShaderTemplate::light_volume_mesh_id();

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

            let n_indices = u32::try_from(mesh_gpu_resources.n_indices()).unwrap();

            let unidirectional_light_shadow_map_manager =
                light_buffer_manager.unidirectional_light_shadow_map_manager();
            let unidirectional_light_shadow_map_textures =
                unidirectional_light_shadow_map_manager.textures();

            assert_eq!(
                unidirectional_light_shadow_map_textures.len(),
                n_shadowable_unidirectional_lights
            );

            for (light_idx, (light, shadow_map_texture)) in light_storage
                .shadowable_unidirectional_lights()
                .iter()
                .zip(unidirectional_light_shadow_map_textures)
                .enumerate()
            {
                if light.flags().contains(LightFlags::IS_DISABLED) {
                    continue;
                }

                self.set_light_idx_push_constant(
                    &mut render_pass,
                    u32::try_from(light_idx).unwrap(),
                );

                render_pass.set_bind_group(
                    shadow_map_bind_group_index,
                    shadow_map_texture.bind_group(),
                    &[],
                );

                render_pass.draw_indexed(0..n_indices, 0, 0..1);
                draw_call_count += 1;
            }
        }

        impact_log::trace!(
            "Recorded lighting pass for {n_omnidirectional_lights} unshadowable and {n_shadowable_omnidirectional_lights} shadowable omnidirectional lights and {n_unidirectional_lights} unshadowable and {n_shadowable_unidirectional_lights} shadowable unidirectional lights ({draw_call_count} draw calls)",
        );

        Ok(())
    }
}

impl OmnidirectionalLightPipeline {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        mut bind_group_layouts: Vec<wgpu::BindGroupLayout>,
        push_constant_ranges: &[wgpu::PushConstantRange],
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
    ) -> OmnidirectionalLightPipeline {
        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;
        let shader_template = OmnidirectionalLightShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        bind_group_layouts.push(
            LightGPUBufferManager::get_or_create_omnidirectional_light_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            ),
        );

        let bind_group_layout_refs: Vec<&wgpu::BindGroupLayout> =
            bind_group_layouts.iter().collect();
        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layout_refs,
            push_constant_ranges,
            "Omnidirectional light pass render pipeline layout",
        );

        let pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Omnidirectional light pass render pipeline",
        );

        OmnidirectionalLightPipeline {
            layout: pipeline_layout,
            pipeline,
            max_light_count,
        }
    }

    fn update_shader_with_new_max_light_count(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        new_max_light_count: usize,
    ) {
        let shader_template = OmnidirectionalLightShaderTemplate::new(new_max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        self.pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &self.layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Omnidirectional light pass render pipeline",
        );
        self.max_light_count = new_max_light_count;
    }
}

impl ShadowableOmnidirectionalLightPipeline {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        mut bind_group_layouts: Vec<wgpu::BindGroupLayout>,
        push_constant_ranges: &[wgpu::PushConstantRange],
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
    ) -> ShadowableOmnidirectionalLightPipeline {
        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;
        let shader_template = ShadowableOmnidirectionalLightShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        bind_group_layouts.push(
            LightGPUBufferManager::get_or_create_shadowable_omnidirectional_light_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            ),
        );

        bind_group_layouts.push(
            OmnidirectionalLightShadowMapManager::get_or_create_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            ),
        );

        let bind_group_layout_refs: Vec<&wgpu::BindGroupLayout> =
            bind_group_layouts.iter().collect();
        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layout_refs,
            push_constant_ranges,
            "Shadowable omnidirectional light pass render pipeline layout",
        );

        let pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Shadowable omnidirectional light pass render pipeline",
        );

        ShadowableOmnidirectionalLightPipeline {
            layout: pipeline_layout,
            pipeline,
            max_light_count,
        }
    }

    fn update_shader_with_new_max_light_count(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        new_max_light_count: usize,
    ) {
        let shader_template =
            ShadowableOmnidirectionalLightShaderTemplate::new(new_max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        self.pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &self.layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Shadowable omnidirectional light pass render pipeline",
        );
        self.max_light_count = new_max_light_count;
    }
}

impl UnidirectionalLightPipeline {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        mut bind_group_layouts: Vec<wgpu::BindGroupLayout>,
        push_constant_ranges: &[wgpu::PushConstantRange],
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
    ) -> UnidirectionalLightPipeline {
        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;
        let shader_template = UnidirectionalLightShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        bind_group_layouts.push(
            LightGPUBufferManager::get_or_create_unidirectional_light_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            ),
        );

        let bind_group_layout_refs: Vec<&wgpu::BindGroupLayout> =
            bind_group_layouts.iter().collect();
        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layout_refs,
            push_constant_ranges,
            "Unidirectional light pass render pipeline layout",
        );

        let pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Unidirectional light pass render pipeline",
        );

        UnidirectionalLightPipeline {
            layout: pipeline_layout,
            pipeline,
            max_light_count,
        }
    }

    fn update_shader_with_new_max_light_count(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        new_max_light_count: usize,
    ) {
        let shader_template = UnidirectionalLightShaderTemplate::new(new_max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        self.pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &self.layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Unidirectional light pass render pipeline",
        );
        self.max_light_count = new_max_light_count;
    }
}

impl ShadowableUnidirectionalLightPipeline {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        mut bind_group_layouts: Vec<wgpu::BindGroupLayout>,
        push_constant_ranges: &[wgpu::PushConstantRange],
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
    ) -> ShadowableUnidirectionalLightPipeline {
        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;
        let shader_template = ShadowableUnidirectionalLightShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        bind_group_layouts.push(
            LightGPUBufferManager::get_or_create_shadowable_unidirectional_light_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            ),
        );

        bind_group_layouts.push(
            UnidirectionalLightShadowMapManager::get_or_create_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            ),
        );

        let bind_group_layout_refs: Vec<&wgpu::BindGroupLayout> =
            bind_group_layouts.iter().collect();
        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layout_refs,
            push_constant_ranges,
            "Shadowable unidirectional light pass render pipeline layout",
        );

        let pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Shadowable unidirectional light pass render pipeline",
        );

        ShadowableUnidirectionalLightPipeline {
            layout: pipeline_layout,
            pipeline,
            max_light_count,
        }
    }

    fn update_shader_with_new_max_light_count(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        new_max_light_count: usize,
    ) {
        let shader_template = ShadowableUnidirectionalLightShaderTemplate::new(new_max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        self.pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &self.layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Shadowable unidirectional light pass render pipeline",
        );
        self.max_light_count = new_max_light_count;
    }
}
