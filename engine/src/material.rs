//! Materials.

pub mod components;
pub mod entity;
mod features;

pub use features::{
    MaterialInstanceFeatureFlags, MaterialInstanceFeatureLocation, register_material_feature_types,
};
use roc_codegen::roc;

use crate::{
    assets::Assets,
    gpu::{GraphicsDevice, resource_group::GPUResourceGroup, texture::TextureID},
    mesh::VertexAttributeSet,
    model::{InstanceFeatureID, InstanceFeatureTypeID},
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use entity::{fixed::FixedMaterialTextureBindings, physical::PhysicalMaterialTextureBindings};
use impact_math::{Hash64, StringHash64, hash64, stringhash64_newtype};
use nalgebra::Vector3;
use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    fmt,
};

/// A color with RGB components.
pub type RGBColor = Vector3<f32>;

stringhash64_newtype!(
    /// Identifier for specific material types.
    /// Wraps a [`StringHash64`](impact_math::StringHash64).
    #[roc(prefix = "Engine")]
    [pub] MaterialID
);

stringhash64_newtype!(
    /// Identifier for group of textures used for material properties. Wraps a
    /// [`StringHash64`](impact_math::StringHash64).
    #[roc(prefix = "Engine")]
    [pub] MaterialPropertyTextureGroupID
);

/// A handle for a material, containing the IDs for the pieces of data holding
/// information about the material.
#[roc(prefix = "Engine")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct MaterialHandle {
    /// The ID of the material's [`MaterialSpecification`].
    material_id: MaterialID,
    /// The ID of the entry for the material's per-instance material properties
    /// in the [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage)
    /// (may be N/A).
    material_property_feature_id: InstanceFeatureID,
    /// The ID of the material's [`MaterialPropertyTextureGroup`] (may represent
    /// an empty group).
    material_property_texture_group_id: MaterialPropertyTextureGroupID,
}

/// A material description specifying a material's set of required vertex
/// attributes, associated render attachments, material-specific resources,
/// untextured per-material properties (as instance features), render pass hints
/// and shader input.
#[derive(Debug)]
pub struct MaterialSpecification {
    vertex_attribute_requirements: VertexAttributeSet,
    instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
    instance_feature_flags: MaterialInstanceFeatureFlags,
    material_specific_resources: Option<GPUResourceGroup>,
    shader_input: MaterialShaderInput,
}

/// A group of textures used for textured material properties.
#[derive(Debug)]
pub struct MaterialPropertyTextureGroup {
    texture_ids: Vec<TextureID>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

/// Container for material specifications and material property texture groups.
#[derive(Debug, Default)]
pub struct MaterialLibrary {
    material_specifications: HashMap<MaterialID, MaterialSpecification>,
    material_property_texture_groups:
        HashMap<MaterialPropertyTextureGroupID, MaterialPropertyTextureGroup>,
}

/// Record of the state of a [`MaterialLibrary`].
#[derive(Clone, Debug)]
pub struct MaterialLibraryState {
    material_ids: HashSet<MaterialID>,
    material_property_texture_group_ids: HashSet<MaterialPropertyTextureGroupID>,
}

/// Input required for using a material in a shader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialShaderInput {
    Fixed(FixedMaterialTextureBindings),
    Physical(PhysicalMaterialTextureBindings),
}

impl MaterialSpecification {
    /// Creates a new material specification with the given vertex attribute
    /// requirements, input and output render attachment descriptions,
    /// material-specific resources, untextured material property types, render
    /// pass hints and shader input.
    pub fn new(
        vertex_attribute_requirements: VertexAttributeSet,
        instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
        instance_feature_flags: MaterialInstanceFeatureFlags,
        material_specific_resources: Option<GPUResourceGroup>,
        shader_input: MaterialShaderInput,
    ) -> Self {
        Self {
            vertex_attribute_requirements,
            instance_feature_type_ids,
            instance_feature_flags,
            material_specific_resources,
            shader_input,
        }
    }

    /// Returns a [`VertexAttributeSet`] encoding the vertex attributes required
    /// to be available in any mesh using the material.
    pub fn vertex_attribute_requirements(&self) -> VertexAttributeSet {
        self.vertex_attribute_requirements
    }

    /// Returns the IDs of the material property types used
    /// for the material.
    pub fn instance_feature_type_ids(&self) -> &[InstanceFeatureTypeID] {
        &self.instance_feature_type_ids
    }

    /// Returns the flags for the material's set of per-instance features.
    pub fn instance_feature_flags(&self) -> MaterialInstanceFeatureFlags {
        self.instance_feature_flags
    }

    /// Returns a reference to the [`GPUResourceGroup`] of material-specific
    /// resources of the material, or [`None`] if the material has no
    /// material-specific resources.
    pub fn material_specific_resources(&self) -> Option<&GPUResourceGroup> {
        self.material_specific_resources.as_ref()
    }

    /// Returns the input required for using the material in a shader.
    pub fn shader_input(&self) -> &MaterialShaderInput {
        &self.shader_input
    }
}

