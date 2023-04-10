//! Rendering pipelines.

mod tasks;

pub use tasks::SyncRenderPasses;

use crate::{
    geometry::{CubemapFace, VertexAttributeSet},
    rendering::{
        camera::CameraRenderBufferManager, instance::InstanceFeatureRenderBufferManager,
        light::LightRenderBufferManager, mesh::MeshRenderBufferManager,
        resource::SynchronizedRenderResources, texture::SHADOW_MAP_FORMAT, CameraShaderInput,
        CascadeIdx, CoreRenderingSystem, DepthTexture, InstanceFeatureShaderInput,
        LightShaderInput, MaterialPropertyTextureManager, MaterialRenderResourceManager,
        MaterialShaderInput, MeshShaderInput, RenderingConfig, Shader,
    },
    scene::{
        GlobalAmbientColorMaterial, LightID, LightType, MaterialID, MaterialPropertyTextureSetID,
        MeshID, ModelID, ShaderManager, MAX_SHADOW_MAP_CASCADES,
    },
};
use anyhow::{anyhow, Result};
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
    /// Passes for shading each model that depends on light sources with the
    /// global ambient color contribution. This also does the job of filling the
    /// remainder of the depth map.
    light_shaded_model_global_ambient_color_shading_passes: Vec<RenderPassRecorder>,
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
    /// Whether and how the depth map will be used.
    depth_map_usage: DepthMapUsage,
    /// Light source whose contribution is computed in this pass.
    light: Option<LightInfo>,
    /// Whether and how the shadow map will be used.
    shadow_map_usage: ShadowMapUsage,
    /// ID of the material to use for shading the model, or [`None`] if the pass
    /// should use the material associated with the model. The override material
    /// is assumed not to have textured material properties.
    override_material: Option<MaterialID>,
    label: String,
}

