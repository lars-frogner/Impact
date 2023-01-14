//! Management of shaders.

use crate::rendering::{
    CameraShaderInput, InstanceFeatureShaderInput, MaterialTextureShaderInput, MeshShaderInput,
    Shader,
};
use anyhow::Result;
use impact_utils::stringhash64_newtype;
use std::{collections::HashMap, sync::Arc};

stringhash64_newtype!(
    /// Identifier for specific shaders.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] ShaderID
);

#[derive(Clone, Debug)]
pub struct ShaderManager {
    /// Shader programs.
    pub shaders: HashMap<ShaderID, Arc<Shader>>,
}

impl ShaderManager {
    /// Creates a new empty shader library.
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
        }
    }

    pub fn obtain_shader(
        &mut self,
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_texture_shader_input: Option<&MaterialTextureShaderInput>,
    ) -> Result<&Shader> {
        todo!()
    }
}
