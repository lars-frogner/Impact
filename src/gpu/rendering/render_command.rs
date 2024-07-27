//! Rendering pipelines.

pub mod tasks;

use crate::{
    camera::buffer::CameraGPUBufferManager,
    geometry::CubemapFace,
    gpu::{
        compute::{ComputePassRecorder, ComputePassSpecification},
        push_constant::{PushConstant, PushConstantGroup, PushConstantVariant},
        rendering::{
            postprocessing::Postprocessor, resource::SynchronizedRenderResources,
            surface::RenderingSurface, RenderingConfig,
        },
        resource_group::{GPUResourceGroupID, GPUResourceGroupManager},
        shader::{
            CameraShaderInput, InstanceFeatureShaderInput, LightShaderInput, MaterialShaderInput,
            MeshShaderInput, Shader, ShaderID, ShaderManager,
        },
        storage::{StorageBufferID, StorageGPUBufferManager},
        texture::{
            attachment::{
                OutputAttachmentSampling, RenderAttachmentInputDescriptionSet,
                RenderAttachmentOutputDescription, RenderAttachmentOutputDescriptionSet,
                RenderAttachmentQuantity, RenderAttachmentQuantitySet,
                RenderAttachmentTextureManager,
            },
            shadow_map::{CascadeIdx, SHADOW_MAP_FORMAT},
        },
        GraphicsDevice,
    },
    light::{buffer::LightGPUBufferManager, LightID, LightType, MAX_SHADOW_MAP_CASCADES},
    material::{
        MaterialID, MaterialLibrary, MaterialPropertyTextureGroup, MaterialPropertyTextureGroupID,
        MaterialSpecification,
    },
    mesh::{buffer::MeshGPUBufferManager, MeshID, VertexAttributeSet},
    model::{buffer::InstanceFeatureGPUBufferManager, ModelID},
};
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use impact_utils::KeyIndexMapper;
use std::collections::{hash_map::Entry, HashMap};

/// Manager and owner of render and compute passes for rendering.
#[derive(Debug)]
pub struct RenderCommandManager {
    /// Passes for clearing the render attachments.
    clearing_passes: Vec<RenderCommandRecorder>,
    /// Passes for filling the depth map with the depths of the models that do
    /// not depend on light sources.
    non_light_shaded_model_depth_prepasses: Vec<RenderCommandRecorder>,
    /// Passes for shading each model that depends on light sources with their
    /// prepass material. This also does the job of filling the remainder of the
    /// depth map.
    light_shaded_model_shading_prepasses: Vec<RenderCommandRecorder>,
    /// Passes for shading models that do not depend on light sources.
    non_light_shaded_model_shading_passes: Vec<RenderCommandRecorder>,
    /// Passes for shading models that depend on light sources, including passes
    /// for clearing and filling the shadow map.
    light_shaded_model_shading_passes: HashMap<LightID, LightShadedModelShadingPasses>,
    non_light_shaded_model_index_mapper: KeyIndexMapper<ModelID>,
    light_shaded_model_index_mapper: KeyIndexMapper<ModelID>,
    /// Passes for applying postprocessing.
    postprocessing_passes: Vec<RenderCommandRecorder>,
}

/// Holds the information describing a specific render pass.
#[derive(Clone, Debug)]
pub struct RenderPassSpecification {
    /// Whether to clear the rendering surface.
    pub clear_surface: bool,
    /// Which non-depth render attachments to clear in this pass.
    pub color_attachments_to_clear: RenderAttachmentQuantitySet,
    /// ID of the model type to render, or [`None`] if the pass does not render
    /// a model (e.g. a clearing pass).
    pub model_id: Option<ModelID>,
    /// If present, use this mesh rather than a mesh associated with a model.
    pub explicit_mesh_id: Option<MeshID>,
    /// If present, use this material rather than a material associated with a
    /// model.
    pub explicit_material_id: Option<MaterialID>,
    /// If present, bind this GPU resource group for the pass.
    pub resource_group_id: Option<GPUResourceGroupID>,
    /// If present, using this shader for the pass rather than generating one.
    pub explicit_shader_id: Option<ShaderID>,
    /// Whether to use the prepass material associated with the model's material
    /// rather than using the model's material.
    pub use_prepass_material: bool,
    /// Whether and how the depth map will be used.
    pub depth_map_usage: DepthMapUsage,
    /// Light source whose contribution is computed in this pass.
    pub light: Option<LightInfo>,
    /// Whether and how the shadow map will be used.
    pub shadow_map_usage: ShadowMapUsage,
    /// The vertex attributes to use in the pass.
    pub vertex_attribute_requirements: VertexAttributeSet,
    /// Descriptions for the render attachments to bind as inputs to the pass.
    pub input_render_attachments: RenderAttachmentInputDescriptionSet,
    /// Descriptions for the render attachments to use as outputs in the pass.
    pub output_render_attachments: RenderAttachmentOutputDescriptionSet,
    /// The group of push constants to use in the pass.
    pub push_constants: PushConstantGroup,
    pub hints: RenderPassHints,
    pub label: String,
}

/// Holds the information describing a specific render command.
#[derive(Clone, Debug)]
pub enum RenderCommandSpecification {
    RenderPass(RenderPassSpecification),
    ComputePass(ComputePassSpecification),
    RenderAttachmentMipmappingPass { quantity: RenderAttachmentQuantity },
    StorageBufferResultCopyPass { buffer_id: StorageBufferID },
}

/// Recorder for a specific render pass.
#[derive(Debug)]
pub struct RenderPassRecorder {
    specification: RenderPassSpecification,
    attachments_to_resolve: RenderAttachmentQuantitySet,
    pipeline: Option<wgpu::RenderPipeline>,
    state: RenderCommandState,
}

#[derive(Debug)]
pub struct RenderAttachmentMipmappingPassRecorder {
    quantity: RenderAttachmentQuantity,
    state: RenderCommandState,
}

/// Recorder for a pass copying the contents of a storage buffer into its
/// associated result buffer (which can be mapped to the CPU).
#[derive(Debug)]
pub struct StorageBufferResultCopyPassRecorder {
    buffer_id: StorageBufferID,
    state: RenderCommandState,
}

#[derive(Debug)]
pub enum RenderCommandRecorder {
    RenderPass(RenderPassRecorder),
    ComputePass(ComputePassRecorder),
    RenderAttachmentMipmappingPass(RenderAttachmentMipmappingPassRecorder),
    StorageBufferResultCopyPass(StorageBufferResultCopyPassRecorder),
}

/// The active state of a render command.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderCommandState {
    Active,
    Disabled,
}

/// The outcome of a request to record a pipeline pass.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderCommandOutcome {
    Recorded,
    Skipped,
}

bitflags! {
    /// Bitflag encoding a set of hints for configuring a render pass.
    #[derive(Debug, Clone, Copy)]
    pub struct RenderPassHints: u8 {
        /// The appearance of the rendered material is affected by light
        /// sources.
        const AFFECTED_BY_LIGHT = 1 << 0;
        /// No depth prepass should be performed for the model.
        const NO_DEPTH_PREPASS  = 1 << 1;
        /// The render pass does not make use of a camera.
        const NO_CAMERA         = 1 << 2;
        /// The render pass writes directly to the rendering surface.
        const WRITES_TO_SURFACE = 1 << 3;
    }
}

#[derive(Debug, Default)]
struct LightShadedModelShadingPasses {
    /// Passes for clearing the shadow maps to the maximum depth.
    shadow_map_clearing_passes: Vec<RenderCommandRecorder>,
    /// Passes for writing the depths of each model from the light's point of
    /// view to the shadow map.
    shadow_map_update_passes: Vec<Vec<RenderCommandRecorder>>,
    /// Passes for shading each model with contributions from the light.
    shading_passes: Vec<RenderCommandRecorder>,
}

#[derive(Copy, Clone, Debug)]
pub struct LightInfo {
    light_type: LightType,
    light_id: LightID,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DepthMapUsage {
    /// No depth map is used.
    None,
    /// Clear the depth map with the maximum depth (1.0).
    Clear,
    /// Fill the depth map with model depths without doing shading.
    Prepass,
    /// Use the depth map for depth testing when shading. The [`WriteDepths`]
    /// value decides whether depths (and stencil values) will be written to the
    /// depth map during the pass.
    Use(WriteDepths),
    /// Use the value in the stencil map to determine whether a fragment should
    /// be ignored.
    StencilTest,
}

type WriteDepths = bool;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ShadowMapUsage {
    /// No shadow map is used.
    None,
    /// Clear the specified shadow map with the maximum depth (1.0).
    Clear(ShadowMapIdentifier),
    /// Fill the specified shadow map with model depths from the light's point
    /// of view.
    Update(ShadowMapIdentifier),
    /// Make the shadow map texture available for sampling in the shader.
    Use,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ShadowMapIdentifier {
    ForUnidirectionalLight(CascadeIdx),
    ForOmnidirectionalLight(CubemapFace),
}

/// The blending mode to use for a render pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blending {
    Replace,
    Additive,
}

#[derive(Debug)]
struct BindGroupRenderingShaderInput<'a> {
    camera: Option<&'a CameraShaderInput>,
    light: Option<&'a LightShaderInput>,
    material: Option<&'a MaterialShaderInput>,
}

impl RenderCommandManager {
    /// Creates a new manager with no render commands.
    pub fn new() -> Self {
        Self {
            clearing_passes: Vec::with_capacity(2),
            non_light_shaded_model_depth_prepasses: Vec::new(),
            light_shaded_model_shading_prepasses: Vec::new(),
            non_light_shaded_model_shading_passes: Vec::new(),
            light_shaded_model_shading_passes: HashMap::new(),
            non_light_shaded_model_index_mapper: KeyIndexMapper::new(),
            light_shaded_model_index_mapper: KeyIndexMapper::new(),
            postprocessing_passes: Vec::new(),
        }
    }

    /// Returns an iterator over all render command recorders in the appropriate
    /// order.
    pub fn recorders(&self) -> impl Iterator<Item = &RenderCommandRecorder> {
        self.clearing_passes
            .iter()
            .chain(self.non_light_shaded_model_depth_prepasses.iter())
            .chain(self.light_shaded_model_shading_prepasses.iter())
            .chain(self.non_light_shaded_model_shading_passes.iter())
            .chain(
                self.light_shaded_model_shading_passes
                    .values()
                    .flat_map(|passes| {
                        passes
                            .shadow_map_clearing_passes
                            .iter()
                            .chain(passes.shadow_map_update_passes.iter().flatten())
                            .chain(passes.shading_passes.iter())
                    }),
            )
            .chain(self.postprocessing_passes.iter())
    }