/// Recorder for a specific render pass.
#[derive(Debug)]
pub struct RenderPassRecorder {
    specification: RenderPassSpecification,
    vertex_attribute_requirements: VertexAttributeSet,
    pipeline: Option<wgpu::RenderPipeline>,
    color_load_operation: wgpu::LoadOp<wgpu::Color>,
    depth_operations: wgpu::Operations<f32>,
    disabled: bool,
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
            light_shaded_model_global_ambient_color_shading_passes: Vec::new(),
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
            .chain(
                self.light_shaded_model_global_ambient_color_shading_passes
                    .iter(),
            )
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
        self.light_shaded_model_global_ambient_color_shading_passes
            .clear();
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
        shader_manager: &mut ShaderManager,
    ) -> Result<()> {
        let light_buffer_manager = render_resources.get_light_buffer_manager();

        let omnidirectional_light_ids = light_buffer_manager
            .map_or_else(|| &[], LightRenderBufferManager::omnidirectional_light_ids);
        let unidirectional_light_ids = light_buffer_manager
            .map_or_else(|| &[], LightRenderBufferManager::unidirectional_light_ids);

        // Remove shading passes for lights that are no longer present
        self.light_shaded_model_shading_passes
            .retain(|light_id, _| {
                omnidirectional_light_ids.contains(light_id)
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

            self.light_shaded_model_global_ambient_color_shading_passes
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

            let material_requires_lights = render_resources
                .get_material_resource_manager(model_id.material_id())
                .expect("Missing resource manager for material after synchronization")
                .shader_input()
                .requires_lights();

            if material_requires_lights {
                match self.light_shaded_model_index_mapper.try_push_key(model_id) {
                    // The model has no existing shading passes
                    Ok(_) => {
                        // Create a global ambient color shading pass for the new model
                        self.light_shaded_model_global_ambient_color_shading_passes
                            .push(RenderPassRecorder::new(
                                core_system,
                                config,
                                render_resources,
                                shader_manager,
                                RenderPassSpecification::global_ambient_color_shading_pass(
                                    model_id,
                                ),
                                no_visible_instances,
                            )?);

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
                                    shader_manager,
                                    RenderPassSpecification::shadow_map_update_pass(
                                        light,
                                        model_id,
                                        ShadowMapIdentifier::ForOmnidirectionalLight(face),
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
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_with_shadow_map(
                                    light, model_id,
                                ),
                                no_visible_instances,
                            )?);
                        }

                        for &light_id in unidirectional_light_ids {
                            let cascades_have_shadow_casting_model_instances: Vec<_> = (0
                                ..MAX_SHADOW_MAP_CASCADES)
                                .into_iter()
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
                                        shader_manager,
                                        RenderPassSpecification::shadow_map_update_pass(
                                            light,
                                            model_id,
                                            ShadowMapIdentifier::ForUnidirectionalLight(
                                                cascade_idx,
                                            ),
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
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_with_shadow_map(
                                    light, model_id,
                                ),
                                no_visible_instances,
                            )?);
                        }
                    }
                    // The model already has shading passes
                    Err(model_idx) => {
                        // Set the disabled state of the passes for the existing model

                        self.light_shaded_model_global_ambient_color_shading_passes[model_idx]
                            .set_disabled(no_visible_instances);

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
                                shader_manager,
                                RenderPassSpecification::depth_prepass(model_id),
                                no_visible_instances,
                            )?);

                        // Create a shading pass for the new model
                        self.non_light_shaded_model_shading_passes
                            .push(RenderPassRecorder::new(
                                core_system,
                                config,
                                render_resources,
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_without_shadow_map(
                                    None, model_id,
                                ),
                                no_visible_instances,
                            )?);
                    }
                    // The model already has shading passes
                    Err(model_idx) => {
                        // Set the disabled state of the passes for the existing model

                        self.non_light_shaded_model_depth_prepasses[model_idx]
                            .set_disabled(no_visible_instances);

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
    pub fn surface_clearing_pass(clear_color: wgpu::Color) -> Self {
        Self {
            clear_color: Some(clear_color),
            model_id: None,
            depth_map_usage: DepthMapUsage::Clear,
            light: None,
            shadow_map_usage: ShadowMapUsage::None,
            override_material: None,
            label: "Surface clearing pass".to_string(),
        }
    }

    /// Creates the specification for the render pass that will update the depth
    /// map with the depths of the model with the given ID.
    pub fn depth_prepass(model_id: ModelID) -> Self {
        Self {
            clear_color: None,
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::Prepass,
            light: None,
            shadow_map_usage: ShadowMapUsage::None,
            override_material: None,
            label: format!("Depth prepass for model {}", model_id),
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID with the global ambient color.
    pub fn global_ambient_color_shading_pass(model_id: ModelID) -> Self {
        Self {
            clear_color: None,
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::use_readwrite(),
            light: None,
            shadow_map_usage: ShadowMapUsage::None,
            override_material: Some(GlobalAmbientColorMaterial::material_id()),
            label: format!("Global ambient color shading of model {}", model_id),
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID without making use of a shadow map.
    fn model_shading_pass_without_shadow_map(light: Option<LightInfo>, model_id: ModelID) -> Self {
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
            depth_map_usage: DepthMapUsage::use_readonly(),
            light,
            shadow_map_usage: ShadowMapUsage::None,
            override_material: None,
            label,
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID making use of a shadow map.
    fn model_shading_pass_with_shadow_map(light: LightInfo, model_id: ModelID) -> Self {
        Self {
            clear_color: None,
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::use_readonly(),
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Use,
            override_material: None,
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
            depth_map_usage: DepthMapUsage::None,
            light: None,
            shadow_map_usage: ShadowMapUsage::Clear(shadow_map_id),
            override_material: None,
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
    ) -> Self {
        Self {
            clear_color: None,
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::None,
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Update(shadow_map_id),
            override_material: None,
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
        depth_map_usage: DepthMapUsage,
        shadow_map_usage: ShadowMapUsage,
        override_material: Option<MaterialID>,
    ) -> Result<impl Iterator<Item = &InstanceFeatureRenderBufferManager>> {
        render_resources
            .get_instance_feature_buffer_managers(model_id)
            .map(|buffers| {
                if depth_map_usage.is_prepass()
                    || shadow_map_usage.is_update()
                    || override_material.is_some()
                {
                    // For depth prepass or shadow map update we only need
                    // transforms, and we do not support other instance features
                    // for override materials
                    &buffers[..1]
                } else {
                    &buffers[..]
                }
                .iter()
            })
            .ok_or_else(|| anyhow!("Missing instance render buffer for model {}", model_id))
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
    /// 2. Instance feature buffers.
    fn get_vertex_buffer_layouts_and_shader_inputs<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
        vertex_attribute_requirements: VertexAttributeSet,
    ) -> Result<(
        Vec<wgpu::VertexBufferLayout<'static>>,
        Option<&'a MeshShaderInput>,
        Vec<&'a InstanceFeatureShaderInput>,
    )> {
        let mut layouts = Vec::with_capacity(2);
        let mut mesh_shader_input = None;
        let mut instance_feature_shader_inputs = Vec::with_capacity(1);

        if let Some(model_id) = self.model_id {
            let mesh_buffer_manager =
                Self::get_mesh_buffer_manager(render_resources, model_id.mesh_id())?;

            layouts.extend(
                mesh_buffer_manager.request_vertex_buffer_layouts_including_position(
                    vertex_attribute_requirements,
                )?,
            );
            mesh_shader_input = Some(mesh_buffer_manager.shader_input());

            if self.depth_map_usage.is_prepass()
                || self.shadow_map_usage.is_update()
                || self.override_material.is_some()
            {
                // For depth prepass or shadow map update we only need
                // transforms, and we do not support other instance features for
                // override materials
                if let Some(buffer) =
                    render_resources.get_instance_transform_buffer_manager(model_id)
                {
                    layouts.push(buffer.vertex_buffer_layout().clone());
                    instance_feature_shader_inputs.push(buffer.shader_input());
                }
            } else if let Some(buffers) =
                render_resources.get_instance_feature_buffer_managers(model_id)
            {
                for buffer in buffers {
                    layouts.push(buffer.vertex_buffer_layout().clone());
                    instance_feature_shader_inputs.push(buffer.shader_input());
                }
            }
        }

        Ok((layouts, mesh_shader_input, instance_feature_shader_inputs))
    }

    /// Obtains the bind group layouts for any camera, material or lights
    /// involved in the render pass, as well as the associated shader
    /// inputs and the vertex attribute requirements of the material.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Shadow map textures.
    /// 4. Fixed material resources.
    /// 5. Material property textures.
    fn get_bind_group_layouts_and_shader_inputs<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Result<(
        Vec<&'a wgpu::BindGroupLayout>,
        BindGroupShaderInput<'a>,
        VertexAttributeSet,
    )> {
        let mut layouts = Vec::with_capacity(5);

        let mut shader_input = BindGroupShaderInput {
            camera: None,
            light: None,
            material: None,
        };
        let mut vertex_attribute_requirements = VertexAttributeSet::empty();

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
                    light_buffer_manager.shadow_map_bind_group_layout_for_light_type(light_type),
                );
            }

            shader_input.light = Some(light_buffer_manager.shader_input_for_light_type(light_type));
        }

        if let Some(model_id) = self.model_id {
            // We do not need a material if we are doing a depth prepass or
            // updating shadow map
            if !(self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_update()) {
                let material_id = self
                    .override_material
                    .unwrap_or_else(|| model_id.material_id());

                let material_resource_manager =
                    Self::get_material_resource_manager(render_resources, material_id)?;

                shader_input.material = Some(material_resource_manager.shader_input());

                vertex_attribute_requirements =
                    material_resource_manager.vertex_attribute_requirements();

                if let Some(fixed_resources) = material_resource_manager.fixed_resources() {
                    layouts.push(fixed_resources.bind_group_layout());
                }

                // We do not support textured material properties for override materials
                if self.override_material.is_none() {
                    if let Some(texture_set_id) = model_id.material_property_texture_set_id() {
                        let material_property_texture_manager =
                            Self::get_material_property_texture_manager(
                                render_resources,
                                texture_set_id,
                            )?;

                        layouts.push(material_property_texture_manager.bind_group_layout());
                    }
                }
            }
        }

        Ok((layouts, shader_input, vertex_attribute_requirements))
    }

    /// Obtains all bind groups involved in the render pass.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Shadow map textures.
    /// 4. Fixed material resources.
    /// 5. Material property textures.
    fn get_bind_groups<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Result<Vec<&'a wgpu::BindGroup>> {
        let mut bind_groups = Vec::with_capacity(4);

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
                bind_groups
                    .push(light_buffer_manager.shadow_map_bind_group_for_light_type(light_type));
            }
        }

        if let Some(model_id) = self.model_id {
            // We do not need a material if we are doing a depth prepass or
            // updating shadow map
            if !(self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_update()) {
                let material_id = self
                    .override_material
                    .unwrap_or_else(|| model_id.material_id());

                let material_resource_manager =
                    Self::get_material_resource_manager(render_resources, material_id)?;

                if let Some(fixed_resources) = material_resource_manager.fixed_resources() {
                    bind_groups.push(fixed_resources.bind_group());
                }

                // We do not support textured material properties for override materials
                if self.override_material.is_none() {
                    if let Some(texture_set_id) = model_id.material_property_texture_set_id() {
                        let material_property_texture_manager =
                            Self::get_material_property_texture_manager(
                                render_resources,
                                texture_set_id,
                            )?;

                        bind_groups.push(material_property_texture_manager.bind_group());
                    }
                }
            }
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

    fn determine_blend_state(&self) -> wgpu::BlendState {
        if self.light.is_some() {
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

    fn determine_color_target_state(
        &self,
        core_system: &CoreRenderingSystem,
    ) -> Option<wgpu::ColorTargetState> {
        if self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_clear_or_update() {
            // For depth prepasses and shadow map clearing or updates we only
            // work with depths, so we don't need a color target
            None
        } else {
            Some(wgpu::ColorTargetState {
                format: core_system.surface_config().format,
                // Since we determine contributions from each light in
                // separate render passes, we need to add up the color
                // contributions. We simply ignore alpha.
                blend: Some(self.determine_blend_state()),
                write_mask: wgpu::ColorWrites::COLOR,
            })
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
                format: DepthTexture::FORMAT,
                depth_write_enabled,
                depth_compare,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        }
    }

    fn determine_multisampling_sample_count(&self, config: &RenderingConfig) -> u32 {
        if self.shadow_map_usage.is_clear_or_update() {
            1
        } else {
            config.multisampling_sample_count
        }
    }

    fn determine_color_load_operation(&self) -> wgpu::LoadOp<wgpu::Color> {
        match self.clear_color {
            Some(clear_color) => wgpu::LoadOp::Clear(clear_color),
            None => wgpu::LoadOp::Load,
        }
    }

    fn determine_depth_operations(&self) -> wgpu::Operations<f32> {
        if self.depth_map_usage.is_clear() || self.shadow_map_usage.is_clear() {
            wgpu::Operations {
                load: wgpu::LoadOp::Clear(Self::CLEAR_DEPTH),
                store: true,
            }
        } else {
            wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            }
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
        shader_manager: &mut ShaderManager,
        specification: RenderPassSpecification,
        disabled: bool,
    ) -> Result<Self> {
        let (pipeline, vertex_attribute_requirements) = if specification.model_id.is_some() {
            let (bind_group_layouts, bind_group_shader_input, vertex_attribute_requirements) =
                specification.get_bind_group_layouts_and_shader_inputs(render_resources)?;

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
            )?;

            let pipeline_layout = Self::create_render_pipeline_layout(
                core_system.device(),
                &bind_group_layouts,
                &[push_constant_range],
                &format!("{} render pipeline layout", &specification.label),
            );

            let color_target_state = specification.determine_color_target_state(core_system);

            let front_face = specification.determine_front_face();

            let depth_stencil_state = specification.determine_depth_stencil_state();

            let multisampling_sample_count =
                specification.determine_multisampling_sample_count(config);

            let pipeline = Some(Self::create_render_pipeline(
                core_system.device(),
                &pipeline_layout,
                shader,
                &vertex_buffer_layouts,
                color_target_state,
                front_face,
                depth_stencil_state,
                multisampling_sample_count,
                config,
                &format!("{} render pipeline", &specification.label),
            ));

            (pipeline, vertex_attribute_requirements)
        } else {
            // If we don't have vertices and a material we don't need a pipeline
            (None, VertexAttributeSet::empty())
        };

        let color_load_operation = specification.determine_color_load_operation();
        let depth_operations = specification.determine_depth_operations();

        Ok(Self {
            specification,
            vertex_attribute_requirements,
            pipeline,
            color_load_operation,
            depth_operations,
            disabled,
        })
    }

    pub fn surface_clearing_pass(clear_color: wgpu::Color) -> Self {
        let specification = RenderPassSpecification::surface_clearing_pass(clear_color);
        let color_load_operation = specification.determine_color_load_operation();
        let depth_operations = specification.determine_depth_operations();
        Self {
            specification,
            vertex_attribute_requirements: VertexAttributeSet::empty(),
            pipeline: None,
            color_load_operation,
            depth_operations,
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
        render_resources: &SynchronizedRenderResources,
        color_attachment_texture_view: &wgpu::TextureView,
        color_attachment_resolve_target: Option<&wgpu::TextureView>,
        depth_texture_view: &wgpu::TextureView,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.disabled() {
            log::debug!("Skipping render pass: {}", &self.specification.label);
            return Ok(());
        }

        log::debug!("Recording render pass: {}", &self.specification.label);

        // Make sure all data is available before doing anything else

        let bind_groups = self.specification.get_bind_groups(render_resources)?;

        let (mesh_buffer_manager, feature_buffer_managers) = match self.specification.model_id {
            Some(model_id) => (
                Some(RenderPassSpecification::get_mesh_buffer_manager(
                    render_resources,
                    model_id.mesh_id(),
                )?),
                Some(
                    RenderPassSpecification::get_instance_feature_buffer_managers(
                        render_resources,
                        model_id,
                        self.specification.depth_map_usage,
                        self.specification.shadow_map_usage,
                        self.specification.override_material,
                    )?,
                ),
            ),
            _ => (None, None),
        };

        let color_attachment = if self.specification.depth_map_usage.is_prepass()
            || self.specification.shadow_map_usage.is_clear_or_update()
        {
            // For depth prepasses and shadow map clearing or updates we only
            // work with depths, so we don't need a color target
            None
        } else {
            Some(wgpu::RenderPassColorAttachment {
                view: color_attachment_texture_view,
                resolve_target: color_attachment_resolve_target,
                ops: wgpu::Operations {
                    load: self.color_load_operation,
                    store: true,
                },
            })
        };

        let depth_stencil_attachment = if self.specification.shadow_map_usage.is_clear_or_update() {
            // For modifying the shadow map we have to set it as the depth
            // map for the pipeline
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: self
                    .specification
                    .get_shadow_map_texture_view(render_resources)
                    .unwrap(),
                depth_ops: Some(self.depth_operations),
                stencil_ops: None,
            })
        } else if !self.specification.depth_map_usage.is_none() {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_texture_view,
                depth_ops: Some(self.depth_operations),
                stencil_ops: None,
            })
        } else {
            None
        };

        let has_color_attachements = color_attachment.is_some();
        let color_attachments = &[color_attachment];

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            // A `@location(i)` directive in the fragment shader output targets color attachment `i` here
            color_attachments: if has_color_attachements {
                color_attachments
            } else {
                &[]
            },
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

            let instance_range = if let Some(mut feature_buffer_managers) = feature_buffer_managers
            {
                let transform_buffer_manager = feature_buffer_managers.next().unwrap();

                render_pass.set_vertex_buffer(
                    vertex_buffer_slot,
                    transform_buffer_manager
                        .vertex_render_buffer()
                        .valid_buffer_slice(),
                );
                vertex_buffer_slot += 1;

                if let ShadowMapUsage::Update(shadow_map_id) = self.specification.shadow_map_usage {
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
                } else if self.specification.depth_map_usage.is_prepass() {
                    // When doing a depth prepass we use the model view
                    // transforms, which are in the initial range of the buffer,
                    // but we don't include any other instance features
                    transform_buffer_manager.initial_feature_range()
                } else {
                    // When doing a shading pass we use the model view
                    // transforms and also include any other instance features
                    for feature_buffer_manager in feature_buffer_managers {
                        render_pass.set_vertex_buffer(
                            vertex_buffer_slot,
                            feature_buffer_manager
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
        color_target_state: Option<wgpu::ColorTargetState>,
        front_face: wgpu::FrontFace,
        depth_stencil_state: Option<wgpu::DepthStencilState>,
        multisampling_sample_count: u32,
        config: &RenderingConfig,
        label: &str,
    ) -> wgpu::RenderPipeline {
        let has_fragment_state_targets = color_target_state.is_some();
        let fragment_state_targets = &[color_target_state];

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
                    targets: if has_fragment_state_targets {
                        fragment_state_targets
                    } else {
                        &[]
                    },
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
