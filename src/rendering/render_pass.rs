//! Rendering pipelines.

mod tasks;

pub use tasks::SyncRenderPasses;

use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        camera::CameraRenderBufferManager, instance::InstanceFeatureRenderBufferManager,
        light::LightRenderBufferManager, mesh::MeshRenderBufferManager,
        resource::SynchronizedRenderResources, texture::ShadowMapTexture, CameraShaderInput,
        CoreRenderingSystem, DepthTexture, InstanceFeatureShaderInput, LightShaderInput,
        MaterialPropertyTextureManager, MaterialRenderResourceManager, MaterialShaderInput,
        MeshShaderInput, RenderingConfig, Shader,
    },
    scene::{
        LightID, LightType, MaterialID, MaterialPropertyTextureSetID, MeshID, ModelID,
        ShaderManager,
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
    /// Passes for filling the depth map before any shading is done.
    depth_prepasses: HashMap<ModelID, RenderPassRecorder>,
    /// Passes for shading models that do not depend on light sources.
    non_light_shaded_model_shading_passes: HashMap<ModelID, RenderPassRecorder>,
    /// Passes for shading models that depend on light sources, including passes
    /// for clearing and filling the shadow map.
    light_shaded_model_shading_passes: HashMap<LightID, LightShadedModelShadingPasses>,
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
    /// Pass for clearing the shadow map to the maximum depth.
    shadow_map_clearing_pass: Option<RenderPassRecorder>,
    /// Passes for writing the depths of each model from the light's point of
    /// view to the shadow map.
    shadow_map_update_passes: Vec<RenderPassRecorder>,
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
    /// Use the depth map for depth testing when shading.
    Use,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ShadowMapUsage {
    /// No shadow map is used.
    None,
    /// Clear the shadow map with the maximum depth (1.0).
    Clear,
    /// Fill the shadow map with model depths from the light's point of view.
    Update,
    /// Make the shadow map texture available for sampling in the shader.
    Use,
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
            depth_prepasses: HashMap::new(),
            non_light_shaded_model_shading_passes: HashMap::new(),
            light_shaded_model_shading_passes: HashMap::new(),
            light_shaded_model_index_mapper: KeyIndexMapper::new(),
        }
    }

    /// Returns an iterator over all render passes in the appropriate order.
    pub fn recorders(&self) -> impl Iterator<Item = &RenderPassRecorder> {
        iter::once(&self.clearing_pass_recorder)
            .chain(self.depth_prepasses.values())
            .chain(self.non_light_shaded_model_shading_passes.values())
            .chain(
                self.light_shaded_model_shading_passes
                    .values()
                    .flat_map(|passes| {
                        passes
                            .shadow_map_clearing_pass
                            .iter()
                            .chain(passes.shadow_map_update_passes.iter())
                            .chain(passes.shading_passes.iter())
                    }),
            )
    }

    /// Deletes all the render passes except for the initial clearing pass.
    pub fn clear_model_render_pass_recorders(&mut self) {
        self.depth_prepasses.clear();
        self.non_light_shaded_model_shading_passes.clear();
        self.light_shaded_model_shading_passes.clear();
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

        let point_light_ids =
            light_buffer_manager.map_or_else(|| &[], LightRenderBufferManager::point_light_ids);
        let directional_light_ids = light_buffer_manager
            .map_or_else(|| &[], LightRenderBufferManager::directional_light_ids);

        // Remove shading passes for lights that are no longer present
        self.light_shaded_model_shading_passes
            .retain(|light_id, _| {
                point_light_ids.contains(light_id) || directional_light_ids.contains(light_id)
            });

        let feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        // Remove depth prepasses for models that are no longer present
        self.depth_prepasses
            .retain(|model_id, _| feature_buffer_managers.contains_key(model_id));

        // Remove shading passes for non light shaded models that are no longer present
        self.non_light_shaded_model_shading_passes
            .retain(|model_id, _| feature_buffer_managers.contains_key(model_id));

        // Remove shading passes for light shaded models that are no longer present
        let removed_light_shaded_model_ids: Vec<_> = self
            .light_shaded_model_index_mapper
            .key_at_each_idx()
            .filter(|model_id| !feature_buffer_managers.contains_key(model_id))
            .collect();

        for model_id in removed_light_shaded_model_ids {
            let model_idx = self
                .light_shaded_model_index_mapper
                .swap_remove_key(model_id);
            self.light_shaded_model_shading_passes
                .values_mut()
                .for_each(|passes| {
                    if !passes.shadow_map_update_passes.is_empty() {
                        passes.shadow_map_update_passes.swap_remove(model_idx);
                    }
                    passes.shading_passes.swap_remove(model_idx);
                });
        }

        for (&model_id, feature_buffer_manager) in feature_buffer_managers {
            // Avoid rendering the model if there are currently no instances
            let disable_pass = feature_buffer_manager
                .first()
                .unwrap()
                .initial_feature_range()
                .is_empty();

            // Create depth prepasses for new models and update disabled state
            // of existing ones
            match self.depth_prepasses.entry(model_id) {
                Entry::Vacant(entry) => {
                    entry.insert(RenderPassRecorder::new(
                        core_system,
                        config,
                        render_resources,
                        shader_manager,
                        RenderPassSpecification::depth_prepass(model_id),
                        disable_pass,
                    )?);
                }
                Entry::Occupied(mut entry) => {
                    let recorder = entry.get_mut();
                    recorder.set_disabled(disable_pass);
                }
            }

            let material_requires_lights = render_resources
                .get_material_resource_manager(model_id.material_id())
                .expect("Missing resource manager for material after synchronization")
                .shader_input()
                .requires_lights();

            if material_requires_lights {
                match self.light_shaded_model_index_mapper.try_push_key(model_id) {
                    // The model has no existing shading passes
                    Ok(_) => {
                        for &light_id in point_light_ids {
                            let passes = self
                                .light_shaded_model_shading_passes
                                .entry(light_id)
                                .or_default();

                            // Create a point light shading pass for the new model
                            passes.shading_passes.push(RenderPassRecorder::new(
                                core_system,
                                config,
                                render_resources,
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_without_shadow_map(
                                    Some(LightInfo {
                                        light_type: LightType::PointLight,
                                        light_id,
                                    }),
                                    model_id,
                                ),
                                disable_pass,
                            )?);
                        }
                        for &light_id in directional_light_ids {
                            let passes = match self
                                .light_shaded_model_shading_passes
                                .entry(light_id)
                            {
                                Entry::Occupied(entry) => entry.into_mut(),
                                Entry::Vacant(entry) => {
                                    entry.insert(LightShadedModelShadingPasses {
                                        shadow_map_clearing_pass: Some(RenderPassRecorder::new(
                                            core_system,
                                            config,
                                            render_resources,
                                            shader_manager,
                                            RenderPassSpecification::shadow_map_clearing_pass(),
                                            disable_pass,
                                        )?),
                                        ..Default::default()
                                    })
                                }
                            };

                            let light = LightInfo {
                                light_type: LightType::DirectionalLight,
                                light_id,
                            };

                            // Create a directional light shadow map update pass
                            // for the new model
                            passes
                                .shadow_map_update_passes
                                .push(RenderPassRecorder::new(
                                    core_system,
                                    config,
                                    render_resources,
                                    shader_manager,
                                    RenderPassSpecification::shadow_map_update_pass(
                                        light, model_id,
                                    ),
                                    disable_pass,
                                )?);

                            // Create a directional light shading pass for the
                            // new model
                            passes.shading_passes.push(RenderPassRecorder::new(
                                core_system,
                                config,
                                render_resources,
                                shader_manager,
                                RenderPassSpecification::model_shading_pass_with_shadow_map(
                                    light, model_id,
                                ),
                                disable_pass,
                            )?);
                        }
                    }
                    // The model already has shading passes
                    Err(model_idx) => {
                        // Set the disabled state of the passes for the existing model
                        self.light_shaded_model_shading_passes
                            .values_mut()
                            .for_each(|passes| {
                                if let Some(recorder) =
                                    passes.shadow_map_update_passes.get_mut(model_idx)
                                {
                                    recorder.set_disabled(disable_pass);
                                }
                                passes.shading_passes[model_idx].set_disabled(disable_pass);
                            });
                    }
                }
            } else {
                // Create a shading pass for the model if it is new, or update
                // the disabled state of its shading pass if it already exists
                match self.non_light_shaded_model_shading_passes.entry(model_id) {
                    Entry::Vacant(entry) => {
                        entry.insert(RenderPassRecorder::new(
                            core_system,
                            config,
                            render_resources,
                            shader_manager,
                            RenderPassSpecification::model_shading_pass_without_shadow_map(
                                None, model_id,
                            ),
                            disable_pass,
                        )?);
                    }
                    Entry::Occupied(mut entry) => {
                        let recorder = entry.get_mut();
                        recorder.set_disabled(disable_pass);
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
            label: "Clearing pass".to_string(),
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
            label: "Depth prepass".to_string(),
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID without making use of a shadow map.
    fn model_shading_pass_without_shadow_map(light: Option<LightInfo>, model_id: ModelID) -> Self {
        Self {
            clear_color: None,
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::Use,
            light,
            shadow_map_usage: ShadowMapUsage::None,
            label: format!("Shading of model {} without shadow map", model_id),
        }
    }

    /// Creates the specification for the render pass that will render the model
    /// with the given ID making use of a shadow map.
    fn model_shading_pass_with_shadow_map(light: LightInfo, model_id: ModelID) -> Self {
        Self {
            clear_color: None,
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::Use,
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Use,
            label: format!("Shading of model {} with shadow map", model_id),
        }
    }

    /// Creates the specification for the render pass that will clear the
    /// shadow map.
    fn shadow_map_clearing_pass() -> Self {
        Self {
            clear_color: None,
            model_id: None,
            depth_map_usage: DepthMapUsage::None,
            light: None,
            shadow_map_usage: ShadowMapUsage::Clear,
            label: "Shadow map clearing pass".to_string(),
        }
    }

    /// Creates the specification for the render pass that will update a shadow
    /// map with the depths of the model with the given ID from the point of
    /// view of the given light.
    fn shadow_map_update_pass(light: LightInfo, model_id: ModelID) -> Self {
        Self {
            clear_color: None,
            model_id: Some(model_id),
            depth_map_usage: DepthMapUsage::None,
            light: Some(light),
            shadow_map_usage: ShadowMapUsage::Update,
            label: format!("Shadow map update for model {}", model_id),
        }
    }

    /// Obtains the push constant ranges involved in the render pass.
    fn get_push_constant_ranges(&self) -> Vec<wgpu::PushConstantRange> {
        let mut push_constant_ranges = Vec::with_capacity(1);

        if self.light.is_some() {
            push_constant_ranges.push(LightRenderBufferManager::light_idx_push_constant_range());
        }

        push_constant_ranges
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

            if self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_update() {
                // For depth prepass or shadow map update we only need transforms
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
    /// 4. Material property textures.
    fn get_bind_group_layouts_and_shader_inputs<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Result<(
        Vec<&'a wgpu::BindGroupLayout>,
        BindGroupShaderInput<'a>,
        VertexAttributeSet,
    )> {
        let mut layouts = Vec::with_capacity(4);

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
                if let Some(layout) =
                    light_buffer_manager.shadow_map_bind_group_layout_for_light_type(light_type)
                {
                    layouts.push(layout);
                }
            }

            shader_input.light = Some(light_buffer_manager.shader_input_for_light_type(light_type));
        }

        if let Some(model_id) = self.model_id {
            // We do not need a material if we are doing a depth prepass or
            // updating shadow map
            if !(self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_update()) {
                let material_resource_manager =
                    Self::get_material_resource_manager(render_resources, model_id.material_id())?;

                shader_input.material = Some(material_resource_manager.shader_input());

                vertex_attribute_requirements =
                    material_resource_manager.vertex_attribute_requirements();

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

        Ok((layouts, shader_input, vertex_attribute_requirements))
    }

    /// Obtains all bind groups involved in the render pass.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Shadow map textures.
    /// 4. Material textures.
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
                if let Some(bind_group) =
                    light_buffer_manager.shadow_map_bind_group_for_light_type(light_type)
                {
                    bind_groups.push(bind_group);
                }
            }
        }

        if let Some(model_id) = self.model_id {
            // We do not need a material if we are doing a depth prepass or
            // updating shadow map
            if !(self.depth_map_usage.is_prepass() || self.shadow_map_usage.is_update()) {
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

        Ok(bind_groups)
    }

    /// Obtains the shadow map texture involved in the render pass.
    fn get_shadow_map_texture<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Option<&'a ShadowMapTexture> {
        if !self.shadow_map_usage.is_none() {
            Some(
                render_resources
                    .get_light_buffer_manager()
                    .expect("Missing light render buffer manager for shadow mapping render pass")
                    .directional_light_shadow_map_texture(),
            )
        } else {
            None
        }
    }

    fn determine_color_load_operation(&self) -> wgpu::LoadOp<wgpu::Color> {
        match self.clear_color {
            Some(clear_color) => wgpu::LoadOp::Clear(clear_color),
            None => wgpu::LoadOp::Load,
        }
    }

    fn determine_depth_loperations(&self) -> wgpu::Operations<f32> {
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
    ) -> Result<impl Iterator<Item = &InstanceFeatureRenderBufferManager>> {
        render_resources
            .get_instance_feature_buffer_managers(model_id)
            .map(|buffers| {
                if depth_map_usage.is_prepass() || shadow_map_usage.is_update() {
                    // For depth prepass or shadow map update we only need transforms
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

            let push_constant_ranges = specification.get_push_constant_ranges();

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
                &push_constant_ranges,
                &format!("{} render pipeline layout", &specification.label),
            );

            let target_color_state = if specification.depth_map_usage.is_prepass()
                || specification.shadow_map_usage.is_clear_or_update()
            {
                // For depth prepasses and shadow map clearing or updates we only
                // work with depths, so we don't need a color target
                None
            } else {
                Some(wgpu::ColorTargetState {
                    format: core_system.surface_config().format,
                    // Since we determine contributions from each light in
                    // separate render passes, we need to add up the color
                    // contributions. We simply ignore alpha.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::default(),
                    }),
                    write_mask: wgpu::ColorWrites::COLOR,
                })
            };

            let depth_stencil = if specification.shadow_map_usage.is_clear_or_update() {
                // For modifying the shadow map we have to set it as the depth
                // map for the pipeline
                Some(wgpu::DepthStencilState {
                    format: ShadowMapTexture::FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 8,
                        slope_scale: 1.5,
                        clamp: 0.0,
                    },
                })
            } else if !specification.depth_map_usage.is_none() {
                Some(wgpu::DepthStencilState {
                    format: DepthTexture::FORMAT,
                    // No need to write depths during shading passes since we
                    // are doing depth prepasses
                    depth_write_enabled: specification.depth_map_usage.is_clear_or_prepass(),
                    // Since we determine all depths in advance before doing any
                    // shading, we must allow shading when the depth is equal to
                    // the depth in the depth map (which will be the case for
                    // every shaded fragment)
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                })
            } else {
                None
            };

            let pipeline = Some(Self::create_render_pipeline(
                core_system.device(),
                &pipeline_layout,
                shader,
                &vertex_buffer_layouts,
                target_color_state,
                depth_stencil,
                config,
                &format!("{} render pipeline", &specification.label),
            ));

            (pipeline, vertex_attribute_requirements)
        } else {
            // If we don't have vertices and a material we don't need a pipeline
            (None, VertexAttributeSet::empty())
        };

        let color_load_operation = specification.determine_color_load_operation();
        let depth_operations = specification.determine_depth_loperations();

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
        let depth_operations = specification.determine_depth_loperations();
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
        depth_texture_view: &wgpu::TextureView,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.disabled() {
            return Ok(());
        }

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
                resolve_target: None,
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
                    .get_shadow_map_texture(render_resources)
                    .unwrap()
                    .view(),
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

            if let Some(LightInfo {
                light_type,
                light_id,
            }) = self.specification.light
            {
                // Write the index of the light to use for this pass into the
                // appropriate push constant range
                render_resources
                    .get_light_buffer_manager()
                    .unwrap()
                    .set_light_idx_push_constant(&mut render_pass, light_type, light_id);
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

                if self.specification.shadow_map_usage.is_update() {
                    // When updating the shadow map, we don't use model view
                    // transforms but rather the model to light space tranforms
                    // that have been written to the range dedicated for the
                    // active light in the transform buffer
                    let buffer_range_id = self
                        .specification
                        .light
                        .unwrap()
                        .light_id
                        .as_instance_feature_buffer_range_id();

                    transform_buffer_manager.feature_range(buffer_range_id)
                } else if self.specification.depth_map_usage.is_prepass() {
                    // When doing a depth prepass we use the mode view
                    // transforms, which are in the initial range of the buffer,
                    // but we don't include any other instance features
                    transform_buffer_manager.initial_feature_range()
                } else {
                    // When doing a shading pass  we use the mode view
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
        target_color_state: Option<wgpu::ColorTargetState>,
        depth_stencil: Option<wgpu::DepthStencilState>,
        config: &RenderingConfig,
        label: &str,
    ) -> wgpu::RenderPipeline {
        let has_fragment_state_targets = target_color_state.is_some();
        let fragment_state_targets = &[target_color_state];

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
                // Because we flip the x-axis of the projection matrix in order
                // to avoid a projected view that is mirrored compared to world
                // coordinates, the face orientations become reversed in
                // framebuffer space compared to world space. We therefore
                // define front faces as having clockwise winding order in
                // framebuffer space, which corresponds to anti-clockwise
                // winding order in world space.
                front_face: wgpu::FrontFace::Cw,
                cull_mode: config.cull_mode,
                polygon_mode: config.polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            label: Some(label),
        })
    }
}

impl DepthMapUsage {
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
}

impl ShadowMapUsage {
    fn is_none(&self) -> bool {
        *self == Self::None
    }

    fn is_clear(&self) -> bool {
        *self == Self::Clear
    }

    fn is_update(&self) -> bool {
        *self == Self::Update
    }

    fn is_use(&self) -> bool {
        *self == Self::Use
    }

    fn is_clear_or_update(&self) -> bool {
        self.is_clear() || self.is_update()
    }
}
