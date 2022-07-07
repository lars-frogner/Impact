//! Graphics shaders.

use crate::rendering::CoreRenderingSystem;
use std::borrow::Cow;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::{fs, path::Path};
        use anyhow::Result;
    }
}

/// A graphics shader program.
#[derive(Debug)]
pub struct Shader {
    module: wgpu::ShaderModule,
}

impl Shader {
    /// Creates a new shader by reading the source from the given file.
    ///
    /// # Errors
    /// Returns an error if the shader file can not be found or read.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path<P: AsRef<Path>>(
        core_system: &CoreRenderingSystem,
        shader_path: P,
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
