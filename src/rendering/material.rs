use crate::rendering::{ShaderID, TextureID};
use std::collections::HashMap;

stringhash_newtype!(
    /// Identifier for specific materials.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] MaterialID
);

#[derive(Clone, Debug)]
pub struct MaterialSpecification {
    pub shader_id: ShaderID,
    pub image_texture_ids: Vec<TextureID>,
}

#[derive(Clone, Debug, Default)]
pub struct MaterialLibrary {
    material_specifications: HashMap<MaterialID, MaterialSpecification>,
}

impl MaterialLibrary {
    pub fn new() -> Self {
        Self {
            material_specifications: HashMap::new(),
        }
    }

    pub fn get_material(&self, material_id: MaterialID) -> Option<&MaterialSpecification> {
        self.material_specifications.get(&material_id)
    }

    pub fn add_material(&mut self, material_id: MaterialID, material_spec: MaterialSpecification) {
        self.material_specifications
            .insert(material_id, material_spec);
    }
}
