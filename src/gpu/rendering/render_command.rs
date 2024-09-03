//! Render commands.

pub mod tasks;

use crate::{
    assets::lookup_table,
    camera::buffer::CameraGPUBufferManager,
    geometry::CubemapFace,
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        query::TimestampQueryRegistry,
        rendering::{
            postprocessing::Postprocessor, resource::SynchronizedRenderResources,
            surface::RenderingSurface, RenderingConfig,
        },
        resource_group::{GPUResourceGroupID, GPUResourceGroupManager},
        shader::{
            template::{
                ambient_light::AmbientLightShaderTemplate,
                model_depth_prepass::ModelDepthPrepassShaderTemplate,
                model_geometry::{ModelGeometryShaderInput, ModelGeometryShaderTemplate},
                omnidirectional_light::OmnidirectionalLightShaderTemplate,
                omnidirectional_light_shadow_map::OmnidirectionalLightShadowMapShaderTemplate,
                skybox::SkyboxShaderTemplate,
                unidirectional_light::UnidirectionalLightShaderTemplate,
                unidirectional_light_shadow_map::UnidirectionalLightShadowMapShaderTemplate,
                PostprocessingShaderTemplate,
            },
            Shader, ShaderManager,
        },
        storage::{StorageBufferID, StorageGPUBufferManager},
        texture::{
            attachment::{
                Blending, RenderAttachmentInputDescriptionSet,
                RenderAttachmentOutputDescriptionSet, RenderAttachmentQuantity,
                RenderAttachmentQuantitySet, RenderAttachmentTextureManager,
            },
            shadow_map::{CascadeIdx, SHADOW_MAP_FORMAT},
        },
        GraphicsDevice,
    },
    light::{
        buffer::{
            LightGPUBufferManager, OmnidirectionalLightShadowMapManager,
            UnidirectionalLightShadowMapManager,
        },
        LightStorage, MAX_SHADOW_MAP_CASCADES,
    },
    material::{MaterialLibrary, MaterialShaderInput},
    mesh::{self, buffer::VertexBufferable, VertexAttributeSet, VertexPosition},
    model::{
        transform::InstanceModelViewTransformWithPrevious, InstanceFeature, InstanceFeatureManager,
        ModelID,
    },
    scene::Scene,
    skybox::Skybox,
    voxel::render_commands::{VoxelGeometryPipeline, VoxelPreRenderCommands},
};
use anyhow::{anyhow, Result};
use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap, HashSet},
};

/// Manager of commands for rendering the scene. Postprocessing commands are
/// managed by the [`Postprocessor`] but evoked by this manager.
#[derive(Debug)]
pub struct RenderCommandManager {
    attachment_clearing_pass: AttachmentClearingPass,
    voxel_pre_render_commands: VoxelPreRenderCommands,
    non_physical_model_depth_prepass: DepthPrepass,
    geometry_pass: GeometryPass,
    omnidirectional_light_shadow_map_update_passes: OmnidirectionalLightShadowMapUpdatePasses,
    unidirectional_light_shadow_map_update_passes: UnidirectionalLightShadowMapUpdatePasses,
    ambient_light_pass: AmbientLightPass,
    directional_light_pass: DirectionalLightPass,
    skybox_pass: SkyboxPass,
}

/// The meaning of a specific value in the stencil buffer.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StencilValue {
    Background = 0,
    NonPhysicalModel = 1,
    PhysicalModel = 2,
}

/// Pass for clearing the render attachments.
#[derive(Debug)]
struct AttachmentClearingPass {
    attachments: RenderAttachmentQuantitySet,
    clear_surface: bool,
}

/// Pass for filling the depth and stencil map.
#[derive(Debug)]
struct DepthPrepass {
    push_constants: PushConstantGroup,
    pipeline: wgpu::RenderPipeline,
    models: HashSet<ModelID>,
    write_stencil_value: StencilValue,
}

/// Pass for filling the G-buffer attachments and the depth and stencil map.
#[derive(Debug)]
struct GeometryPass {
    push_constants: PushConstantGroup,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
    push_constant_ranges: Vec<wgpu::PushConstantRange>,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil_state: wgpu::DepthStencilState,
    model_pipelines: HashMap<ModelGeometryShaderInput, GeometryPassPipeline>,
    voxel_pipeline: VoxelGeometryPipeline,
}

#[derive(Debug)]
struct GeometryPassPipeline {
    pipeline: wgpu::RenderPipeline,
    vertex_attributes: VertexAttributeSet,
    models: HashSet<ModelID>,
}

/// Passes for filling the faces of each omnidirectional light shadow cubemap.
#[derive(Debug)]
struct OmnidirectionalLightShadowMapUpdatePasses {
    push_constants: PushConstantGroup,
    depth_stencil_state: wgpu::DepthStencilState,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
    models: HashSet<ModelID>,
}

/// Passes for filling the cascades of each unidirectional light shadow map.
#[derive(Debug)]
struct UnidirectionalLightShadowMapUpdatePasses {
    push_constants: PushConstantGroup,
    depth_stencil_state: wgpu::DepthStencilState,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
    models: HashSet<ModelID>,
}

/// Pass for computing reflected luminance due to ambient light.
#[derive(Debug)]
struct AmbientLightPass {
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil_state: wgpu::DepthStencilState,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    max_light_count: usize,
}

/// Pass for computing reflected luminance due to directional lights.
#[derive(Debug)]
struct DirectionalLightPass {
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachment_quantity: RenderAttachmentQuantity,
    color_target_state: wgpu::ColorTargetState,
    depth_stencil_state: wgpu::DepthStencilState,
    omnidirectional_light_pipeline: OmnidirectionalLightPipeline,
    unidirectional_light_pipeline: UnidirectionalLightPipeline,
}