    fn recorders_mut(&mut self) -> impl Iterator<Item = &mut RenderCommandRecorder> {
        self.clearing_passes
            .iter_mut()
            .chain(self.non_light_shaded_model_depth_prepasses.iter_mut())
            .chain(self.light_shaded_model_shading_prepasses.iter_mut())
            .chain(self.non_light_shaded_model_shading_passes.iter_mut())
            .chain(
                self.light_shaded_model_shading_passes
                    .values_mut()
                    .flat_map(|passes| {
                        passes
                            .shadow_map_clearing_passes
                            .iter_mut()
                            .chain(passes.shadow_map_update_passes.iter_mut().flatten())
                            .chain(passes.shading_passes.iter_mut())
                    }),
            )
            .chain(self.postprocessing_passes.iter_mut())
    }

    /// Deletes all the render command recorders.
    pub fn clear_recorders(&mut self) {
        self.clearing_passes.clear();
        self.non_light_shaded_model_depth_prepasses.clear();
        self.light_shaded_model_shading_prepasses.clear();
        self.non_light_shaded_model_shading_passes.clear();
        self.light_shaded_model_shading_passes.clear();
        self.non_light_shaded_model_index_mapper.clear();
        self.light_shaded_model_index_mapper.clear();
        self.postprocessing_passes.clear();
    }

    /// Deletes the render command recorders for postprocessing.
    pub fn clear_postprocessing_recorders(&mut self) {
        self.postprocessing_passes.clear();
    }

