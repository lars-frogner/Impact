//! Management of shaders.

use std::{collections::HashMap, sync::Arc};

use impact_utils::stringhash64_newtype;

use crate::rendering::Shader;

stringhash64_newtype!(
    /// Identifier for specific shaders.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] ShaderID
);

#[derive(Clone, Debug)]
pub struct ShaderLibrary {
    /// Shader programs.
    pub shaders: HashMap<ShaderID, Arc<Shader>>,
}

impl ShaderLibrary {
    /// Creates a new empty shader library.
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
        }
    }
}
