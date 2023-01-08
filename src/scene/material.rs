//! Management of materials.

use crate::{
    geometry::InstanceFeatureTypeID,
    rendering::{ShaderID, TextureID},
};
use impact_utils::stringhash_newtype;
use std::collections::HashMap;

stringhash_newtype!(
    /// Identifier for specific materials.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] MaterialID
);

/// A material specified by a shader with associated
/// textures and material properties.
#[derive(Clone, Debug)]
pub struct MaterialSpecification {
    shader_id: ShaderID,
    image_texture_ids: Vec<TextureID>,
    instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
}

/// Container for different material specifications.
#[derive(Clone, Debug, Default)]
pub struct MaterialLibrary {
    material_specifications: HashMap<MaterialID, MaterialSpecification>,
}

impl MaterialSpecification {
    /// Creates a new material specification with the
    /// given shader, textures and material properties.
    pub fn new(
        shader_id: ShaderID,
        image_texture_ids: Vec<TextureID>,
        instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
    ) -> Self {
        Self {
            shader_id,
            image_texture_ids,
            instance_feature_type_ids,
        }
    }

    /// Returns the ID of the shader used for the material.
    pub fn shader_id(&self) -> ShaderID {
        self.shader_id
    }

    /// Returns the IDs of the image textures used for the
    /// material.
    pub fn image_texture_ids(&self) -> &[TextureID] {
        &self.image_texture_ids
    }

    /// Returns the IDs of the material property types used
    /// for the material.
    pub fn instance_feature_type_ids(&self) -> &[InstanceFeatureTypeID] {
        &self.instance_feature_type_ids
    }
}

impl MaterialLibrary {
    /// Creates a new empty material library.
    pub fn new() -> Self {
        Self {
            material_specifications: HashMap::new(),
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`MaterialSpecification`]s.
    pub fn material_specifications(&self) -> &HashMap<MaterialID, MaterialSpecification> {
        &self.material_specifications
    }

    /// Returns the specification for the material with the
    /// given ID, or [`None`] if the material does not exist.
    pub fn get_material_specification(
        &self,
        material_id: MaterialID,
    ) -> Option<&MaterialSpecification> {
        self.material_specifications.get(&material_id)
    }

    /// Includes the given material specification in the library
    /// under the given ID. If a material with the same ID exists,
    /// it will be overwritten.
    pub fn add_material_specification(
        &mut self,
        material_id: MaterialID,
        material_spec: MaterialSpecification,
    ) {
        self.material_specifications
            .insert(material_id, material_spec);
    }
}