    /// Ensures that all render commands required for rendering the entities
    /// present in the given render resources are available and configured
    /// correctly.
    ///
    /// Render commands whose entities are no longer present in the resources
    /// will be removed, and missing render commands for new entities will be
    /// created.
    fn sync_with_render_resources(
        &mut self,
        config: &RenderingConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        shader_manager: &mut ShaderManager,
        postprocessor: &Postprocessor,
    ) -> Result<()> {
        // We do not attempt to render anything without a camera
        if render_resources.get_camera_buffer_manager().is_none() {
            self.clear_recorders();
            return Ok(());
        }

        self.sync_clearing_passes(config);

        let light_buffer_manager = render_resources.get_light_buffer_manager();

        let ambient_light_ids =
            light_buffer_manager.map_or_else(|| &[], LightGPUBufferManager::ambient_light_ids);
        let omnidirectional_light_ids = light_buffer_manager
            .map_or_else(|| &[], LightGPUBufferManager::omnidirectional_light_ids);
        let unidirectional_light_ids = light_buffer_manager
            .map_or_else(|| &[], LightGPUBufferManager::unidirectional_light_ids);

        // Remove shading passes for lights that are no longer present
        self.light_shaded_model_shading_passes
            .retain(|light_id, _| {
                ambient_light_ids.contains(light_id)
                    || omnidirectional_light_ids.contains(light_id)
                    || unidirectional_light_ids.contains(light_id)
            });

        let all_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        // Remove passes for non light shaded models that are no longer present
        let removed_non_light_shaded_model_ids: Vec<_> = self
            .non_light_shaded_model_index_mapper
            .key_at_each_idx()
            .filter(|model_id| !all_feature_buffer_managers.contains_key(model_id))
            .collect();

        for model_id in removed_non_light_shaded_model_ids {
            let model_idx = self
                .non_light_shaded_model_index_mapper
                .swap_remove_key(model_id);

            self.non_light_shaded_model_depth_prepasses
                .swap_remove(model_idx);

            self.non_light_shaded_model_shading_passes
                .swap_remove(model_idx);
        }

        // Remove passes for light shaded models that are no longer present
        let removed_light_shaded_model_ids: Vec<_> = self
            .light_shaded_model_index_mapper
            .key_at_each_idx()
            .filter(|model_id| !all_feature_buffer_managers.contains_key(model_id))
            .collect();

        for model_id in removed_light_shaded_model_ids {
            let model_idx = self
                .light_shaded_model_index_mapper
                .swap_remove_key(model_id);

            self.light_shaded_model_shading_prepasses
                .swap_remove(model_idx);

            self.light_shaded_model_shading_passes
                .values_mut()
                .for_each(|passes| {
                    if !passes.shadow_map_update_passes.is_empty() {
                        passes.shadow_map_update_passes.swap_remove(model_idx);
                    }
                    passes.shading_passes.swap_remove(model_idx);
                });
        }

        for (&model_id, feature_buffer_managers) in all_feature_buffer_managers {
            let transform_buffer_manager = feature_buffer_managers.first().unwrap();

            // Avoid rendering the model if there are currently no instances
            let no_visible_instances = transform_buffer_manager.initial_feature_range().is_empty();

            let material_specification = material_library
                .get_material_specification(model_id.material_handle().material_id())
                .expect("Missing specification for material");

            let hints = material_specification.render_pass_hints();

            if hints.contains(RenderPassHints::AFFECTED_BY_LIGHT) {
                match self.light_shaded_model_index_mapper.try_push_key(model_id) {
                    // The model has no existing shading passes
                    Ok(_) => {
                        if let Some(prepass_material_handle) = model_id.prepass_material_handle() {
                            let prepass_material_specification = material_library
                                .get_material_specification(prepass_material_handle.material_id())
                                .expect("Missing specification for prepass material");

                            if ambient_light_ids.is_empty() {
                                self.light_shaded_model_shading_prepasses.push(
                                    RenderCommandRecorder::new_render_pass(
                                        config,
                                        graphics_device,
                                        rendering_surface,
                                        material_library,
                                        render_resources,
                                        render_attachment_texture_manager,
                                        gpu_resource_group_manager,
                                        shader_manager,
                                        RenderPassSpecification::shading_prepass(
                                            None,
                                            model_id,
                                            prepass_material_specification,
                                        ),
                                        RenderCommandState::disabled_if(no_visible_instances),
                                    )?,
                                );
                            } else {
                                assert_eq!(
                                    ambient_light_ids.len(),
                                    1,
                                    "Multiple ambient lights not supported"
                                );

                                for &light_id in ambient_light_ids {
                                    let light = LightInfo {
                                        light_type: LightType::AmbientLight,
                                        light_id,
                                    };

                                    // If there are ambient lights and the new
                                    // model has a prepass material, we create a
                                    // shading prepass with each ambient light.
                                    // TODO: If the prepass material is
                                    // unaffected by ambient light, only a
                                    // single prepass without a light is
                                    // actually needed.
                                    self.light_shaded_model_shading_prepasses.push(
                                        RenderCommandRecorder::new_render_pass(
                                            config,
                                            graphics_device,
                                            rendering_surface,
                                            material_library,
                                            render_resources,
                                            render_attachment_texture_manager,
                                            gpu_resource_group_manager,
                                            shader_manager,
                                            RenderPassSpecification::shading_prepass(
                                                Some(light),
                                                model_id,
                                                prepass_material_specification,
                                            ),
                                            RenderCommandState::disabled_if(no_visible_instances),
                                        )?,
                                    );
                                }
                            }
                        } else {
                            // If the new model has no prepass material, we
                            // create a pure depth prepass
                            self.light_shaded_model_shading_prepasses.push(
                                RenderCommandRecorder::new_render_pass(
                                    config,
                                    graphics_device,
                                    rendering_surface,
                                    material_library,
                                    render_resources,
                                    render_attachment_texture_manager,
                                    gpu_resource_group_manager,
                                    shader_manager,
                                    RenderPassSpecification::depth_prepass(model_id, hints),
                                    RenderCommandState::disabled_if(
                                        no_visible_instances
                                            || hints.contains(RenderPassHints::NO_DEPTH_PREPASS),
                                    ),
                                )?,
                            );
                        }

                        for &light_id in omnidirectional_light_ids {
                            let faces_have_shadow_casting_model_instances: Vec<_> =
                                CubemapFace::all()
                                    .into_iter()
                                    .map(|face| {
                                        config.shadow_mapping_enabled
                                            && !transform_buffer_manager
                                                .feature_range(
                                                    light_id.as_instance_feature_buffer_range_id()
                                                        + face.as_idx_u32(),
                                                )
                                                .is_empty()
                                    })
                                    .collect();

                            let passes =
                                match self.light_shaded_model_shading_passes.entry(light_id) {
                                    Entry::Occupied(entry) => entry.into_mut(),
                                    Entry::Vacant(entry) => {
                                        let mut shadow_map_clearing_passes = Vec::with_capacity(6);

                                        for face in CubemapFace::all() {
                                            shadow_map_clearing_passes
                                                .push(RenderCommandRecorder::new_render_pass(
                                                config,
                                                graphics_device,
                                                rendering_surface,
                                                material_library,
                                                render_resources,
                                                render_attachment_texture_manager,
                                                gpu_resource_group_manager,
                                                shader_manager,
                                                RenderPassSpecification::shadow_map_clearing_pass(
                                                    ShadowMapIdentifier::ForOmnidirectionalLight(
                                                        face,
                                                    ),
                                                ),
                                                RenderCommandState::Active,
                                            )?);
                                        }

                                        entry.insert(LightShadedModelShadingPasses {
                                            shadow_map_clearing_passes,
                                            ..Default::default()
                                        })
                                    }
                                };

                            let light = LightInfo {
                                light_type: LightType::OmnidirectionalLight,
                                light_id,
                            };

                            // Create an omnidirectional light shadow map update
                            // pass for each cubemap face for the new model

                            passes.shadow_map_update_passes.push(Vec::with_capacity(6));

                            let shadow_map_update_passes_for_faces =
                                passes.shadow_map_update_passes.last_mut().unwrap();

                            for face in CubemapFace::all() {
                                shadow_map_update_passes_for_faces.push(
                                    RenderCommandRecorder::new_render_pass(
                                        config,
                                        graphics_device,
                                        rendering_surface,
                                        material_library,
                                        render_resources,
                                        render_attachment_texture_manager,
                                        gpu_resource_group_manager,
                                        shader_manager,
                                        RenderPassSpecification::shadow_map_update_pass(
                                            light,
                                            model_id,
                                            ShadowMapIdentifier::ForOmnidirectionalLight(face),
                                            hints,
                                        ),
                                        RenderCommandState::disabled_if(
                                            !faces_have_shadow_casting_model_instances
                                                [face.as_idx_usize()],
                                        ),
                                    )?,
                                );
                            }

                            // Create an omnidirectional light shading pass for
                            // the new model
                            passes
                                .shading_passes
                                .push(RenderCommandRecorder::new_render_pass(
                                    config,
                                    graphics_device,
                                    rendering_surface,
                                    material_library,
                                    render_resources,
                                    render_attachment_texture_manager,
                                    gpu_resource_group_manager,
                                    shader_manager,
                                    RenderPassSpecification::model_shading_pass_with_shadow_map(
                                        light,
                                        model_id,
                                        material_specification,
                                    ),
                                    RenderCommandState::disabled_if(no_visible_instances),
                                )?);
                        }

                        for &light_id in unidirectional_light_ids {
                            let cascades_have_shadow_casting_model_instances: Vec<_> = (0
                                ..MAX_SHADOW_MAP_CASCADES)
                                .map(|cascade_idx| {
                                    config.shadow_mapping_enabled
                                        && !transform_buffer_manager
                                            .feature_range(
                                                light_id.as_instance_feature_buffer_range_id()
                                                    + cascade_idx,
                                            )
                                            .is_empty()
                                })
                                .collect();

                            let passes =
                                match self.light_shaded_model_shading_passes.entry(light_id) {
                                    Entry::Occupied(entry) => entry.into_mut(),
                                    Entry::Vacant(entry) => {
                                        let mut shadow_map_clearing_passes =
                                            Vec::with_capacity(MAX_SHADOW_MAP_CASCADES as usize);

                                        for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                                            shadow_map_clearing_passes
                                                .push(RenderCommandRecorder::new_render_pass(
                                                config,
                                                graphics_device,
                                                rendering_surface,
                                                material_library,
                                                render_resources,
                                                render_attachment_texture_manager,
                                                gpu_resource_group_manager,
                                                shader_manager,
                                                RenderPassSpecification::shadow_map_clearing_pass(
                                                    ShadowMapIdentifier::ForUnidirectionalLight(
                                                        cascade_idx,
                                                    ),
                                                ),
                                                RenderCommandState::Active,
                                            )?);
                                        }

                                        entry.insert(LightShadedModelShadingPasses {
                                            shadow_map_clearing_passes,
                                            ..Default::default()
                                        })
                                    }
                                };

                            let light = LightInfo {
                                light_type: LightType::UnidirectionalLight,
                                light_id,
                            };

                            // Create a unidirectional light shadow map update
                            // pass for each cascade for the new model

                            passes
                                .shadow_map_update_passes
                                .push(Vec::with_capacity(MAX_SHADOW_MAP_CASCADES as usize));

                            let shadow_map_update_passes_for_cascades =
                                passes.shadow_map_update_passes.last_mut().unwrap();

                            for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                                shadow_map_update_passes_for_cascades.push(
                                    RenderCommandRecorder::new_render_pass(
                                        config,
                                        graphics_device,
                                        rendering_surface,
                                        material_library,
                                        render_resources,
                                        render_attachment_texture_manager,
                                        gpu_resource_group_manager,
                                        shader_manager,
                                        RenderPassSpecification::shadow_map_update_pass(
                                            light,
                                            model_id,
                                            ShadowMapIdentifier::ForUnidirectionalLight(
                                                cascade_idx,
                                            ),
                                            hints,
                                        ),
                                        RenderCommandState::disabled_if(
                                            !cascades_have_shadow_casting_model_instances
                                                [cascade_idx as usize],
                                        ),
                                    )?,
                                );
                            }

                            // Create a unidirectional light shading pass for
                            // the new model
                            passes
                                .shading_passes
                                .push(RenderCommandRecorder::new_render_pass(
                                    config,
                                    graphics_device,
                                    rendering_surface,
                                    material_library,
                                    render_resources,
                                    render_attachment_texture_manager,
                                    gpu_resource_group_manager,
                                    shader_manager,
                                    RenderPassSpecification::model_shading_pass_with_shadow_map(
                                        light,
                                        model_id,
                                        material_specification,
                                    ),
                                    RenderCommandState::disabled_if(no_visible_instances),
                                )?);
                        }
                    }
                    // The model already has shading passes
                    Err(model_idx) => {
                        // Set the disabled state of the passes for the existing model

                        self.light_shaded_model_shading_prepasses[model_idx].set_disabled(
                            no_visible_instances
                                || (model_id.prepass_material_handle().is_none()
                                    && hints.contains(RenderPassHints::NO_DEPTH_PREPASS)),
                        );

                        self.light_shaded_model_shading_passes.iter_mut().for_each(
                            |(&light_id, passes)| {
                                if let Some(recorders) =
                                    passes.shadow_map_update_passes.get_mut(model_idx)
                                {
                                    for recorder in recorders
                                        .iter_mut()
                                        .filter_map(RenderCommandRecorder::as_render_pass_mut)
                                    {
                                        let range_id =
                                            if let ShadowMapUsage::Update(shadow_map_id) =
                                                recorder.specification.shadow_map_usage
                                            {
                                                match shadow_map_id {
                                                ShadowMapIdentifier::ForOmnidirectionalLight(
                                                    face,
                                                ) => {
                                                    light_id.as_instance_feature_buffer_range_id()
                                                        + face.as_idx_u32()
                                                }
                                                ShadowMapIdentifier::ForUnidirectionalLight(
                                                    cascade_idx,
                                                ) => {
                                                    light_id.as_instance_feature_buffer_range_id()
                                                        + cascade_idx
                                                }
                                            }
                                            } else {
                                                unreachable!()
                                            };

                                        let no_shadow_casting_instances = !config
                                            .shadow_mapping_enabled
                                            || transform_buffer_manager
                                                .feature_range(range_id)
                                                .is_empty();

                                        recorder.set_disabled(no_shadow_casting_instances);
                                    }
                                }

                                passes.shading_passes[model_idx].set_disabled(no_visible_instances);
                            },
                        );
                    }
                }
            } else {
                match self
                    .non_light_shaded_model_index_mapper
                    .try_push_key(model_id)
                {
                    // The model has no existing shading passes
                    Ok(_) => {
                        // Create a depth prepass for the new model
                        self.non_light_shaded_model_depth_prepasses.push(
                            RenderCommandRecorder::new_render_pass(
                                config,
                                graphics_device,
                                rendering_surface,
                                material_library,
                                render_resources,
                                render_attachment_texture_manager,
                                gpu_resource_group_manager,
                                shader_manager,
                                RenderPassSpecification::depth_prepass(model_id, hints),
                                RenderCommandState::disabled_if(
                                    no_visible_instances
                                        || hints.contains(RenderPassHints::NO_DEPTH_PREPASS),
                                ),
                            )?,
                        );

                        // Create a shading pass for the new model
                        self.non_light_shaded_model_shading_passes.push(
                            RenderCommandRecorder::new_render_pass(
                                config,
                                graphics_device,
                                rendering_surface,
                                material_library,
                                render_resources,
                                render_attachment_texture_manager,
                                gpu_resource_group_manager,
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_without_shadow_map(
                                    None,
                                    model_id,
                                    material_specification,
                                ),
                                RenderCommandState::disabled_if(no_visible_instances),
                            )?,
                        );
                    }
                    // The model already has shading passes
                    Err(model_idx) => {
                        // Set the disabled state of the passes for the existing model

                        self.non_light_shaded_model_depth_prepasses[model_idx].set_disabled(
                            no_visible_instances
                                || hints.contains(RenderPassHints::NO_DEPTH_PREPASS),
                        );

                        self.non_light_shaded_model_shading_passes[model_idx]
                            .set_disabled(no_visible_instances);
                    }
                }
            }
        }

        if self.postprocessing_passes.is_empty() {
            for (specification, state) in postprocessor
                .render_commands()
                .zip(postprocessor.render_command_states())
            {
                self.postprocessing_passes.push(RenderCommandRecorder::new(
                    config,
                    graphics_device,
                    rendering_surface,
                    material_library,
                    render_resources,
                    render_attachment_texture_manager,
                    gpu_resource_group_manager,
                    shader_manager,
                    specification,
                    state,
                )?);
            }
        } else {
            for (recorder, state) in self
                .postprocessing_passes
                .iter_mut()
                .zip(postprocessor.render_command_states())
            {
                recorder.set_state(state);
            }
        }

        self.update_render_attachment_resolve_flags();

        Ok(())
    }

    fn sync_clearing_passes(&mut self, config: &RenderingConfig) {
        self.clearing_passes.clear();

        if config.multisampling_sample_count > 1 {
            let non_multisampling_quantities =
                RenderAttachmentQuantitySet::non_multisampling_quantities();
            if !non_multisampling_quantities.is_empty() {
                self.clearing_passes
                    .push(RenderCommandRecorder::clearing_render_pass(
                        false,
                        non_multisampling_quantities,
                    ));
            }

            let multisampling_quantities = RenderAttachmentQuantitySet::multisampling_quantities();
            if !multisampling_quantities.is_empty() {
                self.clearing_passes
                    .push(RenderCommandRecorder::clearing_render_pass(
                        false,
                        multisampling_quantities,
                    ));
            }
        } else {
            self.clearing_passes
                .push(RenderCommandRecorder::clearing_render_pass(
                    false,
                    RenderAttachmentQuantitySet::all(),
                ));
        }
    }

    fn update_render_attachment_resolve_flags(&mut self) {
        let mut last_indices_of_multisampled_output_attachments =
            [Option::<usize>::None; RenderAttachmentQuantity::count()];
        let mut first_indices_of_input_attachments =
            [Option::<usize>::None; RenderAttachmentQuantity::count()];

        let mut recorders = Vec::with_capacity(64);

        for (idx, recorder) in self
            .recorders_mut()
            .filter_map(RenderCommandRecorder::as_render_pass_mut)
            .enumerate()
        {
            recorder.attachments_to_resolve = RenderAttachmentQuantitySet::empty();

            if !recorder.state().is_disabled() {
                for &quantity in RenderAttachmentQuantity::all() {
                    if quantity.supports_multisampling() {
                        if let Some(output_description) = recorder
                            .specification
                            .output_render_attachments
                            .get_description(quantity)
                        {
                            if output_description.sampling().is_multi_if_available() {
                                last_indices_of_multisampled_output_attachments[quantity.index()] =
                                    Some(idx);
                            }
                        }

                        if recorder
                            .specification
                            .input_render_attachments
                            .quantities()
                            .contains(quantity.flag())
                        {
                            let first_idx =
                                &mut first_indices_of_input_attachments[quantity.index()];
                            if first_idx.is_none() {
                                *first_idx = Some(idx);
                            }
                        }
                    }
                }
            }

            recorders.push(recorder);
        }

        for quantity in RenderAttachmentQuantity::all() {
            match (
                last_indices_of_multisampled_output_attachments[quantity.index()],
                first_indices_of_input_attachments[quantity.index()],
            ) {
                (Some(last_idx), Some(first_idx)) if first_idx <= last_idx => {
                    panic!(
                        "multisampled {} render attachment is used as input before it is last used as output \
                        (first used as input in render pass {} ({}), \
                        last used as multisampled output in render pass {} ({}))",
                        quantity,
                        first_idx,
                        &recorders[first_idx].specification.label,
                        last_idx,
                        &recorders[last_idx].specification.label
                    );
                }
                (Some(last_idx), _) => {
                    // Make sure the last render pass to use this attachment as
                    // multisampled output resolves it
                    recorders[last_idx].attachments_to_resolve |= quantity.flag();
                }
                _ => {}
            }
        }
    }
}

