//! Generation and management of shaders.

pub mod template;

use crate::gpu::GraphicsDevice;
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_containers::HashMap;
use impact_math::Hash64;
use naga::{Module, ShaderStage};
use std::{borrow::Cow, collections::hash_map::Entry, fs, hash::Hash, path::Path};
use template::SpecificShaderTemplate;

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

/// A graphics shader program.
#[derive(Debug)]
pub struct Shader {
    module: wgpu::ShaderModule,
    entry_point_names: EntryPointNames,
    source_code: Option<String>,
}

/// Names of the different shader entry point functions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryPointNames {
    /// Name of the vertex entry point function, or [`None`] if there is no
    /// vertex entry point.
    pub vertex: Option<Cow<'static, str>>,
    /// Name of the fragment entry point function, or [`None`] if there is no
    /// fragment entry point.
    pub fragment: Option<Cow<'static, str>>,
    /// Name of the compute entry point function, or [`None`] if there is no
    /// compute entry point.
    pub compute: Option<Cow<'static, str>>,
}

impl ShaderManager {
    /// Creates a new empty shader library.
    pub fn new() -> Self {
        Self {
            rendering_shaders: HashMap::default(),
            compute_shaders: HashMap::default(),
        }
    }

    /// Determines the shader ID for the given shader template, resolves it and
    /// stores it as a rendering shader if it does not already exist and returns
    /// the shader ID.
    ///
    /// # Panics
    /// If the shader template can not be compiled.
    pub fn get_or_create_rendering_shader_from_template(
        &mut self,
        graphics_device: &GraphicsDevice,
        template: &impl SpecificShaderTemplate,
    ) -> (ShaderID, &Shader) {
        Self::get_or_create_shader_from_template(
            &mut self.rendering_shaders,
            graphics_device,
            template,
        )
    }

    /// Determines the shader ID for the given shader template, resolves it and
    /// stores it as a compute shader if it does not already exist and returns
    /// the shader ID.
    ///
    /// # Panics
    /// If the shader template can not be compiled.
    pub fn get_or_create_compute_shader_from_template(
        &mut self,
        graphics_device: &GraphicsDevice,
        template: &impl SpecificShaderTemplate,
    ) -> (ShaderID, &Shader) {
        Self::get_or_create_shader_from_template(
            &mut self.compute_shaders,
            graphics_device,
            template,
        )
    }

    /// Determines the shader ID for the given shader template, resolves it and
    /// stores it as a rendering shader under the shader ID, replacing any
    /// existing rendering shader under that ID. Returns the shader and ID.
    ///
    /// # Panics
    /// If the shader template can not be compiled.
    pub fn insert_and_get_rendering_shader_from_template(
        &mut self,
        graphics_device: &GraphicsDevice,
        template: &impl SpecificShaderTemplate,
    ) -> (ShaderID, &Shader) {
        Self::insert_and_get_shader_with_template(
            &mut self.rendering_shaders,
            graphics_device,
            template,
        )
    }

    /// Determines the shader ID for the given shader template, resolves it and
    /// stores it as a compute shader under the shader ID, replacing any
    /// existing compute shader under that ID. Returns the shader and ID.
    ///
    /// # Panics
    /// If the shader template can not be compiled.
    pub fn insert_and_get_compute_shader_from_template(
        &mut self,
        graphics_device: &GraphicsDevice,
        template: &impl SpecificShaderTemplate,
    ) -> (ShaderID, &Shader) {
        Self::insert_and_get_shader_with_template(
            &mut self.compute_shaders,
            graphics_device,
            template,
        )
    }

    fn get_or_create_shader_from_template<'a>(
        shaders: &'a mut HashMap<ShaderID, Shader>,
        graphics_device: &GraphicsDevice,
        template: &impl SpecificShaderTemplate,
    ) -> (ShaderID, &'a Shader) {
        let shader_id = template.shader_id();

        let shader = match shaders.entry(shader_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(
                Shader::from_template(graphics_device, template)
                    .expect("Failed to create shader from template"),
            ),
        };

        (shader_id, shader)
    }

    fn insert_and_get_shader_with_template<'a>(
        shaders: &'a mut HashMap<ShaderID, Shader>,
        graphics_device: &GraphicsDevice,
        template: &impl SpecificShaderTemplate,
    ) -> (ShaderID, &'a Shader) {
        let shader_id = template.shader_id();

        let new_shader = Shader::from_template(graphics_device, template)
            .expect("Failed to create shader from template");

        let shader = match shaders.entry(shader_id) {
            Entry::Occupied(entry) => {
                let shader = entry.into_mut();
                *shader = new_shader;
                shader
            }
            Entry::Vacant(entry) => entry.insert(new_shader),
        };
        (shader_id, shader)
    }
}

