//! Render commands for voxels.

use crate::{
    VoxelObjectID,
    gpu_resource::{
        VOXEL_MODEL_ID, VoxelGPUResources, VoxelMaterialGPUResources, VoxelObjectGPUBuffers,
        VoxelPushConstantGroup, VoxelPushConstantVariant, VoxelResourceRegistries,
    },
    mesh::{CullingFrustum, VoxelMeshIndex, VoxelMeshIndexMaterials},
    shader_templates::{
        voxel_chunk_culling::VoxelChunkCullingShaderTemplate,
        voxel_geometry::VoxelGeometryShaderTemplate,
    },
};
use anyhow::{Result, anyhow};
use impact_camera::gpu_resource::{BufferableCamera, CameraGPUResource};
use impact_geometry::{Frustum, OrientedBox};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    query::TimestampQueryRegistry,
    shader::{ShaderManager, template::SpecificShaderTemplate},
    wgpu,
};
use impact_mesh::gpu_resource::VertexBufferable;
use impact_model::{
    InstanceFeature, InstanceFeatureBufferRangeID, InstanceFeatureBufferRangeManager,
    transform::{
        AsInstanceModelViewTransform, InstanceModelLightTransform, InstanceModelViewTransform,
        InstanceModelViewTransformWithPrevious,
    },
};
use impact_rendering::{
    BasicRenderingConfig, compute,
    postprocessing::Postprocessor,
    push_constant::BasicPushConstantVariant,
    render_command::{self, STANDARD_FRONT_FACE, geometry_pass::GeometryPass},
    resource::BasicGPUResources,
    surface::RenderingSurface,
};
use impact_scene::{camera::SceneCamera, model::ModelInstanceManager};
use nalgebra::Similarity3;
use std::borrow::Cow;

/// GPU commands that should be executed prior to rendering voxel objects.
#[derive(Debug)]
pub struct VoxelRenderCommands {
    geometry_pipeline: VoxelGeometryPipeline,
    chunk_culling_pass: VoxelChunkCullingPass,
}

/// Pass for culling voxel chunks that should not be rendered and updating the
/// indirect draw call arguments for the chunks accordingly.
#[derive(Debug)]
struct VoxelChunkCullingPass {
    push_constants: VoxelPushConstantGroup,
    pipeline_for_non_indexed: wgpu::ComputePipeline,
    pipeline_for_indexed: wgpu::ComputePipeline,
}

/// Pipeline for rendering the geometric and material properties of voxel
/// objects into the G-buffer attachments. Should be executed as part of the
/// [`GeometryPass`].
#[derive(Debug)]
pub struct VoxelGeometryPipeline {
    shader_template: VoxelGeometryShaderTemplate,
    push_constants: VoxelPushConstantGroup,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
}