impl Default for RenderCommandManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderPassSpecification {
    /// Maximum z-value in clip space.
    const CLEAR_DEPTH: f32 = 1.0;

    const CLEAR_STENCIL_VALUE: u32 = 0;
    const REFERENCE_STENCIL_VALUE: u32 = 1;

    /// Creates the specification for the render pass that will clear the given
    /// render attachments.
    fn clearing_pass(
        clear_surface: bool,
        render_attachment_quantities_to_clear: RenderAttachmentQuantitySet,
    ) -> Self {
        Self {
            clear_surface,
            color_attachments_to_clear: render_attachment_quantities_to_clear.color_only(),
            depth_map_usage: if render_attachment_quantities_to_clear
                .contains(RenderAttachmentQuantitySet::DEPTH_STENCIL)
            {
                DepthMapUsage::Clear
            } else {
                DepthMapUsage::None
            },
            hints: if clear_surface {
                RenderPassHints::WRITES_TO_SURFACE
            } else {
                RenderPassHints::empty()
            },
            label: "Clearing pass".to_string(),
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will update the depth
    /// map with the depths of the model with the given ID.
    fn depth_prepass(model_id: ModelID, hints: RenderPassHints) -> Self {
        let push_constants: PushConstantGroup = [
            PushConstant::new(
                PushConstantVariant::FrameCounter,
                wgpu::ShaderStages::VERTEX,
            ),
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::VERTEX,
            ),
        ]
        .into_iter()
        .collect();

        Self {
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::Prepass,
            push_constants,
            hints,
            label: format!("Depth prepass for model {}", model_id),
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID and prepass material.
    fn shading_prepass(
        light: Option<LightInfo>,
        model_id: ModelID,
        material_specification: &MaterialSpecification,
    ) -> Self {
        let mut push_constants: PushConstantGroup = [
            PushConstant::new(
                PushConstantVariant::FrameCounter,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // VERTEX
            ),
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // VERTEX, and also FRAGMENT if there are input attachments
            ),
            PushConstant::new(
                PushConstantVariant::Exposure,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // FRAGMENT
            ),
        ]
        .into_iter()
        .collect();

        if matches!(
            light,
            Some(LightInfo {
                light_type: LightType::AmbientLight,
                light_id: _
            })
        ) {
            push_constants.add_push_constant(PushConstant::new(
                PushConstantVariant::LightIdx,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // FRAGMENT
            ));
        }

        let label = if let Some(light) = light {
            format!(
                "Shading prepass for model {} with light {} ({:?})",
                model_id, light.light_id, light.light_type
            )
        } else {
            format!("Shading prepass for model {}", model_id)
        };

        Self {
            model_id: Some(model_id),
            use_prepass_material: true,
            depth_map_usage: DepthMapUsage::use_readwrite(),
            light,
            vertex_attribute_requirements: material_specification
                .vertex_attribute_requirements_for_shader(),
            input_render_attachments: material_specification.input_render_attachments().clone(),
            output_render_attachments: material_specification.output_render_attachments().clone(),
            push_constants,
            hints: material_specification.render_pass_hints(),
            label,
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID and material without making use of a shadow map.
    fn model_shading_pass_without_shadow_map(
        light: Option<LightInfo>,
        model_id: ModelID,
        material_specification: &MaterialSpecification,
    ) -> Self {
        let mut push_constants: PushConstantGroup = [
            PushConstant::new(
                PushConstantVariant::FrameCounter,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // VERTEX
            ),
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // VERTEX, and also FRAGMENT if there are input attachments
            ),
            PushConstant::new(
                PushConstantVariant::Exposure,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // FRAGMENT
            ),
        ]
        .into_iter()
        .collect();

        if light.is_some() {
            push_constants.add_push_constant(PushConstant::new(
                PushConstantVariant::LightIdx,
                // FRAGMENT for AmbientLight or OmnidirectionalLight, VERTEX_FRAGMENT for UnidirectionalLight
                wgpu::ShaderStages::VERTEX_FRAGMENT,
            ));
        }

        let label = if let Some(light) = light {
            format!(
                "Shading of model {} for light {} ({:?}) without shadow map",
                model_id, light.light_id, light.light_type
            )
        } else {
            format!("Shading of model {}", model_id)
        };

        Self {
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::use_readonly(),
            light,
            vertex_attribute_requirements: material_specification
                .vertex_attribute_requirements_for_shader(),
            input_render_attachments: material_specification.input_render_attachments().clone(),
            output_render_attachments: material_specification.output_render_attachments().clone(),
            push_constants,
            hints: material_specification.render_pass_hints(),
            label,
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID and material, making use of a shadow map.
    fn model_shading_pass_with_shadow_map(
        light: LightInfo,
        model_id: ModelID,
        material_specification: &MaterialSpecification,
    ) -> Self {
        let push_constants: PushConstantGroup = [
            PushConstant::new(
                PushConstantVariant::FrameCounter,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // VERTEX
            ),
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // VERTEX, and also FRAGMENT if there are input attachments
            ),
            PushConstant::new(
                PushConstantVariant::Exposure,
                wgpu::ShaderStages::VERTEX_FRAGMENT, // FRAGMENT
            ),
            PushConstant::new(
                PushConstantVariant::LightIdx,
                // FRAGMENT for AmbientLight or OmnidirectionalLight, VERTEX_FRAGMENT for UnidirectionalLight
                wgpu::ShaderStages::VERTEX_FRAGMENT,
            ),
        ]
        .into_iter()
        .collect();

        Self {
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::use_readonly(),
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Use,
            vertex_attribute_requirements: material_specification
                .vertex_attribute_requirements_for_shader(),
            input_render_attachments: material_specification.input_render_attachments().clone(),
            output_render_attachments: material_specification.output_render_attachments().clone(),
            push_constants,
            hints: material_specification.render_pass_hints(),
            label: format!(
                "Shading of model {} for light {} ({:?}) with shadow map",
                model_id, light.light_id, light.light_type
            ),
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will clear the given
    /// shadow map.
    fn shadow_map_clearing_pass(shadow_map_id: ShadowMapIdentifier) -> Self {
        Self {
            shadow_map_usage: ShadowMapUsage::Clear(shadow_map_id),
            hints: RenderPassHints::empty(),
            label: format!("Shadow map clearing pass ({:?})", shadow_map_id),
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will update the given
    /// shadow map with the depths of the model with the given ID from the point
    /// of view of the given light.
    fn shadow_map_update_pass(
        light: LightInfo,
        model_id: ModelID,
        shadow_map_id: ShadowMapIdentifier,
        hints: RenderPassHints,
    ) -> Self {
        let light_idx_stages = match light.light_type {
            LightType::AmbientLight | LightType::OmnidirectionalLight => {
                wgpu::ShaderStages::FRAGMENT
            }
            LightType::UnidirectionalLight => wgpu::ShaderStages::VERTEX,
        };
        let mut push_constants: PushConstantGroup =
            PushConstant::new(PushConstantVariant::LightIdx, light_idx_stages).into();

        if matches!(
            shadow_map_id,
            ShadowMapIdentifier::ForUnidirectionalLight(_)
        ) {
            push_constants.add_push_constant(PushConstant::new(
                PushConstantVariant::CascadeIdx,
                wgpu::ShaderStages::VERTEX,
            ));
        }

        Self {
            model_id: Some(model_id),
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Update(shadow_map_id),
            push_constants,
            hints,
            label: format!(
                "Shadow map update for model {} and light {} ({:?})",
                model_id, light.light_id, shadow_map_id
            ),
            ..Default::default()
        }
    }

    fn get_mesh_buffer_manager(
        render_resources: &SynchronizedRenderResources,
        mesh_id: MeshID,
    ) -> Result<&MeshGPUBufferManager> {
        render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))
    }

    fn get_instance_feature_buffer_managers(
        render_resources: &SynchronizedRenderResources,
        model_id: ModelID,
        use_prepass_material: bool,
        depth_map_usage: DepthMapUsage,
        shadow_map_usage: ShadowMapUsage,
    ) -> Result<(
        &InstanceFeatureGPUBufferManager,
        Option<&InstanceFeatureGPUBufferManager>,
    )> {
        if let Some(buffers) = render_resources.get_instance_feature_buffer_managers(model_id) {
            // Transform buffer is always present and always first
            let transform_buffer_manager = &buffers[0];

            let material_property_buffer_manager = if depth_map_usage.is_prepass()
                || shadow_map_usage.is_update()
            {
                // For pure depth prepass or shadow map update we only need
                // transforms
                None
            } else if use_prepass_material {
                let prepass_material_handle = model_id
                    .prepass_material_handle()
                    .ok_or_else(|| anyhow!("Missing prepass material for model {}", model_id))?;

                // We assume that if both the prepass material and main material
                // have material property features, they are the same, so we can
                // use the same instance feature buffer (which will be placed
                // directly after the transform buffer)

                match (
                    prepass_material_handle.material_property_feature_id(),
                    model_id.material_handle().material_property_feature_id(),
                ) {
                    (None, _) => None,
                    (Some(_), None) => Some(&buffers[1]),
                    (Some(prepass_feature_id), Some(main_feature_id)) => {
                        assert_eq!(
                            prepass_feature_id, main_feature_id,
                            "Prepass material must use the same feature as main material"
                        );
                        Some(&buffers[1])
                    }
                }
            } else {
                // When using the main material we know its buffer, if it
                // exists, will be directly after the transform buffer
                if model_id
                    .material_handle()
                    .material_property_feature_id()
                    .is_some()
                {
                    Some(&buffers[1])
                } else {
                    None
                }
            };

            Ok((transform_buffer_manager, material_property_buffer_manager))
        } else {
            Err(anyhow!(
                "Missing instance GPU buffer for model {}",
                model_id
            ))
        }
    }

    fn get_material_specification(
        material_library: &MaterialLibrary,
        material_id: MaterialID,
    ) -> Result<&MaterialSpecification> {
        material_library
            .get_material_specification(material_id)
            .ok_or_else(|| anyhow!("Missing specification for material {}", material_id))
    }

    fn get_material_property_texture_group(
        material_library: &MaterialLibrary,
        texture_group_id: MaterialPropertyTextureGroupID,
    ) -> Result<&MaterialPropertyTextureGroup> {
        material_library
            .get_material_property_texture_group(texture_group_id)
            .ok_or_else(|| {
                anyhow!(
                    "Missing material property texture group {}",
                    texture_group_id
                )
            })
    }

    /// Obtains the vertex buffer layouts for the required mesh vertex
    /// attributes and instance features involved in the render pass, as well as
    /// the associated shader inputs.
    ///
    /// The order of the layouts is:
    /// 1. Mesh vertex attribute buffers.
    /// 2. Transform feature buffer.
    /// 2. Material property feature buffer.
    fn get_vertex_buffer_layouts_and_shader_inputs<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Result<(
        Vec<wgpu::VertexBufferLayout<'static>>,
        Option<&'a MeshShaderInput>,
        Vec<&'a InstanceFeatureShaderInput>,
    )> {
        let mut layouts = Vec::with_capacity(8);
        let mut mesh_shader_input = None;
        let mut instance_feature_shader_inputs = Vec::with_capacity(2);

        if let Some(mesh_id) = self
            .explicit_mesh_id
            .or_else(|| self.model_id.map(|model_id| model_id.mesh_id()))
        {
            let mesh_buffer_manager = Self::get_mesh_buffer_manager(render_resources, mesh_id)?;

            layouts.extend(
                mesh_buffer_manager.request_vertex_buffer_layouts_including_position(
                    self.vertex_attribute_requirements,
                )?,
            );
            mesh_shader_input = Some(mesh_buffer_manager.shader_input());
        }

        if let Some(model_id) = self.model_id {
            let (transform_buffer_manager, material_property_buffer_manager) =
                Self::get_instance_feature_buffer_managers(
                    render_resources,
                    model_id,
                    self.use_prepass_material,
                    self.depth_map_usage,
                    self.shadow_map_usage,
                )?;

            layouts.push(transform_buffer_manager.vertex_buffer_layout().clone());
            instance_feature_shader_inputs.push(transform_buffer_manager.shader_input());

            if let Some(material_property_buffer_manager) = material_property_buffer_manager {
                layouts.push(
                    material_property_buffer_manager
                        .vertex_buffer_layout()
                        .clone(),
                );
                instance_feature_shader_inputs
                    .push(material_property_buffer_manager.shader_input());
            }
        }

        Ok((layouts, mesh_shader_input, instance_feature_shader_inputs))
    }

    /// Obtains the bind group layouts for any camera, material or lights
    /// involved in the render pass, as well as the associated shader inputs.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Shadow map textures.
    /// 4. Material-specific resources.
    /// 5. Material property textures.
    /// 6. Render attachment textures.
    /// 7. Generic GPU resource group.
    fn get_bind_group_layouts_and_shader_inputs<'a>(
        &self,
        graphics_device: &'a GraphicsDevice,
        material_library: &'a MaterialLibrary,
        render_resources: &'a SynchronizedRenderResources,
        render_attachment_texture_manager: &'a mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &'a GPUResourceGroupManager,
    ) -> Result<(
        Vec<&'a wgpu::BindGroupLayout>,
        BindGroupRenderingShaderInput<'a>,
    )> {
        let mut layouts = Vec::with_capacity(8);

        let mut shader_input = BindGroupRenderingShaderInput {
            camera: None,
            light: None,
            material: None,
        };

        // We do not need a camera if we are updating shadow map
        if !self.shadow_map_usage.is_update() && !self.hints.contains(RenderPassHints::NO_CAMERA) {
            if let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() {
                layouts.push(camera_buffer_manager.bind_group_layout());
                shader_input.camera = Some(CameraGPUBufferManager::shader_input());
            }
        }

        if let Some(LightInfo { light_type, .. }) = self.light {
            let light_buffer_manager = render_resources
                .get_light_buffer_manager()
                .expect("Missing light GPU buffer manager for shading pass with light");

            layouts.push(light_buffer_manager.light_bind_group_layout());

            if self.shadow_map_usage.is_use() {
                layouts.push(
                    light_buffer_manager
                        .shadow_map_bind_group_layout_for_light_type(light_type)
                        .expect("Missing shadow map bind group layout for shading with shadow map"),
                );
            }

            shader_input.light = Some(light_buffer_manager.shader_input_for_light_type(light_type));
        }

        if let Some(material_id) = self.explicit_material_id {
            let material_specification =
                Self::get_material_specification(material_library, material_id)?;

            if let Some(material_specific_resources) =
                material_specification.material_specific_resources()
            {
                layouts.push(material_specific_resources.bind_group_layout());
            }

            shader_input.material = Some(material_specification.shader_input());
        } else if let Some(model_id) = self.model_id {
            // We do not need a material if we are doing a pure depth prepass or
            // updating a shadow map
            if !(self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_update()) {
                let material_handle = if self.use_prepass_material {
                    model_id
                        .prepass_material_handle()
                        .ok_or_else(|| anyhow!("Missing prepass material for model {}", model_id))?
                } else {
                    model_id.material_handle()
                };

                let material_specification = Self::get_material_specification(
                    material_library,
                    material_handle.material_id(),
                )?;

                if let Some(material_specific_resources) =
                    material_specification.material_specific_resources()
                {
                    layouts.push(material_specific_resources.bind_group_layout());
                }

                shader_input.material = Some(material_specification.shader_input());

                if let Some(texture_group_id) = material_handle.material_property_texture_group_id()
                {
                    let material_property_texture_group =
                        Self::get_material_property_texture_group(
                            material_library,
                            texture_group_id,
                        )?;

                    layouts.push(material_property_texture_group.bind_group_layout());
                }
            }
        }

        if !self.input_render_attachments.is_empty() {
            layouts.extend(
                render_attachment_texture_manager
                    .create_and_get_render_attachment_texture_bind_group_layouts(
                        graphics_device,
                        &self.input_render_attachments,
                    ),
            );
        }

        if let Some(resource_group_id) = self.resource_group_id {
            let resource_group = gpu_resource_group_manager
                .get_resource_group(resource_group_id)
                .ok_or_else(|| anyhow!("Missing GPU resource group {}", resource_group_id))?;

            layouts.push(resource_group.bind_group_layout());
        }

        Ok((layouts, shader_input))
    }

    /// Obtains all bind groups involved in the render pass.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Shadow map textures.
    /// 4. Material-specific resources.
    /// 5. Material property textures.
    /// 6. Render attachment textures.
    /// 7. Generic GPU resource group.
    fn get_bind_groups<'a>(
        &self,
        material_library: &'a MaterialLibrary,
        render_resources: &'a SynchronizedRenderResources,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
        gpu_resource_group_manager: &'a GPUResourceGroupManager,
    ) -> Result<Vec<&'a wgpu::BindGroup>> {
        let mut bind_groups = Vec::with_capacity(8);

        // We do not need a camera if we are updating shadow map
        if !self.shadow_map_usage.is_update() && !self.hints.contains(RenderPassHints::NO_CAMERA) {
            if let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() {
                bind_groups.push(camera_buffer_manager.bind_group());
            }
        }

        if let Some(LightInfo { light_type, .. }) = self.light {
            let light_buffer_manager = render_resources
                .get_light_buffer_manager()
                .expect("Missing light GPU buffer manager for shading pass with light");

            bind_groups.push(light_buffer_manager.light_bind_group());

            if self.shadow_map_usage.is_use() {
                bind_groups.push(
                    light_buffer_manager
                        .shadow_map_bind_group_for_light_type(light_type)
                        .expect("Missing shadow map bind group for shading with shadow map"),
                );
            }
        }

        if let Some(material_id) = self.explicit_material_id {
            let material_specification =
                Self::get_material_specification(material_library, material_id)?;

            if let Some(material_specific_resources) =
                material_specification.material_specific_resources()
            {
                bind_groups.push(material_specific_resources.bind_group());
            }
        } else if let Some(model_id) = self.model_id {
            // We do not need a material if we are doing a pure depth prepass or
            // updating a shadow map
            if !(self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_update()) {
                let material_handle = if self.use_prepass_material {
                    model_id
                        .prepass_material_handle()
                        .ok_or_else(|| anyhow!("Missing prepass material for model {}", model_id))?
                } else {
                    model_id.material_handle()
                };

                let material_specification = Self::get_material_specification(
                    material_library,
                    material_handle.material_id(),
                )?;

                if let Some(material_specific_resources) =
                    material_specification.material_specific_resources()
                {
                    bind_groups.push(material_specific_resources.bind_group());
                }

                if let Some(texture_group_id) = material_handle.material_property_texture_group_id()
                {
                    let material_property_texture_group =
                        Self::get_material_property_texture_group(
                            material_library,
                            texture_group_id,
                        )?;

                    bind_groups.push(material_property_texture_group.bind_group());
                }
            }
        }

        if !self.input_render_attachments.is_empty() {
            bind_groups.extend(
                render_attachment_texture_manager
                    .get_render_attachment_texture_bind_groups(&self.input_render_attachments),
            );
        }

        if let Some(resource_group_id) = self.resource_group_id {
            let resource_group = gpu_resource_group_manager
                .get_resource_group(resource_group_id)
                .ok_or_else(|| anyhow!("Missing GPU resource group {}", resource_group_id))?;

            bind_groups.push(resource_group.bind_group());
        }

        Ok(bind_groups)
    }

    /// Obtains a view into the shadow map texture involved in the render pass.
    fn get_shadow_map_texture_view<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Option<&'a wgpu::TextureView> {
        if let Some(shadow_map_id) = self.shadow_map_usage.get_shadow_map_to_clear_or_update() {
            let light_buffer_manager = render_resources
                .get_light_buffer_manager()
                .expect("Missing light GPU buffer manager for shadow mapping render pass");

            Some(match shadow_map_id {
                ShadowMapIdentifier::ForOmnidirectionalLight(face) => light_buffer_manager
                    .omnidirectional_light_shadow_map_texture()
                    .face_view(face),
                ShadowMapIdentifier::ForUnidirectionalLight(cascade_idx) => light_buffer_manager
                    .unidirectional_light_shadow_map_texture()
                    .cascade_view(cascade_idx),
            })
        } else {
            None
        }
    }

    fn determine_color_blend_state(
        output_description: &RenderAttachmentOutputDescription,
    ) -> Option<wgpu::BlendState> {
        match output_description.blending() {
            Blending::Replace => Some(wgpu::BlendState::REPLACE),
            Blending::Additive => {
                Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    // We simply ignore alpha for now
                    alpha: wgpu::BlendComponent::default(),
                })
            }
        }
    }