impl MaterialPropertyTextureGroup {
    /// Creates a new group of material property textures comprising the
    /// textures with the given IDs.
    ///
    /// # Panics
    /// If the given list of texture IDs is empty.
    pub fn new(
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        texture_ids: Vec<TextureID>,
        label: String,
    ) -> Result<Self> {
        let (bind_group_layout, bind_group) = Self::create_texture_bind_group_and_layout(
            graphics_device.device(),
            assets,
            &texture_ids,
            &label,
        )?;

        Ok(Self {
            texture_ids,
            bind_group_layout,
            bind_group,
        })
    }

    /// Returns the binding that will be used for the texture at the given index
    /// and its sampler in the bind group.
    pub const fn get_texture_and_sampler_bindings(texture_idx: usize) -> (u32, u32) {
        let texture_binding = (2 * texture_idx) as u32;
        let sampler_binding = texture_binding + 1;
        (texture_binding, sampler_binding)
    }

    /// Returns the IDs of the managed material property textures.
    pub fn texture_ids(&self) -> &[TextureID] {
        &self.texture_ids
    }

    /// Returns a reference to the bind group layout for the group of textures.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns a reference to the bind group for the group of textures.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn create_texture_bind_group_and_layout(
        device: &wgpu::Device,
        assets: &Assets,
        texture_ids: &[TextureID],
        label: &str,
    ) -> Result<(wgpu::BindGroupLayout, wgpu::BindGroup)> {
        let n_entries = 2 * texture_ids.len();

        let mut bind_group_layout_entries = Vec::with_capacity(n_entries);
        let mut bind_group_entries = Vec::with_capacity(n_entries);

        for (idx, texture_id) in texture_ids.iter().enumerate() {
            let texture = assets
                .textures
                .get(texture_id)
                .ok_or_else(|| anyhow!("Texture {} missing from assets", texture_id))?;

            let sampler = assets
                .samplers
                .get(&texture.sampler_id().ok_or_else(|| {
                    anyhow!("Material texture {} has no associated sampler", texture_id)
                })?)
                .ok_or_else(|| anyhow!("Sampler for texture {} missing from assets", texture_id))?;

            let (texture_binding, sampler_binding) = Self::get_texture_and_sampler_bindings(idx);

            bind_group_layout_entries.push(
                texture
                    .create_bind_group_layout_entry(texture_binding, wgpu::ShaderStages::FRAGMENT),
            );
            bind_group_layout_entries.push(
                sampler
                    .create_bind_group_layout_entry(sampler_binding, wgpu::ShaderStages::FRAGMENT),
            );

            bind_group_entries.push(texture.create_bind_group_entry(texture_binding));
            bind_group_entries.push(sampler.create_bind_group_entry(sampler_binding));
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &bind_group_layout_entries,
            label: Some(&format!("{} bind group layout", label)),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &bind_group_entries,
            label: Some(&format!("{} bind group", label)),
        });

        Ok((bind_group_layout, bind_group))
    }
}

impl MaterialLibrary {
    /// Creates a new empty material library.
    pub fn new() -> Self {
        Self {
            material_specifications: HashMap::new(),
            material_property_texture_groups: HashMap::new(),
        }
    }

    /// Records the current state of the library and returns it as a
    /// [`MaterialLibraryState`].
    pub fn record_state(&self) -> MaterialLibraryState {
        MaterialLibraryState {
            material_ids: self.material_specifications.keys().cloned().collect(),
            material_property_texture_group_ids: self
                .material_property_texture_groups
                .keys()
                .cloned()
                .collect(),
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`MaterialSpecification`]s.
    pub fn material_specifications(&self) -> &HashMap<MaterialID, MaterialSpecification> {
        &self.material_specifications
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`MaterialPropertyTextureGroup`]s.
    pub fn material_property_texture_groups(
        &self,
    ) -> &HashMap<MaterialPropertyTextureGroupID, MaterialPropertyTextureGroup> {
        &self.material_property_texture_groups
    }

    /// Returns the specification of the material with the given ID, or [`None`]
    /// if the material does not exist.
    pub fn get_material_specification(
        &self,
        material_id: MaterialID,
    ) -> Option<&MaterialSpecification> {
        self.material_specifications.get(&material_id)
    }

    /// Returns the material property texture group with the given ID, or
    /// [`None`] if the texture group does not exist.
    pub fn get_material_property_texture_group(
        &self,
        texture_group_id: MaterialPropertyTextureGroupID,
    ) -> Option<&MaterialPropertyTextureGroup> {
        self.material_property_texture_groups.get(&texture_group_id)
    }

    /// Returns a hashmap entry for the specification of the material with the
    /// given ID.
    pub fn material_specification_entry(
        &mut self,
        material_id: MaterialID,
    ) -> Entry<'_, MaterialID, MaterialSpecification> {
        self.material_specifications.entry(material_id)
    }

    /// Returns a hashmap entry for the material property texture group with the
    /// given ID.
    pub fn material_property_texture_group_entry(
        &mut self,
        texture_group_id: MaterialPropertyTextureGroupID,
    ) -> Entry<'_, MaterialPropertyTextureGroupID, MaterialPropertyTextureGroup> {
        self.material_property_texture_groups
            .entry(texture_group_id)
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

