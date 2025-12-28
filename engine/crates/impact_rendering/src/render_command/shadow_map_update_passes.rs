//! Passes for filling shadow maps.

use crate::{
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
    render_command::{self, INVERTED_FRONT_FACE, STANDARD_FRONT_FACE, begin_single_render_pass},
    resource::BasicGPUResources,
    shader_templates::{
        omnidirectional_light_shadow_map::OmnidirectionalLightShadowMapShaderTemplate,
        unidirectional_light_shadow_map::UnidirectionalLightShadowMapShaderTemplate,
    },
};
use anyhow::{Result, anyhow};
use impact_containers::HashSet;
use impact_geometry::{FrustumA, OrientedBoxA, projection::CubemapFace};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice, shader::ShaderManager,
    timestamp_query::TimestampQueryRegistry, wgpu,
};
use impact_light::{
    LightFlags, LightManager, MAX_SHADOW_MAP_CASCADES,
    gpu_resource::LightGPUResources,
    shadow_map::{CascadeIdx, SHADOW_MAP_FORMAT},
};
use impact_mesh::{VertexAttributeSet, VertexPosition, gpu_resource::VertexBufferable};
use impact_model::{
    InstanceFeature, InstanceFeatureBufferRangeID, gpu_resource::InstanceFeatureGPUBuffer,
    transform::InstanceModelLightTransform,
};
use impact_scene::model::ModelID;
use std::borrow::Cow;

/// Passes for filling the faces of each omnidirectional light shadow cubemap.
#[derive(Debug)]
pub struct OmnidirectionalLightShadowMapUpdatePasses {
    push_constants: BasicPushConstantGroup,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
    models: HashSet<ModelID>,
}

/// Passes for filling the cascades of each unidirectional light shadow map.
#[derive(Debug)]
pub struct UnidirectionalLightShadowMapUpdatePasses {
    push_constants: BasicPushConstantGroup,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
    models: HashSet<ModelID>,
}

impl OmnidirectionalLightShadowMapUpdatePasses {
    const CLEAR_COLOR: wgpu::Color = wgpu::Color::WHITE;

    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let max_light_count = LightManager::INITIAL_LIGHT_CAPACITY;

        let shader_template = OmnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let push_constants = OmnidirectionalLightShadowMapShaderTemplate::push_constants();