    fn determine_color_target_states(
        &self,
        rendering_surface: &RenderingSurface,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
    ) -> Vec<Option<wgpu::ColorTargetState>> {
        if self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_clear_or_update() {
            // For pure depth prepasses and shadow map clearing or updates we
            // only work with depths, so we don't need a color target
            Vec::new()
        } else {
            let mut color_target_states = Vec::with_capacity(RenderAttachmentQuantity::count());

            if !self.output_render_attachments.is_empty() {
                color_target_states.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_textures(
                            self.output_render_attachments.quantities(),
                        )
                        .map(|texture| {
                            let output_description = self
                                .output_render_attachments
                                .get_description(texture.quantity())
                                .unwrap();
                            Some(wgpu::ColorTargetState {
                                format: texture.format(),
                                blend: Self::determine_color_blend_state(&output_description),
                                write_mask: wgpu::ColorWrites::COLOR,
                            })
                        }),
                );
            }

            if self.hints.contains(RenderPassHints::WRITES_TO_SURFACE) {
                color_target_states.push(Some(wgpu::ColorTargetState {
                    format: rendering_surface.texture_format(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::COLOR,
                }));
            }

            color_target_states
        }
    }

    fn determine_front_face(&self) -> wgpu::FrontFace {
        if let ShadowMapUsage::Update(ShadowMapIdentifier::ForOmnidirectionalLight(_)) =
            self.shadow_map_usage
        {
            // The cubemap projection does not flip the z-axis, so the front
            // faces will have the opposite winding order compared to normal
            wgpu::FrontFace::Cw
        } else {
            wgpu::FrontFace::Ccw
        }
    }

