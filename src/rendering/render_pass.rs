//! Rendering pipelines.

mod tasks;

pub use tasks::SyncRenderPasses;

use crate::{
    geometry::{CubemapFace, VertexAttributeSet},
    rendering::{
        camera::CameraRenderBufferManager, instance::InstanceFeatureRenderBufferManager,
        light::LightRenderBufferManager, mesh::MeshRenderBufferManager,
        resource::SynchronizedRenderResources, texture::SHADOW_MAP_FORMAT, CameraShaderInput,
        CascadeIdx, CoreRenderingSystem, InstanceFeatureShaderInput, LightShaderInput,
        MaterialPropertyTextureManager, MaterialRenderResourceManager, MaterialShaderInput,
        MeshShaderInput, RenderAttachmentQuantity, RenderAttachmentQuantitySet,
        RenderAttachmentTextureManager, RenderingConfig, Shader, RENDER_ATTACHMENT_FLAGS,
        RENDER_ATTACHMENT_FORMATS,
    },
    scene::{
        LightID, LightType, MaterialID, MaterialPropertyTextureSetID, MeshID, ModelID,
        ShaderManager, MAX_SHADOW_MAP_CASCADES,
    },
};
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use impact_utils::KeyIndexMapper;
use std::{
    collections::{hash_map::Entry, HashMap},
    iter,
};

/// Manager and owner of render passes.
#[derive(Debug)]
pub struct RenderPassManager {
    /// Pass for clearing the rendering surface and depth map.
    clearing_pass_recorder: RenderPassRecorder,
    /// Passes for filling the depth map with the depths of the models that do
    /// not depend on light sources.
    non_light_shaded_model_depth_prepasses: Vec<RenderPassRecorder>,
    /// Passes for shading each model that depends on light sources with their
    /// prepass material. This also does the job of filling the remainder of the
    /// depth map.
    light_shaded_model_shading_prepasses: Vec<RenderPassRecorder>,
    /// Passes for shading models that do not depend on light sources.
    non_light_shaded_model_shading_passes: Vec<RenderPassRecorder>,
    /// Passes for shading models that depend on light sources, including passes
    /// for clearing and filling the shadow map.
    light_shaded_model_shading_passes: HashMap<LightID, LightShadedModelShadingPasses>,
    non_light_shaded_model_index_mapper: KeyIndexMapper<ModelID>,
    light_shaded_model_index_mapper: KeyIndexMapper<ModelID>,
}

/// Holds the information describing a specific render pass.
#[derive(Clone, Debug)]
pub struct RenderPassSpecification {
    /// Color that will be written to the rendering surface when clearing it.
    clear_color: Option<wgpu::Color>,
    /// ID of the model type to render, or [`None`] if the pass does not render
    /// a model (e.g. a clearing pass).
    model_id: Option<ModelID>,
    /// If present, use this mesh rather than a mesh associated with a model.
    explicit_mesh_id: Option<MeshID>,
    /// If present, use this material rather than a material associated with a
    /// model.
    explicit_material_id: Option<MaterialID>,
    /// Whether to use the prepass material associated with the model's material
    /// rather than using the model's material.
    use_prepass_material: bool,
    /// Whether and how the depth map will be used.
    depth_map_usage: DepthMapUsage,
    /// Light source whose contribution is computed in this pass.
    light: Option<LightInfo>,
    /// Whether and how the shadow map will be used.
    shadow_map_usage: ShadowMapUsage,
    hints: RenderPassHints,
    label: String,
}

/// Recorder for a specific render pass.
#[derive(Debug)]
pub struct RenderPassRecorder {
    specification: RenderPassSpecification,
    vertex_attribute_requirements: VertexAttributeSet,
    pipeline: Option<wgpu::RenderPipeline>,
    disabled: bool,
}

bitflags! {
    /// Bitflag encoding a set of hints for configuring a render pass.
    pub struct RenderPassHints: u8 {
        /// The appearance of the rendered material is affected by light
        /// sources.
        const AFFECTED_BY_LIGHT = 0b00000001;
        /// No depth prepass should be performed for the model.
        const NO_DEPTH_PREPASS = 0b00000010;
        /// The render pass renders to the surface color attachment.
        const RENDERS_TO_SURFACE = 0b00000100;
    }
}

