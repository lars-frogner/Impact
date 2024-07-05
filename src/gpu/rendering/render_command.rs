//! Rendering pipelines.

mod tasks;

pub use tasks::SyncRenderCommands;

use crate::{
    geometry::{CubemapFace, VertexAttributeSet},
    gpu::{
        rendering::{
            camera::CameraRenderBufferManager, instance::InstanceFeatureRenderBufferManager,
            light::LightRenderBufferManager, mesh::MeshRenderBufferManager,
            postprocessing::PostprocessingResourceManager, resource::SynchronizedRenderResources,
            texture::SHADOW_MAP_FORMAT, CascadeIdx, GPUComputationID, GPUComputationLibrary,
            GPUComputationSpecification, RenderAttachmentQuantity, RenderAttachmentQuantitySet,
            RenderAttachmentTextureManager, RenderingConfig, RenderingSurface,
        },
        shader::{
            CameraShaderInput, ComputeShaderInput, InstanceFeatureShaderInput, LightShaderInput,
            MaterialShaderInput, MeshShaderInput, Shader, ShaderManager,
        },
        GraphicsDevice,
    },
    scene::{
        LightID, LightType, MaterialID, MaterialLibrary, MaterialPropertyTextureGroup,
        MaterialPropertyTextureGroupID, MaterialSpecification, MeshID, ModelID, Postprocessor,
        MAX_SHADOW_MAP_CASCADES,
    },
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
    /// Whether to use the prepass material associated with the model's material
    /// rather than using the model's material.
    pub use_prepass_material: bool,
    /// Whether and how the depth map will be used.
    pub depth_map_usage: DepthMapUsage,
    /// Light source whose contribution is computed in this pass.
    pub light: Option<LightInfo>,
    /// Whether and how the shadow map will be used.
    pub shadow_map_usage: ShadowMapUsage,
    /// Whether to write to the multisampled versions of the output attachment
    /// textures when available, or only use the regular versions.
    pub output_attachment_sampling: OutputAttachmentSampling,
    /// The blending mode to use for each output attachment in this pass. When
    /// not specified here, the default blending mode for the attachment will be
    /// used.
    pub blending_overrides: HashMap<RenderAttachmentQuantity, Blending>,
    pub hints: RenderPassHints,
    pub label: String,
}

/// Holds the information describing a specific compute pass.
#[derive(Clone, Debug)]
pub struct ComputePassSpecification {
    pub computation_id: GPUComputationID,
    pub workgroups: (u32, u32, u32),
    pub label: String,
}

/// Holds the information describing a specific render command.
#[derive(Clone, Debug)]
pub enum RenderCommandSpecification {
    RenderPass(RenderPassSpecification),
    ComputePass(ComputePassSpecification),
}

/// Recorder for a specific render pass.
#[derive(Debug)]
pub struct RenderPassRecorder {
    specification: RenderPassSpecification,
    vertex_attribute_requirements: VertexAttributeSet,
    input_render_attachment_quantities: RenderAttachmentQuantitySet,
    output_render_attachment_quantities: RenderAttachmentQuantitySet,
    attachments_to_resolve: RenderAttachmentQuantitySet,
    pipeline: Option<wgpu::RenderPipeline>,
    state: RenderCommandState,
}

/// Recorder for a specific compute pass.
#[derive(Debug)]
pub struct ComputePassRecorder {
    specification: ComputePassSpecification,
    pipeline: wgpu::ComputePipeline,
    state: RenderCommandState,
}

#[derive(Debug)]
pub enum RenderCommandRecorder {
    RenderPass(RenderPassRecorder),
    ComputePass(ComputePassRecorder),
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

/// Whether a render pass should write to the multisampled versions of the
/// output attachment textures when available, or only use the regular
/// versions.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OutputAttachmentSampling {
    Single,
    MultiIfAvailable,
}