        let omnidirectional_light_bind_group_layout =
            LightGPUResources::get_or_create_shadowable_omnidirectional_light_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &[&omnidirectional_light_bind_group_layout],
            &push_constants.create_ranges(),
            "Omnidirectional light shadow map update render pipeline layout",
        );

        let color_target_states = Self::color_target_states();

        let pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[
                InstanceModelLightTransform::BUFFER_LAYOUT.unwrap(),
                VertexPosition::BUFFER_LAYOUT,
            ],
            &color_target_states,
            // The cubemap projection does not flip the z-axis, so the front
            // faces will have the opposite winding order compared to normal
            INVERTED_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            None,
            "Omnidirectional light shadow map update render pipeline",
        );

        Self {
            push_constants,
            color_target_states,
            pipeline_layout,
            pipeline,
            max_light_count,
            models: HashSet::default(),
        }
    }

    pub fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl BasicGPUResources,
    ) {
        self.sync_models_with_render_resources(gpu_resources);
        self.sync_shader_with_render_resources(graphics_device, shader_manager, gpu_resources);
    }

    fn sync_models_with_render_resources(&mut self, gpu_resources: &impl BasicGPUResources) {
        // We only keep models that actually have buffered model-to-light transforms,
        // otherwise they will not be rendered into the shadow map anyway
        fn has_features(buffers: &[InstanceFeatureGPUBuffer]) -> bool {
            buffers
                .iter()
                .find(|buffer| buffer.is_for_feature_type::<InstanceModelLightTransform>())
                .is_some_and(|buffer| buffer.n_features() > 0)
        }

        let model_instance_buffers = gpu_resources.model_instance_buffer();

        self.models.retain(|model_id| {
            model_instance_buffers
                .get_model_buffers(model_id)
                .is_some_and(has_features)
        });

        for (model_id, instance_feature_buffers) in model_instance_buffers.iter() {
            if self.models.contains(model_id) {
                continue;
            }
            if !has_features(instance_feature_buffers) {
                continue;
            }
            let Some(material) = gpu_resources.material().get(model_id.material_id()) else {
                continue;
            };
            if material.is_physical()
                && gpu_resources
                    .material_template()
                    .contains(material.template_id)
            {
                self.models.insert(*model_id);
            }
        }
    }

    fn sync_shader_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl BasicGPUResources,
    ) {
        let Some(light_gpu_resources) = gpu_resources.light() else {
            return;
        };

        let max_light_count = light_gpu_resources.max_shadowable_omnidirectional_light_count();

        if max_light_count != self.max_light_count {
            let shader_template = OmnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
            let (_, shader) = shader_manager
                .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

            self.pipeline = render_command::create_render_pipeline(
                graphics_device.device(),
                &self.pipeline_layout,
                shader,
                &[
                    InstanceModelLightTransform::BUFFER_LAYOUT.unwrap(),
                    VertexPosition::BUFFER_LAYOUT,
                ],
                &self.color_target_states,
                INVERTED_FRONT_FACE,
                Some(wgpu::Face::Back),
                wgpu::PolygonMode::Fill,
                None,
                "Omnidirectional light shadow map update render pipeline",
            );
            self.max_light_count = max_light_count;
        }
    }

    fn color_target_states() -> Vec<Option<wgpu::ColorTargetState>> {
        vec![Some(wgpu::ColorTargetState {
            format: SHADOW_MAP_FORMAT,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Min,
                },
                alpha: wgpu::BlendComponent::default(),
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })]
    }

    fn color_attachments(
        shadow_cubemap_face_texture_view: &wgpu::TextureView,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'_>>> {
        vec![Some(wgpu::RenderPassColorAttachment {
            view: shadow_cubemap_face_texture_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(Self::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })]
    }

    fn set_light_idx_push_constant(&self, render_pass: &mut wgpu::RenderPass<'_>, light_idx: u32) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::LightIdx,
                || light_idx,
            );
    }

    pub fn record<R>(
        &self,
        gpu_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        shadow_mapping_enabled: bool,
        command_encoder: &mut wgpu::CommandEncoder,
        record_additional_commands_before_face_update: &mut impl FnMut(
            &FrustumA,
            InstanceFeatureBufferRangeID,
            &mut TimestampQueryRegistry<'_>,
            &mut wgpu::CommandEncoder,
        ) -> Result<()>,
        perform_additional_draw_calls_after_face_update: &mut impl FnMut(
            InstanceFeatureBufferRangeID,
            &mut wgpu::RenderPass<'_>,
        ) -> Result<()>,
    ) -> Result<()>
    where
        R: BasicGPUResources,
    {
        let Some(light_gpu_resources) = gpu_resources.light() else {
            return Ok(());
        };

        let shadow_map_manager = light_gpu_resources.omnidirectional_light_shadow_map_manager();
        let shadow_map_textures = shadow_map_manager.textures();

        if shadow_map_textures.is_empty() {
            return Ok(());
        }

        let mut pass_count = 0;
        let mut draw_call_count = 0;

        for (light_idx, (omnidirectional_light, shadow_map_texture)) in light_gpu_resources
            .shadowable_omnidirectional_light_metadata()
            .iter()
            .zip(shadow_map_textures)
            .enumerate()
        {
            if omnidirectional_light
                .flags
                .contains(LightFlags::IS_DISABLED)
            {
                continue;
            }

            let positive_z_cubemap_face_frustum =
                omnidirectional_light.compute_light_space_frustum_for_positive_z_face();

            for cubemap_face in CubemapFace::all() {
                // When updating the shadow map, we don't use model view transforms but rather
                // the model to light space tranforms that have been written to the range
                // dedicated for the active light in the transform buffer.

                // Offset the light's buffer range ID with the face index to get the index for
                // the range of transforms for the specific cubemap face
                let instance_range_id =
                    impact_scene::light::light_id_to_instance_feature_buffer_range_id(
                        omnidirectional_light.id,
                    ) + cubemap_face.as_idx_u32();

                if shadow_mapping_enabled {
                    record_additional_commands_before_face_update(
                        &positive_z_cubemap_face_frustum,
                        instance_range_id,
                        timestamp_recorder,
                        command_encoder,
                    )?;
                }

                let shadow_cubemap_face_texture_view = shadow_map_texture.face_view(cubemap_face);

                let color_attachments = Self::color_attachments(shadow_cubemap_face_texture_view);

                let (mut render_pass, _span_guard) = begin_single_render_pass(
                    command_encoder,
                    timestamp_recorder,
                    &color_attachments,
                    None,
                    Cow::Owned(format!(
                        "Update pass for shadow cubemap face {cubemap_face:?} for light {light_idx}",
                    )),
                );
                pass_count += 1;

                if !shadow_mapping_enabled {
                    // If shadow mapping is disabled, we don't do anything in the render pass,
                    // which means the shadow map textures will just be cleared
                    continue;
                }

                render_pass.set_pipeline(&self.pipeline);

                self.set_light_idx_push_constant(
                    &mut render_pass,
                    u32::try_from(light_idx).unwrap(),
                );

                render_pass.set_bind_group(
                    0,
                    light_gpu_resources.shadowable_omnidirectional_light_bind_group(),
                    &[],
                );

                for model_id in &self.models {
                    let transform_buffer = gpu_resources
                        .model_instance_buffer()
                        .get_model_buffer_for_feature_feature_type::<InstanceModelLightTransform>(
                            model_id,
                        )
                        .ok_or_else(|| {
                            anyhow!(
                                "Missing model-light transform GPU buffer for model {}",
                                model_id
                            )
                        })?;

                    let transform_range = transform_buffer.feature_range(instance_range_id);

                    if transform_range.is_empty() {
                        continue;
                    }

                    render_pass.set_vertex_buffer(
                        0,
                        transform_buffer.vertex_gpu_buffer().valid_buffer_slice(),
                    );

                    let mesh_id = model_id.triangle_mesh_id();

                    let mesh_gpu_resources = gpu_resources
                        .triangle_mesh()
                        .get(mesh_id)
                        .ok_or_else(|| anyhow!("Missing GPU resources for mesh {}", mesh_id))?;

                    let position_buffer = mesh_gpu_resources
                        .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                        .next()
                        .unwrap();

                    render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

                    render_pass.set_index_buffer(
                        mesh_gpu_resources
                            .triangle_mesh_index_gpu_buffer()
                            .valid_buffer_slice(),
                        mesh_gpu_resources.triangle_mesh_index_format(),
                    );

                    render_pass.draw_indexed(
                        0..u32::try_from(mesh_gpu_resources.n_indices()).unwrap(),
                        0,
                        transform_range,
                    );
                    draw_call_count += 1;
                }

                perform_additional_draw_calls_after_face_update(
                    instance_range_id,
                    &mut render_pass,
                )?;
            }
        }

        impact_log::trace!(
            "Recorded shadow map update passes for {} omnidirectional lights and {} models ({} passes, {} draw calls)",
            shadow_map_textures.len(),
            self.models.len(),
            pass_count,
            draw_call_count
        );

        Ok(())
    }
}