impl VoxelRenderCommands {
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        resource_registries: &impl VoxelResourceRegistries,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        geometry_pass: &GeometryPass,
        config: &BasicRenderingConfig,
    ) -> Option<Self> {
        if resource_registries.voxel_type().n_voxel_types() == 0 {
            return None;
        }

        let geometry_pipeline = VoxelGeometryPipeline::new(
            graphics_device,
            shader_manager,
            resource_registries,
            bind_group_layout_registry,
            geometry_pass.color_target_states().to_vec(),
            Some(geometry_pass.depth_stencil_state().clone()),
            config,
        );

        let chunk_culling_pass =
            VoxelChunkCullingPass::new(graphics_device, shader_manager, bind_group_layout_registry);

        Some(Self {
            geometry_pipeline,
            chunk_culling_pass,
        })
    }

    pub fn sync_with_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        config: &BasicRenderingConfig,
    ) {
        self.geometry_pipeline
            .sync_with_config(graphics_device, shader_manager, config);
    }

    pub fn record_before_geometry_pass<R>(
        &self,
        scene_camera: Option<&SceneCamera>,
        model_instance_manager: &ModelInstanceManager,
        render_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        self.chunk_culling_pass.record_for_geometry_pass(
            scene_camera,
            model_instance_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
        )
    }

    pub fn record_to_geometry_pass<R>(
        &self,
        rendering_surface: &RenderingSurface,
        model_instance_manager: &ModelInstanceManager,
        render_resources: &R,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        self.geometry_pipeline.record(
            rendering_surface,
            model_instance_manager,
            render_resources,
            postprocessor,
            frame_counter,
            render_pass,
        )
    }

    pub fn record_before_omnidirectional_light_shadow_cubemap_face_update<R>(
        &self,
        positive_z_cubemap_face_frustum: &Frustum<f32>,
        instance_range_id: u32,
        model_instance_manager: &ModelInstanceManager,
        render_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        self.chunk_culling_pass
            .record_for_shadow_mapping_with_frustum(
                positive_z_cubemap_face_frustum,
                instance_range_id,
                model_instance_manager,
                render_resources,
                timestamp_recorder,
                command_encoder,
            )
    }

    pub fn record_before_unidirectional_light_shadow_map_cascade_update<R>(
        &self,
        cascade_frustum: &OrientedBox<f32>,
        instance_range_id: u32,
        model_instance_manager: &ModelInstanceManager,
        render_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        self.chunk_culling_pass
            .record_for_shadow_mapping_with_orthographic_frustum(
                cascade_frustum,
                instance_range_id,
                model_instance_manager,
                render_resources,
                timestamp_recorder,
                command_encoder,
            )
    }

    pub fn record_shadow_map_update<R>(
        instance_range_id: u32,
        model_instance_manager: &ModelInstanceManager,
        gpu_resources: &R,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        let voxel_object_buffer_map = gpu_resources.voxel_object_buffer();

        // Return early if there are no voxel objects
        if voxel_object_buffer_map.is_empty() {
            return Ok(());
        }

        let voxel_object_instance_buffer = model_instance_manager
            .get_model_instance_buffer(&VOXEL_MODEL_ID)
            .ok_or_else(|| anyhow!("Missing model instance buffer for voxel objects"))?;

        let voxel_object_id_buffer = voxel_object_instance_buffer
            .get_feature_buffer(VoxelObjectID::FEATURE_TYPE_ID)
            .ok_or_else(|| {
                anyhow!("Missing voxel object ID instance feature buffer for voxel objects")
            })?;

        let (_, voxel_object_ids) =
            voxel_object_id_buffer.range_with_valid_features::<VoxelObjectID>(instance_range_id);

        // Return early if no voxel objects fall within the shadow map
        if voxel_object_ids.is_empty() {
            return Ok(());
        }

        let transform_gpu_buffer = gpu_resources
            .model_instance_buffer()
            .get_model_buffer_for_feature_feature_type::<InstanceModelLightTransform>(
                &VOXEL_MODEL_ID,
            )
            .ok_or_else(|| anyhow!("Missing model-light transform GPU buffer for voxel objects"))?;

        // All draw calls share the same transform buffer
        render_pass.set_vertex_buffer(
            0,
            transform_gpu_buffer
                .vertex_gpu_buffer()
                .valid_buffer_slice(),
        );

        for voxel_object_id in voxel_object_ids {
            let voxel_object_buffers = voxel_object_buffer_map
                .get_voxel_object_buffers(*voxel_object_id)
                .ok_or_else(|| {
                    anyhow!("Missing GPU buffers for voxel object {}", voxel_object_id)
                })?;

            let chunk_count = u32::try_from(voxel_object_buffers.n_chunks()).unwrap();

            render_pass.set_vertex_buffer(
                1,
                voxel_object_buffers
                    .vertex_position_gpu_buffer()
                    .valid_buffer_slice(),
            );

            render_pass.set_index_buffer(
                voxel_object_buffers.index_gpu_buffer().valid_buffer_slice(),
                VoxelMeshIndex::format(),
            );

            let indirect_buffer = voxel_object_buffers
                .indexed_indirect_argument_gpu_buffer()
                .buffer();

            render_pass.multi_draw_indexed_indirect(indirect_buffer, 0, chunk_count);
        }

        Ok(())
    }
}

