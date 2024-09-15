//! Render commands for voxels.

use crate::{
    camera::{buffer::CameraGPUBufferManager, SceneCamera},
    geometry::{Frustum, OrientedBox},
    gpu::{
        compute,
        push_constant::{PushConstantGroup, PushConstantVariant},
        query::TimestampQueryRegistry,
        rendering::{
            fre,
            postprocessing::Postprocessor,
            render_command::{self, STANDARD_FRONT_FACE},
            resource::SynchronizedRenderResources,
            surface::RenderingSurface,
            RenderingConfig,
        },
        shader::{
            template::{
                voxel_chunk_culling::VoxelChunkCullingShaderTemplate,
                voxel_geometry::VoxelGeometryShaderTemplate,
            },
            ShaderManager,
        },
        GraphicsDevice,
    },
    mesh::buffer::VertexBufferable,
    model::{
        transform::{
            AsInstanceModelViewTransform, InstanceModelLightTransform, InstanceModelViewTransform,
            InstanceModelViewTransformWithPrevious,
        },
        InstanceFeature, InstanceFeatureBufferRangeID, InstanceFeatureBufferRangeManager,
        InstanceFeatureManager,
    },
    scene::ModelInstanceNode,
    voxel::{
        buffer::VoxelObjectGPUBufferManager,
        entity::VOXEL_MODEL_ID,
        mesh::{FrustumPlanes, VoxelMeshVertexNormalVector, VoxelMeshVertexPosition},
        VoxelObjectID,
    },
};
use anyhow::{anyhow, Result};
use nalgebra::Similarity3;
use std::borrow::Cow;

/// GPU commands that should be executed prior to rendering voxel objects.
#[derive(Debug)]
pub struct VoxelRenderCommands {
    chunk_culling_pass: VoxelChunkCullingPass,
}

/// Pass for culling voxel chunks that should not be rendered and updating the
/// indirect draw call arguments for the chunks accordingly.
#[derive(Debug)]
struct VoxelChunkCullingPass {
    push_constants: PushConstantGroup,
    pipeline: wgpu::ComputePipeline,
}

/// Pipeline for rendering the geometric and material properties of voxel
/// objects into the G-buffer attachments. Should be executed as part of the
/// [`GeometryPass`](crate::gpu::rendering::render_command::GeometryPass).
#[derive(Debug)]
pub struct VoxelGeometryPipeline {
    push_constants: PushConstantGroup,
    pipeline: wgpu::RenderPipeline,
}