    fn determine_depth_stencil_state(&self) -> Option<wgpu::DepthStencilState> {
        if self.shadow_map_usage.is_clear_or_update() {
            // For modifying the shadow map we have to set it as the depth
            // map for the pipeline
            Some(wgpu::DepthStencilState {
                format: SHADOW_MAP_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                // Biasing is applied manually in shader
                bias: wgpu::DepthBiasState::default(),
            })
        } else if !self.depth_map_usage.is_none() {
            let depth_write_enabled = self.depth_map_usage.make_writeable();

            let (depth_compare, stencil) = if depth_write_enabled {
                (
                    wgpu::CompareFunction::Less,
                    // Write the reference stencil value to the stencil map
                    // whenever the depth test passes
                    wgpu::StencilState {
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
                )
            } else if self.depth_map_usage.is_stencil_test() {
                // When we are doing stencil testing rather than depth testing,
                // we make the depth test always pass and configure the stencil
                // operations to pass only if the stencil value is equal to the
                // reference value
                (
                    wgpu::CompareFunction::Always,
                    wgpu::StencilState {
                        front: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Equal,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Keep,
                        },
                        read_mask: 0xFF,
                        write_mask: 0x00,
                        ..Default::default()
                    },
                )
            } else {
                (
                    // When we turn off depth writing, all closest depths have
                    // been determined. To be able to do subsequent shading, we
                    // must allow shading when the depth is equal to the depth
                    // in the depth map.
                    wgpu::CompareFunction::LessEqual,
                    wgpu::StencilState::default(),
                )
            };

            let depth_stencil_state = wgpu::DepthStencilState {
                format: RenderAttachmentQuantity::depth_texture_format(),
                depth_write_enabled,
                depth_compare,
                stencil,
                bias: wgpu::DepthBiasState::default(),
            };

            Some(depth_stencil_state)
        } else {
            None
        }
    }

    fn determine_multisampling_sample_count(
        &self,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
    ) -> u32 {
        let output_render_attachment_quantities = if self.depth_map_usage.will_update() {
            self.output_render_attachments.quantities() | RenderAttachmentQuantitySet::DEPTH_STENCIL
        } else {
            self.output_render_attachments.quantities()
        };

        let mut sample_count = None;

        if output_render_attachment_quantities.is_empty() {
            1
        } else {
            for texture in render_attachment_texture_manager
                .request_render_attachment_textures(output_render_attachment_quantities)
            {
                let output_description = self
                    .output_render_attachments
                    .get_description(texture.quantity())
                    .unwrap_or_default();

                let sample_count_for_this_attachment = match output_description.sampling() {
                    OutputAttachmentSampling::Single => 1,
                    OutputAttachmentSampling::MultiIfAvailable => {
                        texture.multisampling_sample_count()
                    }
                };

                match sample_count {
                    Some(count) => {
                        if count != sample_count_for_this_attachment {
                            panic!("found multisampling and non-multisampling output render attachments in the same render pass");
                        }
                    }
                    None => {
                        sample_count = Some(sample_count_for_this_attachment);
                    }
                }
            }
            sample_count.unwrap_or(1)
        }
    }

    fn create_color_attachments<'a, 'b: 'a>(
        &'a self,
        surface_texture_view: &'a wgpu::TextureView,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
        output_render_attachments: &RenderAttachmentOutputDescriptionSet,
        attachments_to_resolve: RenderAttachmentQuantitySet,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'_>>> {
        if self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_clear_or_update() {
            // For pure depth prepasses and shadow map clearing or updates we
            // only work with depths, so we don't need any color attachments
            Vec::new()
        } else {
            let mut color_attachments = Vec::with_capacity(RenderAttachmentQuantity::count());

            let render_attachment_quantities = if self.color_attachments_to_clear.is_empty() {
                output_render_attachments.quantities()
            } else {
                self.color_attachments_to_clear.with_clear_color_only()
            };

            if !render_attachment_quantities.is_empty() {
                color_attachments.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_textures(render_attachment_quantities)
                        .map(|texture| {
                            let should_resolve =
                                attachments_to_resolve.contains(texture.quantity().flag());

                            let output_description = output_render_attachments
                                .get_description(texture.quantity())
                                .unwrap_or_default();

                            let (view, resolve_target) = texture.view_and_resolve_target(
                                output_description.sampling().is_multi_if_available(),
                                should_resolve,
                            );

                            let load = if self.color_attachments_to_clear.is_empty() {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(texture.quantity().clear_color().unwrap())
                            };

                            Some(wgpu::RenderPassColorAttachment {
                                view,
                                resolve_target,
                                ops: wgpu::Operations {
                                    load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })
                        }),
                );
            }

            if self.hints.contains(RenderPassHints::WRITES_TO_SURFACE) {
                let load = if self.clear_surface {
                    wgpu::LoadOp::Load
                } else {
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                };
                color_attachments.push(Some(wgpu::RenderPassColorAttachment {
                    view: surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                }));
            }

            color_attachments
        }
    }

    fn create_depth_stencil_attachment<'a, 'b: 'a>(
        &'a self,
        render_resources: &'b SynchronizedRenderResources,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
    ) -> Option<wgpu::RenderPassDepthStencilAttachment<'_>> {
        if self.shadow_map_usage.is_clear() {
            // For modifying the shadow map we have to set it as the depth
            // map for the pipeline
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.get_shadow_map_texture_view(render_resources).unwrap(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Self::CLEAR_DEPTH),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            })
        } else if self.shadow_map_usage.is_update() {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.get_shadow_map_texture_view(render_resources).unwrap(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            })
        } else if self.depth_map_usage.is_clear() {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: render_attachment_texture_manager
                    .render_attachment_texture(RenderAttachmentQuantity::DepthStencil)
                    .multisampled_if_available_and(true)
                    .attachment_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Self::CLEAR_DEPTH),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Self::CLEAR_STENCIL_VALUE),
                    store: wgpu::StoreOp::Store,
                }),
            })
        } else if !self.depth_map_usage.is_none() {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: render_attachment_texture_manager
                    .render_attachment_texture(RenderAttachmentQuantity::DepthStencil)
                    .multisampled_if_available_and(
                        self.output_render_attachments
                            .get_description(RenderAttachmentQuantity::DepthStencil)
                            .unwrap_or_default()
                            .sampling()
                            .is_multi_if_available(),
                    )
                    .attachment_view(),
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
}

impl Default for RenderPassSpecification {
    fn default() -> Self {
        Self {
            clear_surface: false,
            color_attachments_to_clear: RenderAttachmentQuantitySet::empty(),
            model_id: None,
            explicit_mesh_id: None,
            explicit_material_id: None,
            resource_group_id: None,
            explicit_shader_id: None,
            use_prepass_material: false,
            depth_map_usage: DepthMapUsage::None,
            light: None,
            shadow_map_usage: ShadowMapUsage::None,
            vertex_attribute_requirements: VertexAttributeSet::empty(),
            input_render_attachments: RenderAttachmentInputDescriptionSet::empty(),
            output_render_attachments: RenderAttachmentOutputDescriptionSet::empty(),
            push_constants: PushConstantGroup::new(),
            hints: RenderPassHints::empty(),
            label: String::new(),
        }
    }
}