#[derive(Debug)]
struct OmnidirectionalLightPipeline {
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

/// Pass for filling in emitted luminance from the skybox.
#[derive(Debug)]
struct SkyboxPass {
    push_constants: PushConstantGroup,
    output_render_attachment_quantity: RenderAttachmentQuantity,
    push_constant_ranges: Vec<wgpu::PushConstantRange>,
    color_target_state: wgpu::ColorTargetState,
    depth_stencil_state: wgpu::DepthStencilState,
    pipeline: Option<wgpu::RenderPipeline>,
    skybox: Option<Skybox>,
}

/// Generic pass for postprocessing effects.
#[derive(Debug)]
pub struct PostprocessingRenderPass {
    push_constants: PushConstantGroup,
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

/// Recorder for a command copying the contents of one render attachment texture
/// into another.
#[derive(Debug)]
pub struct RenderAttachmentTextureCopyCommand {
    source: RenderAttachmentQuantity,
    destination: RenderAttachmentQuantity,
}

/// Recorder for a command copying the contents of a storage buffer into its
/// associated result buffer (which can be mapped to the CPU).
#[derive(Debug)]
pub struct StorageBufferResultCopyCommand {
    buffer_id: StorageBufferID,
}

pub const STANDARD_FRONT_FACE: wgpu::FrontFace = wgpu::FrontFace::Ccw;
const INVERTED_FRONT_FACE: wgpu::FrontFace = wgpu::FrontFace::Cw;

impl RenderCommandManager {
    /// Creates a new render command manager, initializing all
    /// non-postprocessing render commands.
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    ) -> Self {
        let attachment_clearing_pass = AttachmentClearingPass::new(
            (RenderAttachmentQuantitySet::DEPTH_STENCIL
                | RenderAttachmentQuantitySet::all().with_clear_color_only())
                - RenderAttachmentQuantitySet::g_buffer(),
            false,
        );

        let voxel_pre_render_commands =
            VoxelPreRenderCommands::new(graphics_device, shader_manager);

        let non_physical_model_depth_prepass = DepthPrepass::new(
            graphics_device,
            shader_manager,
            StencilValue::NonPhysicalModel,
        );

        let geometry_pass = GeometryPass::new(graphics_device, shader_manager);

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

        Self {
            attachment_clearing_pass,
            voxel_pre_render_commands,
            non_physical_model_depth_prepass,
            geometry_pass,
            omnidirectional_light_shadow_map_update_passes,
            unidirectional_light_shadow_map_update_passes,
            ambient_light_pass,
            directional_light_pass,
            skybox_pass,
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
        config: &RenderingConfig,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.attachment_clearing_pass.record(
            surface_texture_view,
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )?;

        self.voxel_pre_render_commands.record(
            &scene.instance_feature_manager().read().unwrap(),
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
            &scene.instance_feature_manager().read().unwrap(),
            render_resources,
            render_attachment_texture_manager,
            postprocessor,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )?;

        self.omnidirectional_light_shadow_map_update_passes.record(
            render_resources,
            timestamp_recorder,
            config.shadow_mapping_enabled,
            command_encoder,
        )?;

        self.unidirectional_light_shadow_map_update_passes.record(
            render_resources,
            timestamp_recorder,
            config.shadow_mapping_enabled,
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

        Ok(())
    }
}

impl AttachmentClearingPass {
    const CLEAR_DEPTH: f32 = 1.0;

    const MAX_ATTACHMENTS_PER_PASS: usize = 8;

    fn new(attachments: RenderAttachmentQuantitySet, clear_surface: bool) -> Self {
        Self {
            attachments,
            clear_surface,
        }
    }

    fn color_attachments<'a, 'b: 'a>(
        &self,
        surface_texture_view: &'a wgpu::TextureView,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'a>>> {
        let mut color_attachments = Vec::with_capacity(RenderAttachmentQuantity::count());

        color_attachments.extend(
            render_attachment_texture_manager
                .request_render_attachment_textures(self.attachments.with_clear_color_only())
                .map(|texture| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: texture.base_texture_view(),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(texture.quantity().clear_color().unwrap()),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                }),
        );

        if self.clear_surface {
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
        if self
            .attachments
            .contains(RenderAttachmentQuantitySet::DEPTH_STENCIL)
        {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: render_attachment_texture_manager
                    .render_attachment_texture(RenderAttachmentQuantity::DepthStencil)
                    .base_texture_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Self::CLEAR_DEPTH),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(StencilValue::Background as u32),
                    store: wgpu::StoreOp::Store,
                }),
            })
        } else {
            None
        }
    }

    fn record(
        &self,
        surface_texture_view: &wgpu::TextureView,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let color_attachments =
            self.color_attachments(surface_texture_view, render_attachment_texture_manager);

        let mut depth_stencil_attachment =
            self.depth_stencil_attachment(render_attachment_texture_manager);

        let n_attachments =
            color_attachments.len() + usize::from(depth_stencil_attachment.is_some());

        if color_attachments.len() < Self::MAX_ATTACHMENTS_PER_PASS {
            begin_single_render_pass(
                command_encoder,
                timestamp_recorder,
                &color_attachments,
                depth_stencil_attachment,
                Cow::Borrowed("Clearing pass"),
            );
        } else {
            // Chunk up the passes to avoid exceeding the maximum number of color
            // attachments
            for (idx, color_attachments) in color_attachments.chunks(8).enumerate() {
                // Only clear depth once
                let depth_stencil_attachment = depth_stencil_attachment.take();
                command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments,
                    depth_stencil_attachment,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    label: Some(&format!("Clearing pass {}", idx)),
                });
            }
        }

        log::debug!(
            "Recorded clearing pass for {} render attachments",
            n_attachments
        );

        Ok(())
    }
}

impl DepthPrepass {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        write_stencil_value: StencilValue,
    ) -> Self {
        let (_, shader) = shader_manager.get_or_create_rendering_shader_from_template(
            graphics_device,
            &ModelDepthPrepassShaderTemplate,
        );

        let push_constants = ModelDepthPrepassShaderTemplate::push_constants();

        let pipeline_layout = create_render_pipeline_layout(
            graphics_device.device(),
            &[CameraGPUBufferManager::get_or_create_bind_group_layout(
                graphics_device,
            )],
            &push_constants.create_ranges(),
            "Depth prepass render pipeline layout",
        );

        let pipeline = create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[
                InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT,
                VertexPosition::BUFFER_LAYOUT,
            ],
            &[],
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            Some(depth_stencil_state_for_depth_stencil_write()),
            "Depth prepass render pipeline",
        );

        Self {
            push_constants,
            pipeline,
            models: HashSet::new(),
            write_stencil_value,
        }
    }

    fn sync_with_render_resources_for_non_physical_models(
        &mut self,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
    ) {
        let instance_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        self.models
            .retain(|model_id| instance_feature_buffer_managers.contains_key(model_id));

        for model_id in instance_feature_buffer_managers.keys() {
            if self.models.contains(model_id) {
                continue;
            }
            if let Some(material_specification) = material_library
                .get_material_specification(model_id.material_handle().material_id())
            {
                if let MaterialShaderInput::Fixed(_) = material_specification.shader_input() {
                    self.models.insert(*model_id);
                }
            }
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

    fn set_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        rendering_surface: &RenderingSurface,
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
    }

    fn record(
        &self,
        rendering_surface: &RenderingSurface,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.models.is_empty() {
            return Ok(());
        }

        let depth_stencil_attachment =
            Self::depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[],
            Some(depth_stencil_attachment),
            Cow::Borrowed("Depth prepass"),
        );

        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_stencil_reference(self.write_stencil_value as u32);

        self.set_push_constants(&mut render_pass, rendering_surface, frame_counter);

        let camera_buffer_manager = render_resources
            .get_camera_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for camera"))?;

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        for model_id in &self.models {
            let transform_buffer_manager = render_resources
                .get_instance_feature_buffer_managers(model_id)
                .and_then(|buffers| buffers.first())
                .ok_or_else(|| anyhow!("Missing transform GPU buffer for model {}", model_id))?;

            let transform_range = transform_buffer_manager.initial_feature_range();

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
                .get_mesh_buffer_manager(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

            let position_buffer = mesh_buffer_manager
                .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                .next()
                .unwrap();

            render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

            render_pass.set_index_buffer(
                mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
                mesh_buffer_manager.index_format(),
            );

            render_pass.draw_indexed(
                0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                0,
                transform_range,
            );
        }

        log::debug!(
            "Recorded depth prepass for {} models ({} draw calls)",
            self.models.len(),
            self.models.len()
        );

        Ok(())
    }
}

