//! Render commands.

pub mod ambient_light_pass;
pub mod clearing_pass;
pub mod depth_prepass;
pub mod directional_light_pass;
pub mod geometry_pass;
pub mod gizmo_pass;
pub mod postprocessing_pass;
pub mod render_attachment_texture_copy_command;
pub mod shadow_map_update_passes;
pub mod skybox_pass;
pub mod storage_buffer_result_copy_command;
pub mod tasks;

use crate::{
    gpu::{
        GraphicsDevice,
        query::TimestampQueryRegistry,
        rendering::{
            BasicRenderingConfig, ShadowMappingConfig, postprocessing::Postprocessor,
            resource::SynchronizedRenderResources, surface::RenderingSurface,
        },
        resource_group::GPUResourceGroupManager,
        shader::{Shader, ShaderManager},
        storage::StorageGPUBufferManager,
        texture::attachment::{
            RenderAttachmentQuantity, RenderAttachmentQuantitySet, RenderAttachmentTextureManager,
        },
    },
    material::MaterialLibrary,
    scene::Scene,
    voxel::render_commands::VoxelRenderCommands,
};
use ambient_light_pass::AmbientLightPass;
use anyhow::Result;
use clearing_pass::AttachmentClearingPass;
use depth_prepass::DepthPrepass;
use directional_light_pass::DirectionalLightPass;
use geometry_pass::GeometryPass;
use gizmo_pass::GizmoPass;
use shadow_map_update_passes::{
    OmnidirectionalLightShadowMapUpdatePasses, UnidirectionalLightShadowMapUpdatePasses,
};
use skybox_pass::SkyboxPass;
use std::borrow::Cow;

/// Manager of commands for rendering the scene. Postprocessing commands are
/// managed by the [`Postprocessor`], but evoked by this manager.
#[derive(Debug)]
pub struct RenderCommandManager {
    attachment_clearing_pass: AttachmentClearingPass,
    non_physical_model_depth_prepass: DepthPrepass,
    geometry_pass: GeometryPass,
    omnidirectional_light_shadow_map_update_passes: OmnidirectionalLightShadowMapUpdatePasses,
    unidirectional_light_shadow_map_update_passes: UnidirectionalLightShadowMapUpdatePasses,
    ambient_light_pass: AmbientLightPass,
    directional_light_pass: DirectionalLightPass,
    skybox_pass: SkyboxPass,
    voxel_render_commands: VoxelRenderCommands,
    gizmo_pass: GizmoPass,
}

/// The meaning of a specific value in the stencil buffer.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StencilValue {
    Background = 0,
    NonPhysicalModel = 1,
    PhysicalModel = 2,
}

pub const STANDARD_FRONT_FACE: wgpu::FrontFace = wgpu::FrontFace::Ccw;
pub const INVERTED_FRONT_FACE: wgpu::FrontFace = wgpu::FrontFace::Cw;