impl VoxelRenderCommands {
    pub fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let chunk_culling_pass = VoxelChunkCullingPass::new(graphics_device, shader_manager);
        Self { chunk_culling_pass }
    }

    pub fn record_before_geometry_pass(
        &self,
        scene_camera: Option<&SceneCamera<fre>>,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.chunk_culling_pass.record_for_camera(
            scene_camera,
            instance_feature_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
        )
    }

    pub fn record_before_omnidirectional_light_shadow_cubemap_face_update(
        &self,
        positive_z_cubemap_face_frustum: &Frustum<fre>,
        instance_range_id: u32,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.chunk_culling_pass
            .record_for_shadow_mapping_with_frustum(
                positive_z_cubemap_face_frustum,
                instance_range_id,
                instance_feature_manager,
                render_resources,
                timestamp_recorder,
                command_encoder,
            )
    }

    pub fn record_before_unidirectional_light_shadow_map_cascade_update(
        &self,
        cascade_frustum: &OrientedBox<fre>,
        instance_range_id: u32,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.chunk_culling_pass
            .record_for_shadow_mapping_with_orthographic_frustum(
                cascade_frustum,
                instance_range_id,
                instance_feature_manager,
                render_resources,
                timestamp_recorder,
                command_encoder,
            )
    }

    pub fn record_shadow_map_update(
        instance_range_id: u32,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()> {
        let voxel_object_buffer_managers = render_resources.voxel_object_buffer_managers();

        // Return early if there are no voxel objects
        if voxel_object_buffer_managers.is_empty() {
            return Ok(());
        }

        let voxel_object_instance_buffer = instance_feature_manager
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

        let instance_feature_gpu_buffer_managers = render_resources
            .get_instance_feature_buffer_managers(&VOXEL_MODEL_ID)
            .ok_or_else(|| anyhow!("Missing instance GPU buffers for voxel objects"))?;

        let transform_gpu_buffer_manager = instance_feature_gpu_buffer_managers
            .get(ModelInstanceNode::model_light_transform_feature_idx())
            .ok_or_else(|| anyhow!("Missing model-light transform GPU buffer for voxel objects"))?;

        // All draw calls share the same transform buffer
        render_pass.set_vertex_buffer(
            0,
            transform_gpu_buffer_manager
                .vertex_gpu_buffer()
                .valid_buffer_slice(),
        );

        for voxel_object_id in voxel_object_ids {
            let voxel_object_buffer_manager = voxel_object_buffer_managers
                .get(voxel_object_id)
                .ok_or_else(|| {
                    anyhow!("Missing GPU buffer for voxel object {}", voxel_object_id)
                })?;

            let chunk_count = u32::try_from(voxel_object_buffer_manager.n_chunks()).unwrap();

            render_pass.set_vertex_buffer(
                1,
                voxel_object_buffer_manager
                    .vertex_position_gpu_buffer()
                    .valid_buffer_slice(),
            );

            render_pass.set_index_buffer(
                voxel_object_buffer_manager
                    .index_gpu_buffer()
                    .valid_buffer_slice(),
                voxel_object_buffer_manager.index_format(),
            );

            let indirect_buffer = voxel_object_buffer_manager
                .indirect_argument_gpu_buffer()
                .buffer();

            render_pass.multi_draw_indexed_indirect(indirect_buffer, 0, chunk_count);
        }

        Ok(())
    }
}

impl VoxelChunkCullingPass {
    fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let bind_group_layout =
            VoxelObjectGPUBufferManager::get_or_create_submesh_and_argument_buffer_bind_group_layout(
                graphics_device,
            );

        let push_constants = VoxelChunkCullingShaderTemplate::push_constants();

        let pipeline_layout = compute::create_compute_pipeline_layout(
            graphics_device.device(),
            &[bind_group_layout],
            &push_constants.create_ranges(),
            "Voxel chunk culling pass compute pipeline layout",
        );

        let (_, shader) = shader_manager.get_or_create_compute_shader_from_template(
            graphics_device,
            &VoxelChunkCullingShaderTemplate,
        );