impl GeometryPass {
    fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let push_constants = ModelGeometryShaderTemplate::push_constants();
        let output_render_attachments = ModelGeometryShaderTemplate::output_render_attachments();

        let push_constant_ranges = push_constants.create_ranges();

        let color_target_states = Self::color_target_states(&output_render_attachments);

        let depth_stencil_state = depth_stencil_state_for_depth_stencil_write();

        let voxel_pipeline = VoxelGeometryPipeline::new(
            graphics_device,
            shader_manager,
            &color_target_states,
            Some(depth_stencil_state.clone()),
        );

        Self {
            push_constants,
            output_render_attachments,
            push_constant_ranges,
            color_target_states,
            depth_stencil_state,
            model_pipelines: HashMap::new(),
            voxel_pipeline,
        }
    }

    fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
    ) -> Result<()> {
        let instance_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        for pipeline in self.model_pipelines.values_mut() {
            pipeline
                .models
                .retain(|model_id| instance_feature_buffer_managers.contains_key(model_id));
        }

        let added_models: Vec<_> = instance_feature_buffer_managers
            .iter()
            .filter_map(|(model_id, instance_feature_buffer_manager)| {
                for pipeline in self.model_pipelines.values() {
                    if pipeline.models.contains(model_id) {
                        return None;
                    }
                }
                // We only add a pipeline for the model if it actually has
                // buffered transforms, otherwise it will not be rendered
                // anyway
                if instance_feature_buffer_manager
                    .first()
                    .map_or(false, |buffer| buffer.has_features_in_initial_range())
                {
                    Some(*model_id)
                } else {
                    None
                }
            })
            .collect();

        self.add_models(
            graphics_device,
            shader_manager,
            material_library,
            render_resources,
            &added_models,
        )
    }

    fn add_models<'a>(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
        models: impl IntoIterator<Item = &'a ModelID>,
    ) -> Result<()> {
        let camera_bind_group_layout =
            CameraGPUBufferManager::get_or_create_bind_group_layout(graphics_device);

        for model_id in models {
            let material_handle = model_id.material_handle();
            if let Some(material_specification) =
                material_library.get_material_specification(material_handle.material_id())
            {
                if let Some(input) = ModelGeometryShaderInput::for_material(material_specification)
                {
                    match self.model_pipelines.entry(input.clone()) {
                        Entry::Occupied(mut entry) => {
                            entry.get_mut().models.insert(*model_id);
                        }
                        Entry::Vacant(entry) => {
                            let shader_template = ModelGeometryShaderTemplate::new(input);
                            let (_, shader) = shader_manager
                                .get_or_create_rendering_shader_from_template(
                                    graphics_device,
                                    &shader_template,
                                );

                            let vertex_attributes = shader_template.input().vertex_attributes;

                            let material_texture_bind_group_layout = material_handle
                                .material_property_texture_group_id()
                                .and_then(|texture_group_id| {
                                    material_library
                                        .get_material_property_texture_group(texture_group_id)
                                })
                                .map(|material_property_texture_group| {
                                    material_property_texture_group.bind_group_layout()
                                });

                            let mut bind_group_layouts = vec![camera_bind_group_layout];
                            if let Some(material_texture_bind_group_layout) =
                                material_texture_bind_group_layout
                            {
                                bind_group_layouts.push(material_texture_bind_group_layout);
                            }

                            let pipeline_layout = create_render_pipeline_layout(
                                graphics_device.device(),
                                &bind_group_layouts,
                                &self.push_constant_ranges,
                                &format!(
                                    "Geometry pass render pipeline layout for shader: {:?}",
                                    &shader_template
                                ),
                            );

                            let vertex_buffer_layouts = Self::vertex_buffer_layouts(
                                render_resources,
                                model_id,
                                vertex_attributes,
                            )?;

                            let pipeline = create_render_pipeline(
                                graphics_device.device(),
                                &pipeline_layout,
                                shader,
                                &vertex_buffer_layouts,
                                &self.color_target_states,
                                STANDARD_FRONT_FACE,
                                Some(wgpu::Face::Back),
                                wgpu::PolygonMode::Fill,
                                Some(self.depth_stencil_state.clone()),
                                &format!(
                                    "Geometry pass render pipeline for shader: {:?}",
                                    &shader_template
                                ),
                            );

                            let mut models = HashSet::with_capacity(4);
                            models.insert(*model_id);

                            entry.insert(GeometryPassPipeline {
                                pipeline,
                                vertex_attributes,
                                models,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn vertex_buffer_layouts(
        render_resources: &SynchronizedRenderResources,
        model_id: &ModelID,
        vertex_attributes: VertexAttributeSet,
    ) -> Result<Vec<wgpu::VertexBufferLayout<'static>>> {
        let mut layouts = Vec::with_capacity(8);

        let instance_feature_buffer_managers = render_resources
            .get_instance_feature_buffer_managers(model_id)
            .ok_or_else(|| anyhow!("Missing instance GPU buffers for model {}", model_id))?;

        let transform_buffer_manager = instance_feature_buffer_managers
            .first()
            .ok_or_else(|| anyhow!("Missing transform GPU buffer for model {}", model_id))?;

        layouts.push(transform_buffer_manager.vertex_buffer_layout().clone());

        // If the material has a buffer of per-instance features, it will be directly
        // after the transform buffer
        if model_id
            .material_handle()
            .material_property_feature_id()
            .is_some()
        {
            let material_property_buffer_manager = instance_feature_buffer_managers
                .get(1)
                .ok_or_else(|| anyhow!("Missing material GPU buffer for model {}", model_id))?;

            layouts.push(
                material_property_buffer_manager
                    .vertex_buffer_layout()
                    .clone(),
            );
        }

        let mesh_id = model_id.mesh_id();
        let mesh_buffer_manager = render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        layouts.extend(mesh_buffer_manager.request_vertex_buffer_layouts(vertex_attributes)?);

        Ok(layouts)
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
                        Blending::Additive => additive_blend_state(),
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
        let mut color_attachments = Vec::with_capacity(self.color_target_states.len());
        color_attachments.extend(
            render_attachment_texture_manager
                .request_render_attachment_textures(self.output_render_attachments.quantities())
                .map(|texture| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: texture.base_texture_view(),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(texture.quantity().clear_color().unwrap()),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                }),
        );
        color_attachments
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

    fn record(
        &self,
        rendering_surface: &RenderingSurface,
        material_library: &MaterialLibrary,
        instance_feature_manager: &InstanceFeatureManager,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let color_attachments = self.color_attachments(render_attachment_texture_manager);

        let depth_stencil_attachment =
            Self::depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &color_attachments,
            Some(depth_stencil_attachment),
            Cow::Borrowed("Geometry pass"),
        );

        render_pass.set_stencil_reference(StencilValue::PhysicalModel as u32);

        let camera_buffer_manager = render_resources
            .get_camera_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for camera"))?;

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        for pipeline in self.model_pipelines.values() {
            render_pass.set_pipeline(&pipeline.pipeline);

            self.set_push_constants(
                &mut render_pass,
                rendering_surface,
                postprocessor,
                frame_counter,
            );

            for model_id in &pipeline.models {
                let instance_feature_buffer_managers = render_resources
                    .get_instance_feature_buffer_managers(model_id)
                    .ok_or_else(|| {
                        anyhow!("Missing instance GPU buffers for model {}", model_id)
                    })?;

                let transform_buffer_manager =
                    instance_feature_buffer_managers.first().ok_or_else(|| {
                        anyhow!("Missing transform GPU buffer for model {}", model_id)
                    })?;

                let instance_range = transform_buffer_manager.initial_feature_range();

                if instance_range.is_empty() {
                    continue;
                }

                if let Some(material_property_texture_group) = model_id
                    .material_handle()
                    .material_property_texture_group_id()
                    .and_then(|texture_group_id| {
                        material_library.get_material_property_texture_group(texture_group_id)
                    })
                {
                    render_pass.set_bind_group(
                        1,
                        material_property_texture_group.bind_group(),
                        &[],
                    );
                }

                render_pass.set_vertex_buffer(
                    0,
                    transform_buffer_manager
                        .vertex_gpu_buffer()
                        .valid_buffer_slice(),
                );

                let mut vertex_buffer_slot = 1;

                if model_id
                    .material_handle()
                    .material_property_feature_id()
                    .is_some()
                {
                    let material_property_buffer_manager =
                        instance_feature_buffer_managers.get(1).ok_or_else(|| {
                            anyhow!("Missing material GPU buffer for model {}", model_id)
                        })?;

                    render_pass.set_vertex_buffer(
                        vertex_buffer_slot,
                        material_property_buffer_manager
                            .vertex_gpu_buffer()
                            .valid_buffer_slice(),
                    );
                    vertex_buffer_slot += 1;
                }

                let mesh_id = model_id.mesh_id();

                let mesh_buffer_manager = render_resources
                    .get_mesh_buffer_manager(mesh_id)
                    .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

                for vertex_buffer in
                    mesh_buffer_manager.request_vertex_gpu_buffers(pipeline.vertex_attributes)?
                {
                    render_pass
                        .set_vertex_buffer(vertex_buffer_slot, vertex_buffer.valid_buffer_slice());

                    vertex_buffer_slot += 1;
                }

                render_pass.set_index_buffer(
                    mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
                    mesh_buffer_manager.index_format(),
                );

                render_pass.draw_indexed(
                    0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                    0,
                    instance_range,
                );
            }
        }

        let n_models: usize = self
            .model_pipelines
            .values()
            .map(|pipeline| pipeline.models.len())
            .product();

        log::debug!(
            "Recorded geometry pass for {} models ({} pipelines, {} draw calls)",
            n_models,
            self.model_pipelines.len(),
            n_models
        );

        self.voxel_pipeline.record(
            rendering_surface,
            instance_feature_manager,
            render_resources,
            postprocessor,
            frame_counter,
            &mut render_pass,
        )?;

        Ok(())
    }
}

impl OmnidirectionalLightShadowMapUpdatePasses {
    const CLEAR_DEPTH: f32 = 1.0;

    fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;

        let shader_template = OmnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let push_constants = OmnidirectionalLightShadowMapShaderTemplate::push_constants();

        let pipeline_layout = create_render_pipeline_layout(
            graphics_device.device(),
            &[
                LightGPUBufferManager::get_or_create_omnidirectional_light_bind_group_layout(
                    graphics_device,
                ),
            ],
            &push_constants.create_ranges(),
            "Omnidirectional light shadow map update render pipeline layout",
        );

        let depth_stencil_state = depth_stencil_state_for_shadow_map_update();

        let pipeline = create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[
                InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT,
                VertexPosition::BUFFER_LAYOUT,
            ],
            &[],
            // The cubemap projection does not flip the z-axis, so the front
            // faces will have the opposite winding order compared to normal
            INVERTED_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            Some(depth_stencil_state.clone()),
            "Omnidirectional light shadow map update render pipeline",
        );

        Self {
            push_constants,
            depth_stencil_state,
            pipeline_layout,
            pipeline,
            max_light_count,
            models: HashSet::new(),
        }
    }

    fn sync_with_render_resources(
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
        let instance_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        self.models
            .retain(|model_id| instance_feature_buffer_managers.contains_key(model_id));

        for (model_id, instance_feature_buffer_manager) in instance_feature_buffer_managers {
            if self.models.contains(model_id) {
                continue;
            }
            // We only add the model if it actually has buffered model-to-light transforms,
            // otherwise it will not be rendered into the shadow map anyway
            if instance_feature_buffer_manager
                .first()
                .map_or(true, |buffer| !buffer.has_features_after_initial_range())
            {
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

        let max_light_count = light_buffer_manager.max_omnidirectional_light_count();

        if max_light_count != self.max_light_count {
            let shader_template = OmnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
            let (_, shader) = shader_manager
                .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

            self.pipeline = create_render_pipeline(
                graphics_device.device(),
                &self.pipeline_layout,
                shader,
                &[
                    InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT,
                    VertexPosition::BUFFER_LAYOUT,
                ],
                &[],
                INVERTED_FRONT_FACE,
                Some(wgpu::Face::Back),
                wgpu::PolygonMode::Fill,
                Some(self.depth_stencil_state.clone()),
                "Omnidirectional light shadow map update render pipeline",
            );
            self.max_light_count = max_light_count;
        }

        Ok(())
    }

    fn depth_stencil_attachment(
        shadow_cubemap_face_texture_view: &wgpu::TextureView,
    ) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: shadow_cubemap_face_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(Self::CLEAR_DEPTH),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    fn set_light_idx_push_constant(&self, render_pass: &mut wgpu::RenderPass<'_>, light_idx: u32) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::LightIdx,
                || light_idx,
            );
    }

    fn record(
        &self,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        shadow_mapping_enabled: bool,
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

        let [first_timestamp_writes, last_timestamp_writes] = timestamp_recorder
            .register_timestamp_writes_for_first_and_last_of_render_passes(Cow::Borrowed(
                "Omnidirectional light shadow map update passes",
            ));

        let last_pass_idx = 6 * shadow_map_textures.len() - 1;
        let mut pass_idx = 0;

        for (light_idx, (light_id, shadow_map_texture)) in light_buffer_manager
            .omnidirectional_light_ids()
            .iter()
            .zip(shadow_map_textures)
            .enumerate()
        {
            for cubemap_face in CubemapFace::all() {
                let shadow_cubemap_face_texture_view = shadow_map_texture.face_view(cubemap_face);

                let depth_stencil_attachment =
                    Self::depth_stencil_attachment(shadow_cubemap_face_texture_view);

                let timestamp_writes = if pass_idx == 0 {
                    first_timestamp_writes.clone()
                } else if pass_idx == last_pass_idx {
                    last_timestamp_writes.clone()
                } else {
                    None
                };
                pass_idx += 1;

                let mut render_pass =
                    command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[],
                        depth_stencil_attachment: Some(depth_stencil_attachment),
                        timestamp_writes,
                        occlusion_query_set: None,
                        label: Some(&format!(
                            "Update pass for shadow cubemap face {:?} for light {}",
                            cubemap_face, light_idx,
                        )),
                    });

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
                    light_buffer_manager.omnidirectional_light_bind_group(),
                    &[],
                );

                for model_id in &self.models {
                    let transform_buffer_manager = render_resources
                        .get_instance_feature_buffer_managers(model_id)
                        .and_then(|buffers| buffers.first())
                        .ok_or_else(|| {
                            anyhow!("Missing transform GPU buffer for model {}", model_id)
                        })?;

                    // When updating the shadow map, we don't use model view transforms but rather
                    // the model to light space tranforms that have been written to the range
                    // dedicated for the active light in the transform buffer.

                    // Offset the light's buffer range ID with the face index to get the index for
                    // the range of transforms for the specific cubemap face
                    let buffer_range_id =
                        light_id.as_instance_feature_buffer_range_id() + cubemap_face.as_idx_u32();

                    let transform_range = transform_buffer_manager.feature_range(buffer_range_id);

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
                        .get_mesh_buffer_manager(mesh_id)
                        .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

                    let position_buffer = mesh_buffer_manager
                        .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                        .next()
                        .unwrap();

                    render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

                    render_pass.set_index_buffer(
                        mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
                        mesh_buffer_manager.index_format(),
                    );

                    render_pass.draw_indexed(
                        0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                        0,
                        transform_range,
                    );
                }
            }
        }

        let n_passes = 6 * shadow_map_textures.len();
        let n_draw_calls = self.models.len() * n_passes;

        log::debug!(
            "Recorded shadow map update passes for {} omnidirectional lights and {} models ({} passes, {} draw calls)",
            shadow_map_textures.len(),
            self.models.len(),
            n_passes,
            n_draw_calls
        );

        Ok(())
    }
}

impl UnidirectionalLightShadowMapUpdatePasses {
    const CLEAR_DEPTH: f32 = 1.0;

    fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;

        let shader_template = UnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let push_constants = UnidirectionalLightShadowMapShaderTemplate::push_constants();

        let pipeline_layout = create_render_pipeline_layout(
            graphics_device.device(),
            &[
                LightGPUBufferManager::get_or_create_unidirectional_light_bind_group_layout(
                    graphics_device,
                ),
            ],
            &push_constants.create_ranges(),
            "Unidirectional light shadow map update render pipeline layout",
        );

        let depth_stencil_state = depth_stencil_state_for_shadow_map_update();

        let pipeline = create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[
                InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT,
                VertexPosition::BUFFER_LAYOUT,
            ],
            &[],
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            wgpu::PolygonMode::Fill,
            Some(depth_stencil_state.clone()),
            "Unidirectional light shadow map update render pipeline",
        );

        Self {
            push_constants,
            depth_stencil_state,
            pipeline_layout,
            pipeline,
            max_light_count,
            models: HashSet::new(),
        }
    }

    fn sync_with_render_resources(
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
        let instance_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        self.models
            .retain(|model_id| instance_feature_buffer_managers.contains_key(model_id));

        for (model_id, instance_feature_buffer_manager) in instance_feature_buffer_managers {
            if self.models.contains(model_id) {
                continue;
            }
            // We only add the model if it actually has buffered model-to-light transforms,
            // otherwise it will not be rendered into the shadow map anyway
            if instance_feature_buffer_manager
                .first()
                .map_or(true, |buffer| !buffer.has_features_after_initial_range())
            {
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

        let max_light_count = light_buffer_manager.max_unidirectional_light_count();

        if max_light_count != self.max_light_count {
            let shader_template = UnidirectionalLightShadowMapShaderTemplate::new(max_light_count);
            let (_, shader) = shader_manager
                .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

            self.pipeline = create_render_pipeline(
                graphics_device.device(),
                &self.pipeline_layout,
                shader,
                &[
                    InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT,
                    VertexPosition::BUFFER_LAYOUT,
                ],
                &[],
                STANDARD_FRONT_FACE,
                Some(wgpu::Face::Back),
                wgpu::PolygonMode::Fill,
                Some(self.depth_stencil_state.clone()),
                "Unidirectional light shadow map update render pipeline",
            );
            self.max_light_count = max_light_count;
        }

        Ok(())
    }

    fn depth_stencil_attachment(
        shadow_map_cascade_texture_view: &wgpu::TextureView,
    ) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: shadow_map_cascade_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(Self::CLEAR_DEPTH),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
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
                PushConstantVariant::LightIdx,
                || light_idx,
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::CascadeIdx,
                || cascade_idx,
            );
    }

    fn record(
        &self,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        shadow_mapping_enabled: bool,
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

        let [first_timestamp_writes, last_timestamp_writes] = timestamp_recorder
            .register_timestamp_writes_for_first_and_last_of_render_passes(Cow::Borrowed(
                "Unidirectional light shadow map update passes",
            ));

        let last_pass_idx = MAX_SHADOW_MAP_CASCADES as usize * shadow_map_textures.len() - 1;
        let mut pass_idx = 0;

        for (light_idx, (light_id, shadow_map_texture)) in light_buffer_manager
            .unidirectional_light_ids()
            .iter()
            .zip(shadow_map_textures)
            .enumerate()
        {
            for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                let shadow_map_cascade_texture_view = shadow_map_texture.cascade_view(cascade_idx);

                let depth_stencil_attachment =
                    Self::depth_stencil_attachment(shadow_map_cascade_texture_view);

                let timestamp_writes = if pass_idx == 0 {
                    first_timestamp_writes.clone()
                } else if pass_idx == last_pass_idx {
                    last_timestamp_writes.clone()
                } else {
                    None
                };
                pass_idx += 1;

                let mut render_pass =
                    command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[],
                        depth_stencil_attachment: Some(depth_stencil_attachment),
                        timestamp_writes,
                        occlusion_query_set: None,
                        label: Some(&format!(
                            "Update pass for shadow map cascade {} for light {}",
                            cascade_idx, light_idx,
                        )),
                    });

                if !shadow_mapping_enabled {
                    // If shadow mapping is disabled, we don't do anything in the render pass,
                    // which means the shadow map textures will just be cleared
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
                    light_buffer_manager.unidirectional_light_bind_group(),
                    &[],
                );

                for model_id in &self.models {
                    let transform_buffer_manager = render_resources
                        .get_instance_feature_buffer_managers(model_id)
                        .and_then(|buffers| buffers.first())
                        .ok_or_else(|| {
                            anyhow!("Missing transform GPU buffer for model {}", model_id)
                        })?;

                    // When updating the shadow map, we don't use model view transforms but rather
                    // the model to light space tranforms that have been written to the range
                    // dedicated for the active light in the transform buffer.

                    // Offset the light's buffer range ID with the cascade index to get the index
                    // for the range of transforms for the specific cascade
                    let buffer_range_id =
                        light_id.as_instance_feature_buffer_range_id() + cascade_idx;

                    let transform_range = transform_buffer_manager.feature_range(buffer_range_id);

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
                        .get_mesh_buffer_manager(mesh_id)
                        .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

                    let position_buffer = mesh_buffer_manager
                        .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                        .next()
                        .unwrap();

                    render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

                    render_pass.set_index_buffer(
                        mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
                        mesh_buffer_manager.index_format(),
                    );

                    render_pass.draw_indexed(
                        0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                        0,
                        transform_range,
                    );
                }
            }
        }

        let n_passes = MAX_SHADOW_MAP_CASCADES as usize * shadow_map_textures.len();
        let n_draw_calls = self.models.len() * n_passes;

        log::debug!(
            "Recorded shadow map update passes for {} unidirectional lights and {} models ({} passes, {} draw calls)",
            shadow_map_textures.len(),
            self.models.len(),
            n_passes,
            n_draw_calls
        );

        Ok(())
    }
}