    /// Includes the given material property texture group in the library under
    /// the given ID. If a texture group with the same ID exists, it will be
    /// overwritten.
    pub fn add_material_property_texture_group(
        &mut self,
        texture_group_id: MaterialPropertyTextureGroupID,
        texture_group: MaterialPropertyTextureGroup,
    ) {
        self.material_property_texture_groups
            .insert(texture_group_id, texture_group);
    }

    /// Removes the materials and material property texture groups that are not
    /// part of the given library state.
    pub fn reset_to_state(&mut self, state: &MaterialLibraryState) {
        self.material_specifications
            .retain(|material_id, _| state.material_ids.contains(material_id));
        self.material_property_texture_groups
            .retain(|texture_group_id, _| {
                state
                    .material_property_texture_group_ids
                    .contains(texture_group_id)
            });
    }
}

impl MaterialID {
    /// Creates an ID that does not represent a valid material.
    pub fn not_applicable() -> Self {
        Self(StringHash64::zeroed())
    }

    /// Returns `true` if this ID does not represent a valid material.
    pub fn is_not_applicable(&self) -> bool {
        self.0 == StringHash64::zeroed()
    }
}

impl MaterialPropertyTextureGroupID {
    /// Generates a material property texture group ID that will always be the
    /// same for a specific ordered group of texture IDs.
    pub fn from_texture_ids(texture_ids: &[TextureID]) -> Self {
        Self(hash64!(format!(
            "{}",
            texture_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(" - ")
        )))
    }

    /// Creates an ID representing an empty texture group.
    pub fn empty() -> Self {
        Self(StringHash64::zeroed())
    }

    /// Returns `true` if this ID represents an empty texture group.
    pub fn is_empty(&self) -> bool {
        self.0 == StringHash64::zeroed()
    }
}

impl MaterialHandle {
    /// Creates a new handle for a material with the given IDs for the
    /// [`MaterialSpecification`](crate::material::MaterialSpecification),
    /// per-instance material data and textures (the latter two are optional) .
    pub fn new(
        material_id: MaterialID,
        material_property_feature_id: Option<InstanceFeatureID>,
        material_property_texture_group_id: Option<MaterialPropertyTextureGroupID>,
    ) -> Self {
        let material_property_feature_id =
            material_property_feature_id.unwrap_or_else(InstanceFeatureID::not_applicable);
        let material_property_texture_group_id = material_property_texture_group_id
            .unwrap_or_else(MaterialPropertyTextureGroupID::empty);
        Self {
            material_id,
            material_property_feature_id,
            material_property_texture_group_id,
        }
    }

    /// Creates a handle that does not represent a valid material.
    pub fn not_applicable() -> Self {
        Self {
            material_id: MaterialID::not_applicable(),
            material_property_feature_id: InstanceFeatureID::not_applicable(),
            material_property_texture_group_id: MaterialPropertyTextureGroupID::empty(),
        }
    }

    /// Returns `true` if this handle does not represent a valid material.
    pub fn is_not_applicable(&self) -> bool {
        self.material_id.is_not_applicable()
    }

    /// Returns the ID of the material.
    pub fn material_id(&self) -> MaterialID {
        self.material_id
    }

    /// Returns the ID of the entry for the per-instance material properties in
    /// the [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage), or
    /// [`None`] if there are no untextured per-instance material properties.
    pub fn material_property_feature_id(&self) -> Option<InstanceFeatureID> {
        if self.material_property_feature_id.is_not_applicable() {
            None
        } else {
            Some(self.material_property_feature_id)
        }
    }

    /// Returns the ID of the material property texture group, or [`None`] if no
    /// material properties are textured.
    pub fn material_property_texture_group_id(&self) -> Option<MaterialPropertyTextureGroupID> {
        if self.material_property_texture_group_id.is_empty() {
            None
        } else {
            Some(self.material_property_texture_group_id)
        }
    }

    /// Computes a unique hash for this material handle.
    pub fn compute_hash(&self) -> Hash64 {
        let mut hash = self.material_id.0.hash();

        if !self.material_property_texture_group_id.is_empty() {
            hash = impact_math::compute_hash_64_of_two_hash_64(
                hash,
                self.material_property_texture_group_id.0.hash(),
            );
        }

        hash
    }
}

impl fmt::Display for MaterialHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{material: {}{}}}",
            if self.material_id.is_not_applicable() {
                "N/A".to_string()
            } else {
                self.material_id.to_string()
            },
            if self.material_property_texture_group_id.is_empty() {
                String::new()
            } else {
                format!(", textures: {}", self.material_property_texture_group_id)
            },
        )
    }
}

impl VertexAttributeSet {
    pub const FOR_LIGHT_SHADING: Self = Self::POSITION.union(Self::NORMAL_VECTOR);
    pub const FOR_TEXTURED_LIGHT_SHADING: Self =
        Self::FOR_LIGHT_SHADING.union(Self::TEXTURE_COORDS);
}