impl RenderPassRecorder {
    /// Creates a new recorder for the render pass defined by the given
    /// specification.
    ///
    /// Shader inputs extracted from the specification are used to build or
    /// fetch the appropriate shader.
    pub fn new(
        config: &RenderingConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        shader_manager: &mut ShaderManager,
        specification: RenderPassSpecification,
        state: RenderCommandState,
    ) -> Result<Self> {
        let pipeline = if specification.model_id.is_some()
            || specification.explicit_mesh_id.is_some()
            || specification.explicit_material_id.is_some()
        {
            let (bind_group_layouts, bind_group_shader_input) = specification
                .get_bind_group_layouts_and_shader_inputs(
                    graphics_device,
                    material_library,
                    render_resources,
                    render_attachment_texture_manager,
                    gpu_resource_group_manager,
                )?;

            let (vertex_buffer_layouts, mesh_shader_input, instance_feature_shader_inputs) =
                specification.get_vertex_buffer_layouts_and_shader_inputs(render_resources)?;

            let push_constant_ranges = specification.push_constants.create_ranges();

            assert!(
                push_constant_ranges.len() < 2,
                "Push constants don't work correctly with multiple ranges"
            );

            let shader = if let Some(shader_id) = &specification.explicit_shader_id {
                shader_manager
                    .rendering_shaders
                    .get(shader_id)
                    .ok_or_else(|| {
                        anyhow!(
                            "Missing explicit shader for render pass: {}",
                            specification.label
                        )
                    })?
            } else {
                shader_manager.obtain_rendering_shader(
                    graphics_device,
                    bind_group_shader_input.camera,
                    mesh_shader_input,
                    bind_group_shader_input.light,
                    &instance_feature_shader_inputs,
                    bind_group_shader_input.material,
                    specification.vertex_attribute_requirements,
                    specification.input_render_attachments.quantities(),
                    specification.output_render_attachments.quantities(),
                    specification.push_constants.clone(),
                )?
            };

            let pipeline_layout = Self::create_pipeline_layout(
                graphics_device.device(),
                &bind_group_layouts,
                &push_constant_ranges,
                &format!("{} render pipeline layout", &specification.label),
            );

            let color_target_states = specification.determine_color_target_states(
                rendering_surface,
                render_attachment_texture_manager,
            );

            let front_face = specification.determine_front_face();

            let depth_stencil_state = specification.determine_depth_stencil_state();

            let multisampling_sample_count = specification
                .determine_multisampling_sample_count(render_attachment_texture_manager);

            let pipeline = Some(Self::create_pipeline(
                graphics_device.device(),
                &pipeline_layout,
                shader,
                &vertex_buffer_layouts,
                &color_target_states,
                front_face,
                depth_stencil_state,
                multisampling_sample_count,
                config,
                &format!("{} render pipeline", &specification.label),
            ));

            pipeline
        } else {
            // If we don't have vertices and a material we don't need a pipeline
            None
        };

        Ok(Self {
            specification,
            attachments_to_resolve: RenderAttachmentQuantitySet::empty(),
            pipeline,
            state,
        })
    }

    pub fn clearing_pass(
        clear_surface: bool,
        render_attachment_quantities_to_clear: RenderAttachmentQuantitySet,
    ) -> Self {
        let specification = RenderPassSpecification::clearing_pass(
            clear_surface,
            render_attachment_quantities_to_clear,
        );
        Self {
            specification,
            attachments_to_resolve: RenderAttachmentQuantitySet::empty(),
            pipeline: None,
            state: RenderCommandState::Active,
        }
    }

    /// Records the render pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the render resources used in this render pass
    /// are no longer available.
    pub fn record_pass(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        command_encoder: &mut wgpu::CommandEncoder,
        frame_counter: u32,
    ) -> Result<RenderCommandOutcome> {
        if self.state().is_disabled() {
            log::debug!("Skipping render pass: {}", &self.specification.label);
            return Ok(RenderCommandOutcome::Skipped);
        }

        log::debug!("Recording render pass: {}", &self.specification.label);

        // Make sure all data is available before doing anything else

        let bind_groups = self.specification.get_bind_groups(
            material_library,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
        )?;

        let mesh_buffer_manager = if let Some(mesh_id) =
            self.specification.explicit_mesh_id.or_else(|| {
                self.specification
                    .model_id
                    .map(|model_id| model_id.mesh_id())
            }) {
            Some(RenderPassSpecification::get_mesh_buffer_manager(
                render_resources,
                mesh_id,
            )?)
        } else {
            None
        };

        let feature_buffer_managers = if let Some(model_id) = self.specification.model_id {
            Some(
                RenderPassSpecification::get_instance_feature_buffer_managers(
                    render_resources,
                    model_id,
                    self.specification.use_prepass_material,
                    self.specification.depth_map_usage,
                    self.specification.shadow_map_usage,
                )?,
            )
        } else {
            None
        };

        let color_attachments = self.specification.create_color_attachments(
            surface_texture_view,
            render_attachment_texture_manager,
            &self.specification.output_render_attachments,
            self.attachments_to_resolve,
        );

        let mut depth_stencil_attachment = self
            .specification
            .create_depth_stencil_attachment(render_resources, render_attachment_texture_manager);

        if self.pipeline.is_none() && !color_attachments.is_empty() {
            // If we don't have a pipeline, it is safe to split up the
            // attachments into chunks of 8 to avoid exceeding the maximum
            // number of color attachments per render pass (this is probably
            // only relevant for clearing passes)
            for color_attachments in color_attachments.chunks(8) {
                command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments,
                    depth_stencil_attachment: depth_stencil_attachment.take(), // Only include the depth attachment once
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    label: Some(&self.specification.label),
                });
            }
            return Ok(RenderCommandOutcome::Recorded);
        }

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            // A `@location(i)` directive in the fragment shader output targets color attachment `i` here
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            label: Some(&self.specification.label),
        });

        render_pass.set_stencil_reference(RenderPassSpecification::REFERENCE_STENCIL_VALUE);

        if let Some(ref pipeline) = self.pipeline {
            let mesh_buffer_manager = mesh_buffer_manager.expect("Has pipeline but no vertices");

            render_pass.set_pipeline(pipeline);

            self.set_push_constants(
                &mut render_pass,
                rendering_surface,
                render_resources,
                postprocessor,
                frame_counter,
            );

            for (index, &bind_group) in bind_groups.iter().enumerate() {
                render_pass.set_bind_group(u32::try_from(index).unwrap(), bind_group, &[]);
            }

            let mut vertex_buffer_slot = 0;

            for vertex_buffer in mesh_buffer_manager.request_vertex_gpu_buffers_including_position(
                self.specification.vertex_attribute_requirements,
            )? {
                render_pass
                    .set_vertex_buffer(vertex_buffer_slot, vertex_buffer.valid_buffer_slice());

                vertex_buffer_slot += 1;
            }

            let instance_range =
                if let Some((transform_buffer_manager, material_property_buffer_manager)) =
                    feature_buffer_managers
                {
                    render_pass.set_vertex_buffer(
                        vertex_buffer_slot,
                        transform_buffer_manager
                            .vertex_gpu_buffer()
                            .valid_buffer_slice(),
                    );
                    vertex_buffer_slot += 1;

                    if let ShadowMapUsage::Update(shadow_map_id) =
                        self.specification.shadow_map_usage
                    {
                        // When updating the shadow map, we don't use model view
                        // transforms but rather the model to light space tranforms
                        // that have been written to the range dedicated for the
                        // active light in the transform buffer
                        let buffer_range_id = match shadow_map_id {
                            ShadowMapIdentifier::ForOmnidirectionalLight(face) => {
                                // Offset the light index with the face index to get
                                // the index for the range of transforms for the
                                // specific cubemap face
                                self.specification
                                    .light
                                    .unwrap()
                                    .light_id
                                    .as_instance_feature_buffer_range_id()
                                    + face.as_idx_u32()
                            }
                            ShadowMapIdentifier::ForUnidirectionalLight(cascade_idx) => {
                                // Offset the light index with the cascade index to
                                // get the index for the range of transforms for the
                                // specific cascade
                                self.specification
                                    .light
                                    .unwrap()
                                    .light_id
                                    .as_instance_feature_buffer_range_id()
                                    + cascade_idx
                            }
                        };

                        transform_buffer_manager.feature_range(buffer_range_id)
                    } else {
                        #[allow(unused_assignments)]
                        if let Some(material_property_buffer_manager) =
                            material_property_buffer_manager
                        {
                            render_pass.set_vertex_buffer(
                                vertex_buffer_slot,
                                material_property_buffer_manager
                                    .vertex_gpu_buffer()
                                    .valid_buffer_slice(),
                            );
                            vertex_buffer_slot += 1;
                        }

                        transform_buffer_manager.initial_feature_range()
                    }
                } else {
                    0..1
                };

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

        Ok(RenderCommandOutcome::Recorded)
    }

    /// Returns the state of the render pass.
    pub fn state(&self) -> RenderCommandState {
        self.state
    }

    /// Sets the state of the render pass.
    pub fn set_state(&mut self, state: RenderCommandState) {
        self.state = state;
    }

    /// Set whether the render pass should be skipped.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.state = RenderCommandState::disabled_if(disabled);
    }

    fn set_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        rendering_surface: &RenderingSurface,
        render_resources: &SynchronizedRenderResources,
        postprocessor: &Postprocessor,
        frame_counter: u32,
    ) {
        self.specification
            .push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.specification
            .push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::PixelCount,
                || rendering_surface.pixel_count_push_constant(),
            );

        if let Some(LightInfo {
            light_type,
            light_id,
        }) = self.specification.light
        {
            self.specification
                .push_constants
                .set_push_constant_for_render_pass_if_present(
                    render_pass,
                    PushConstantVariant::LightIdx,
                    || {
                        render_resources
                            .get_light_buffer_manager()
                            .unwrap()
                            .light_idx_push_constant(light_type, light_id)
                    },
                );
        }

        if let ShadowMapUsage::Update(ShadowMapIdentifier::ForUnidirectionalLight(cascade_idx)) =
            self.specification.shadow_map_usage
        {
            self.specification
                .push_constants
                .set_push_constant_for_render_pass_if_present(
                    render_pass,
                    PushConstantVariant::CascadeIdx,
                    || cascade_idx,
                );
        }

        self.specification
            .push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );

        self.specification
            .push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::InverseExposure,
                || {
                    postprocessor
                        .capturing_camera()
                        .inverse_exposure_push_constant()
                },
            );

        self.specification
            .push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                PushConstantVariant::FrameCounter,
                || frame_counter,
            );
    }

    fn create_pipeline_layout(
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

    fn create_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &Shader,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
        color_target_states: &[Option<wgpu::ColorTargetState>],
        front_face: wgpu::FrontFace,
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        multisampling_sample_count: u32,
        config: &RenderingConfig,
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
                cull_mode: config.cull_mode,
                polygon_mode: config.polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: depth_stencil_state,
            multisample: wgpu::MultisampleState {
                count: multisampling_sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            label: Some(label),
        })
    }
}