impl AmbientLightPass {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    ) -> Self {
        let push_constants = AmbientLightShaderTemplate::push_constants();
        let input_render_attachments = AmbientLightShaderTemplate::input_render_attachments();
        let output_render_attachments = AmbientLightShaderTemplate::output_render_attachments();

        let max_light_count = LightStorage::INITIAL_LIGHT_CAPACITY;

        let shader_template = AmbientLightShaderTemplate::new(max_light_count);
        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

        let mut bind_group_layouts = vec![CameraGPUBufferManager::get_or_create_bind_group_layout(
            graphics_device,
        )];

        bind_group_layouts.extend(
            render_attachment_texture_manager
                .create_and_get_render_attachment_texture_bind_group_layouts(
                    graphics_device,
                    &input_render_attachments,
                ),
        );

        bind_group_layouts.push(
            LightGPUBufferManager::get_or_create_ambient_light_bind_group_layout(graphics_device),
        );

        bind_group_layouts
            .push(lookup_table::specular_ggx_reflectance::get_or_create_texture_and_sampler_bind_group_layout(graphics_device));

        let pipeline_layout = create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layouts,
            &push_constants.create_ranges(),
            "Ambient light pass render pipeline layout",
        );

        let color_target_states = Self::color_target_states(&output_render_attachments);

        let depth_stencil_state = depth_stencil_state_for_equal_stencil_testing();

        let pipeline = create_render_pipeline(
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

    fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_resources: &SynchronizedRenderResources,
    ) -> Result<()> {
        let light_buffer_manager = render_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

        let max_light_count = light_buffer_manager.max_ambient_light_count();

        if max_light_count != self.max_light_count {
            let shader_template = AmbientLightShaderTemplate::new(max_light_count);
            let (_, shader) = shader_manager
                .get_or_create_rendering_shader_from_template(graphics_device, &shader_template);

            self.pipeline = create_render_pipeline(
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

        Ok(())
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
                        Blending::Additive => additive_blend_state(),
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
                PushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );
    }

    fn record(
        &self,
        rendering_surface: &RenderingSurface,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let light_buffer_manager = render_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

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

        let camera_buffer_manager = render_resources
            .get_camera_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for camera"))?;

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        let mut bind_group_index = 1;
        for bind_group in render_attachment_texture_manager
            .get_render_attachment_texture_bind_groups(&self.input_render_attachments)
        {
            render_pass.set_bind_group(bind_group_index, bind_group, &[]);
            bind_group_index += 1;
        }

        render_pass.set_bind_group(
            bind_group_index,
            light_buffer_manager.ambient_light_bind_group(),
            &[],
        );
        bind_group_index += 1;

        let specular_ggx_reflectance_lookup_table_resource_group = gpu_resource_group_manager
            .get_resource_group(lookup_table::specular_ggx_reflectance::resource_group_id())
            .ok_or_else(|| {
                anyhow!("Missing GPU resource group for specular GGX reflectance lookup table")
            })?;

        render_pass.set_bind_group(
            bind_group_index,
            specular_ggx_reflectance_lookup_table_resource_group.bind_group(),
            &[],
        );

        let mesh_id = AmbientLightShaderTemplate::light_volume_mesh_id();

        let mesh_buffer_manager = render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        let position_buffer = mesh_buffer_manager
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
            mesh_buffer_manager.index_format(),
        );

        render_pass.draw_indexed(
            0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
            0,
            0..1,
        );

        log::debug!("Recorded ambient light pass (1 draw call)");

        Ok(())
    }
}

