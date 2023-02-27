//! Management of materials.

mod blinn_phong;
mod components;
mod depth;
mod fixed;
mod vertex_color;

use crate::{
    geometry::{InstanceFeatureTypeID, VertexAttributeSet},
    rendering::{fre, MaterialShaderInput, TextureID},
};
use impact_utils::{hash64, stringhash64_newtype};
use nalgebra::{Vector3, Vector4};
use std::collections::{hash_map::Entry, HashMap};

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

/// A color with RGB components.
pub type RGBColor = Vector3<fre>;

/// A color with RGBA components.
pub type RGBAColor = Vector4<fre>;

stringhash64_newtype!(
    /// Identifier for specific materials.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] MaterialID
);

/// A material specified by a set of per-material properties (as instance
/// features) and associated textures.
#[derive(Clone, Debug)]
pub struct MaterialSpecification {
    vertex_attribute_requirements: VertexAttributeSet,
    image_texture_ids: Vec<TextureID>,
    instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
    shader_input: MaterialShaderInput,
}

/// Container for different material specifications.
#[derive(Clone, Debug, Default)]
pub struct MaterialLibrary {
    material_specifications: HashMap<MaterialID, MaterialSpecification>,
}

const MATERIAL_VERTEX_BINDING_START: u32 = 20;

impl MaterialSpecification {
    /// Creates a new material specification with the
    /// given textures and material properties.
    pub fn new(
        vertex_attribute_requirements: VertexAttributeSet,
        image_texture_ids: Vec<TextureID>,
        instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
        shader_input: MaterialShaderInput,
    ) -> Self {
        Self {
            vertex_attribute_requirements,
            image_texture_ids,
            instance_feature_type_ids,
            shader_input,
        }
    }

    /// Returns a [`VertexAttributeSet`] encoding the vertex attributes required
    /// for rendering the material.
    pub fn vertex_attribute_requirements(&self) -> VertexAttributeSet {
        self.vertex_attribute_requirements
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

    /// Whether the material requires light sources.
    pub fn shader_input(&self) -> &MaterialShaderInput {
        &self.shader_input
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

/// Generates a material ID that will always be the same
/// for a specific base string and set of texture IDs.
fn generate_material_id<S: AsRef<str>>(base_string: S, texture_ids: &[TextureID]) -> MaterialID {
    MaterialID(hash64!(format!(
        "{} [{}]",
        base_string.as_ref(),
        texture_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}