#[derive(Debug, Default)]
struct LightShadedModelShadingPasses {
    /// Passes for clearing the shadow maps to the maximum depth.
    shadow_map_clearing_passes: Vec<RenderPassRecorder>,
    /// Passes for writing the depths of each model from the light's point of
    /// view to the shadow map.
    shadow_map_update_passes: Vec<Vec<RenderPassRecorder>>,
    /// Passes for shading each model with contributions from the light.
    shading_passes: Vec<RenderPassRecorder>,
}

#[derive(Copy, Clone, Debug)]
struct LightInfo {
    light_type: LightType,
    light_id: LightID,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DepthMapUsage {
    /// No depth map is used.
    None,
    /// Clear the depth map with the maximum depth (1.0).
    Clear,
    /// Fill the depth map with model depths without doing shading.
    Prepass,
    /// Use the depth map for depth testing when shading. The [`WriteDepths`]
    /// value decides whether depths will be written to the depth map during the
    /// pass.
    Use(WriteDepths),
}

type WriteDepths = bool;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ShadowMapUsage {
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
enum ShadowMapIdentifier {
    ForUnidirectionalLight(CascadeIdx),
    ForOmnidirectionalLight(CubemapFace),
}

#[derive(Debug)]
struct BindGroupShaderInput<'a> {
    camera: Option<&'a CameraShaderInput>,
    light: Option<&'a LightShaderInput>,
    material: Option<&'a MaterialShaderInput>,
}

impl RenderPassManager {
    /// Creates a new manager with a pass that clears the surface with the given
    /// color.
    pub fn new(clear_color: wgpu::Color) -> Self {
        Self {
            clearing_pass_recorder: RenderPassRecorder::surface_clearing_pass(clear_color),
            non_light_shaded_model_depth_prepasses: Vec::new(),
            light_shaded_model_shading_prepasses: Vec::new(),
            non_light_shaded_model_shading_passes: Vec::new(),
            light_shaded_model_shading_passes: HashMap::new(),
            non_light_shaded_model_index_mapper: KeyIndexMapper::new(),
            light_shaded_model_index_mapper: KeyIndexMapper::new(),
        }
    }

    /// Returns an iterator over all render passes in the appropriate order.
    pub fn recorders(&self) -> impl Iterator<Item = &RenderPassRecorder> {
        iter::once(&self.clearing_pass_recorder)
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
    }

    /// Deletes all the render passes except for the initial clearing pass.
    pub fn clear_model_render_pass_recorders(&mut self) {
        self.non_light_shaded_model_depth_prepasses.clear();
        self.light_shaded_model_shading_prepasses.clear();
        self.non_light_shaded_model_shading_passes.clear();
        self.light_shaded_model_shading_passes.clear();
        self.non_light_shaded_model_index_mapper.clear();
        self.light_shaded_model_index_mapper.clear();
    }

