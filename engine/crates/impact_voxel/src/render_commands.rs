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
use impact_camera::gpu_resource::CameraGPUResource;
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
    InstanceFeature,
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
use nalgebra::Similarity3;
use std::{borrow::Cow, ops::Range};

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

    pub fn record_before_geometry_pass<GR>(
        &self,
        gpu_resources: &GR,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        GR: BasicGPUResources + VoxelGPUResources,
    {
        self.chunk_culling_pass.record_for_geometry_pass(
            gpu_resources,
            timestamp_recorder,
            command_encoder,
        )
    }

    pub fn record_to_geometry_pass<GR>(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resources: &GR,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()>
    where
        GR: BasicGPUResources + VoxelGPUResources,
    {
        self.geometry_pipeline.record(
            rendering_surface,
            gpu_resources,
            postprocessor,
            frame_counter,
            render_pass,
        )
    }

    pub fn record_before_omnidirectional_light_shadow_cubemap_face_update<GR>(
        &self,
        positive_z_cubemap_face_frustum: &Frustum<f32>,
        instance_range_id: u32,
        gpu_resources: &GR,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        GR: VoxelGPUResources,
    {
        self.chunk_culling_pass
            .record_for_shadow_mapping_with_frustum(
                positive_z_cubemap_face_frustum,
                instance_range_id,
                gpu_resources,
                timestamp_recorder,
                command_encoder,
            )
    }

    pub fn record_before_unidirectional_light_shadow_map_cascade_update<GR>(
        &self,
        cascade_frustum: &OrientedBox<f32>,
        instance_range_id: u32,
        gpu_resources: &GR,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        GR: VoxelGPUResources,
    {
        self.chunk_culling_pass
            .record_for_shadow_mapping_with_orthographic_frustum(
                cascade_frustum,
                instance_range_id,
                gpu_resources,
                timestamp_recorder,
                command_encoder,
            )
    }

    pub fn record_shadow_map_update<GR>(
        instance_range_id: u32,
        gpu_resources: &GR,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()>
    where
        GR: BasicGPUResources + VoxelGPUResources,
    {
        // Return early if there are no voxel objects
        if !gpu_resources.voxel_objects().has_voxel_objects() {
            return Ok(());
        }

        let visible_voxel_object_ids = gpu_resources
            .voxel_objects()
            .visible_voxel_object_ids_in_range(instance_range_id);

        // Return early if no voxel objects fall within the shadow map
        if visible_voxel_object_ids.is_empty() {
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

        for voxel_object_id in visible_voxel_object_ids {
            let voxel_object_buffers = gpu_resources
                .voxel_objects()
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

    fn record_for_geometry_pass<GR>(
        &self,
        gpu_resources: &GR,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        GR: BasicGPUResources + VoxelGPUResources,
    {
        let Some(camera) = gpu_resources.camera() else {
            return Ok(());
        };

        // Return early if there are no voxel objects
        if !gpu_resources.voxel_objects().has_voxel_objects() {
            return Ok(());
        }

        let frustum = camera.view_frustum();

        let visible_voxel_object_ids = gpu_resources
            .voxel_objects()
            .visible_voxel_object_ids_in_initial_range();

        let (visible_voxel_object_to_frustum_transforms, instance_range_for_transforms) =
            gpu_resources
                .voxel_objects()
                .visible_object_model_view_transforms();

        self.record(
            gpu_resources,
            timestamp_recorder,
            command_encoder,
            visible_voxel_object_ids,
            visible_voxel_object_to_frustum_transforms,
            instance_range_for_transforms,
            &|frustum_to_voxel_object_transform| {
                CullingFrustum::for_transformed_frustum(frustum, frustum_to_voxel_object_transform)
            },
            // The geometry pass uses non-indexed draw calls
            false,
            Cow::Borrowed("Voxel chunk camera culling pass"),
        )
    }

    fn record_for_shadow_mapping_with_frustum<GR>(
        &self,
        frustum: &Frustum<f32>,
        instance_range_id: u32,
        gpu_resources: &GR,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        GR: VoxelGPUResources,
    {
        // Return early if there are no voxel objects
        if !gpu_resources.voxel_objects().has_voxel_objects() {
            return Ok(());
        }

        let visible_voxel_object_ids = gpu_resources
            .voxel_objects()
            .visible_voxel_object_ids_in_range(instance_range_id);

        let (visible_voxel_object_to_frustum_transforms, instance_range_for_transforms) =
            gpu_resources
                .voxel_objects()
                .visible_object_model_light_transforms_in_range(instance_range_id);

        self.record(
            gpu_resources,
            timestamp_recorder,
            command_encoder,
            visible_voxel_object_ids,
            visible_voxel_object_to_frustum_transforms,
            instance_range_for_transforms,
            &|frustum_to_voxel_object_transform| {
                CullingFrustum::for_transformed_frustum(frustum, frustum_to_voxel_object_transform)
            },
            // Shadow map update passes use indexed draw calls
            true,
            Cow::Borrowed("Voxel chunk culling pass for shadow map"),
        )
    }

    fn record_for_shadow_mapping_with_orthographic_frustum<GR>(
        &self,
        orthographic_frustum: &OrientedBox<f32>,
        instance_range_id: u32,
        gpu_resources: &GR,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>
    where
        GR: VoxelGPUResources,
    {
        // Return early if there are no voxel objects
        if !gpu_resources.voxel_objects().has_voxel_objects() {
            return Ok(());
        }

        let visible_voxel_object_ids = gpu_resources
            .voxel_objects()
            .visible_voxel_object_ids_in_range(instance_range_id);

        let (visible_voxel_object_to_frustum_transforms, instance_range_for_transforms) =
            gpu_resources
                .voxel_objects()
                .visible_object_model_light_transforms_in_range(instance_range_id);

        self.record(
            gpu_resources,
            timestamp_recorder,
            command_encoder,
            visible_voxel_object_ids,
            visible_voxel_object_to_frustum_transforms,
            instance_range_for_transforms,
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

    fn record<GR, T>(
        &self,
        gpu_resources: &GR,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
        visible_voxel_object_ids: &[VoxelObjectID],
        visible_voxel_object_to_frustum_transforms: &[T],
        instance_range_for_transforms: Range<u32>,
        obtain_frustum_planes_in_voxel_object_space: &impl Fn(&Similarity3<f32>) -> CullingFrustum,
        for_indexed_draw_calls: bool,
        tag: Cow<'static, str>,
    ) -> Result<()>
    where
        GR: VoxelGPUResources,
        T: AsInstanceModelViewTransform,
    {
        assert_eq!(
            visible_voxel_object_ids.len(),
            visible_voxel_object_to_frustum_transforms.len()
        );
        assert_eq!(
            instance_range_for_transforms.len(),
            visible_voxel_object_ids.len()
        );

        // Return early if no voxel objects are buffered in the specified range
        if visible_voxel_object_ids.is_empty() {
            return Ok(());
        }

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

        for ((instance_idx, voxel_object_id), voxel_object_to_frustum_transform) in
            instance_range_for_transforms
                .zip(visible_voxel_object_ids)
                .zip(visible_voxel_object_to_frustum_transforms)
        {
            let voxel_object_buffers = gpu_resources
                .voxel_objects()
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
            visible_voxel_object_ids.len()
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

    pub fn record<GR>(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resources: &GR,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()>
    where
        GR: BasicGPUResources + VoxelGPUResources,
    {
        // Return early if there are no voxel objects
        if !gpu_resources.voxel_objects().has_voxel_objects() {
            return Ok(());
        }

        let visible_voxel_object_ids = gpu_resources
            .voxel_objects()
            .visible_voxel_object_ids_in_initial_range();

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
            let voxel_object_buffers = gpu_resources
                .voxel_objects()
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
            visible_voxel_object_ids.len()
        );

        Ok(())
    }
}
