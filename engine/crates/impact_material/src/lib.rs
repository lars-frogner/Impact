//! Materials.

#[macro_use]
mod macros;

pub mod gpu_resource;
pub mod import;
pub mod setup;
pub mod values;

pub use values::register_material_feature_types;

use bytemuck::{Pod, Zeroable};
use impact_containers::DefaultHasher;
use impact_gpu::wgpu;
use impact_math::{Hash64, StringHash64, hash64};
use impact_mesh::VertexAttributeSet;
use impact_model::InstanceFeatureTypeID;
use impact_resource::{Resource, ResourceID, registry::ImmutableResourceRegistry};
use impact_texture::{
    TextureID,
    gpu_resource::{SamplerBindGroupLayoutEntryProps, TextureBindGroupLayoutEntryProps},
};
use nalgebra::Vector3;
use roc_integration::roc;
use setup::{
    fixed::FixedMaterialTextureBindingLocations, physical::PhysicalMaterialTextureBindingLocations,
};
use std::{
    fmt,
    hash::{Hash, Hasher},
};
use values::{MaterialPropertyFlags, MaterialPropertyValues};

/// A color with RGB components.
pub type RGBColor = Vector3<f32>;

/// A registry of [`Material`]s.
pub type MaterialRegistry = ImmutableResourceRegistry<Material>;

/// A registry of [`MaterialTemplate`]s.
pub type MaterialTemplateRegistry = ImmutableResourceRegistry<MaterialTemplate>;

/// A registry of [`MaterialTextureGroup`]s.
pub type MaterialTextureGroupRegistry = ImmutableResourceRegistry<MaterialTextureGroup>;

define_component_type! {
    /// Identifier for a material.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
    pub struct MaterialID(pub StringHash64);
}

/// Identifier for a [`MaterialTemplate`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaterialTemplateID(u64);

/// Identifier for a [`MaterialTextureGroup`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaterialTextureGroupID(Hash64);

/// A material defined by a template, texture group, and property values.
///
/// - **Template** ([`MaterialTemplate`]) - Defines the material's overall
///   structure and requirements.
/// - **Texture Group** ([`MaterialTextureGroup`]) - Specifies the textures
///   used by the material.
/// - **Property values** - Per-material values of uniform properties like
///   colors and PBR parameters.
///
/// This separation allows efficient sharing of templates and texture groups
/// between multiple materials.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// The ID of the material's [`MaterialTemplate`].
    pub template_id: MaterialTemplateID,
    /// The ID of the material's [`MaterialTextureGroup`] (may represent
    /// an empty group).
    pub texture_group_id: MaterialTextureGroupID,
    /// The material's values for all uniform material properties (properties
    /// that are not sampled from a texture).
    pub property_values: MaterialPropertyValues,
}

/// A template defining the overall structure and requirements of a class of
/// materials.
///
/// Material templates serve as blueprints that define:
/// - What vertex attributes are required (position, normals, texture coords).
/// - The type and meaning of the texture or sampler bound to each location in
///   the bind group (but not which specific textures are bound).
/// - The type of the data structure holding uniform material properties (but
///   not their values).
///
/// Materials with the same template can use the exact same render pipeline.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialTemplate {
    pub vertex_attribute_requirements: VertexAttributeSet,
    pub bind_group_template: MaterialBindGroupTemplate,
    pub texture_binding_locations: MaterialTextureBindingLocations,
    pub property_flags: MaterialPropertyFlags,
    pub instance_feature_type_id: InstanceFeatureTypeID,
}

/// Template for creating a bind group layout usable by multiple material
/// texture groups.
///
/// The template defines how textures and samplers are arranged in a bind group.
/// Each slot contains binding properties for one texture-sampler pair, and
/// slots are assigned consecutive binding indices starting from 0.
///
/// An empty template (no slots) represent materials that don't use any
/// textures.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialBindGroupTemplate {
    pub slots: Vec<MaterialBindGroupSlot>,
}

/// Holds the properties of a pair of entries for a texture and its sampler in a
/// material texture bind group.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialBindGroupSlot {
    pub texture: TextureBindGroupLayoutEntryProps,
    pub sampler: SamplerBindGroupLayoutEntryProps,
}

