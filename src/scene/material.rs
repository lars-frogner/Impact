//! Management of materials.

mod blinn_phong;
mod components;
mod fixed_color;

use crate::{
    geometry::InstanceFeatureTypeID,
    rendering::{fre, MaterialTextureShaderInput, TextureID},
};
use impact_utils::stringhash64_newtype;
use nalgebra::{Vector3, Vector4};
use std::collections::{hash_map::Entry, HashMap};

pub use blinn_phong::{
    BlinnPhongMaterial, DiffuseTexturedBlinnPhongMaterial, TexturedBlinnPhongMaterial,
};
pub use components::{
    BlinnPhongComp, DiffuseTexturedBlinnPhongComp, FixedColorComp, MaterialComp,
    TexturedBlinnPhongComp,
};
pub use fixed_color::FixedColorMaterial;

/// A color with RGB components.
pub type RGBColor = Vector3<fre>;

/// A color with RGBA components.
pub type RGBAColor = Vector4<fre>;

stringhash64_newtype!(
    /// Identifier for specific materials.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] MaterialID
);

/// A material specified by a shader with associated
/// textures and material properties.
#[derive(Clone, Debug)]
pub struct MaterialSpecification {
    image_texture_ids: Vec<TextureID>,
    instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
    texture_shader_input: MaterialTextureShaderInput,
}

/// Container for different material specifications.
#[derive(Clone, Debug, Default)]
pub struct MaterialLibrary {
    material_specifications: HashMap<MaterialID, MaterialSpecification>,
}

impl MaterialSpecification {
    /// Creates a new material specification with the
    /// given textures and material properties.
    pub fn new(
        image_texture_ids: Vec<TextureID>,
        instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
        texture_shader_input: MaterialTextureShaderInput,
    ) -> Self {
        Self {
            image_texture_ids,
            instance_feature_type_ids,
            texture_shader_input,
        }
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

    /// Returns the input required for accessing the textures
    /// in a shader.
    pub fn texture_shader_input(&self) -> &MaterialTextureShaderInput {
        &self.texture_shader_input
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

    /// Returns the specification of the material with the
    /// given ID, or [`None`] if the material does not exist.
    pub fn get_material_specification(
        &self,
        material_id: MaterialID,
    ) -> Option<&MaterialSpecification> {
        self.material_specifications.get(&material_id)
    }

    /// Returns a hashmap entry for the specification of the
    /// material with the given ID.
    pub fn material_specification_entry(
        &mut self,
        material_id: MaterialID,
    ) -> Entry<'_, MaterialID, MaterialSpecification> {
        self.material_specifications.entry(material_id)
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
