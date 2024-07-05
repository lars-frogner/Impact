//! Generation of compute shaders.

use super::EntryPointNames;
use anyhow::Result;
use naga::{Function, Module};
use std::borrow::Cow;

/// Input description for any kind of compute shader.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ComputeShaderInput {}

/// Shader generator for any kind of GPU computation.
#[derive(Clone, Debug)]
pub struct ComputeShaderGenerator {}

impl ComputeShaderGenerator {
    pub fn generate_shader_module(
        shader_input: &ComputeShaderInput,
    ) -> Result<(Module, EntryPointNames)> {
        let mut module = Module::default();
        let mut compute_function = Function::default();

        todo!();

        let entry_point_names = EntryPointNames {
            vertex: None,
            fragment: None,
            compute: Some(Cow::Borrowed("mainCS")),
        };

        Ok((module, entry_point_names))
    }
}
