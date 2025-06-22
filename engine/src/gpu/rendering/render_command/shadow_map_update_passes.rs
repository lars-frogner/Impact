//! Passes for filling shadow maps.

use super::{INVERTED_FRONT_FACE, STANDARD_FRONT_FACE};
use crate::{
    gpu::rendering::{
        push_constant::{RenderingPushConstantGroup, RenderingPushConstantVariant},
        render_command::begin_single_render_pass,
        resource::SynchronizedRenderResources,
        shader_templates::{
            omnidirectional_light_shadow_map::OmnidirectionalLightShadowMapShaderTemplate,
            unidirectional_light_shadow_map::UnidirectionalLightShadowMapShaderTemplate,
        },
    },
    light,
    material::{MaterialLibrary, MaterialShaderInput},
    mesh::{VertexAttributeSet, VertexPosition, buffer::VertexBufferable},
    model::{
        InstanceFeature, InstanceFeatureManager, ModelID, buffer::InstanceFeatureGPUBufferManager,
        transform::InstanceModelLightTransform,
    },
    scene::ModelInstanceNode,
    voxel::render_commands::VoxelRenderCommands,
};
use anyhow::{Result, anyhow};
use impact_containers::HashSet;
use impact_geometry::CubemapFace;
use impact_gpu::{device::GraphicsDevice, query::TimestampQueryRegistry, shader::ShaderManager};
use impact_light::{
    LightFlags, LightStorage, MAX_SHADOW_MAP_CASCADES,
    buffer::LightGPUBufferManager,
    shadow_map::{CascadeIdx, SHADOW_MAP_FORMAT},
};
use std::borrow::Cow;

/// Passes for filling the faces of each omnidirectional light shadow cubemap.
#[derive(Debug)]
pub struct OmnidirectionalLightShadowMapUpdatePasses {
    push_constants: RenderingPushConstantGroup,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
    models: HashSet<ModelID>,
}

/// Passes for filling the cascades of each unidirectional light shadow map.
#[derive(Debug)]
pub struct UnidirectionalLightShadowMapUpdatePasses {
    push_constants: RenderingPushConstantGroup,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
    models: HashSet<ModelID>,
}

impl OmnidirectionalLightShadowMapUpdatePasses {
    const CLEAR_COLOR: wgpu::Color = wgpu::Color::WHITE;

    pub fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;

        let shader_template = OmnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let push_constants = OmnidirectionalLightShadowMapShaderTemplate::push_constants();

        let pipeline_layout = super::create_render_pipeline_layout(
            graphics_device.device(),
            &[
                LightGPUBufferManager::get_or_create_shadowable_omnidirectional_light_bind_group_layout(
                    graphics_device,
                ),
            ],
            &push_constants.create_ranges(),
            "Omnidirectional light shadow map update render pipeline layout",
        );

        let color_target_states = Self::color_target_states();

        let pipeline = super::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[
                InstanceModelLightTransform::BUFFER_LAYOUT,
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
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
    ) -> Result<()> {
        self.sync_models_with_render_resources(material_library, render_resources);
        self.sync_shader_with_render_resources(graphics_device, shader_manager, render_resources)
    }

    fn sync_models_with_render_resources(
        &mut self,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
    ) {
        // We only keep models that actually have buffered model-to-light transforms,
        // otherwise they will not be rendered into the shadow map anyway
        #[allow(clippy::ptr_arg)]
        fn has_features(buffer_managers: &Vec<InstanceFeatureGPUBufferManager>) -> bool {
            buffer_managers
                .get(ModelInstanceNode::model_light_transform_feature_idx())
                .is_some_and(|buffer| buffer.n_features() > 0)
        }

        let instance_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        self.models.retain(|model_id| {
            instance_feature_buffer_managers
                .get(model_id)
                .is_some_and(has_features)
        });

        for (model_id, instance_feature_buffer_manager) in instance_feature_buffer_managers {
            if self.models.contains(model_id) {
                continue;
            }
            if !has_features(instance_feature_buffer_manager) {
                continue;
            }
            if let Some(material_specification) = material_library
                .get_material_specification(model_id.material_handle().material_id())
            {
                if let MaterialShaderInput::Physical(_) = material_specification.shader_input() {
                    self.models.insert(*model_id);
                }
            }
        }
    }