impl DirectionalLightPass {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    ) -> Self {
        let push_constants = OmnidirectionalLightShaderTemplate::push_constants();
        let input_render_attachments =
            OmnidirectionalLightShaderTemplate::input_render_attachments();
        let output_render_attachment_quantity =
            OmnidirectionalLightShaderTemplate::output_render_attachment_quantity();

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

        let mut bind_group_layouts = vec![CameraGPUBufferManager::get_or_create_bind_group_layout(
            graphics_device,
        )];

        bind_group_layouts.extend(
            render_attachment_texture_manager
                .create_and_get_render_attachment_texture_bind_group_layouts(
                    graphics_device,
                    &input_render_attachments,
                ),
        );

        let push_constant_ranges = push_constants.create_ranges();

        let color_target_state = Self::color_target_state(output_render_attachment_quantity);

        let depth_stencil_state = depth_stencil_state_for_equal_stencil_testing();

        let omnidirectional_light_pipeline = OmnidirectionalLightPipeline::new(
            graphics_device,
            shader_manager,
            bind_group_layouts.clone(),
            &push_constant_ranges,
            &[Some(color_target_state.clone())],
            Some(depth_stencil_state.clone()),
        );

        let unidirectional_light_pipeline = UnidirectionalLightPipeline::new(
            graphics_device,
            shader_manager,
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
            unidirectional_light_pipeline,
        }
    }

    fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_resources: &SynchronizedRenderResources,
    ) -> Result<()> {
        let light_buffer_manager = render_resources
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

        Ok(())
    }

    fn color_target_state(
        output_render_attachment_quantity: RenderAttachmentQuantity,
    ) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: output_render_attachment_quantity.texture_format(),
            blend: Some(additive_blend_state()),
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
                PushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );
    }

    fn set_light_idx_push_constant(&self, render_pass: &mut wgpu::RenderPass<'_>, light_idx: u32) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::LightIdx,
                || light_idx,
            );
    }

    fn record(
        &self,
        rendering_surface: &RenderingSurface,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let light_buffer_manager = render_resources
            .get_light_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for lights"))?;

        if light_buffer_manager.omnidirectional_light_ids().is_empty()
            && light_buffer_manager.unidirectional_light_ids().is_empty()
        {
            return Ok(());
        }

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

        let camera_buffer_manager = render_resources
            .get_camera_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for camera"))?;

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

        // **** Omnidirectional lights ****

        render_pass.set_pipeline(&self.omnidirectional_light_pipeline.pipeline);

        self.set_constant_push_constants(&mut render_pass, rendering_surface, postprocessor);

        render_pass.set_bind_group(
            light_bind_group_index,
            light_buffer_manager.omnidirectional_light_bind_group(),
            &[],
        );

        let mesh_id = OmnidirectionalLightShaderTemplate::light_volume_mesh_id();

        let mesh_buffer_manager = render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        let position_buffer = mesh_buffer_manager
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
            mesh_buffer_manager.index_format(),
        );

        let n_indices = u32::try_from(mesh_buffer_manager.n_indices()).unwrap();

        let omnidirectional_light_shadow_map_manager =
            light_buffer_manager.omnidirectional_light_shadow_map_manager();
        let omnidirectional_light_shadow_map_textures =
            omnidirectional_light_shadow_map_manager.textures();

        for (light_idx, shadow_map_texture) in
            omnidirectional_light_shadow_map_textures.iter().enumerate()
        {
            self.set_light_idx_push_constant(&mut render_pass, u32::try_from(light_idx).unwrap());

            render_pass.set_bind_group(
                shadow_map_bind_group_index,
                shadow_map_texture.bind_group(),
                &[],
            );

            render_pass.draw_indexed(0..n_indices, 0, 0..1);
        }

        // **** Unidirectional lights ****

        render_pass.set_pipeline(&self.unidirectional_light_pipeline.pipeline);

        self.set_constant_push_constants(&mut render_pass, rendering_surface, postprocessor);

        render_pass.set_bind_group(
            light_bind_group_index,
            light_buffer_manager.unidirectional_light_bind_group(),
            &[],
        );

        let mesh_id = UnidirectionalLightShaderTemplate::light_volume_mesh_id();

        let mesh_buffer_manager = render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        let position_buffer = mesh_buffer_manager
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
            mesh_buffer_manager.index_format(),
        );

        let n_indices = u32::try_from(mesh_buffer_manager.n_indices()).unwrap();

        let unidirectional_light_shadow_map_manager =
            light_buffer_manager.unidirectional_light_shadow_map_manager();
        let unidirectional_light_shadow_map_textures =
            unidirectional_light_shadow_map_manager.textures();

        for (light_idx, shadow_map_texture) in
            unidirectional_light_shadow_map_textures.iter().enumerate()
        {
            self.set_light_idx_push_constant(&mut render_pass, u32::try_from(light_idx).unwrap());

            render_pass.set_bind_group(
                shadow_map_bind_group_index,
                shadow_map_texture.bind_group(),
                &[],
            );

            render_pass.draw_indexed(0..n_indices, 0, 0..1);
        }

        log::debug!(
            "Recorded lighting pass for {} omnidirectional lights and {} unidirectional lights ({} draw calls)",
            omnidirectional_light_shadow_map_textures.len(),
            unidirectional_light_shadow_map_textures.len(),
            omnidirectional_light_shadow_map_textures.len()
                + unidirectional_light_shadow_map_textures.len()
        );

        Ok(())
    }
}