impl VoxelChunkCullingPass {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let bind_group_layout =
            VoxelObjectGPUBuffers::get_or_create_submesh_and_argument_buffer_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let push_constants = VoxelChunkCullingShaderTemplate::push_constants();

        let pipeline_layout = compute::create_compute_pipeline_layout(
            graphics_device.device(),
            &[&bind_group_layout],
            &push_constants.create_ranges(),
            "Voxel chunk culling pass compute pipeline layout",
        );

        let (_, shader_for_non_indexed) = shader_manager
            .get_or_create_compute_shader_from_template(
                graphics_device,
                &VoxelChunkCullingShaderTemplate {
                    for_indexed_draw_calls: false,
                },
            );

        let pipeline_for_non_indexed = compute::create_compute_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader_for_non_indexed,
            "Voxel chunk culling pass compute pipeline for non-indexed draw calls",
        );

        let (_, shader_for_indexed) = shader_manager.get_or_create_compute_shader_from_template(
            graphics_device,
            &VoxelChunkCullingShaderTemplate {
                for_indexed_draw_calls: true,
            },
        );

        let pipeline_for_indexed = compute::create_compute_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader_for_indexed,
            "Voxel chunk culling pass compute pipeline for indexed draw calls",
        );

        Self {
            push_constants,
            pipeline_for_non_indexed,
            pipeline_for_indexed,
        }
    }

    fn set_push_constants(
        &self,
        compute_pass: &mut wgpu::ComputePass<'_>,
        culling_frustum: CullingFrustum,
        chunk_count: u32,
        instance_idx: u32,
    ) {
        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                VoxelPushConstantVariant::CullingFrustum,
                || culling_frustum,
            );

        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                VoxelPushConstantVariant::ChunkCount,
                || chunk_count,
            );

        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                VoxelPushConstantVariant::Rendering(BasicPushConstantVariant::InstanceIdx),
                || instance_idx,
            );
    }

    fn record_for_geometry_pass<R>(
        &self,
        scene_camera: Option<&SceneCamera>,
        model_instance_manager: &ModelInstanceManager,
        render_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        let Some(scene_camera) = scene_camera else {
            return Ok(());
        };

        let frustum = scene_camera.camera().view_frustum();

        let instance_range_id = InstanceFeatureBufferRangeManager::INITIAL_RANGE_ID;

        self.record::<R, InstanceModelViewTransformWithPrevious>(
            model_instance_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
            instance_range_id,
            &|frustum_to_voxel_object_transform| {
                CullingFrustum::for_transformed_frustum(frustum, frustum_to_voxel_object_transform)
            },
            // The geometry pass uses non-indexed draw calls
            false,
            Cow::Borrowed("Voxel chunk camera culling pass"),
        )
    }

    fn record_for_shadow_mapping_with_frustum<R>(
        &self,
        frustum: &Frustum<f32>,
        instance_range_id: u32,
        model_instance_manager: &ModelInstanceManager,
        render_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        self.record::<R, InstanceModelLightTransform>(
            model_instance_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
            instance_range_id,
            &|frustum_to_voxel_object_transform| {
                CullingFrustum::for_transformed_frustum(frustum, frustum_to_voxel_object_transform)
            },
            // Shadow map update passes use indexed draw calls
            true,
            Cow::Borrowed("Voxel chunk culling pass for shadow map"),
        )
    }

    fn record_for_shadow_mapping_with_orthographic_frustum<R>(
        &self,
        orthographic_frustum: &OrientedBox<f32>,
        instance_range_id: u32,
        model_instance_manager: &ModelInstanceManager,
        render_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        self.record::<R, InstanceModelLightTransform>(
            model_instance_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
            instance_range_id,
            &|frustum_to_voxel_object_transform| {
                CullingFrustum::for_transformed_orthographic_frustum(
                    orthographic_frustum,
                    frustum_to_voxel_object_transform,
                    10000.0, // Put the apex this many chunks away to emulate infinity
                )
            },
            // Shadow map update passes use indexed draw calls
            true,
            Cow::Borrowed("Voxel chunk culling pass for shadow map"),
        )
    }

    fn record<R, F>(
        &self,
        model_instance_manager: &ModelInstanceManager,
        render_resources: &R,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
        instance_range_id: InstanceFeatureBufferRangeID,
        obtain_frustum_planes_in_voxel_object_space: &impl Fn(&Similarity3<f32>) -> CullingFrustum,
        for_indexed_draw_calls: bool,
        tag: Cow<'static, str>,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
        F: InstanceFeature + AsInstanceModelViewTransform,
    {
        let voxel_object_buffer_map = render_resources.voxel_object_buffer();

        // Return early if there are no voxel objects
        if voxel_object_buffer_map.is_empty() {
            return Ok(());
        }

        let voxel_object_instance_buffer = model_instance_manager
            .get_model_instance_buffer(&VOXEL_MODEL_ID)
            .ok_or_else(|| anyhow!("Missing model instance buffer for voxel objects"))?;

        let voxel_object_id_buffer = voxel_object_instance_buffer
            .get_feature_buffer(VoxelObjectID::FEATURE_TYPE_ID)
            .ok_or_else(|| {
                anyhow!("Missing voxel object ID instance feature buffer for voxel objects")
            })?;

        let (_, voxel_object_ids) =
            voxel_object_id_buffer.range_with_valid_features::<VoxelObjectID>(instance_range_id);

        // Return early if no voxel objects are buffered in the specified range
        if voxel_object_ids.is_empty() {
            return Ok(());
        }

        let instance_transform_buffer = voxel_object_instance_buffer
            .get_feature_buffer(F::FEATURE_TYPE_ID)
            .ok_or_else(|| {
                anyhow!("Missing transform instance feature buffer for voxel objects")
            })?;

        let timestamp_writes =
            timestamp_recorder.register_timestamp_writes_for_single_compute_pass(tag);

        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            timestamp_writes,
            label: Some("Voxel chunk culling pass"),
        });

        compute_pass.set_pipeline(if for_indexed_draw_calls {
            &self.pipeline_for_indexed
        } else {
            &self.pipeline_for_non_indexed
        });

        // It's important that we use the instance range associated with the
        // transforms and not with the voxel object IDs, since these can be
        // different due to the object ID buffer being used for both rendering
        // and shadow mapping
        let (instance_range, voxel_object_to_frustum_transforms) =
            instance_transform_buffer.range_with_valid_features::<F>(instance_range_id);

        for ((instance_idx, voxel_object_id), voxel_object_to_frustum_transform) in instance_range
            .zip(voxel_object_ids)
            .zip(voxel_object_to_frustum_transforms)
        {
            let voxel_object_buffers = voxel_object_buffer_map
                .get_voxel_object_buffers(*voxel_object_id)
                .ok_or_else(|| {
                    anyhow!("Missing GPU buffers for voxel object {}", voxel_object_id)
                })?;

            let chunk_count = u32::try_from(voxel_object_buffers.n_chunks()).unwrap();

            // We want to transform the frustum planes to the normalized voxel object space
            // where the chunk extent is unity
            let frustum_to_voxel_object_transform =
                Self::compute_transform_from_frustum_space_to_normalized_voxel_object_space(
                    *voxel_object_to_frustum_transform.as_instance_model_view_transform(),
                    voxel_object_buffers.chunk_extent(),
                );

            let frustum_planes =
                obtain_frustum_planes_in_voxel_object_space(&frustum_to_voxel_object_transform);

            self.set_push_constants(&mut compute_pass, frustum_planes, chunk_count, instance_idx);

            compute_pass.set_bind_group(
                0,
                if for_indexed_draw_calls {
                    voxel_object_buffers.submesh_and_indexed_argument_buffer_bind_group()
                } else {
                    voxel_object_buffers.submesh_and_argument_buffer_bind_group()
                },
                &[],
            );

            let [x, y, z] =
                VoxelChunkCullingShaderTemplate::determine_workgroup_counts(chunk_count);
            compute_pass.dispatch_workgroups(x, y, z);
        }

        impact_log::trace!(
            "Recorded chunk culling pass for {} voxel objects",
            voxel_object_buffer_map.len()
        );

        Ok(())
    }

    fn compute_transform_from_frustum_space_to_normalized_voxel_object_space(
        voxel_object_to_frustum_transform: InstanceModelViewTransform,
        voxel_extent: f64,
    ) -> Similarity3<f32> {
        let mut frustum_to_voxel_object_transform =
            Similarity3::from(voxel_object_to_frustum_transform);
        frustum_to_voxel_object_transform.prepend_scaling_mut(voxel_extent as f32);
        frustum_to_voxel_object_transform.inverse_mut();
        frustum_to_voxel_object_transform
    }
}