    fn sync_shader_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_resources: &SynchronizedRenderResources,
    ) -> Result<()> {
        let light_buffer_manager = render_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

        let max_light_count = light_buffer_manager.max_shadowable_omnidirectional_light_count();

        if max_light_count != self.max_light_count {
            let shader_template = OmnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
            let (_, shader) = shader_manager
                .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

            self.pipeline = super::create_render_pipeline(
                graphics_device.device(),
                &self.pipeline_layout,
                shader,
                &[
                    InstanceModelLightTransform::BUFFER_LAYOUT,
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

        Ok(())
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
                RenderingPushConstantVariant::LightIdx,
                || light_idx,
            );
    }

    pub fn record(
        &self,
        light_storage: &LightStorage,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        shadow_mapping_enabled: bool,
        voxel_render_commands: &VoxelRenderCommands,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let light_buffer_manager = render_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

        let shadow_map_manager = light_buffer_manager.omnidirectional_light_shadow_map_manager();
        let shadow_map_textures = shadow_map_manager.textures();

        if shadow_map_textures.is_empty() {
            return Ok(());
        }

        let mut pass_count = 0;
        let mut draw_call_count = 0;

        for (light_idx, (&light_id, shadow_map_texture)) in light_buffer_manager
            .shadowable_omnidirectional_light_ids()
            .iter()
            .zip(shadow_map_textures)
            .enumerate()
        {
            let omnidirectional_light = light_storage.shadowable_omnidirectional_light(light_id);

            if omnidirectional_light
                .flags()
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
                    light::light_id_to_instance_feature_buffer_range_id(light_id)
                        + cubemap_face.as_idx_u32();

                if shadow_mapping_enabled {
                    voxel_render_commands
                        .record_before_omnidirectional_light_shadow_cubemap_face_update(
                            &positive_z_cubemap_face_frustum,
                            instance_range_id,
                            instance_feature_manager,
                            render_resources,
                            timestamp_recorder,
                            command_encoder,
                        )?;
                }

                let shadow_cubemap_face_texture_view = shadow_map_texture.face_view(cubemap_face);

                let color_attachments = Self::color_attachments(shadow_cubemap_face_texture_view);

                let mut render_pass = begin_single_render_pass(
                    command_encoder,
                    timestamp_recorder,
                    &color_attachments,
                    None,
                    Cow::Owned(format!(
                        "Update pass for shadow cubemap face {:?} for light {}",
                        cubemap_face, light_idx,
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
                    light_buffer_manager.shadowable_omnidirectional_light_bind_group(),
                    &[],
                );

                for model_id in &self.models {
                    let transform_buffer_manager = render_resources
                        .get_instance_feature_buffer_managers(model_id)
                        .and_then(|buffers| {
                            buffers.get(ModelInstanceNode::model_light_transform_feature_idx())
                        })
                        .ok_or_else(|| {
                            anyhow!(
                                "Missing model-light transform GPU buffer for model {}",
                                model_id
                            )
                        })?;

                    let transform_range = transform_buffer_manager.feature_range(instance_range_id);

                    if transform_range.is_empty() {
                        continue;
                    }

                    render_pass.set_vertex_buffer(
                        0,
                        transform_buffer_manager
                            .vertex_gpu_buffer()
                            .valid_buffer_slice(),
                    );

                    let mesh_id = model_id.mesh_id();

                    let mesh_buffer_manager = render_resources
                        .get_triangle_mesh_buffer_manager(mesh_id)
                        .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

                    let position_buffer = mesh_buffer_manager
                        .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                        .next()
                        .unwrap();

                    render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

                    render_pass.set_index_buffer(
                        mesh_buffer_manager
                            .triangle_mesh_index_gpu_buffer()
                            .valid_buffer_slice(),
                        mesh_buffer_manager.triangle_mesh_index_format(),
                    );

                    render_pass.draw_indexed(
                        0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                        0,
                        transform_range,
                    );
                    draw_call_count += 1;
                }

                VoxelRenderCommands::record_shadow_map_update(
                    instance_range_id,
                    instance_feature_manager,
                    render_resources,
                    &mut render_pass,
                )?;
            }
        }

        log::trace!(
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

    pub fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;

        let shader_template = UnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let push_constants = UnidirectionalLightShadowMapShaderTemplate::push_constants();

        let pipeline_layout = super::create_render_pipeline_layout(
            graphics_device.device(),
            &[
                LightGPUBufferManager::get_or_create_shadowable_unidirectional_light_bind_group_layout(
                    graphics_device,
                ),
            ],
            &push_constants.create_ranges(),
            "Unidirectional light shadow map update render pipeline layout",
        );

        let color_target_states = Self::color_target_states();

        let pipeline = super::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[
                InstanceModelLightTransform::BUFFER_LAYOUT,
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
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
    ) -> Result<()> {
        self.sync_models_with_render_resources(material_library, render_resources);
        self.sync_shader_with_render_resources(graphics_device, shader_manager, render_resources)
    }

    fn sync_models_with_render_resources(
        &mut self,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
    ) {
        // We only keep models that actually have buffered model-to-light transforms,
        // otherwise they will not be rendered into the shadow map anyway
        #[allow(clippy::ptr_arg)]
        fn has_features(buffer_managers: &Vec<InstanceFeatureGPUBufferManager>) -> bool {
            buffer_managers
                .get(ModelInstanceNode::model_light_transform_feature_idx())
                .is_some_and(|buffer| buffer.n_features() > 0)
        }

        let instance_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        self.models.retain(|model_id| {
            instance_feature_buffer_managers
                .get(model_id)
                .is_some_and(has_features)
        });

        for (model_id, instance_feature_buffer_manager) in instance_feature_buffer_managers {
            if self.models.contains(model_id) {
                continue;
            }
            if !has_features(instance_feature_buffer_manager) {
                continue;
            }
            if let Some(material_specification) = material_library
                .get_material_specification(model_id.material_handle().material_id())
            {
                if let MaterialShaderInput::Physical(_) = material_specification.shader_input() {
                    self.models.insert(*model_id);
                }
            }
        }
    }

    fn sync_shader_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_resources: &SynchronizedRenderResources,
    ) -> Result<()> {
        let light_buffer_manager = render_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

        let max_light_count = light_buffer_manager.max_shadowable_unidirectional_light_count();

        if max_light_count != self.max_light_count {
            let shader_template = UnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
            let (_, shader) = shader_manager
                .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

            self.pipeline = super::create_render_pipeline(
                graphics_device.device(),
                &self.pipeline_layout,
                shader,
                &[
                    InstanceModelLightTransform::BUFFER_LAYOUT,
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

        Ok(())
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
                RenderingPushConstantVariant::LightIdx,
                || light_idx,
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                RenderingPushConstantVariant::ShadowMapArrayIdx,
                || cascade_idx,
            );
    }

    pub fn record(
        &self,
        light_storage: &LightStorage,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        shadow_mapping_enabled: bool,
        voxel_render_commands: &VoxelRenderCommands,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let light_buffer_manager = render_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

        let shadow_map_manager = light_buffer_manager.unidirectional_light_shadow_map_manager();
        let shadow_map_textures = shadow_map_manager.textures();

        if shadow_map_textures.is_empty() {
            return Ok(());
        }

        let mut pass_count = 0;
        let mut draw_call_count = 0;

        for (light_idx, (&light_id, shadow_map_texture)) in light_buffer_manager
            .shadowable_unidirectional_light_ids()
            .iter()
            .zip(shadow_map_textures)
            .enumerate()
        {
            let unidirectional_light = light_storage.shadowable_unidirectional_light(light_id);

            if unidirectional_light
                .flags()
                .contains(LightFlags::IS_DISABLED)
            {
                continue;
            }

            for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                // When updating the shadow map, we don't use model view transforms but rather
                // the model to light space tranforms that have been written to the range
                // dedicated for the active light in the transform buffer.

                // Offset the light's buffer range ID with the cascade index to get the index
                // for the range of transforms for the specific cascade
                let instance_range_id =
                    light::light_id_to_instance_feature_buffer_range_id(light_id) + cascade_idx;

                let cascade_frustum = unidirectional_light
                    .create_light_space_orthographic_obb_for_cascade(cascade_idx);

                if shadow_mapping_enabled {
                    voxel_render_commands
                        .record_before_unidirectional_light_shadow_map_cascade_update(
                            &cascade_frustum,
                            instance_range_id,
                            instance_feature_manager,
                            render_resources,
                            timestamp_recorder,
                            command_encoder,
                        )?;
                }

                let shadow_map_cascade_texture_view = shadow_map_texture.cascade_view(cascade_idx);

                let color_attachments = Self::color_attachments(shadow_map_cascade_texture_view);

                let mut render_pass = begin_single_render_pass(
                    command_encoder,
                    timestamp_recorder,
                    &color_attachments,
                    None,
                    Cow::Owned(format!(
                        "Update pass for shadow map cascade {} for light {}",
                        cascade_idx, light_idx,
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
                    light_buffer_manager.shadowable_unidirectional_light_bind_group(),
                    &[],
                );

                for model_id in &self.models {
                    let transform_buffer_manager = render_resources
                        .get_instance_feature_buffer_managers(model_id)
                        .and_then(|buffers| {
                            buffers.get(ModelInstanceNode::model_light_transform_feature_idx())
                        })
                        .ok_or_else(|| {
                            anyhow!(
                                "Missing model-light transform GPU buffer for model {}",
                                model_id
                            )
                        })?;

                    let transform_range = transform_buffer_manager.feature_range(instance_range_id);

                    if transform_range.is_empty() {
                        continue;
                    }

                    render_pass.set_vertex_buffer(
                        0,
                        transform_buffer_manager
                            .vertex_gpu_buffer()
                            .valid_buffer_slice(),
                    );

                    let mesh_id = model_id.mesh_id();

                    let mesh_buffer_manager = render_resources
                        .get_triangle_mesh_buffer_manager(mesh_id)
                        .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

                    let position_buffer = mesh_buffer_manager
                        .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                        .next()
                        .unwrap();

                    render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

                    render_pass.set_index_buffer(
                        mesh_buffer_manager
                            .triangle_mesh_index_gpu_buffer()
                            .valid_buffer_slice(),
                        mesh_buffer_manager.triangle_mesh_index_format(),
                    );

                    render_pass.draw_indexed(
                        0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                        0,
                        transform_range,
                    );
                    draw_call_count += 1;
                }

                VoxelRenderCommands::record_shadow_map_update(
                    instance_range_id,
                    instance_feature_manager,
                    render_resources,
                    &mut render_pass,
                )?;
            }
        }

        log::trace!(
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
