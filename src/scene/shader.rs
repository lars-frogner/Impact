//! Management of shaders.

use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        CameraShaderInput, CoreRenderingSystem, InstanceFeatureShaderInput, LightShaderInput,
        MaterialShaderInput, MeshShaderInput, RenderAttachmentQuantitySet, Shader, ShaderGenerator,
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
    /// Shader programs.
    pub shaders: HashMap<ShaderID, Shader>,
}

impl ShaderManager {
    /// Creates a new empty shader library.
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
        }
    }

    /// Obtains the appropriate [`Shader`] for the given set of shader inputs.
    ///
    /// If a shader for the given inputs already exists, it is returned,
    /// otherwise a new shader is generated, compiled and cached.
    ///
    /// # Errors
    /// See [`ShaderGenerator::generate_shader_module`].
    pub fn obtain_shader(
        &mut self,
        core_system: &CoreRenderingSystem,
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        light_shader_input: Option<&LightShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_shader_input: Option<&MaterialShaderInput>,
        vertex_attribute_requirements: VertexAttributeSet,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
    ) -> Result<&Shader> {
        let shader_id = ShaderID::from_input(
            camera_shader_input,
            mesh_shader_input,
            light_shader_input,
            instance_feature_shader_inputs,
            material_shader_input,
            vertex_attribute_requirements,
            input_render_attachment_quantities,
            output_render_attachment_quantities,
        );

        match self.shaders.entry(shader_id) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let (module, entry_point_names) = ShaderGenerator::generate_shader_module(
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
                    core_system,
                    module,
                    entry_point_names,
                    format!("Generated shader (hash {})", shader_id.0).as_str(),
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
    fn from_input(
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
}