/// The blending mode to use for a render pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        self.postprocessing_passes.clear();
        self.non_light_shaded_model_shading_passes.clear();
        self.light_shaded_model_shading_passes.clear();
        self.non_light_shaded_model_index_mapper.clear();
        self.light_shaded_model_index_mapper.clear();
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
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_computation_library: &GPUComputationLibrary,
        shader_manager: &mut ShaderManager,
        postprocessor: &Postprocessor,
    ) -> Result<()> {
        self.sync_clearing_passes(config);

        let light_buffer_manager = render_resources.get_light_buffer_manager();

        let ambient_light_ids =
            light_buffer_manager.map_or_else(|| &[], LightRenderBufferManager::ambient_light_ids);
        let omnidirectional_light_ids = light_buffer_manager
            .map_or_else(|| &[], LightRenderBufferManager::omnidirectional_light_ids);
        let unidirectional_light_ids = light_buffer_manager
            .map_or_else(|| &[], LightRenderBufferManager::unidirectional_light_ids);

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

            let hints = material_library
                .get_material_specification(model_id.material_handle().material_id())
                .expect("Missing specification for material")
                .render_pass_hints();

            if hints.contains(RenderPassHints::AFFECTED_BY_LIGHT) {
                match self.light_shaded_model_index_mapper.try_push_key(model_id) {
                    // The model has no existing shading passes
                    Ok(_) => {
                        if let Some(prepass_material_handle) = model_id.prepass_material_handle() {
                            let prepass_hints = material_library
                                .get_material_specification(prepass_material_handle.material_id())
                                .expect("Missing rpecification for prepass material")
                                .render_pass_hints();

                            if ambient_light_ids.is_empty() {
                                self.light_shaded_model_shading_prepasses.push(
                                    RenderCommandRecorder::new_render_pass(
                                        config,
                                        graphics_device,
                                        rendering_surface,
                                        material_library,
                                        render_resources,
                                        render_attachment_texture_manager,
                                        shader_manager,
                                        RenderPassSpecification::shading_prepass(
                                            None,
                                            model_id,
                                            prepass_hints,
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
                                            shader_manager,
                                            RenderPassSpecification::shading_prepass(
                                                Some(light),
                                                model_id,
                                                prepass_hints,
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
                                    shader_manager,
                                    RenderPassSpecification::model_shading_pass_with_shadow_map(
                                        light, model_id, hints,
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
                                    shader_manager,
                                    RenderPassSpecification::model_shading_pass_with_shadow_map(
                                        light, model_id, hints,
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
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_without_shadow_map(
                                    None, model_id, hints,
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
                    gpu_computation_library,
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
                for quantity in RenderAttachmentQuantity::all() {
                    if quantity.supports_multisampling() {
                        if recorder
                            .output_render_attachment_quantities
                            .contains(quantity.flag())
                            && recorder
                                .specification
                                .output_attachment_sampling
                                .is_multi_if_available()
                        {
                            last_indices_of_multisampled_output_attachments[quantity.index()] =
                                Some(idx);
                        }

                        if recorder
                            .input_render_attachment_quantities
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
                .contains(RenderAttachmentQuantitySet::DEPTH)
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
        Self {
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::Prepass,
            hints,
            label: format!("Depth prepass for model {}", model_id),
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID with its prepass material.
    fn shading_prepass(
        light: Option<LightInfo>,
        model_id: ModelID,
        hints: RenderPassHints,
    ) -> Self {
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
            hints,
            label,
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID without making use of a shadow map.
    fn model_shading_pass_without_shadow_map(
        light: Option<LightInfo>,
        model_id: ModelID,
        hints: RenderPassHints,
    ) -> Self {
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
            hints,
            label,
            ..Default::default()
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID making use of a shadow map.
    fn model_shading_pass_with_shadow_map(
        light: LightInfo,
        model_id: ModelID,
        hints: RenderPassHints,
    ) -> Self {
        Self {
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::use_readonly(),
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Use,
            hints,
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
        Self {
            model_id: Some(model_id),
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Update(shadow_map_id),
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
    ) -> Result<&MeshRenderBufferManager> {
        render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing render buffer for mesh {}", mesh_id))
    }

    fn get_instance_feature_buffer_managers(
        render_resources: &SynchronizedRenderResources,
        model_id: ModelID,
        use_prepass_material: bool,
        depth_map_usage: DepthMapUsage,
        shadow_map_usage: ShadowMapUsage,
    ) -> Result<(
        &InstanceFeatureRenderBufferManager,
        Option<&InstanceFeatureRenderBufferManager>,
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
                "Missing instance render buffer for model {}",
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

    /// Obtains the push constant range involved in the render pass.
    fn get_push_constant_range(&self) -> wgpu::PushConstantRange {
        let mut size = RenderingSurface::INVERSE_WINDOW_DIMENSIONS_PUSH_CONSTANT_SIZE;

        size += PostprocessingResourceManager::EXPOSURE_PUSH_CONSTANT_SIZE;

        if self.light.is_some() {
            size += LightRenderBufferManager::LIGHT_IDX_PUSH_CONSTANT_SIZE;

            if matches!(
                self.shadow_map_usage,
                ShadowMapUsage::Update(ShadowMapIdentifier::ForUnidirectionalLight(_))
            ) {
                size += LightRenderBufferManager::CASCADE_IDX_PUSH_CONSTANT_SIZE;
            }
        }

        wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
            range: 0..size,
        }
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
        vertex_attribute_requirements: VertexAttributeSet,
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
                    vertex_attribute_requirements,
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
    /// involved in the render pass, as well as the associated shader inputs and
    /// the vertex attribute requirements and output render attachment
    /// quantities of the material.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Shadow map textures.
    /// 4. Material-specific resources.
    /// 5. Render attachment textures.
    /// 6. Material property textures.
    fn get_bind_group_layouts_shader_inputs_and_material_data<'a>(
        &self,
        material_library: &'a MaterialLibrary,
        render_resources: &'a SynchronizedRenderResources,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
    ) -> Result<(
        Vec<&'a wgpu::BindGroupLayout>,
        BindGroupRenderingShaderInput<'a>,
        VertexAttributeSet,
        RenderAttachmentQuantitySet,
        RenderAttachmentQuantitySet,
    )> {
        let mut layouts = Vec::with_capacity(8);

        let mut shader_input = BindGroupRenderingShaderInput {
            camera: None,
            light: None,
            material: None,
        };

        let mut vertex_attribute_requirements = VertexAttributeSet::empty();

        let mut input_render_attachment_quantities = RenderAttachmentQuantitySet::empty();
        let mut output_render_attachment_quantities = RenderAttachmentQuantitySet::empty();

        // We do not need a camera if we are updating shadow map
        if !self.shadow_map_usage.is_update() && !self.hints.contains(RenderPassHints::NO_CAMERA) {
            if let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() {
                layouts.push(camera_buffer_manager.bind_group_layout());
                shader_input.camera = Some(CameraRenderBufferManager::shader_input());
            }
        }

        if let Some(LightInfo { light_type, .. }) = self.light {
            let light_buffer_manager = render_resources
                .get_light_buffer_manager()
                .expect("Missing light render buffer manager for shading pass with light");

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

            input_render_attachment_quantities =
                material_specification.input_render_attachment_quantities();

            output_render_attachment_quantities =
                material_specification.output_render_attachment_quantities();

            if !input_render_attachment_quantities.is_empty() {
                layouts.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_texture_bind_group_layouts(
                            input_render_attachment_quantities,
                        ),
                );
            }

            shader_input.material = Some(material_specification.shader_input());

            vertex_attribute_requirements =
                material_specification.vertex_attribute_requirements_for_shader();
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

                input_render_attachment_quantities =
                    material_specification.input_render_attachment_quantities();

                output_render_attachment_quantities =
                    material_specification.output_render_attachment_quantities();

                if !input_render_attachment_quantities.is_empty() {
                    layouts.extend(
                        render_attachment_texture_manager
                            .request_render_attachment_texture_bind_group_layouts(
                                input_render_attachment_quantities,
                            ),
                    );
                }

                shader_input.material = Some(material_specification.shader_input());

                vertex_attribute_requirements =
                    material_specification.vertex_attribute_requirements_for_shader();

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

        Ok((
            layouts,
            shader_input,
            vertex_attribute_requirements,
            input_render_attachment_quantities,
            output_render_attachment_quantities,
        ))
    }

    /// Obtains all bind groups involved in the render pass as well as the
    /// output render attachment quantities of the material.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Shadow map textures.
    /// 4. Material-specific resources.
    /// 5. Render attachment textures.
    /// 6. Material property textures.
    fn get_bind_groups_and_material_data<'a>(
        &self,
        material_library: &'a MaterialLibrary,
        render_resources: &'a SynchronizedRenderResources,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
    ) -> Result<(Vec<&'a wgpu::BindGroup>, RenderAttachmentQuantitySet)> {
        let mut bind_groups = Vec::with_capacity(8);

        let mut output_render_attachment_quantities = RenderAttachmentQuantitySet::empty();

        // We do not need a camera if we are updating shadow map
        if !self.shadow_map_usage.is_update() && !self.hints.contains(RenderPassHints::NO_CAMERA) {
            if let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() {
                bind_groups.push(camera_buffer_manager.bind_group());
            }
        }

        if let Some(LightInfo { light_type, .. }) = self.light {
            let light_buffer_manager = render_resources
                .get_light_buffer_manager()
                .expect("Missing light render buffer manager for shading pass with light");

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

            let input_render_attachment_quantities =
                material_specification.input_render_attachment_quantities();

            output_render_attachment_quantities =
                material_specification.output_render_attachment_quantities();

            if !input_render_attachment_quantities.is_empty() {
                bind_groups.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_texture_bind_groups(
                            input_render_attachment_quantities,
                        ),
                );
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

                let input_render_attachment_quantities =
                    material_specification.input_render_attachment_quantities();

                output_render_attachment_quantities =
                    material_specification.output_render_attachment_quantities();

                if !input_render_attachment_quantities.is_empty() {
                    bind_groups.extend(
                        render_attachment_texture_manager
                            .request_render_attachment_texture_bind_groups(
                                input_render_attachment_quantities,
                            ),
                    );
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

        Ok((bind_groups, output_render_attachment_quantities))
    }

    /// Obtains a view into the shadow map texture involved in the render pass.
    fn get_shadow_map_texture_view<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Option<&'a wgpu::TextureView> {
        if let Some(shadow_map_id) = self.shadow_map_usage.get_shadow_map_to_clear_or_update() {
            let light_buffer_manager = render_resources
                .get_light_buffer_manager()
                .expect("Missing light render buffer manager for shadow mapping render pass");

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
        &self,
        quantity: RenderAttachmentQuantity,
    ) -> Option<wgpu::BlendState> {
        let blending = self
            .blending_overrides
            .get(&quantity)
            .copied()
            .unwrap_or_else(|| quantity.blending());

        match blending {
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
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
    ) -> Vec<Option<wgpu::ColorTargetState>> {
        if self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_clear_or_update() {
            // For pure depth prepasses and shadow map clearing or updates we
            // only work with depths, so we don't need a color target
            Vec::new()
        } else {
            let mut color_target_states = Vec::with_capacity(RenderAttachmentQuantity::count());

            if !output_render_attachment_quantities.is_empty() {
                color_target_states.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_textures(output_render_attachment_quantities)
                        .map(|texture| {
                            Some(wgpu::ColorTargetState {
                                format: texture.format(),
                                blend: self.determine_color_blend_state(texture.quantity()),
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
        mut output_render_attachment_quantities: RenderAttachmentQuantitySet,
    ) -> u32 {
        if self.output_attachment_sampling.is_single() {
            return 1;
        }

        if self.depth_map_usage.will_update() {
            output_render_attachment_quantities |= RenderAttachmentQuantitySet::DEPTH;
        }

        let mut sample_count = None;

        if output_render_attachment_quantities.is_empty() {
            1
        } else {
            for texture in render_attachment_texture_manager
                .request_render_attachment_textures(output_render_attachment_quantities)
            {
                match sample_count {
                    Some(count) => {
                        if count != texture.multisampling_sample_count() {
                            panic!("found multisampling and non-multisampling output render attachments in the same render pass");
                        }
                    }
                    None => {
                        sample_count = Some(texture.multisampling_sample_count());
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
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
        attachments_to_resolve: RenderAttachmentQuantitySet,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'_>>> {
        if self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_clear_or_update() {
            // For pure depth prepasses and shadow map clearing or updates we
            // only work with depths, so we don't need any color attachments
            Vec::new()
        } else {
            let mut color_attachments = Vec::with_capacity(RenderAttachmentQuantity::count());

            let render_attachment_quantities = if self.color_attachments_to_clear.is_empty() {
                output_render_attachment_quantities
            } else {
                self.color_attachments_to_clear
            };

            if !render_attachment_quantities.is_empty() {
                color_attachments.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_textures(render_attachment_quantities)
                        .map(|texture| {
                            let should_resolve =
                                attachments_to_resolve.contains(texture.quantity().flag());

                            let (view, resolve_target) = texture.view_and_resolve_target(
                                self.output_attachment_sampling.is_multi_if_available(),
                                should_resolve,
                            );

                            let load = if self.color_attachments_to_clear.is_empty() {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(texture.quantity().clear_color())
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
                    .render_attachment_texture(RenderAttachmentQuantity::Depth)
                    .multisampled_if_available_and(
                        self.output_attachment_sampling.is_multi_if_available(),
                    )
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
                    .render_attachment_texture(RenderAttachmentQuantity::Depth)
                    .multisampled_if_available_and(
                        self.output_attachment_sampling.is_multi_if_available(),
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

impl ComputePassSpecification {
    /// Obtains the push constant range involved in the compute pass.
    #[allow(clippy::unused_self)]
    fn get_push_constant_range(&self) -> wgpu::PushConstantRange {
        wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::COMPUTE,
            range: 0..0,
        }
    }

    fn get_bind_group_layouts_and_shader_inputs<'a>(
        &self,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
        gpu_computation_library: &GPUComputationLibrary,
    ) -> Result<(Vec<&'a wgpu::BindGroupLayout>, ComputeShaderInput)> {
        let computation_specification =
            Self::get_computation_specification(gpu_computation_library, self.computation_id)?;

        let mut layouts = Vec::with_capacity(4);

        if let Some(resources) = computation_specification.resources() {
            layouts.push(resources.bind_group_layout());

            if !resources.input_render_attachment_quantities().is_empty() {
                layouts.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_texture_bind_group_layouts(
                            resources.input_render_attachment_quantities(),
                        ),
                );
            }
        }

        Ok((layouts, computation_specification.shader_input().clone()))
    }

    fn get_bind_groups<'a>(
        &self,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
        gpu_computation_library: &'a GPUComputationLibrary,
    ) -> Result<Vec<&'a wgpu::BindGroup>> {
        let computation_specification =
            Self::get_computation_specification(gpu_computation_library, self.computation_id)?;

        let mut bind_groups = Vec::with_capacity(4);

        if let Some(resources) = computation_specification.resources() {
            bind_groups.push(resources.bind_group());

            if !resources.input_render_attachment_quantities().is_empty() {
                bind_groups.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_texture_bind_groups(
                            resources.input_render_attachment_quantities(),
                        ),
                );
            }
        }

        Ok(bind_groups)
    }

    fn get_computation_specification(
        gpu_computation_library: &GPUComputationLibrary,
        computation_id: GPUComputationID,
    ) -> Result<&GPUComputationSpecification> {
        gpu_computation_library
            .get_computation_specification(computation_id)
            .ok_or_else(|| {
                anyhow!(
                    "Missing specification for GPU computation {}",
                    computation_id
                )
            })
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
            use_prepass_material: false,
            depth_map_usage: DepthMapUsage::None,
            light: None,
            shadow_map_usage: ShadowMapUsage::None,
            output_attachment_sampling: OutputAttachmentSampling::MultiIfAvailable,
            blending_overrides: HashMap::new(),
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
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        shader_manager: &mut ShaderManager,
        specification: RenderPassSpecification,
        state: RenderCommandState,
    ) -> Result<Self> {
        let (
            pipeline,
            vertex_attribute_requirements,
            input_render_attachment_quantities,
            output_render_attachment_quantities,
        ) = if specification.model_id.is_some()
            || specification.explicit_mesh_id.is_some()
            || specification.explicit_material_id.is_some()
        {
            let (
                bind_group_layouts,
                bind_group_shader_input,
                vertex_attribute_requirements,
                input_render_attachment_quantities,
                output_render_attachment_quantities,
            ) = specification.get_bind_group_layouts_shader_inputs_and_material_data(
                material_library,
                render_resources,
                render_attachment_texture_manager,
            )?;

            let (vertex_buffer_layouts, mesh_shader_input, instance_feature_shader_inputs) =
                specification.get_vertex_buffer_layouts_and_shader_inputs(
                    render_resources,
                    vertex_attribute_requirements,
                )?;

            let push_constant_range = specification.get_push_constant_range();

            let shader = shader_manager.obtain_rendering_shader(
                graphics_device,
                bind_group_shader_input.camera,
                mesh_shader_input,
                bind_group_shader_input.light,
                &instance_feature_shader_inputs,
                bind_group_shader_input.material,
                vertex_attribute_requirements,
                input_render_attachment_quantities,
                output_render_attachment_quantities,
            )?;

            let pipeline_layout = Self::create_pipeline_layout(
                graphics_device.device(),
                &bind_group_layouts,
                &[push_constant_range],
                &format!("{} render pipeline layout", &specification.label),
            );

            let color_target_states = specification.determine_color_target_states(
                rendering_surface,
                render_attachment_texture_manager,
                output_render_attachment_quantities,
            );

            let front_face = specification.determine_front_face();

            let depth_stencil_state = specification.determine_depth_stencil_state();

            let multisampling_sample_count = specification.determine_multisampling_sample_count(
                render_attachment_texture_manager,
                output_render_attachment_quantities,
            );

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

            (
                pipeline,
                vertex_attribute_requirements,
                input_render_attachment_quantities,
                output_render_attachment_quantities,
            )
        } else {
            // If we don't have vertices and a material we don't need a pipeline
            (
                None,
                VertexAttributeSet::empty(),
                RenderAttachmentQuantitySet::empty(),
                RenderAttachmentQuantitySet::empty(),
            )
        };

        Ok(Self {
            specification,
            vertex_attribute_requirements,
            input_render_attachment_quantities,
            output_render_attachment_quantities,
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
            vertex_attribute_requirements: VertexAttributeSet::empty(),
            input_render_attachment_quantities: RenderAttachmentQuantitySet::empty(),
            output_render_attachment_quantities: RenderAttachmentQuantitySet::empty(),
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
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<RenderCommandOutcome> {
        if self.state().is_disabled() {
            log::debug!("Skipping render pass: {}", &self.specification.label);
            return Ok(RenderCommandOutcome::Skipped);
        }

        log::debug!("Recording render pass: {}", &self.specification.label);

        // Make sure all data is available before doing anything else

        let (bind_groups, output_render_attachment_quantities) =
            self.specification.get_bind_groups_and_material_data(
                material_library,
                render_resources,
                render_attachment_texture_manager,
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
            output_render_attachment_quantities,
            self.attachments_to_resolve,
        );

        let depth_stencil_attachment = self
            .specification
            .create_depth_stencil_attachment(render_resources, render_attachment_texture_manager);

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

            self.set_push_constants(&mut render_pass, rendering_surface, render_resources);

            for (index, &bind_group) in bind_groups.iter().enumerate() {
                render_pass.set_bind_group(u32::try_from(index).unwrap(), bind_group, &[]);
            }

            let mut vertex_buffer_slot = 0;

            for vertex_buffer in mesh_buffer_manager
                .request_vertex_render_buffers_including_position(
                    self.vertex_attribute_requirements,
                )?
            {
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
                            .vertex_render_buffer()
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
                                    .vertex_render_buffer()
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
                mesh_buffer_manager
                    .index_render_buffer()
                    .valid_buffer_slice(),
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
    ) {
        let mut push_constant_offset = 0;

        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            push_constant_offset,
            bytemuck::bytes_of(&rendering_surface.get_inverse_window_dimensions_push_constant()),
        );
        push_constant_offset += RenderingSurface::INVERSE_WINDOW_DIMENSIONS_PUSH_CONSTANT_SIZE;

        // Write the exposure value to the appropriate push constant range
        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            push_constant_offset,
            bytemuck::bytes_of(
                &render_resources
                    .postprocessing_resource_manager()
                    .exposure(),
            ),
        );
        push_constant_offset += PostprocessingResourceManager::EXPOSURE_PUSH_CONSTANT_SIZE;

        if let Some(LightInfo {
            light_type,
            light_id,
        }) = self.specification.light
        {
            // Write the index of the light to use for this pass into the
            // appropriate push constant range
            render_pass.set_push_constants(
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                push_constant_offset,
                bytemuck::bytes_of(
                    &render_resources
                        .get_light_buffer_manager()
                        .unwrap()
                        .get_light_idx_push_constant(light_type, light_id),
                ),
            );
            push_constant_offset += LightRenderBufferManager::LIGHT_IDX_PUSH_CONSTANT_SIZE;
        }

        #[allow(unused_assignments)]
        if let ShadowMapUsage::Update(ShadowMapIdentifier::ForUnidirectionalLight(cascade_idx)) =
            self.specification.shadow_map_usage
        {
            // Write the index of the cascade to use for this pass into the
            // appropriate push constant range
            render_pass.set_push_constants(
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                push_constant_offset,
                bytemuck::bytes_of(&cascade_idx),
            );
            push_constant_offset += LightRenderBufferManager::CASCADE_IDX_PUSH_CONSTANT_SIZE;
        }
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

impl ComputePassRecorder {
    /// Creates a new recorder for the compute pass defined by the given
    /// specification.
    ///
    /// Shader inputs extracted from the specification are used to build or
    /// fetch the appropriate shader.
    pub fn new(
        _config: &RenderingConfig,
        graphics_device: &GraphicsDevice,
        _rendering_surface: &RenderingSurface,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_computation_library: &GPUComputationLibrary,
        shader_manager: &mut ShaderManager,
        specification: ComputePassSpecification,
        state: RenderCommandState,
    ) -> Result<Self> {
        let (bind_group_layouts, shader_input) = specification
            .get_bind_group_layouts_and_shader_inputs(
                render_attachment_texture_manager,
                gpu_computation_library,
            )?;

        let push_constant_range = specification.get_push_constant_range();

        let shader = shader_manager.obtain_compute_shader(graphics_device, &shader_input)?;

        let pipeline_layout = Self::create_pipeline_layout(
            graphics_device.device(),
            &bind_group_layouts,
            &[push_constant_range],
            &format!("{} compute pipeline layout", &specification.label),
        );

        let pipeline = Self::create_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &format!("{} compute pipeline", &specification.label),
        );

        Ok(Self {
            specification,
            pipeline,
            state,
        })
    }

    /// Records the compute pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the resources used in this compute pass are
    /// no longer available.
    pub fn record_pass(
        &self,
        rendering_surface: &RenderingSurface,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_computation_library: &GPUComputationLibrary,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<RenderCommandOutcome> {
        if self.state().is_disabled() {
            log::debug!("Skipping compute pass: {}", &self.specification.label);
            return Ok(RenderCommandOutcome::Skipped);
        }

        log::debug!("Recording compute pass: {}", &self.specification.label);

        let bind_groups = self
            .specification
            .get_bind_groups(render_attachment_texture_manager, gpu_computation_library)?;

        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            timestamp_writes: None,
            label: Some(&self.specification.label),
        });

        compute_pass.set_pipeline(&self.pipeline);

        self.set_push_constants(
            &mut compute_pass,
            rendering_surface,
            gpu_computation_library,
        );

        for (index, &bind_group) in bind_groups.iter().enumerate() {
            compute_pass.set_bind_group(u32::try_from(index).unwrap(), bind_group, &[]);
        }

        let (x, y, z) = self.specification.workgroups;
        compute_pass.dispatch_workgroups(x, y, z);

        Ok(RenderCommandOutcome::Recorded)
    }

    /// Returns the state of the compute pass.
    pub fn state(&self) -> RenderCommandState {
        self.state
    }

    /// Sets the state of the compute pass.
    pub fn set_state(&mut self, state: RenderCommandState) {
        self.state = state;
    }

    /// Set whether the compute pass should be skipped.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.state = RenderCommandState::disabled_if(disabled);
    }

    #[allow(clippy::unused_self)]
    fn set_push_constants(
        &self,
        _render_pass: &mut wgpu::ComputePass<'_>,
        _rendering_surface: &RenderingSurface,
        _gpu_computation_library: &GPUComputationLibrary,
    ) {
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
        label: &str,
    ) -> wgpu::ComputePipeline {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: Some(layout),
            module: shader.compute_module(),
            entry_point: shader.compute_entry_point_name().unwrap(),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            label: Some(label),
        })
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
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_computation_library: &GPUComputationLibrary,
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
                shader_manager,
                specification,
                state,
            ),
            RenderCommandSpecification::ComputePass(specification) => Self::new_compute_pass(
                config,
                graphics_device,
                rendering_surface,
                render_attachment_texture_manager,
                gpu_computation_library,
                shader_manager,
                specification,
                state,
            ),
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
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
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
        config: &RenderingConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_computation_library: &GPUComputationLibrary,
        shader_manager: &mut ShaderManager,
        specification: ComputePassSpecification,
        state: RenderCommandState,
    ) -> Result<Self> {
        Ok(Self::ComputePass(ComputePassRecorder::new(
            config,
            graphics_device,
            rendering_surface,
            render_attachment_texture_manager,
            gpu_computation_library,
            shader_manager,
            specification,
            state,
        )?))
    }

    pub fn as_render_pass_mut(&mut self) -> Option<&mut RenderPassRecorder> {
        match self {
            Self::RenderPass(recorder) => Some(recorder),
            Self::ComputePass(_) => None,
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
        gpu_computation_library: &GPUComputationLibrary,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<RenderCommandOutcome> {
        match self {
            Self::RenderPass(recorder) => recorder.record_pass(
                rendering_surface,
                surface_texture_view,
                material_library,
                render_resources,
                render_attachment_texture_manager,
                command_encoder,
            ),
            Self::ComputePass(recorder) => recorder.record_pass(
                rendering_surface,
                render_attachment_texture_manager,
                gpu_computation_library,
                command_encoder,
            ),
        }
    }

    /// Returns the state of the command.
    pub fn state(&self) -> RenderCommandState {
        match self {
            Self::RenderPass(recorder) => recorder.state(),
            Self::ComputePass(recorder) => recorder.state(),
        }
    }

    /// Sets the state of the command.
    pub fn set_state(&mut self, state: RenderCommandState) {
        match self {
            Self::RenderPass(recorder) => recorder.set_state(state),
            Self::ComputePass(recorder) => recorder.set_state(state),
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

impl OutputAttachmentSampling {
    fn is_single(&self) -> bool {
        *self == Self::Single
    }

    fn is_multi_if_available(&self) -> bool {
        *self == Self::MultiIfAvailable
    }
}