impl OmnidirectionalLightPipeline {
    fn new<'a>(
        graphics_device: &'a GraphicsDevice,
        shader_manager: &mut ShaderManager,
        mut bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
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
            ),
        );

        bind_group_layouts.push(
            OmnidirectionalLightShadowMapManager::get_or_create_bind_group_layout(graphics_device),
        );

        let pipeline_layout = create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layouts,
            push_constant_ranges,
            "Omnidirectional light pass render pipeline layout",
        );

        let pipeline = create_render_pipeline(
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

        self.pipeline = create_render_pipeline(
            graphics_device.device(),
            &self.layout,
            shader,
            &[VertexPosition::BUFFER_LAYOUT],
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Front),
            wgpu::PolygonMode::Fill,
            depth_stencil_state,
            "Omnidirectional light pass render pipeline",
        );
        self.max_light_count = new_max_light_count;
    }
}

impl UnidirectionalLightPipeline {
    fn new<'a>(
        graphics_device: &'a GraphicsDevice,
        shader_manager: &mut ShaderManager,
        mut bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
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
            ),
        );

        bind_group_layouts.push(
            UnidirectionalLightShadowMapManager::get_or_create_bind_group_layout(graphics_device),
        );

        let pipeline_layout = create_render_pipeline_layout(
            graphics_device.device(),
            &bind_group_layouts,
            push_constant_ranges,
            "Unidirectional light pass render pipeline layout",
        );

        let pipeline = create_render_pipeline(
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

        self.pipeline = create_render_pipeline(
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

impl SkyboxPass {
    fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let push_constants = SkyboxShaderTemplate::push_constants();
        let output_render_attachment_quantity =
            SkyboxShaderTemplate::output_render_attachment_quantity();

        let push_constant_ranges = push_constants.create_ranges();
        let color_target_state = Self::color_target_state(output_render_attachment_quantity);
        let depth_stencil_state = depth_stencil_state_for_equal_stencil_testing();

        // Make sure the shader is compiled
        shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &SkyboxShaderTemplate);

        Self {
            push_constants,
            output_render_attachment_quantity,
            push_constant_ranges,
            color_target_state,
            depth_stencil_state,
            pipeline: None,
            skybox: None,
        }
    }

    fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_resources: &SynchronizedRenderResources,
    ) {
        match (
            self.skybox.as_ref(),
            render_resources.get_skybox_resource_manager(),
        ) {
            (Some(skybox), Some(skybox_resource_manager))
                if skybox == skybox_resource_manager.skybox() => {}
            (_, None) => {
                self.pipeline = None;
                self.skybox = None;
            }
            (_, Some(skybox_resource_manager)) => {
                let (_, shader) = shader_manager.get_or_create_rendering_shader_from_template(
                    graphics_device,
                    &SkyboxShaderTemplate,
                );

                let pipeline_layout = create_render_pipeline_layout(
                    graphics_device.device(),
                    &[
                        CameraGPUBufferManager::get_or_create_bind_group_layout(graphics_device),
                        skybox_resource_manager.bind_group_layout(),
                    ],
                    &self.push_constant_ranges,
                    "Skybox pass render pipeline layout",
                );

                self.pipeline = Some(create_render_pipeline(
                    graphics_device.device(),
                    &pipeline_layout,
                    shader,
                    &[VertexPosition::BUFFER_LAYOUT],
                    &[Some(self.color_target_state.clone())],
                    STANDARD_FRONT_FACE,
                    Some(wgpu::Face::Back),
                    wgpu::PolygonMode::Fill,
                    Some(self.depth_stencil_state.clone()),
                    "Skybox pass render pipeline",
                ));
                self.skybox = Some(skybox_resource_manager.skybox().clone());
            }
        }
    }

    fn color_target_state(
        output_render_attachment_quantity: RenderAttachmentQuantity,
    ) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: output_render_attachment_quantity.texture_format(),
            blend: Some(additive_blend_state()),
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

    fn set_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        postprocessor: &Postprocessor,
        camera_buffer_manager: &CameraGPUBufferManager,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::CameraRotationQuaternion,
                || camera_buffer_manager.camera_rotation_quaternion_push_constant(),
            );
    }

    fn record(
        &self,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let pipeline = if let Some(pipeline) = self.pipeline.as_ref() {
            pipeline
        } else {
            return Ok(());
        };

        let color_attachment = self.color_attachment(render_attachment_texture_manager);

        let depth_stencil_attachment =
            Self::depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[Some(color_attachment)],
            Some(depth_stencil_attachment),
            Cow::Borrowed("Skybox pass"),
        );

        render_pass.set_pipeline(pipeline);

        render_pass.set_stencil_reference(StencilValue::Background as u32);

        let camera_buffer_manager = render_resources
            .get_camera_buffer_manager()
            .ok_or_else(|| anyhow!("Missing GPU buffer for camera"))?;

        self.set_push_constants(&mut render_pass, postprocessor, camera_buffer_manager);

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        let skybox_resource_manager = render_resources
            .get_skybox_resource_manager()
            .ok_or_else(|| anyhow!("Missing GPU resources for skybox"))?;

        render_pass.set_bind_group(1, skybox_resource_manager.bind_group(), &[]);

        let mesh_id = mesh::skybox_mesh_id();

        let mesh_buffer_manager = render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        let position_buffer = mesh_buffer_manager
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
            mesh_buffer_manager.index_format(),
        );

        render_pass.draw_indexed(
            0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
            0,
            0..1,
        );

        log::debug!("Recorded skybox pass");

        Ok(())
    }
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

        let pipeline_layout = create_render_pipeline_layout(
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

        let depth_stencil_state =
            stencil_test.map(|(compare, _)| depth_stencil_state_for_stencil_testing(compare));

        let pipeline = create_render_pipeline(
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
                        Blending::Additive => additive_blend_state(),
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
                PushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::PixelCount,
                || rendering_surface.pixel_count_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::InverseExposure,
                || {
                    postprocessor
                        .capturing_camera()
                        .inverse_exposure_push_constant()
                },
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::FrameCounter,
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
            let camera_buffer_manager = render_resources
                .get_camera_buffer_manager()
                .ok_or_else(|| anyhow!("Missing GPU buffer for camera"))?;

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
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        let position_buffer = mesh_buffer_manager
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_buffer_manager.index_gpu_buffer().valid_buffer_slice(),
            mesh_buffer_manager.index_format(),
        );

        render_pass.draw_indexed(
            0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
            0,
            0..1,
        );

        log::debug!("Recorded postprocessing pass: {}", &self.label);

        Ok(())
    }
}