        let pipeline = compute::create_compute_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            "Voxel chunk culling pass compute pipeline",
        );

        Self {
            push_constants,
            pipeline,
        }
    }

    fn set_push_constants(
        &self,
        compute_pass: &mut wgpu::ComputePass<'_>,
        frustum_planes: FrustumPlanes,
        chunk_count: u32,
        instance_idx: u32,
    ) {
        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                PushConstantVariant::FrustumPlanes,
                || frustum_planes,
            );

        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                PushConstantVariant::ChunkCount,
                || chunk_count,
            );

        self.push_constants
            .set_push_constant_for_compute_pass_if_present(
                compute_pass,
                PushConstantVariant::InstanceIdx,
                || instance_idx,
            );
    }

    fn record_for_camera(
        &self,
        scene_camera: Option<&SceneCamera<fre>>,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let scene_camera =
            scene_camera.ok_or_else(|| anyhow!("Missing scene camera for voxel chunk culling"))?;

        let frustum = scene_camera.camera().view_frustum();

        let instance_range_id = InstanceFeatureBufferRangeManager::INITIAL_RANGE_ID;

        self.record::<InstanceModelViewTransformWithPrevious>(
            instance_feature_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
            instance_range_id,
            &|frustum_to_voxel_object_transform| {
                FrustumPlanes::for_transformed_frustum(frustum, frustum_to_voxel_object_transform)
            },
            Cow::Borrowed("Voxel chunk camera culling pass"),
        )
    }

    fn record_for_shadow_mapping_with_frustum(
        &self,
        frustum: &Frustum<fre>,
        instance_range_id: u32,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.record::<InstanceModelLightTransform>(
            instance_feature_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
            instance_range_id,
            &|frustum_to_voxel_object_transform| {
                FrustumPlanes::for_transformed_frustum(frustum, frustum_to_voxel_object_transform)
            },
            Cow::Borrowed("Voxel chunk culling pass for shadow map"),
        )
    }

    fn record_for_shadow_mapping_with_orthographic_frustum(
        &self,
        orthographic_frustum: &OrientedBox<fre>,
        instance_range_id: u32,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.record::<InstanceModelLightTransform>(
            instance_feature_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
            instance_range_id,
            &|frustum_to_voxel_object_transform| {
                FrustumPlanes::for_transformed_orthographic_frustum(
                    orthographic_frustum,
                    frustum_to_voxel_object_transform,
                )
            },
            Cow::Borrowed("Voxel chunk culling pass for shadow map"),
        )
    }

    fn record<F>(
        &self,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
        instance_range_id: InstanceFeatureBufferRangeID,
        obtain_frustum_planes_in_voxel_object_space: &impl Fn(&Similarity3<fre>) -> FrustumPlanes,
        tag: Cow<'static, str>,
    ) -> Result<()>
    where
        F: InstanceFeature + AsInstanceModelViewTransform,
    {
        let voxel_object_buffer_managers = render_resources.voxel_object_buffer_managers();

        // Return early if there are no voxel objects
        if voxel_object_buffer_managers.is_empty() {
            return Ok(());
        }

        let voxel_object_instance_buffer = instance_feature_manager
            .get_model_instance_buffer(&VOXEL_MODEL_ID)
            .ok_or_else(|| anyhow!("Missing model instance buffer for voxel objects"))?;

        let voxel_object_id_buffer = voxel_object_instance_buffer
            .get_feature_buffer(VoxelObjectID::FEATURE_TYPE_ID)
            .ok_or_else(|| {
                anyhow!("Missing voxel object ID instance feature buffer for voxel objects")
            })?;

        let (instance_range, voxel_object_ids) =
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

        compute_pass.set_pipeline(&self.pipeline);

        let (_, voxel_object_to_frustum_transforms) =
            instance_transform_buffer.range_with_valid_features::<F>(instance_range_id);

        for ((instance_idx, voxel_object_id), voxel_object_to_frustum_transform) in instance_range
            .zip(voxel_object_ids)
            .zip(voxel_object_to_frustum_transforms)
        {
            let voxel_object_buffer_manager = voxel_object_buffer_managers
                .get(voxel_object_id)
                .ok_or_else(|| {
                    anyhow!("Missing GPU buffer for voxel object {}", voxel_object_id)
                })?;

            let chunk_count = u32::try_from(voxel_object_buffer_manager.n_chunks()).unwrap();

            // We want to transform the frustum planes to the normalized voxel object space
            // where the chunk extent is unity
            let frustum_to_voxel_object_transform =
                Self::compute_transform_from_frustum_space_to_normalized_voxel_object_space(
                    *voxel_object_to_frustum_transform.as_instance_model_view_transform(),
                    voxel_object_buffer_manager.chunk_extent(),
                );

            let frustum_planes =
                obtain_frustum_planes_in_voxel_object_space(&frustum_to_voxel_object_transform);

            self.set_push_constants(&mut compute_pass, frustum_planes, chunk_count, instance_idx);

            compute_pass.set_bind_group(
                0,
                voxel_object_buffer_manager.submesh_and_argument_buffer_bind_group(),
                &[],
            );

            let [x, y, z] =
                VoxelChunkCullingShaderTemplate::determine_workgroup_counts(chunk_count);
            compute_pass.dispatch_workgroups(x, y, z);
        }

        log::debug!(
            "Recorded chunk culling pass for {} voxel objects",
            voxel_object_buffer_managers.len()
        );

        Ok(())
    }

    fn compute_transform_from_frustum_space_to_normalized_voxel_object_space(
        voxel_object_to_frustum_transform: InstanceModelViewTransform,
        voxel_extent: f64,
    ) -> Similarity3<fre> {
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
        color_target_states: &[Option<wgpu::ColorTargetState>],
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        config: &RenderingConfig,
    ) -> Self {
        let push_constants = VoxelGeometryShaderTemplate::push_constants();

        let camera_bind_group_layout =
            CameraGPUBufferManager::get_or_create_bind_group_layout(graphics_device);

        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &[camera_bind_group_layout],
            &push_constants.create_ranges(),
            "Voxel geometry pass render pipeline layout",
        );

        let (_, shader) = shader_manager.get_or_create_rendering_shader_from_template(
            graphics_device,
            &VoxelGeometryShaderTemplate,
        );

        let vertex_buffer_layouts = &[
            InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT,
            VoxelMeshVertexPosition::BUFFER_LAYOUT,
            VoxelMeshVertexNormalVector::BUFFER_LAYOUT,
        ];

        let pipeline = render_command::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
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
        );

        Self {
            push_constants,
            pipeline,
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
                PushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::FrameCounter,
                || frame_counter,
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );
    }

    pub fn record(
        &self,
        rendering_surface: &RenderingSurface,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()> {
        let voxel_object_buffer_managers = render_resources.voxel_object_buffer_managers();

        // Return early if there are no voxel objects
        if voxel_object_buffer_managers.is_empty() {
            return Ok(());
        }

        let voxel_object_instance_buffer = instance_feature_manager
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

        let instance_feature_gpu_buffer_managers = render_resources
            .get_instance_feature_buffer_managers(&VOXEL_MODEL_ID)
            .ok_or_else(|| anyhow!("Missing instance GPU buffers for voxel objects"))?;

        let transform_gpu_buffer_manager = instance_feature_gpu_buffer_managers
            .get(ModelInstanceNode::model_view_transform_feature_idx())
            .ok_or_else(|| anyhow!("Missing model-view transform GPU buffer for voxel objects"))?;

        // We don't assign the camera projection uniform bind group here, as it will
        // already have been assigned by the caller

        render_pass.set_pipeline(&self.pipeline);

        self.set_push_constants(render_pass, rendering_surface, postprocessor, frame_counter);

        // All draw calls share the same transform buffer
        render_pass.set_vertex_buffer(
            0,
            transform_gpu_buffer_manager
                .vertex_gpu_buffer()
                .valid_buffer_slice(),
        );

        for voxel_object_id in visible_voxel_object_ids {
            let voxel_object_buffer_manager = voxel_object_buffer_managers
                .get(voxel_object_id)
                .ok_or_else(|| {
                    anyhow!("Missing GPU buffer for voxel object {}", voxel_object_id)
                })?;

            let chunk_count = u32::try_from(voxel_object_buffer_manager.n_chunks()).unwrap();

            render_pass.set_vertex_buffer(
                1,
                voxel_object_buffer_manager
                    .vertex_position_gpu_buffer()
                    .valid_buffer_slice(),
            );

            render_pass.set_vertex_buffer(
                2,
                voxel_object_buffer_manager
                    .vertex_normal_vector_gpu_buffer()
                    .valid_buffer_slice(),
            );

            render_pass.set_index_buffer(
                voxel_object_buffer_manager
                    .index_gpu_buffer()
                    .valid_buffer_slice(),
                voxel_object_buffer_manager.index_format(),
            );

            let indirect_buffer = voxel_object_buffer_manager
                .indirect_argument_gpu_buffer()
                .buffer();

            render_pass.multi_draw_indexed_indirect(indirect_buffer, 0, chunk_count);
        }

        log::debug!(
            "Recorded geometry pass for {} voxel objects",
            voxel_object_buffer_managers.len()
        );

        Ok(())
    }
}