impl RenderAttachmentMipmappingPassRecorder {
    /// Creates a new mipmapping pass recorder for the render attachment for the
    /// given quantity.
    pub fn new(quantity: RenderAttachmentQuantity, state: RenderCommandState) -> Self {
        Self { quantity, state }
    }

    /// Returns the state of the mipmapping pass.
    pub fn state(&self) -> RenderCommandState {
        self.state
    }

    /// Sets the state of the mipmapping pass.
    pub fn set_state(&mut self, state: RenderCommandState) {
        self.state = state;
    }

    /// Set whether the mipmapping pass should be skipped.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.state = RenderCommandState::disabled_if(disabled);
    }

    /// Records the mipmapping pass to the given command encoder.
    pub fn record_pass(
        &self,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> RenderCommandOutcome {
        if self.state().is_disabled() {
            log::debug!(
                "Skipping {} render attachment mipmapping pass",
                self.quantity
            );
            return RenderCommandOutcome::Skipped;
        }

        log::debug!(
            "Recording {} render attachment mipmapping pass",
            self.quantity
        );

        let texture = render_attachment_texture_manager.render_attachment_texture(self.quantity);

        texture
            .regular
            .mipmapper()
            .expect("Missing mipmapper for mipmapped render attachment texture")
            .encode_mipmap_passes(command_encoder);

        RenderCommandOutcome::Recorded
    }
}

impl StorageBufferResultCopyPassRecorder {
    /// Creates a new result copy pass recorder for the storage buffer with the
    /// given ID.
    pub fn new(buffer_id: StorageBufferID, state: RenderCommandState) -> Self {
        Self { buffer_id, state }
    }

    /// Returns the state of the copy pass.
    pub fn state(&self) -> RenderCommandState {
        self.state
    }

    /// Sets the state of the copy pass.
    pub fn set_state(&mut self, state: RenderCommandState) {
        self.state = state;
    }

    /// Set whether the copy pass should be skipped.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.state = RenderCommandState::disabled_if(disabled);
    }

    /// Records the copy pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if the storage buffer is not available or does not have
    /// a result buffer.
    pub fn record_pass(
        &self,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<RenderCommandOutcome> {
        if self.state().is_disabled() {
            log::debug!(
                "Skipping storage buffer result copy pass for {}",
                self.buffer_id
            );
            return Ok(RenderCommandOutcome::Skipped);
        }

        log::debug!(
            "Recording storage buffer result copy pass for {}",
            self.buffer_id
        );

        let storage_buffer = storage_gpu_buffer_manager
            .get_storage_buffer(self.buffer_id)
            .ok_or_else(|| anyhow!("Missing storage buffer {}", self.buffer_id))?;

        storage_buffer.encode_copy_to_result_buffer(command_encoder)?;

        Ok(RenderCommandOutcome::Recorded)
    }
}

impl RenderCommandRecorder {
    /// Creates a new recorder for the command defined by the given
    /// specification.
    ///
    /// Shader inputs extracted from the specification are used to build or
    /// fetch the appropriate shader.
    pub fn new(
        config: &RenderingConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        shader_manager: &mut ShaderManager,
        specification: RenderCommandSpecification,
        state: RenderCommandState,
    ) -> Result<Self> {
        match specification {
            RenderCommandSpecification::RenderPass(specification) => Self::new_render_pass(
                config,
                graphics_device,
                rendering_surface,
                material_library,
                render_resources,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                shader_manager,
                specification,
                state,
            ),
            RenderCommandSpecification::ComputePass(specification) => Self::new_compute_pass(
                graphics_device,
                shader_manager,
                gpu_resource_group_manager,
                render_attachment_texture_manager,
                specification,
                state,
            ),
            RenderCommandSpecification::RenderAttachmentMipmappingPass { quantity } => {
                Ok(Self::new_render_attachment_mipmapping_pass(quantity, state))
            }
            RenderCommandSpecification::StorageBufferResultCopyPass { buffer_id } => {
                Ok(Self::new_storage_buffer_result_copy_pass(buffer_id, state))
            }
        }
    }

    /// Creates a new recorder for the render pass defined by the given
    /// specification.
    ///
    /// Shader inputs extracted from the specification are used to build or
    /// fetch the appropriate shader.
    pub fn new_render_pass(
        config: &RenderingConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        shader_manager: &mut ShaderManager,
        specification: RenderPassSpecification,
        state: RenderCommandState,
    ) -> Result<Self> {
        Ok(Self::RenderPass(RenderPassRecorder::new(
            config,
            graphics_device,
            rendering_surface,
            material_library,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            shader_manager,
            specification,
            state,
        )?))
    }

    pub fn clearing_render_pass(
        clear_surface: bool,
        render_attachment_quantities_to_clear: RenderAttachmentQuantitySet,
    ) -> Self {
        Self::RenderPass(RenderPassRecorder::clearing_pass(
            clear_surface,
            render_attachment_quantities_to_clear,
        ))
    }

    /// Creates a new recorder for the compute pass defined by the given
    /// specification.
    ///
    /// Shader inputs extracted from the specification are used to build or
    /// fetch the appropriate shader.
    pub fn new_compute_pass(
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        specification: ComputePassSpecification,
        state: RenderCommandState,
    ) -> Result<Self> {
        Ok(Self::ComputePass(ComputePassRecorder::new(
            graphics_device,
            shader_manager,
            gpu_resource_group_manager,
            render_attachment_texture_manager,
            specification,
            state,
        )?))
    }

    /// Creates a new mipmapping pass recorder for the render attachment for the
    /// given quantity.
    pub fn new_render_attachment_mipmapping_pass(
        quantity: RenderAttachmentQuantity,
        state: RenderCommandState,
    ) -> Self {
        Self::RenderAttachmentMipmappingPass(RenderAttachmentMipmappingPassRecorder::new(
            quantity, state,
        ))
    }

    /// Creates a new result copy pass recorder for the storage buffer with the
    /// given ID.
    pub fn new_storage_buffer_result_copy_pass(
        buffer_id: StorageBufferID,
        state: RenderCommandState,
    ) -> Self {
        Self::StorageBufferResultCopyPass(StorageBufferResultCopyPassRecorder::new(
            buffer_id, state,
        ))
    }

    pub fn as_render_pass_mut(&mut self) -> Option<&mut RenderPassRecorder> {
        match self {
            Self::RenderPass(recorder) => Some(recorder),
            _ => None,
        }
    }

    /// Records the command to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the render resources used in this command are
    /// no longer available.
    pub fn record(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        material_library: &MaterialLibrary,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        postprocessor: &Postprocessor,
        command_encoder: &mut wgpu::CommandEncoder,
        frame_counter: u32,
    ) -> Result<RenderCommandOutcome> {
        match self {
            Self::RenderPass(recorder) => recorder.record_pass(
                rendering_surface,
                surface_texture_view,
                material_library,
                render_resources,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                postprocessor,
                command_encoder,
                frame_counter,
            ),
            Self::ComputePass(recorder) => recorder.record_pass(
                rendering_surface,
                gpu_resource_group_manager,
                render_attachment_texture_manager,
                postprocessor,
                command_encoder,
            ),
            Self::RenderAttachmentMipmappingPass(recorder) => {
                Ok(recorder.record_pass(render_attachment_texture_manager, command_encoder))
            }
            Self::StorageBufferResultCopyPass(recorder) => {
                recorder.record_pass(storage_gpu_buffer_manager, command_encoder)
            }
        }
    }

    /// Returns the state of the command.
    pub fn state(&self) -> RenderCommandState {
        match self {
            Self::RenderPass(recorder) => recorder.state(),
            Self::ComputePass(recorder) => recorder.state(),
            Self::RenderAttachmentMipmappingPass(recorder) => recorder.state(),
            Self::StorageBufferResultCopyPass(recorder) => recorder.state(),
        }
    }

    /// Sets the state of the command.
    pub fn set_state(&mut self, state: RenderCommandState) {
        match self {
            Self::RenderPass(recorder) => recorder.set_state(state),
            Self::ComputePass(recorder) => recorder.set_state(state),
            Self::RenderAttachmentMipmappingPass(recorder) => recorder.set_state(state),
            Self::StorageBufferResultCopyPass(recorder) => recorder.set_state(state),
        }
    }

    /// Set whether the command should be skipped.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.set_state(RenderCommandState::disabled_if(disabled));
    }
}

impl RenderCommandState {
    /// Returns `Active` if the given `bool` is `true`, otherwise `Disabled`.
    pub fn active_if(active: bool) -> Self {
        if active {
            Self::Active
        } else {
            Self::Disabled
        }
    }

    /// Returns `Disabled` if the given `bool` is `true`, otherwise `Active`.
    pub fn disabled_if(disabled: bool) -> Self {
        if disabled {
            Self::Disabled
        } else {
            Self::Active
        }
    }

    /// Whether the render command is disabled.
    pub fn is_disabled(&self) -> bool {
        *self == Self::Disabled
    }
}

impl DepthMapUsage {
    fn use_readonly() -> Self {
        Self::Use(false)
    }

    fn use_readwrite() -> Self {
        Self::Use(true)
    }

    fn is_none(&self) -> bool {
        *self == Self::None
    }

    fn is_clear(&self) -> bool {
        *self == Self::Clear
    }

    fn is_prepass(&self) -> bool {
        *self == Self::Prepass
    }

    fn is_stencil_test(&self) -> bool {
        *self == Self::StencilTest
    }

    fn will_update(&self) -> bool {
        self.is_prepass() || *self == Self::Use(true)
    }

    fn make_writeable(&self) -> bool {
        self.is_clear() || self.will_update()
    }
}

impl ShadowMapUsage {
    fn is_clear(&self) -> bool {
        matches!(*self, Self::Clear(_))
    }

    fn is_update(&self) -> bool {
        matches!(*self, Self::Update(_))
    }

    fn is_use(&self) -> bool {
        *self == Self::Use
    }

    fn is_clear_or_update(&self) -> bool {
        self.is_clear() || self.is_update()
    }

    fn get_shadow_map_to_clear_or_update(&self) -> Option<ShadowMapIdentifier> {
        match self {
            Self::Update(shadow_map_id) | Self::Clear(shadow_map_id) => Some(*shadow_map_id),
            _ => None,
        }
    }
}