impl RenderAttachmentTextureCopyCommand {
    /// Creates a new render attachment texture copy command for the given
    /// source and destination render attachment quantities.
    ///
    /// # Panics
    /// - If the source and destination render attachment quantities are the
    ///   same.
    /// - If the source and destination texture formats are not the same.
    pub fn new(source: RenderAttachmentQuantity, destination: RenderAttachmentQuantity) -> Self {
        if source == destination {
            panic!(
                "Tried to create render attachment texture copy command with same source and destination: {:?}",
                source,
            );
        }
        if source.texture_format() != destination.texture_format() {
            panic!(
                "Tried to create render attachment texture copy command with different formats: {:?} and {:?}",
                source,
                destination,
            );
        }
        Self {
            source,
            destination,
        }
    }

    /// Records the copy pass to the given command encoder.
    pub fn record(
        &self,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        command_encoder: &mut wgpu::CommandEncoder,
    ) {
        let source_texture = render_attachment_texture_manager
            .render_attachment_texture(self.source)
            .texture()
            .texture();
        let destination_texture = render_attachment_texture_manager
            .render_attachment_texture(self.destination)
            .texture()
            .texture();

        command_encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: source_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: destination_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            source_texture.size(),
        );

        log::debug!(
            "Recorded texture copy command ({:?} to {:?})",
            self.source,
            self.destination
        );
    }
}

impl StorageBufferResultCopyCommand {
    /// Creates a new result copy command for the storage buffer with the given
    /// ID.
    pub fn new(buffer_id: StorageBufferID) -> Self {
        Self { buffer_id }
    }

    /// Records the copy pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if the storage buffer is not available or does not have
    /// a result buffer.
    pub fn record(
        &self,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let storage_buffer = storage_gpu_buffer_manager
            .get_storage_buffer(self.buffer_id)
            .ok_or_else(|| anyhow!("Missing storage buffer {}", self.buffer_id))?;

        storage_buffer.encode_copy_to_result_buffer(command_encoder)?;

        log::debug!(
            "Recorded result copy command for storage buffer ({})",
            self.buffer_id
        );

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
    create_render_pipeline(
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
            entry_point: shader.vertex_entry_point_name().unwrap(),
            buffers: vertex_buffer_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: shader
            .fragment_entry_point_name()
            .map(|entry_point| wgpu::FragmentState {
                module: shader.fragment_module(),
                entry_point,
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