/// Specifies where the texture and sampler for specific material properties are
/// bound in a material texture bind group.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MaterialTextureBindingLocations {
    Fixed(FixedMaterialTextureBindingLocations),
    Physical(PhysicalMaterialTextureBindingLocations),
}

/// A group of textures that can be shared across multiple materials.
///
/// Texture groups enable efficient sharing of texture combinations between
/// materials that use the same set of textures but may have different uniform
/// material property values. The group must be compatible with its associated
/// material template - the number and types of textures must match the
/// template's bind group slots.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialTextureGroup {
    pub template_id: MaterialTemplateID,
    pub texture_ids: Vec<TextureID>,
}

#[roc(dependencies = [impact_math::Hash64])]
impl MaterialID {
    #[roc(body = "Hashing.hash_str_64(name)")]
    /// Creates a material ID hashed from the given name.
    pub fn from_name(name: &str) -> Self {
        Self(hash64!(name))
    }

    /// Creates an ID that does not represent a valid material.
    pub fn not_applicable() -> Self {
        Self(StringHash64::zeroed())
    }

    /// Returns `true` if this ID does not represent a valid material.
    pub fn is_not_applicable(&self) -> bool {
        self.0 == StringHash64::zeroed()
    }
}

impl fmt::Display for MaterialID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResourceID for MaterialID {}

impl MaterialTemplateID {
    /// Creates an ID by hashing the given material template.
    pub fn for_template(template: &MaterialTemplate) -> Self {
        let mut hasher = DefaultHasher::default();
        template.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl fmt::Display for MaterialTemplateID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MaterialTemplateID({})", self.0)
    }
}

impl ResourceID for MaterialTemplateID {}

impl MaterialTextureGroupID {
    /// Generates a material texture group ID that will always be the same for a
    /// specific ordered group of texture IDs.
    pub fn from_texture_ids(texture_ids: &[TextureID]) -> Self {
        texture_ids
            .iter()
            .map(|texture_id| texture_id.0.hash())
            .reduce(impact_math::compute_hash_64_of_two_hash_64)
            .map_or_else(Self::empty, Self)
    }

    /// Creates an ID representing an empty texture group.
    pub fn empty() -> Self {
        Self(Hash64::zeroed())
    }

    /// Whether this ID represents an empty texture group.
    pub fn is_empty(&self) -> bool {
        *self == Self::empty()
    }
}

impl fmt::Display for MaterialTextureGroupID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MaterialTextureGroupID({})", self.0)
    }
}

impl ResourceID for MaterialTextureGroupID {}

impl Material {
    /// Returns the texture group ID if it represents a non-empty group.
    pub fn texture_group_id_if_non_empty(&self) -> Option<MaterialTextureGroupID> {
        if self.texture_group_id.is_empty() {
            None
        } else {
            Some(self.texture_group_id)
        }
    }
}

impl Resource for Material {
    type ID = MaterialID;
}

impl MaterialTemplate {
    /// Returns the instance feature type ID if it is not N/A.
    pub fn instance_feature_type_id_if_applicable(&self) -> Option<InstanceFeatureTypeID> {
        if self.instance_feature_type_id.is_not_applicable() {
            None
        } else {
            Some(self.instance_feature_type_id)
        }
    }
}

impl Resource for MaterialTemplate {
    type ID = MaterialTemplateID;
}

impl MaterialBindGroupTemplate {
    /// Creates an empty bind group template with no slots.
    pub fn empty() -> Self {
        Self { slots: Vec::new() }
    }

    /// Whether the bind group has no slots.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Returns the total number of bind group entries (2 per slot).
    pub fn n_entries(&self) -> usize {
        2 * self.slots.len()
    }

    /// Returns the shader stages where the bind group is visible.
    pub const fn visibility() -> wgpu::ShaderStages {
        wgpu::ShaderStages::FRAGMENT
    }

    /// Returns the binding that will be used for the texture at the given index
    /// and its sampler in the bind group.
    pub const fn get_texture_and_sampler_bindings(texture_idx: usize) -> (u32, u32) {
        let texture_binding = (2 * texture_idx) as u32;
        let sampler_binding = texture_binding + 1;
        (texture_binding, sampler_binding)
    }
}

impl Resource for MaterialTextureGroup {
    type ID = MaterialTextureGroupID;
}
