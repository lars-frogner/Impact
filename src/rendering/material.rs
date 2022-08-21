//! Management of materials.

use crate::rendering::{ShaderID, TextureID};
use std::collections::HashMap;

stringhash_newtype!(
    /// Identifier for specific materials.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] MaterialID
);

/// A material specified by textures and a shader.
#[derive(Clone, Debug)]
pub struct MaterialSpecification {
    pub shader_id: ShaderID,
    pub image_texture_ids: Vec<TextureID>,
}

/// Container for different material specifications.
#[derive(Clone, Debug, Default)]
pub struct MaterialLibrary {
    material_specifications: HashMap<MaterialID, MaterialSpecification>,
}

impl MaterialLibrary {
    /// Creates a new empty material library.
    pub fn new() -> Self {
        Self {
            material_specifications: HashMap::new(),
        }
    }

    /// Returns the specification for the material with the
    /// given ID, or [`None`] if the material does not exist.
    pub fn get_material(&self, material_id: MaterialID) -> Option<&MaterialSpecification> {
        self.material_specifications.get(&material_id)
    }

    /// Includes the given material specification in the library
    /// under the given ID. If a material with the same ID exists,
    /// it will be overwritten.
    pub fn add_material(&mut self, material_id: MaterialID, material_spec: MaterialSpecification) {
        self.material_specifications
            .insert(material_id, material_spec);
    }
}
