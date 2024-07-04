//! Management of shaders.

use crate::{
    geometry::VertexAttributeSet,
    gpu::{
        rendering::{
            CameraShaderInput, ComputeShaderGenerator, ComputeShaderInput,
            InstanceFeatureShaderInput, LightShaderInput, MaterialShaderInput, MeshShaderInput,
            RenderAttachmentQuantitySet, RenderShaderGenerator, Shader,
        },
        GraphicsDevice,
    },
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use std::{
    collections::{
        hash_map::{DefaultHasher, Entry},
        HashMap,
    },
    hash::{Hash, Hasher},
};

/// Identifier for specific shaders.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ShaderID(u64);

#[derive(Debug)]
pub struct ShaderManager {
    /// Rendering shader programs.
    pub rendering_shaders: HashMap<ShaderID, Shader>,
    /// Compute shader programs.
    pub compute_shaders: HashMap<ShaderID, Shader>,
}

impl ShaderManager {
    /// Creates a new empty shader library.
    pub fn new() -> Self {
        Self {
            rendering_shaders: HashMap::new(),
            compute_shaders: HashMap::new(),
        }
    }

    /// Obtains the appropriate rendering [`Shader`] for the given set of shader
    /// inputs.
    ///
    /// If a shader for the given inputs already exists, it is returned,
    /// otherwise a new shader is generated, compiled and cached.
    ///
    /// # Errors
    /// See [`ShaderGenerator::generate_rendering_shader_module`].
    pub fn obtain_rendering_shader(
        &mut self,
        graphics_device: &GraphicsDevice,
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        light_shader_input: Option<&LightShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_shader_input: Option<&MaterialShaderInput>,
        vertex_attribute_requirements: VertexAttributeSet,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
    ) -> Result<&Shader> {
        let shader_id = ShaderID::from_rendering_input(
            camera_shader_input,
            mesh_shader_input,
            light_shader_input,
            instance_feature_shader_inputs,
            material_shader_input,
            vertex_attribute_requirements,
            input_render_attachment_quantities,
            output_render_attachment_quantities,
        );

        match self.rendering_shaders.entry(shader_id) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let (module, entry_point_names) = RenderShaderGenerator::generate_shader_module(
                    camera_shader_input,
                    mesh_shader_input,
                    light_shader_input,
                    instance_feature_shader_inputs,
                    material_shader_input,
                    vertex_attribute_requirements,
                    input_render_attachment_quantities,
                    output_render_attachment_quantities,
                )?;
                Ok(entry.insert(Shader::from_naga_module(
                    graphics_device,
                    module,
                    entry_point_names,
                    format!("Generated rendering shader (hash {})", shader_id.0).as_str(),
                )))
            }
        }
    }

    /// Obtains the appropriate compute [`Shader`] for the given set of shader
    /// inputs.
    ///
    /// If a shader for the given inputs already exists, it is returned,
    /// otherwise a new shader is generated, compiled and cached.
    ///
    /// # Errors
    /// See [`ShaderGenerator::generate_shader_module`].
    pub fn obtain_compute_shader(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_input: &ComputeShaderInput,
    ) -> Result<&Shader> {
        let shader_id = ShaderID::from_compute_input(shader_input);

        match self.compute_shaders.entry(shader_id) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let (module, entry_point_names) =
                    ComputeShaderGenerator::generate_shader_module(shader_input)?;
                Ok(entry.insert(Shader::from_naga_module(
                    graphics_device,
                    module,
                    entry_point_names,
                    format!("Generated compute shader (hash {})", shader_id.0).as_str(),
                )))
            }
        }
    }
}

impl Default for ShaderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderID {
    fn from_rendering_input(
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        light_shader_input: Option<&LightShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_shader_input: Option<&MaterialShaderInput>,
        vertex_attribute_requirements: VertexAttributeSet,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
    ) -> Self {
        let mut hasher = DefaultHasher::new();
        "rendering".hash(&mut hasher);
        camera_shader_input.hash(&mut hasher);
        mesh_shader_input.hash(&mut hasher);
        light_shader_input.hash(&mut hasher);
        instance_feature_shader_inputs.hash(&mut hasher);
        material_shader_input.hash(&mut hasher);
        vertex_attribute_requirements.hash(&mut hasher);
        input_render_attachment_quantities.hash(&mut hasher);
        output_render_attachment_quantities.hash(&mut hasher);
        Self(hasher.finish())
    }

    fn from_compute_input(shader_input: &ComputeShaderInput) -> Self {
        let mut hasher = DefaultHasher::new();
        "compute".hash(&mut hasher);
        shader_input.hash(&mut hasher);
        Self(hasher.finish())
    }
}
