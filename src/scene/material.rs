//! Management of materials.

mod blinn_phong;
mod components;
mod depth;
mod fixed;
mod vertex_color;

pub use blinn_phong::{
    BlinnPhongMaterial, DiffuseTexturedBlinnPhongMaterial, TexturedBlinnPhongMaterial,
};
pub use components::{
    BlinnPhongComp, DiffuseTexturedBlinnPhongComp, FixedColorComp, FixedTextureComp,
    LightSpaceDepthComp, MaterialComp, TexturedBlinnPhongComp, VertexColorComp,
};
pub use depth::LightSpaceDepthMaterial;
pub use fixed::{FixedColorMaterial, FixedTextureMaterial};
pub use vertex_color::VertexColorMaterial;

use crate::{
    geometry::{InstanceFeatureTypeID, VertexAttributeSet},
    rendering::{fre, MaterialShaderInput, TextureID},
};
use bytemuck::Zeroable;
use impact_utils::{hash64, stringhash64_newtype, StringHash64};
use nalgebra::{Vector3, Vector4};
use std::collections::{hash_map::Entry, HashMap};

/// A color with RGB components.
pub type RGBColor = Vector3<fre>;

/// A color with RGBA components.
pub type RGBAColor = Vector4<fre>;

stringhash64_newtype!(
    /// Identifier for specific material types.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] MaterialID
);

stringhash64_newtype!(
    /// Identifier for sets of textures used for material properties.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] MaterialPropertyTextureSetID
);

/// A material type specified by a set of untextured per-material properties (as
/// instance features).
#[derive(Clone, Debug)]
pub struct MaterialSpecification {
    vertex_attribute_requirements: VertexAttributeSet,
    instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
    shader_input: MaterialShaderInput,
}

/// Specifies a set of textures used for textured material properties.
#[derive(Clone, Debug)]
pub struct MaterialPropertyTextureSet {
    image_texture_ids: Vec<TextureID>,
}

/// Container for material specifications and material property texture sets.
#[derive(Clone, Debug, Default)]
pub struct MaterialLibrary {
    material_specifications: HashMap<MaterialID, MaterialSpecification>,
    material_property_texture_sets:
        HashMap<MaterialPropertyTextureSetID, MaterialPropertyTextureSet>,
}

const MATERIAL_VERTEX_BINDING_START: u32 = 20;

impl MaterialSpecification {
    /// Creates a new material specification with the given untextured material
    /// properties.
    pub fn new(
        vertex_attribute_requirements: VertexAttributeSet,
        instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
        shader_input: MaterialShaderInput,
    ) -> Self {
        Self {
            vertex_attribute_requirements,
            instance_feature_type_ids,
            shader_input,
        }
    }

    /// Returns a [`VertexAttributeSet`] encoding the vertex attributes required
    /// for rendering the material.
    pub fn vertex_attribute_requirements(&self) -> VertexAttributeSet {
        self.vertex_attribute_requirements
    }

    /// Returns the IDs of the material property types used
    /// for the material.
    pub fn instance_feature_type_ids(&self) -> &[InstanceFeatureTypeID] {
        &self.instance_feature_type_ids
    }

    /// Returns the input required for using the material in a shader.
    pub fn shader_input(&self) -> &MaterialShaderInput {
        &self.shader_input
    }
}

impl MaterialPropertyTextureSet {
    /// Creates a new material property texture set for the image textures with
    /// the given IDs.
    ///
    /// # Panics
    /// If the given list of texture IDs is empty.
    pub fn new(image_texture_ids: Vec<TextureID>) -> Self {
        assert!(!image_texture_ids.is_empty());
        Self { image_texture_ids }
    }

    /// Returns the IDs of the image textures in the texture set.
    pub fn image_texture_ids(&self) -> &[TextureID] {
        &self.image_texture_ids
    }
}

impl MaterialLibrary {
    /// Creates a new empty material library.
    pub fn new() -> Self {
        Self {
            material_specifications: HashMap::new(),
            material_property_texture_sets: HashMap::new(),
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`MaterialSpecification`]s.
    pub fn material_specifications(&self) -> &HashMap<MaterialID, MaterialSpecification> {
        &self.material_specifications
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`MaterialPropertyTextureSet`]s.
    pub fn material_property_texture_sets(
        &self,
    ) -> &HashMap<MaterialPropertyTextureSetID, MaterialPropertyTextureSet> {
        &self.material_property_texture_sets
    }

    /// Returns the specification of the material with the given ID, or [`None`]
    /// if the material does not exist.
    pub fn get_material_specification(
        &self,
        material_id: MaterialID,
    ) -> Option<&MaterialSpecification> {
        self.material_specifications.get(&material_id)
    }

    /// Returns the material property texture set with the given ID, or [`None`]
    /// if the texture set does not exist.
    pub fn get_material_property_texture_set(
        &self,
        texture_set_id: MaterialPropertyTextureSetID,
    ) -> Option<&MaterialPropertyTextureSet> {
        self.material_property_texture_sets.get(&texture_set_id)
    }

    /// Returns a hashmap entry for the specification of the material with the
    /// given ID.
    pub fn material_specification_entry(
        &mut self,
        material_id: MaterialID,
    ) -> Entry<'_, MaterialID, MaterialSpecification> {
        self.material_specifications.entry(material_id)
    }

    /// Returns a hashmap entry for the material property texture set with the
    /// given ID.
    pub fn material_property_texture_set_entry(
        &mut self,
        texture_set_id: MaterialPropertyTextureSetID,
    ) -> Entry<'_, MaterialPropertyTextureSetID, MaterialPropertyTextureSet> {
        self.material_property_texture_sets.entry(texture_set_id)
    }

    /// Includes the given material specification in the library under the given
    /// ID. If a material with the same ID exists, it will be overwritten.
    pub fn add_material_specification(
        &mut self,
        material_id: MaterialID,
        material_spec: MaterialSpecification,
    ) {
        self.material_specifications
            .insert(material_id, material_spec);
    }

    /// Includes the given material property texture set in the library under
    /// the given ID. If a texture set with the same ID exists, it will be
    /// overwritten.
    pub fn add_material_property_texture_set(
        &mut self,
        texture_set_id: MaterialPropertyTextureSetID,
        texture_set: MaterialPropertyTextureSet,
    ) {
        self.material_property_texture_sets
            .insert(texture_set_id, texture_set);
    }
}

impl MaterialPropertyTextureSetID {
    /// Generates a material property texture set ID that will always be the same
    /// for a specific ordered set of texture IDs.
    pub fn from_texture_ids(texture_ids: &[TextureID]) -> Self {
        Self(hash64!(format!(
            "{}",
            texture_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join("-")
        )))
    }

    /// Creates an ID representing an empty texture set.
    pub fn empty() -> Self {
        Self(StringHash64::zeroed())
    }

    /// Returns `true` if this ID represents an empty texture set.
    pub fn is_empty(&self) -> bool {
        self.0 == StringHash64::zeroed()
    }
}