impl UnidirectionalLightShadowMapUpdatePasses {
    const CLEAR_COLOR: wgpu::Color = wgpu::Color::WHITE;

    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let max_light_count = LightManager::INITIAL_LIGHT_CAPACITY;

        let shader_template = UnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let push_constants = UnidirectionalLightShadowMapShaderTemplate::push_constants();

        let unidirectional_light_bind_group_layout =
            LightGPUResources::get_or_create_shadowable_unidirectional_light_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &[&unidirectional_light_bind_group_layout],
            &push_constants.create_ranges(),
            "Unidirectional light shadow map update render pipeline layout",
        );

        let color_target_states = Self::color_target_states();

        let pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[
                InstanceModelLightTransform::BUFFER_LAYOUT.unwrap(),
                VertexPosition::BUFFER_LAYOUT,
            ],
            &color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            None,
            "Unidirectional light shadow map update render pipeline",
        );

        Self {
            push_constants,
            color_target_states,
            pipeline_layout,
            pipeline,
            max_light_count,
            models: HashSet::default(),
        }
    }

    pub fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl BasicGPUResources,
    ) {
        self.sync_models_with_render_resources(gpu_resources);
        self.sync_shader_with_render_resources(graphics_device, shader_manager, gpu_resources);
    }

    fn sync_models_with_render_resources(&mut self, gpu_resources: &impl BasicGPUResources) {
        // We only keep models that actually have buffered model-to-light transforms,
        // otherwise they will not be rendered into the shadow map anyway
        fn has_features(buffers: &[InstanceFeatureGPUBuffer]) -> bool {
            buffers
                .iter()
                .find(|buffer| buffer.is_for_feature_type::<InstanceModelLightTransform>())
                .is_some_and(|buffer| buffer.n_features() > 0)
        }

        let model_instance_buffers = gpu_resources.model_instance_buffer();

        self.models.retain(|model_id| {
            model_instance_buffers
                .get_model_buffers(model_id)
                .is_some_and(has_features)
        });

        for (model_id, instance_feature_buffers) in model_instance_buffers.iter() {
            if self.models.contains(model_id) {
                continue;
            }
            if !has_features(instance_feature_buffers) {
                continue;
            }
            let Some(material) = gpu_resources.material().get(model_id.material_id()) else {
                continue;
            };
            if material.is_physical()
                && gpu_resources
                    .material_template()
                    .contains(material.template_id)
            {
                self.models.insert(*model_id);
            }
        }
    }

    fn sync_shader_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl BasicGPUResources,
    ) {
        let Some(light_gpu_resources) = gpu_resources.light() else {
            return;
        };

        let max_light_count = light_gpu_resources.max_shadowable_unidirectional_light_count();

        if max_light_count != self.max_light_count {
            let shader_template = UnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
            let (_, shader) = shader_manager
                .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

            self.pipeline = render_command::create_render_pipeline(
                graphics_device.device(),
                &self.pipeline_layout,
                shader,
                &[
                    InstanceModelLightTransform::BUFFER_LAYOUT.unwrap(),
                    VertexPosition::BUFFER_LAYOUT,
                ],
                &self.color_target_states,
                STANDARD_FRONT_FACE,
                Some(wgpu::Face::Back),
                wgpu::PolygonMode::Fill,
                None,
                "Unidirectional light shadow map update render pipeline",
            );
            self.max_light_count = max_light_count;
        }
    }

    fn color_target_states() -> Vec<Option<wgpu::ColorTargetState>> {
        vec![Some(wgpu::ColorTargetState {
            format: SHADOW_MAP_FORMAT,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Min,
                },
                alpha: wgpu::BlendComponent::default(),
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })]
    }

    fn color_attachments(
        shadow_map_cascade_texture_view: &wgpu::TextureView,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'_>>> {
        vec![Some(wgpu::RenderPassColorAttachment {
            view: shadow_map_cascade_texture_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(Self::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })]
    }

    fn set_light_and_cascade_idx_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        light_idx: u32,
        cascade_idx: CascadeIdx,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::LightIdx,
                || light_idx,
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::ShadowMapArrayIdx,
                || cascade_idx,
            );
    }

    pub fn record<R>(
        &self,
        gpu_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        shadow_mapping_enabled: bool,
        command_encoder: &mut wgpu::CommandEncoder,
        record_additional_commands_before_cascade_update: &mut impl FnMut(
            &OrientedBoxA,
            InstanceFeatureBufferRangeID,
            &mut TimestampQueryRegistry<'_>,
            &mut wgpu::CommandEncoder,
        ) -> Result<()>,
        perform_additional_draw_calls_after_cascade_update: &mut impl FnMut(
            InstanceFeatureBufferRangeID,
            &mut wgpu::RenderPass<'_>,
        ) -> Result<()>,
    ) -> Result<()>
    where
        R: BasicGPUResources,
    {
        let Some(light_gpu_resources) = gpu_resources.light() else {
            return Ok(());
        };

        let shadow_map_manager = light_gpu_resources.unidirectional_light_shadow_map_manager();
        let shadow_map_textures = shadow_map_manager.textures();

        if shadow_map_textures.is_empty() {
            return Ok(());
        }

        let mut pass_count = 0;
        let mut draw_call_count = 0;

        for (light_idx, (unidirectional_light, shadow_map_texture)) in light_gpu_resources
            .shadowable_unidirectional_light_metadata()
            .iter()
            .zip(shadow_map_textures)
            .enumerate()
        {
            if unidirectional_light.flags.contains(LightFlags::IS_DISABLED) {
                continue;
            }

            for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                // When updating the shadow map, we don't use model view transforms but rather
                // the model to light space tranforms that have been written to the range
                // dedicated for the active light in the transform buffer.

                // Offset the light's buffer range ID with the cascade index to get the index
                // for the range of transforms for the specific cascade
                let instance_range_id =
                    impact_scene::light::light_id_to_instance_feature_buffer_range_id(
                        unidirectional_light.id,
                    ) + cascade_idx;

                let cascade_frustum = unidirectional_light
                    .create_light_space_orthographic_obb_for_cascade(cascade_idx);

                if shadow_mapping_enabled {
                    record_additional_commands_before_cascade_update(
                        &cascade_frustum,
                        instance_range_id,
                        timestamp_recorder,
                        command_encoder,
                    )?;
                }

                let shadow_map_cascade_texture_view = shadow_map_texture.cascade_view(cascade_idx);

                let color_attachments = Self::color_attachments(shadow_map_cascade_texture_view);

                let (mut render_pass, _timestamp_span_guard) = begin_single_render_pass(
                    command_encoder,
                    timestamp_recorder,
                    &color_attachments,
                    None,
                    Cow::Owned(format!(
                        "Update pass for shadow map cascade {cascade_idx} for light {light_idx}",
                    )),
                );
                pass_count += 1;

                if !shadow_mapping_enabled {
                    // If shadow mapping is disabled, we don't do anything in the render pass, which
                    // means the shadow map textures will just be cleared
                    continue;
                }

                render_pass.set_pipeline(&self.pipeline);

                self.set_light_and_cascade_idx_push_constants(
                    &mut render_pass,
                    u32::try_from(light_idx).unwrap(),
                    cascade_idx,
                );

                render_pass.set_bind_group(
                    0,
                    light_gpu_resources.shadowable_unidirectional_light_bind_group(),
                    &[],
                );

                for model_id in &self.models {
                    let transform_buffer = gpu_resources
                        .model_instance_buffer()
                        .get_model_buffer_for_feature_feature_type::<InstanceModelLightTransform>(
                            model_id,
                        )
                        .ok_or_else(|| {
                            anyhow!(
                                "Missing model-light transform GPU buffer for model {}",
                                model_id
                            )
                        })?;

                    let transform_range = transform_buffer.feature_range(instance_range_id);

                    if transform_range.is_empty() {
                        continue;
                    }

                    render_pass.set_vertex_buffer(
                        0,
                        transform_buffer.vertex_gpu_buffer().valid_buffer_slice(),
                    );

                    let mesh_id = model_id.triangle_mesh_id();

                    let mesh_gpu_resources = gpu_resources
                        .triangle_mesh()
                        .get(mesh_id)
                        .ok_or_else(|| anyhow!("Missing GPU resources for mesh {}", mesh_id))?;

                    let position_buffer = mesh_gpu_resources
                        .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                        .next()
                        .unwrap();

                    render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

                    render_pass.set_index_buffer(
                        mesh_gpu_resources
                            .triangle_mesh_index_gpu_buffer()
                            .valid_buffer_slice(),
                        mesh_gpu_resources.triangle_mesh_index_format(),
                    );

                    render_pass.draw_indexed(
                        0..u32::try_from(mesh_gpu_resources.n_indices()).unwrap(),
                        0,
                        transform_range,
                    );
                    draw_call_count += 1;
                }

                perform_additional_draw_calls_after_cascade_update(
                    instance_range_id,
                    &mut render_pass,
                )?;
            }
        }

        impact_log::trace!(
            "Recorded shadow map update passes for {} unidirectional lights and {} models ({} passes, {} draw calls)",
            shadow_map_textures.len(),
            self.models.len(),
            pass_count,
            draw_call_count
        );

        Ok(())
    }
}

pub fn depth_stencil_state_for_shadow_map_update() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: SHADOW_MAP_FORMAT,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        // Biasing is applied manually in shader
        bias: wgpu::DepthBiasState::default(),
    }
}