impl VoxelGeometryPipeline {
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        resource_registries: &impl VoxelResourceRegistries,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        color_target_states: Vec<Option<wgpu::ColorTargetState>>,
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        config: &BasicRenderingConfig,
    ) -> Self {
        let n_voxel_types = resource_registries.voxel_type().n_voxel_types();

        let shader_template = VoxelGeometryShaderTemplate::new(n_voxel_types, 0.1);

        shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let push_constants = VoxelGeometryShaderTemplate::push_constants();

        let camera_bind_group_layout = CameraGPUResource::get_or_create_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        let material_bind_group_layout = VoxelMaterialGPUResources::get_or_create_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        let position_and_normal_buffer_bind_group_layout =
            VoxelObjectGPUBuffers::get_or_create_position_and_normal_buffer_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &[
                &camera_bind_group_layout,
                &material_bind_group_layout,
                &position_and_normal_buffer_bind_group_layout,
            ],
            &push_constants.create_ranges(),
            "Voxel geometry pass render pipeline layout",
        );

        let pipeline = Self::create_pipeline(
            graphics_device,
            shader_manager,
            &shader_template,
            &pipeline_layout,
            &color_target_states,
            depth_stencil_state.clone(),
            config,
        );

        Self {
            shader_template,
            push_constants,
            color_target_states,
            depth_stencil_state,
            pipeline_layout,
            pipeline,
        }
    }

    fn create_pipeline(
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        shader_template: &VoxelGeometryShaderTemplate,
        pipeline_layout: &wgpu::PipelineLayout,
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        config: &BasicRenderingConfig,
    ) -> wgpu::RenderPipeline {
        let shader = &shader_manager.rendering_shaders[&shader_template.shader_id()];

        let vertex_buffer_layouts = &[
            InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT.unwrap(),
            VoxelMeshIndex::BUFFER_LAYOUT,
            VoxelMeshIndexMaterials::BUFFER_LAYOUT,
        ];

        render_command::create_render_pipeline(
            graphics_device.device(),
            pipeline_layout,
            shader,
            vertex_buffer_layouts,
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            if config.wireframe_mode_on {
                wgpu::PolygonMode::Line
            } else {
                wgpu::PolygonMode::Fill
            },
            depth_stencil_state,
            "Voxel geometry pass render pipeline",
        )
    }

    pub fn sync_with_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        config: &BasicRenderingConfig,
    ) {
        self.pipeline = Self::create_pipeline(
            graphics_device,
            shader_manager,
            &self.shader_template,
            &self.pipeline_layout,
            &self.color_target_states,
            self.depth_stencil_state.clone(),
            config,
        );
    }

    fn set_constant_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        rendering_surface: &RenderingSurface,
        postprocessor: &Postprocessor,
        frame_counter: u32,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                VoxelPushConstantVariant::Rendering(
                    BasicPushConstantVariant::InverseWindowDimensions,
                ),
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                VoxelPushConstantVariant::Rendering(BasicPushConstantVariant::FrameCounter),
                || frame_counter,
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                VoxelPushConstantVariant::Rendering(BasicPushConstantVariant::Exposure),
                || postprocessor.capturing_camera().exposure_push_constant(),
            );
    }

    fn set_per_object_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        voxel_object_buffers: &VoxelObjectGPUBuffers,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                VoxelPushConstantVariant::Rendering(BasicPushConstantVariant::GenericVec3f32),
                || voxel_object_buffers.origin_offset_in_root(),
            );
    }

    pub fn record<R>(
        &self,
        rendering_surface: &RenderingSurface,
        model_instance_manager: &ModelInstanceManager,
        gpu_resources: &R,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()>
    where
        R: BasicGPUResources + VoxelGPUResources,
    {
        let voxel_object_buffer_map = gpu_resources.voxel_object_buffer();

        // Return early if there are no voxel objects
        if voxel_object_buffer_map.is_empty() {
            return Ok(());
        }

        let voxel_object_instance_buffer = model_instance_manager
            .get_model_instance_buffer(&VOXEL_MODEL_ID)
            .ok_or_else(|| anyhow!("Missing model instance buffer for voxel objects"))?;

        let voxel_object_id_buffer = voxel_object_instance_buffer
            .get_feature_buffer(VoxelObjectID::FEATURE_TYPE_ID)
            .ok_or_else(|| {
                anyhow!("Missing voxel object ID instance feature buffer for voxel objects")
            })?;

        let visible_voxel_object_ids =
            voxel_object_id_buffer.valid_features_in_initial_range::<VoxelObjectID>();

        // Return early if no voxel objects are visible
        if visible_voxel_object_ids.is_empty() {
            return Ok(());
        }

        let transform_gpu_buffer = gpu_resources
            .model_instance_buffer()
            .get_model_buffer_for_feature_feature_type::<InstanceModelViewTransformWithPrevious>(
                &VOXEL_MODEL_ID,
            )
            .ok_or_else(|| anyhow!("Missing model-view transform GPU buffer for voxel objects"))?;

        let material_gpu_resources = gpu_resources
            .voxel_materials()
            .ok_or_else(|| anyhow!("Missing voxel material GPU resource manager"))?;

        // We don't assign the camera projection uniform bind group here, as it will
        // already have been assigned by the caller

        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_bind_group(1, material_gpu_resources.bind_group(), &[]);

        self.set_constant_push_constants(
            render_pass,
            rendering_surface,
            postprocessor,
            frame_counter,
        );

        // All draw calls share the same transform buffer
        render_pass.set_vertex_buffer(
            0,
            transform_gpu_buffer
                .vertex_gpu_buffer()
                .valid_buffer_slice(),
        );

        for voxel_object_id in visible_voxel_object_ids {
            let voxel_object_buffers = voxel_object_buffer_map
                .get_voxel_object_buffers(*voxel_object_id)
                .ok_or_else(|| {
                    anyhow!("Missing GPU buffers for voxel object {}", voxel_object_id)
                })?;

            self.set_per_object_push_constants(render_pass, voxel_object_buffers);

            let chunk_count = u32::try_from(voxel_object_buffers.n_chunks()).unwrap();

            render_pass.set_bind_group(
                2,
                voxel_object_buffers.position_and_normal_buffer_bind_group(),
                &[],
            );

            render_pass.set_vertex_buffer(
                1,
                voxel_object_buffers.index_gpu_buffer().valid_buffer_slice(),
            );

            render_pass.set_vertex_buffer(
                2,
                voxel_object_buffers
                    .index_material_gpu_buffer()
                    .valid_buffer_slice(),
            );

            let indirect_buffer = voxel_object_buffers.indirect_argument_gpu_buffer().buffer();

            render_pass.multi_draw_indirect(indirect_buffer, 0, chunk_count);
        }

        impact_log::trace!(
            "Recorded geometry pass for {} voxel objects",
            voxel_object_buffer_map.len()
        );

        Ok(())
    }
}
