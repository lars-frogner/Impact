//! Graphics shaders.

use crate::rendering::CoreRenderingSystem;
use std::borrow::Cow;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::{fs, path::Path};
        use anyhow::Result;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraShaderInput {
    pub view_proj_matrix_binding: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshShaderInput {
    pub position_location: u32,
    pub vertex_normal_location: Option<u32>,
    pub texture_coord_location: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstanceFeatureShaderInput {
    ModelInstanceTransform(ModelInstanceTransformShaderInput),
    FixedColorMaterial(FixedColorFeatureShaderInput),
    BlinnPhongMaterial(BlinnPhongFeatureShaderInput),
    #[cfg(test)]
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInstanceTransformShaderInput {
    pub model_matrix_locations: [u32; 4],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedColorFeatureShaderInput {
    pub color_location: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlinnPhongFeatureShaderInput {
    pub ambient_color_location: u32,
    pub diffuse_color_location: Option<u32>,
    pub specular_color_location: Option<u32>,
    pub shininess_location: u32,
    pub alpha_location: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialTextureShaderInput {
    BlinnPhongMaterial(BlinnPhongTextureShaderInput),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlinnPhongTextureShaderInput {
    pub diffuse_texture_and_sampler_bindings: (u32, u32),
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniformShaderInput {}

/// A graphics shader program.
#[derive(Debug)]
pub struct Shader {
    module: wgpu::ShaderModule,
}

pub struct ShaderBuilder {}

impl Shader {
    /// Creates a new shader by reading the source from the given file.
    ///
    /// # Errors
    /// Returns an error if the shader file can not be found or read.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path(
        core_system: &CoreRenderingSystem,
        shader_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let shader_path = shader_path.as_ref();
        let label = shader_path.to_string_lossy();
        let source = fs::read_to_string(shader_path)?;
        Ok(Self::from_source(core_system, &source, label.as_ref()))
    }

    /// Creates a new shader from the given source code.
    pub fn from_source(core_system: &CoreRenderingSystem, source: &str, label: &str) -> Self {
        let module = core_system
            .device()
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
                label: Some(label),
            });
        Self { module }
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }
}

impl ShaderBuilder {
    pub fn build_shader_source(
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_texture_shader_input: Option<&MaterialTextureShaderInput>,
    ) {
    }
}