impl RenderCommandManager {
    /// Creates a new render command manager, initializing all
    /// non-postprocessing render commands.
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        config: &BasicRenderingConfig,
    ) -> Self {
        let attachment_clearing_pass = AttachmentClearingPass::new(
            (RenderAttachmentQuantitySet::DEPTH_STENCIL
                | RenderAttachmentQuantitySet::all().with_clear_color_only())
                - RenderAttachmentQuantitySet::g_buffer(),
            false,
        );

        let non_physical_model_depth_prepass = DepthPrepass::new(
            graphics_device,
            shader_manager,
            StencilValue::NonPhysicalModel,
            config,
        );

        let geometry_pass = GeometryPass::new(graphics_device, config);

        let omnidirectional_light_shadow_map_update_passes =
            OmnidirectionalLightShadowMapUpdatePasses::new(graphics_device, shader_manager);

        let unidirectional_light_shadow_map_update_passes =
            UnidirectionalLightShadowMapUpdatePasses::new(graphics_device, shader_manager);

        let ambient_light_pass = AmbientLightPass::new(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
        );

        let directional_light_pass = DirectionalLightPass::new(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
        );

        let skybox_pass = SkyboxPass::new(graphics_device, shader_manager);

        let voxel_render_commands = VoxelRenderCommands::new(graphics_device, shader_manager);

        let gizmo_pass = GizmoPass::new(graphics_device, rendering_surface, shader_manager);

        Self {
            attachment_clearing_pass,
            non_physical_model_depth_prepass,
            geometry_pass,
            omnidirectional_light_shadow_map_update_passes,
            unidirectional_light_shadow_map_update_passes,
            ambient_light_pass,
            directional_light_pass,
            skybox_pass,
            voxel_render_commands,
            gizmo_pass,
        }
    }

    /// Makes sure all the render commands are up to date with the given render
    /// resources.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
    ) -> Result<()> {
        self.non_physical_model_depth_prepass
            .sync_with_render_resources_for_non_physical_models(material_library, render_resources);

        self.geometry_pass.sync_with_render_resources(
            graphics_device,
            shader_manager,
            material_library,
            render_resources,
        )?;

        self.omnidirectional_light_shadow_map_update_passes
            .sync_with_render_resources(
                graphics_device,
                shader_manager,
                material_library,
                render_resources,
            )?;

        self.unidirectional_light_shadow_map_update_passes
            .sync_with_render_resources(
                graphics_device,
                shader_manager,
                material_library,
                render_resources,
            )?;

        self.ambient_light_pass.sync_with_render_resources(
            graphics_device,
            shader_manager,
            render_resources,
        )?;

        self.directional_light_pass.sync_with_render_resources(
            graphics_device,
            shader_manager,
            render_resources,
        )?;

        self.skybox_pass.sync_with_render_resources(
            graphics_device,
            shader_manager,
            render_resources,
        );

        Ok(())
    }

    /// Records all render commands (including postprocessing commands) into the
    /// given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        scene: &Scene,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        postprocessor: &Postprocessor,
        shadow_mapping_config: &ShadowMappingConfig,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let instance_feature_manager = scene.instance_feature_manager().read().unwrap();
        let light_storage = scene.light_storage().read().unwrap();

        self.attachment_clearing_pass.record(
            surface_texture_view,
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )?;

        self.voxel_render_commands.record_before_geometry_pass(
            scene.scene_camera().read().unwrap().as_ref(),
            &instance_feature_manager,
            render_resources,
            timestamp_recorder,
            command_encoder,
        )?;

        self.non_physical_model_depth_prepass.record(
            rendering_surface,
            render_resources,
            render_attachment_texture_manager,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;

        self.geometry_pass.record(
            rendering_surface,
            &scene.material_library().read().unwrap(),
            &instance_feature_manager,
            render_resources,
            render_attachment_texture_manager,
            postprocessor,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;

        self.omnidirectional_light_shadow_map_update_passes.record(
            &light_storage,
            &instance_feature_manager,
            render_resources,
            timestamp_recorder,
            shadow_mapping_config.enabled,
            &self.voxel_render_commands,
            command_encoder,
        )?;

        self.unidirectional_light_shadow_map_update_passes.record(
            &light_storage,
            &instance_feature_manager,
            render_resources,
            timestamp_recorder,
            shadow_mapping_config.enabled,
            &self.voxel_render_commands,
            command_encoder,
        )?;

        self.ambient_light_pass.record(
            rendering_surface,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            postprocessor,
            timestamp_recorder,
            command_encoder,
        )?;

        self.directional_light_pass.record(
            rendering_surface,
            &light_storage,
            render_resources,
            render_attachment_texture_manager,
            postprocessor,
            timestamp_recorder,
            command_encoder,
        )?;

        self.skybox_pass.record(
            render_resources,
            render_attachment_texture_manager,
            postprocessor,
            timestamp_recorder,
            command_encoder,
        )?;

        postprocessor.record_commands(
            rendering_surface,
            surface_texture_view,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;

        self.gizmo_pass.record(
            surface_texture_view,
            render_resources,
            timestamp_recorder,
            command_encoder,
        )?;

        Ok(())
    }
}

pub fn create_render_pipeline_layout(
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

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &Shader,
    vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
    color_target_states: &[Option<wgpu::ColorTargetState>],
    front_face: wgpu::FrontFace,
    cull_mode: Option<wgpu::Face>,
    polygon_mode: wgpu::PolygonMode,
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader.vertex_module(),
            entry_point: Some(shader.vertex_entry_point_name().unwrap()),
            buffers: vertex_buffer_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: shader
            .fragment_entry_point_name()
            .map(|entry_point| wgpu::FragmentState {
                module: shader.fragment_module(),
                entry_point: Some(entry_point),
                targets: color_target_states,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face,
            cull_mode,
            polygon_mode,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: depth_stencil_state,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
        label: Some(label),
    })
}

pub fn create_line_list_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &Shader,
    vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
    color_target_states: &[Option<wgpu::ColorTargetState>],
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader.vertex_module(),
            entry_point: Some(shader.vertex_entry_point_name().unwrap()),
            buffers: vertex_buffer_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: shader
            .fragment_entry_point_name()
            .map(|entry_point| wgpu::FragmentState {
                module: shader.fragment_module(),
                entry_point: Some(entry_point),
                targets: color_target_states,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::default(),
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::default(),
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: depth_stencil_state,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
        label: Some(label),
    })
}

pub fn depth_stencil_state_for_depth_stencil_write() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: RenderAttachmentQuantity::depth_texture_format(),
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        // Write the reference stencil value to the stencil map
        // whenever the depth test passes
        stencil: wgpu::StencilState {
            front: wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Always,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Replace,
            },
            read_mask: 0xFF,
            write_mask: 0xFF,
            ..Default::default()
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

pub fn depth_stencil_state_for_equal_stencil_testing() -> wgpu::DepthStencilState {
    depth_stencil_state_for_stencil_testing(wgpu::CompareFunction::Equal)
}

pub fn depth_stencil_state_for_stencil_testing(
    compare: wgpu::CompareFunction,
) -> wgpu::DepthStencilState {
    // When we are doing stencil testing, we make the depth test always pass and
    // configure the stencil operations to pass only if the given comparison of the
    // stencil value with the reference value passes
    wgpu::DepthStencilState {
        format: RenderAttachmentQuantity::depth_texture_format(),
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState {
            front: wgpu::StencilFaceState {
                compare,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Keep,
            },
            read_mask: 0xFF,
            write_mask: 0x00,
            ..Default::default()
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

pub fn additive_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent::default(),
    }
}

pub fn begin_single_render_pass<'a>(
    command_encoder: &'a mut wgpu::CommandEncoder,
    timestamp_recorder: &mut TimestampQueryRegistry<'_>,
    color_attachments: &[Option<wgpu::RenderPassColorAttachment<'_>>],
    depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment<'_>>,
    label: Cow<'static, str>,
) -> wgpu::RenderPass<'a> {
    let timestamp_writes =
        timestamp_recorder.register_timestamp_writes_for_single_render_pass(label.clone());

    command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments,
        depth_stencil_attachment,
        timestamp_writes,
        occlusion_query_set: None,
        label: Some(&label),
    })
}