impl Default for ShaderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderID {
    /// Generates a [`ShaderID`] from the given string identifying the shader.
    pub fn from_identifier(identifier: &str) -> Self {
        Self(Hash64::from_str(identifier).into())
    }
}

impl Shader {
    /// Creates a new shader from the source code resolved from the given
    /// template.
    ///
    /// # Errors
    /// Returns an error if WGSL parsing fails.
    pub fn from_template(
        graphics_device: &GraphicsDevice,
        template: &impl SpecificShaderTemplate,
    ) -> Result<Self> {
        Self::from_wgsl_source(graphics_device, template.resolve(), &template.label())
    }

    /// Creates a new shader by reading the WGSL source from the given file.
    ///
    /// # Errors
    /// Returns an error if the shader file can not be found or read.
    pub fn from_wgsl_path(
        graphics_device: &GraphicsDevice,
        shader_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let shader_path = shader_path.as_ref();
        let label = shader_path.to_string_lossy();
        let source = fs::read_to_string(shader_path)?;
        Self::from_wgsl_source(graphics_device, source, label.as_ref())
    }

    /// Creates a new shader from the given WGSL source code string.
    ///
    /// # Errors
    /// Returns an error if WGSL parsing fails.
    pub fn from_wgsl_source(
        graphics_device: &GraphicsDevice,
        source: String,
        label: &str,
    ) -> Result<Self> {
        let naga_module = naga::front::wgsl::parse_str(&source)?;
        Ok(Self::from_naga_module(
            graphics_device,
            naga_module,
            label,
            Some(source),
        ))
    }

    /// Creates a new shader from the given [`Module`].
    #[allow(unused_mut)]
    pub fn from_naga_module(
        graphics_device: &GraphicsDevice,
        naga_module: Module,
        label: &str,
        mut source_code: Option<String>,
    ) -> Self {
        #[cfg(debug_assertions)]
        if source_code.is_none() {
            source_code = Some(Self::generate_wgsl_from_naga_module(&naga_module));
        }

        let entry_point_names = EntryPointNames::from_naga_module(&naga_module);

        let module = graphics_device
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                source: wgpu::ShaderSource::Naga(Cow::Owned(naga_module)),
                label: Some(label),
            });

        Self {
            module,
            entry_point_names,
            source_code,
        }
    }

    /// Returns a reference to the compiled shader module containing the vertex
    /// stage entry point if it exists.
    pub fn vertex_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    /// Returns a reference to the compiled shader module containing the
    /// fragment stage entry point if it exists.
    pub fn fragment_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    /// Returns a reference to the compiled shader module containing the compute
    /// stage entry point if it exists.
    pub fn compute_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    /// Returns the name of the vertex entry point function, or [`None`] if
    /// there is no vertex entry point.
    pub fn vertex_entry_point_name(&self) -> Option<&str> {
        self.entry_point_names.vertex.as_deref()
    }

    /// Returns the name of the fragment entry point function, or [`None`] if
    /// there is no fragment entry point.
    pub fn fragment_entry_point_name(&self) -> Option<&str> {
        self.entry_point_names.fragment.as_deref()
    }

    /// Returns the name of the compute entry point function, or [`None`] if
    /// there is no compute entry point.
    pub fn compute_entry_point_name(&self) -> Option<&str> {
        self.entry_point_names.compute.as_deref()
    }

    #[cfg(debug_assertions)]
    fn generate_wgsl_from_naga_module(module: &Module) -> String {
        Self::generate_wgsl_from_validated_naga_module(module, &Self::validate_naga_module(module))
    }

    #[cfg(debug_assertions)]
    fn validate_naga_module(module: &Module) -> naga::valid::ModuleInfo {
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(module)
            .expect("Shader validation failed")
    }

    #[cfg(debug_assertions)]
    fn generate_wgsl_from_validated_naga_module(
        module: &Module,
        module_info: &naga::valid::ModuleInfo,
    ) -> String {
        naga::back::wgsl::write_string(module, module_info, naga::back::wgsl::WriterFlags::all())
            .unwrap()
    }
}

impl std::fmt::Display for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self.source_code {
                Some(source) => source,
                None => "<Source code unavailable>",
            }
        )
    }
}

impl EntryPointNames {
    fn from_naga_module(module: &Module) -> Self {
        let mut entry_point_names = Self {
            vertex: None,
            fragment: None,
            compute: None,
        };
        for entry_point in &module.entry_points {
            match entry_point.stage {
                ShaderStage::Vertex => {
                    entry_point_names.vertex = Some(Cow::Owned(entry_point.name.clone()));
                }
                ShaderStage::Fragment => {
                    entry_point_names.fragment = Some(Cow::Owned(entry_point.name.clone()));
                }
                ShaderStage::Compute => {
                    entry_point_names.compute = Some(Cow::Owned(entry_point.name.clone()));
                }
            }
        }
        entry_point_names
    }
}