    /// Ensures that all render passes required for rendering the entities
    /// present in the given render resources are available and configured
    /// correctly.
    ///
    /// Render passes whose entities are no longer present in the resources will
    /// be removed, and missing render passes for new entities will be created.
    fn sync_with_render_resources(
        &mut self,
        core_system: &CoreRenderingSystem,
        config: &RenderingConfig,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        shader_manager: &mut ShaderManager,
    ) -> Result<()> {
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

            let hints = render_resources
                .get_material_resource_manager(model_id.material_handle().material_id())
                .expect("Missing resource manager for material after synchronization")
                .render_pass_hints();

            if hints.contains(RenderPassHints::AFFECTED_BY_LIGHT) {
                match self.light_shaded_model_index_mapper.try_push_key(model_id) {
                    // The model has no existing shading passes
                    Ok(_) => {
                        if let Some(prepass_material_handle) = model_id.prepass_material_handle() {
                            let prepass_hints = render_resources
                                .get_material_resource_manager(
                                    prepass_material_handle.material_id(),
                                )
                                .expect("Missing resource manager for prepass material")
                                .render_pass_hints();

                            if ambient_light_ids.is_empty() {
                                self.light_shaded_model_shading_prepasses.push(
                                    RenderPassRecorder::new(
                                        core_system,
                                        config,
                                        render_resources,
                                        render_attachment_texture_manager,
                                        shader_manager,
                                        RenderPassSpecification::shading_prepass(
                                            None,
                                            model_id,
                                            prepass_hints,
                                        ),
                                        no_visible_instances,
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
                                        RenderPassRecorder::new(
                                            core_system,
                                            config,
                                            render_resources,
                                            render_attachment_texture_manager,
                                            shader_manager,
                                            RenderPassSpecification::shading_prepass(
                                                Some(light),
                                                model_id,
                                                prepass_hints,
                                            ),
                                            no_visible_instances,
                                        )?,
                                    );
                                }
                            }
                        } else {
                            // If the new model has no prepass material, we
                            // create a pure depth prepass
                            self.light_shaded_model_shading_prepasses.push(
                                RenderPassRecorder::new(
                                    core_system,
                                    config,
                                    render_resources,
                                    render_attachment_texture_manager,
                                    shader_manager,
                                    RenderPassSpecification::depth_prepass(model_id, hints),
                                    no_visible_instances
                                        || hints.contains(RenderPassHints::NO_DEPTH_PREPASS),
                                )?,
                            );
                        }

                        for &light_id in omnidirectional_light_ids {
                            let faces_have_shadow_casting_model_instances: Vec<_> =
                                CubemapFace::all()
                                    .into_iter()
                                    .map(|face| {
                                        !transform_buffer_manager
                                            .feature_range(
                                                light_id.as_instance_feature_buffer_range_id()
                                                    + face.as_idx_u32(),
                                            )
                                            .is_empty()
                                    })
                                    .collect();

                            let passes = match self
                                .light_shaded_model_shading_passes
                                .entry(light_id)
                            {
                                Entry::Occupied(entry) => entry.into_mut(),
                                Entry::Vacant(entry) => {
                                    let mut shadow_map_clearing_passes = Vec::with_capacity(6);

                                    for face in CubemapFace::all() {
                                        shadow_map_clearing_passes.push(RenderPassRecorder::new(
                                            core_system,
                                            config,
                                            render_resources,
                                            render_attachment_texture_manager,
                                            shader_manager,
                                            RenderPassSpecification::shadow_map_clearing_pass(
                                                ShadowMapIdentifier::ForOmnidirectionalLight(face),
                                            ),
                                            false,
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
                                shadow_map_update_passes_for_faces.push(RenderPassRecorder::new(
                                    core_system,
                                    config,
                                    render_resources,
                                    render_attachment_texture_manager,
                                    shader_manager,
                                    RenderPassSpecification::shadow_map_update_pass(
                                        light,
                                        model_id,
                                        ShadowMapIdentifier::ForOmnidirectionalLight(face),
                                        hints,
                                    ),
                                    !faces_have_shadow_casting_model_instances[face.as_idx_usize()],
                                )?);
                            }

                            // Create an omnidirectional light shading pass for
                            // the new model
                            passes.shading_passes.push(RenderPassRecorder::new(
                                core_system,
                                config,
                                render_resources,
                                render_attachment_texture_manager,
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_with_shadow_map(
                                    light, model_id, hints,
                                ),
                                no_visible_instances,
                            )?);
                        }

                        for &light_id in unidirectional_light_ids {
                            let cascades_have_shadow_casting_model_instances: Vec<_> = (0
                                ..MAX_SHADOW_MAP_CASCADES)
                                .map(|cascade_idx| {
                                    !transform_buffer_manager
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
                                                .push(RenderPassRecorder::new(
                                                core_system,
                                                config,
                                                render_resources,
                                                render_attachment_texture_manager,
                                                shader_manager,
                                                RenderPassSpecification::shadow_map_clearing_pass(
                                                    ShadowMapIdentifier::ForUnidirectionalLight(
                                                        cascade_idx,
                                                    ),
                                                ),
                                                false,
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
                                    RenderPassRecorder::new(
                                        core_system,
                                        config,
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
                                        !cascades_have_shadow_casting_model_instances
                                            [cascade_idx as usize],
                                    )?,
                                );
                            }

                            // Create a unidirectional light shading pass for
                            // the new model
                            passes.shading_passes.push(RenderPassRecorder::new(
                                core_system,
                                config,
                                render_resources,
                                render_attachment_texture_manager,
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_with_shadow_map(
                                    light, model_id, hints,
                                ),
                                no_visible_instances,
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
                                    for recorder in recorders {
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

                                        let no_shadow_casting_instances = transform_buffer_manager
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
                        self.non_light_shaded_model_depth_prepasses
                            .push(RenderPassRecorder::new(
                                core_system,
                                config,
                                render_resources,
                                render_attachment_texture_manager,
                                shader_manager,
                                RenderPassSpecification::depth_prepass(model_id, hints),
                                no_visible_instances
                                    || hints.contains(RenderPassHints::NO_DEPTH_PREPASS),
                            )?);

                        // Create a shading pass for the new model
                        self.non_light_shaded_model_shading_passes
                            .push(RenderPassRecorder::new(
                                core_system,
                                config,
                                render_resources,
                                render_attachment_texture_manager,
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_without_shadow_map(
                                    None, model_id, hints,
                                ),
                                no_visible_instances,
                            )?);
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

        Ok(())
    }
}

impl RenderPassSpecification {
    /// Maximum z-value in clip space.
    const CLEAR_DEPTH: f32 = 1.0;

    /// Creates the specification for the render pass that will clear the
    /// rendering surface with the given color and clear the depth map.
    fn surface_clearing_pass(clear_color: wgpu::Color) -> Self {
        Self {
            clear_color: Some(clear_color),
            model_id: None,
            explicit_mesh_id: None,
            explicit_material_id: None,
            use_prepass_material: false,
            depth_map_usage: DepthMapUsage::Clear,
            light: None,
            shadow_map_usage: ShadowMapUsage::None,
            hints: RenderPassHints::empty(),
            label: "Surface clearing pass".to_string(),
        }
    }

    /// Creates the specification for the render pass that will update the depth
    /// map with the depths of the model with the given ID.
    fn depth_prepass(model_id: ModelID, hints: RenderPassHints) -> Self {
        Self {
            clear_color: None,
            model_id: Some(model_id),
            explicit_mesh_id: None,
            explicit_material_id: None,
            use_prepass_material: false,
            depth_map_usage: DepthMapUsage::Prepass,
            light: None,
            shadow_map_usage: ShadowMapUsage::None,
            hints,
            label: format!("Depth prepass for model {}", model_id),
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
            clear_color: None,
            model_id: Some(model_id),
            explicit_mesh_id: None,
            explicit_material_id: None,
            use_prepass_material: true,
            depth_map_usage: DepthMapUsage::use_readwrite(),
            light,
            shadow_map_usage: ShadowMapUsage::None,
            hints,
            label,
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
            clear_color: None,
            model_id: Some(model_id),
            explicit_mesh_id: None,
            explicit_material_id: None,
            use_prepass_material: false,
            depth_map_usage: DepthMapUsage::use_readonly(),
            light,
            shadow_map_usage: ShadowMapUsage::None,
            hints,
            label,
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
            clear_color: None,
            model_id: Some(model_id),
            explicit_mesh_id: None,
            explicit_material_id: None,
            use_prepass_material: false,
            depth_map_usage: DepthMapUsage::use_readonly(),
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Use,
            hints,
            label: format!(
                "Shading of model {} for light {} ({:?}) with shadow map",
                model_id, light.light_id, light.light_type
            ),
        }
    }

    /// Creates the specification for the render pass that will clear the given
    /// shadow map.
    fn shadow_map_clearing_pass(shadow_map_id: ShadowMapIdentifier) -> Self {
        Self {
            clear_color: None,
            model_id: None,
            explicit_mesh_id: None,
            explicit_material_id: None,
            use_prepass_material: false,
            depth_map_usage: DepthMapUsage::None,
            light: None,
            shadow_map_usage: ShadowMapUsage::Clear(shadow_map_id),
            hints: RenderPassHints::empty(),
            label: format!("Shadow map clearing pass ({:?})", shadow_map_id),
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
            clear_color: None,
            model_id: Some(model_id),
            explicit_mesh_id: None,
            explicit_material_id: None,
            use_prepass_material: false,
            depth_map_usage: DepthMapUsage::None,
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Update(shadow_map_id),
            hints,
            label: format!(
                "Shadow map update for model {} and light {} ({:?})",
                model_id, light.light_id, shadow_map_id
            ),
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
                // When using a prepass material we have to check both whether
                // the main material has a buffer (which would be placed
                // directly after the transform buffer) and whether the prepass
                // material has a buffer (which would be placed directly after
                // the main material buffer) to determine the existance and
                // location of the prepass material buffer

                let prepass_material_handle = model_id
                    .prepass_material_handle()
                    .ok_or_else(|| anyhow!("Missing prepass material for model {}", model_id))?;

                if prepass_material_handle
                    .material_property_feature_id()
                    .is_some()
                {
                    if model_id
                        .material_handle()
                        .material_property_feature_id()
                        .is_some()
                    {
                        Some(&buffers[2])
                    } else {
                        Some(&buffers[1])
                    }
                } else {
                    None
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

    fn get_material_resource_manager(
        render_resources: &SynchronizedRenderResources,
        material_id: MaterialID,
    ) -> Result<&MaterialRenderResourceManager> {
        render_resources
            .get_material_resource_manager(material_id)
            .ok_or_else(|| anyhow!("Missing resource manager for material {}", material_id))
    }

    fn get_material_property_texture_manager(
        render_resources: &SynchronizedRenderResources,
        texture_set_id: MaterialPropertyTextureSetID,
    ) -> Result<&MaterialPropertyTextureManager> {
        render_resources
            .get_material_property_texture_manager(texture_set_id)
            .ok_or_else(|| {
                anyhow!(
                    "Missing manager for material property texture set {}",
                    texture_set_id
                )
            })
    }

    /// Obtains the push constant range involved in the render pass.
    fn get_push_constant_range(&self) -> wgpu::PushConstantRange {
        let mut size = CoreRenderingSystem::INVERSE_WINDOW_DIMENSIONS_PUSH_CONSTANT_SIZE;

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
        let mut layouts = Vec::with_capacity(6);
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
    /// 4. Fixed material resources.
    /// 5. Render attachment textures.
    /// 6. Material property textures.
    fn get_bind_group_layouts_shader_inputs_and_material_data<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
    ) -> Result<(
        Vec<&'a wgpu::BindGroupLayout>,
        BindGroupShaderInput<'a>,
        VertexAttributeSet,
        RenderAttachmentQuantitySet,
        RenderAttachmentQuantitySet,
    )> {
        let mut layouts = Vec::with_capacity(5);

        let mut shader_input = BindGroupShaderInput {
            camera: None,
            light: None,
            material: None,
        };

        let mut vertex_attribute_requirements = VertexAttributeSet::empty();

        let mut input_render_attachment_quantities = RenderAttachmentQuantitySet::empty();
        let mut output_render_attachment_quantities = RenderAttachmentQuantitySet::empty();

        // We do not need a camera if we are updating shadow map
        if !self.shadow_map_usage.is_update() {
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
            let material_resource_manager =
                Self::get_material_resource_manager(render_resources, material_id)?;

            if let Some(fixed_resources) = material_resource_manager.fixed_resources() {
                layouts.push(fixed_resources.bind_group_layout());
            }

            input_render_attachment_quantities =
                material_resource_manager.input_render_attachment_quantities();

            output_render_attachment_quantities =
                material_resource_manager.output_render_attachment_quantities();

            if !input_render_attachment_quantities.is_empty() {
                layouts.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_texture_bind_group_layouts(
                            input_render_attachment_quantities,
                        )?,
                );
            }

            shader_input.material = Some(material_resource_manager.shader_input());

            vertex_attribute_requirements =
                material_resource_manager.vertex_attribute_requirements_for_shader();
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

                let material_resource_manager = Self::get_material_resource_manager(
                    render_resources,
                    material_handle.material_id(),
                )?;

                if let Some(fixed_resources) = material_resource_manager.fixed_resources() {
                    layouts.push(fixed_resources.bind_group_layout());
                }

                input_render_attachment_quantities =
                    material_resource_manager.input_render_attachment_quantities();

                output_render_attachment_quantities =
                    material_resource_manager.output_render_attachment_quantities();

                if !input_render_attachment_quantities.is_empty() {
                    layouts.extend(
                        render_attachment_texture_manager
                            .request_render_attachment_texture_bind_group_layouts(
                                input_render_attachment_quantities,
                            )?,
                    );
                }

                shader_input.material = Some(material_resource_manager.shader_input());

                vertex_attribute_requirements =
                    material_resource_manager.vertex_attribute_requirements_for_shader();

                if let Some(texture_set_id) = material_handle.material_property_texture_set_id() {
                    let material_property_texture_manager =
                        Self::get_material_property_texture_manager(
                            render_resources,
                            texture_set_id,
                        )?;

                    layouts.push(material_property_texture_manager.bind_group_layout());
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
    /// 4. Fixed material resources.
    /// 5. Render attachment textures.
    /// 6. Material property textures.
    fn get_bind_groups_and_material_data<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
    ) -> Result<(Vec<&'a wgpu::BindGroup>, RenderAttachmentQuantitySet)> {
        let mut bind_groups = Vec::with_capacity(4);

        let mut output_render_attachment_quantities = RenderAttachmentQuantitySet::empty();

        // We do not need a camera if we are updating shadow map
        if !self.shadow_map_usage.is_update() {
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
            let material_resource_manager =
                Self::get_material_resource_manager(render_resources, material_id)?;

            if let Some(fixed_resources) = material_resource_manager.fixed_resources() {
                bind_groups.push(fixed_resources.bind_group());
            }

            let input_render_attachment_quantities =
                material_resource_manager.input_render_attachment_quantities();

            output_render_attachment_quantities =
                material_resource_manager.output_render_attachment_quantities();

            if !input_render_attachment_quantities.is_empty() {
                bind_groups.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_texture_bind_groups(
                            input_render_attachment_quantities,
                        )?,
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

                let material_resource_manager = Self::get_material_resource_manager(
                    render_resources,
                    material_handle.material_id(),
                )?;

                if let Some(fixed_resources) = material_resource_manager.fixed_resources() {
                    bind_groups.push(fixed_resources.bind_group());
                }

                let input_render_attachment_quantities =
                    material_resource_manager.input_render_attachment_quantities();

                output_render_attachment_quantities =
                    material_resource_manager.output_render_attachment_quantities();

                if !input_render_attachment_quantities.is_empty() {
                    bind_groups.extend(
                        render_attachment_texture_manager
                            .request_render_attachment_texture_bind_groups(
                                input_render_attachment_quantities,
                            )?,
                    );
                }

                if let Some(texture_set_id) = material_handle.material_property_texture_set_id() {
                    let material_property_texture_manager =
                        Self::get_material_property_texture_manager(
                            render_resources,
                            texture_set_id,
                        )?;

                    bind_groups.push(material_property_texture_manager.bind_group());
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

    fn determine_color_blend_state(&self) -> wgpu::BlendState {
        if self.light.is_some() && !self.use_prepass_material {
            // Since we determine contributions from each light in
            // separate render passes, we need to add up the color
            // contributions. We simply ignore alpha.
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::default(),
            }
        } else {
            wgpu::BlendState::REPLACE
        }
    }

    fn determine_color_target_states(
        &self,
        core_system: &CoreRenderingSystem,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
    ) -> Vec<Option<wgpu::ColorTargetState>> {
        if self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_clear_or_update() {
            // For pure depth prepasses and shadow map clearing or updates we
            // only work with depths, so we don't need a color target
            Vec::new()
        } else {
            let mut color_target_states = Vec::with_capacity(3);

            if self.hints.contains(RenderPassHints::RENDERS_TO_SURFACE) {
                color_target_states.push(Some(wgpu::ColorTargetState {
                    format: core_system.surface_config().format,
                    blend: Some(self.determine_color_blend_state()),
                    write_mask: wgpu::ColorWrites::COLOR,
                }));
            }

            if !output_render_attachment_quantities.is_empty() {
                color_target_states.extend(
                    RENDER_ATTACHMENT_FLAGS
                        .iter()
                        .zip(RENDER_ATTACHMENT_FORMATS.iter())
                        .filter_map(|(&quantity_flag, &format)| {
                            if output_render_attachment_quantities.contains(quantity_flag) {
                                Some(Some(wgpu::ColorTargetState {
                                    format,
                                    blend: None,
                                    write_mask: wgpu::ColorWrites::COLOR,
                                }))
                            } else {
                                None
                            }
                        }),
                );
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

            let depth_compare = if depth_write_enabled {
                wgpu::CompareFunction::Less
            } else {
                // When we turn off depth writing, all closest depths have
                // been determined. To be able to do subsequent shading, we
                // must allow shading when the depth is equal to the depth
                // in the depth map.
                wgpu::CompareFunction::LessEqual
            };

            Some(wgpu::DepthStencilState {
                format: RENDER_ATTACHMENT_FORMATS[RenderAttachmentQuantity::Depth as usize],
                depth_write_enabled,
                depth_compare,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        }
    }

    fn create_color_attachments<'a, 'b: 'a>(
        &'a self,
        surface_texture_view: &'b wgpu::TextureView,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
    ) -> Result<Vec<Option<wgpu::RenderPassColorAttachment<'_>>>> {
        if self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_clear_or_update() {
            // For pure depth prepasses and shadow map clearing or updates we
            // only work with depths, so we don't need any color attachments
            Ok(Vec::new())
        } else {
            let (surface_load_operations, other_load_operations, render_attachment_quantities) =
                if let Some(clear_color) = self.clear_color {
                    // If we have a clear color, we clear both the surface
                    // texture and any available render attachment textures
                    (
                        wgpu::LoadOp::Clear(clear_color),
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        render_attachment_texture_manager.available_color_quantities(),
                    )
                } else {
                    // Otherwise, we use the surface texture as well as the
                    // textures for all the specified quantities as render
                    // attachments
                    (
                        wgpu::LoadOp::Load,
                        wgpu::LoadOp::Load,
                        output_render_attachment_quantities,
                    )
                };

            let mut color_attachments = Vec::with_capacity(3);

            if self.hints.contains(RenderPassHints::RENDERS_TO_SURFACE) {
                color_attachments.push(Some(wgpu::RenderPassColorAttachment {
                    view: surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: surface_load_operations,
                        store: true,
                    },
                }));
            }

            if !render_attachment_quantities.is_empty() {
                color_attachments.extend(
                    render_attachment_texture_manager
                        .request_render_attachment_texture_views(render_attachment_quantities)?
                        .map(|texture_view| {
                            Some(wgpu::RenderPassColorAttachment {
                                view: texture_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: other_load_operations,
                                    store: true,
                                },
                            })
                        }),
                );
            }

            Ok(color_attachments)
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
                    store: true,
                }),
                stencil_ops: None,
            })
        } else if self.shadow_map_usage.is_update() {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.get_shadow_map_texture_view(render_resources).unwrap(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            })
        } else if self.depth_map_usage.is_clear() {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: render_attachment_texture_manager
                    .render_attachment_texture(RenderAttachmentQuantity::Depth)
                    .view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Self::CLEAR_DEPTH),
                    store: true,
                }),
                stencil_ops: None,
            })
        } else if !self.depth_map_usage.is_none() {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: render_attachment_texture_manager
                    .render_attachment_texture(RenderAttachmentQuantity::Depth)
                    .view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            })
        } else {
            None
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
        core_system: &CoreRenderingSystem,
        config: &RenderingConfig,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        shader_manager: &mut ShaderManager,
        specification: RenderPassSpecification,
        disabled: bool,
    ) -> Result<Self> {
        let (pipeline, vertex_attribute_requirements) = if specification.model_id.is_some()
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
                render_resources,
                render_attachment_texture_manager,
            )?;

            let (vertex_buffer_layouts, mesh_shader_input, instance_feature_shader_inputs) =
                specification.get_vertex_buffer_layouts_and_shader_inputs(
                    render_resources,
                    vertex_attribute_requirements,
                )?;

            let push_constant_range = specification.get_push_constant_range();

            let shader = shader_manager.obtain_shader(
                core_system,
                bind_group_shader_input.camera,
                mesh_shader_input,
                bind_group_shader_input.light,
                &instance_feature_shader_inputs,
                bind_group_shader_input.material,
                vertex_attribute_requirements,
                input_render_attachment_quantities,
                output_render_attachment_quantities,
            )?;

            let pipeline_layout = Self::create_render_pipeline_layout(
                core_system.device(),
                &bind_group_layouts,
                &[push_constant_range],
                &format!("{} render pipeline layout", &specification.label),
            );

            let color_target_states = specification
                .determine_color_target_states(core_system, output_render_attachment_quantities);

            let front_face = specification.determine_front_face();

            let depth_stencil_state = specification.determine_depth_stencil_state();

            let pipeline = Some(Self::create_render_pipeline(
                core_system.device(),
                &pipeline_layout,
                shader,
                &vertex_buffer_layouts,
                &color_target_states,
                front_face,
                depth_stencil_state,
                1,
                config,
                &format!("{} render pipeline", &specification.label),
            ));

            (pipeline, vertex_attribute_requirements)
        } else {
            // If we don't have vertices and a material we don't need a pipeline
            (None, VertexAttributeSet::empty())
        };

        Ok(Self {
            specification,
            vertex_attribute_requirements,
            pipeline,
            disabled,
        })
    }

    pub fn surface_clearing_pass(clear_color: wgpu::Color) -> Self {
        let specification = RenderPassSpecification::surface_clearing_pass(clear_color);
        Self {
            specification,
            vertex_attribute_requirements: VertexAttributeSet::empty(),
            pipeline: None,
            disabled: false,
        }
    }

    /// Records the render pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the render resources
    /// used in this render pass are no longer available.
    pub fn record_render_pass(
        &self,
        core_system: &CoreRenderingSystem,
        render_resources: &SynchronizedRenderResources,
        surface_texture: &wgpu::SurfaceTexture,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.disabled() {
            log::debug!("Skipping render pass: {}", &self.specification.label);
            return Ok(());
        }

        log::debug!("Recording render pass: {}", &self.specification.label);

        // Make sure all data is available before doing anything else

        let (bind_groups, output_render_attachment_quantities) =
            self.specification.get_bind_groups_and_material_data(
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

        let surface_texture_view =
            RenderAttachmentTextureManager::create_surface_texture_view(surface_texture);

        let color_attachments = self.specification.create_color_attachments(
            &surface_texture_view,
            render_attachment_texture_manager,
            output_render_attachment_quantities,
        )?;

        let depth_stencil_attachment = self
            .specification
            .create_depth_stencil_attachment(render_resources, render_attachment_texture_manager);

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            // A `@location(i)` directive in the fragment shader output targets color attachment `i` here
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            label: Some(&self.specification.label),
        });

        if let Some(ref pipeline) = self.pipeline {
            let mesh_buffer_manager = mesh_buffer_manager.expect("Has pipeline but no vertices");

            render_pass.set_pipeline(pipeline);

            let mut push_constant_offset = 0;

            render_pass.set_push_constants(
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                push_constant_offset,
                bytemuck::bytes_of(&core_system.get_inverse_window_dimensions_push_constant()),
            );
            push_constant_offset +=
                CoreRenderingSystem::INVERSE_WINDOW_DIMENSIONS_PUSH_CONSTANT_SIZE;

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
            if let ShadowMapUsage::Update(ShadowMapIdentifier::ForUnidirectionalLight(
                cascade_idx,
            )) = self.specification.shadow_map_usage
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

        Ok(())
    }

    /// Whether the render pass should be skipped.
    pub fn disabled(&self) -> bool {
        self.disabled
    }

    /// Set whether the render pass should be skipped.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    fn create_render_pipeline_layout(
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

    fn create_render_pipeline(
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
                entry_point: shader.vertex_entry_point_name(),
                buffers: vertex_buffer_layouts,
            },
            fragment: shader
                .fragment_entry_point_name()
                .map(|entry_point| wgpu::FragmentState {
                    module: shader.fragment_module(),
                    entry_point,
                    targets: color_target_states,
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

    fn is_clear_or_prepass(&self) -> bool {
        self.is_clear() || self.is_prepass()
    }

    fn make_writeable(&self) -> bool {
        self.is_clear_or_prepass() || self == &Self::Use(true)
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
