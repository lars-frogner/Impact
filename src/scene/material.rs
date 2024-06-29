//! Management of materials.

mod ambient_occlusion;
mod blinn_phong;
mod components;
mod features;
mod fixed;
mod gaussian_blur;
mod microfacet;
mod prepass;
mod skybox;
mod tone_mapping;
mod vertex_color;

pub use ambient_occlusion::{
    create_ambient_occlusion_application_material, create_ambient_occlusion_computation_material,
    MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT,
};
pub use blinn_phong::add_blinn_phong_material_component_for_entity;
pub use components::{
    register_material_components, AlbedoComp, AlbedoTextureComp, EmissiveLuminanceComp,
    EmissiveLuminanceTextureComp, FixedColorComp, FixedTextureComp, MaterialComp,
    MicrofacetDiffuseReflectionComp, MicrofacetSpecularReflectionComp, NormalMapComp,
    ParallaxMapComp, RoughnessComp, RoughnessTextureComp, SkyboxComp, SpecularReflectanceComp,
    SpecularReflectanceTextureComp, VertexColorComp,
};
pub use features::{
    create_material_feature, TexturedEmissiveMaterialFeature, TexturedMaterialFeature,
    TexturedParallaxMappingEmissiveMaterialFeature, TexturedParallaxMappingMaterialFeature,
    UniformDiffuseEmissiveMaterialFeature, UniformDiffuseMaterialFeature,
    UniformDiffuseParallaxMappingEmissiveMaterialFeature,
    UniformDiffuseParallaxMappingMaterialFeature,
    UniformDiffuseUniformSpecularEmissiveMaterialFeature,
    UniformDiffuseUniformSpecularMaterialFeature,
    UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature,
    UniformDiffuseUniformSpecularParallaxMappingMaterialFeature,
    UniformSpecularEmissiveMaterialFeature, UniformSpecularMaterialFeature,
    UniformSpecularParallaxMappingEmissiveMaterialFeature,
    UniformSpecularParallaxMappingMaterialFeature,
};
pub use fixed::{FixedColorMaterial, FixedTextureMaterial};
pub use gaussian_blur::{
    create_gaussian_blur_material, GaussianBlurDirection, GaussianBlurSamples,
    MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS,
};
pub use microfacet::{add_microfacet_material_component_for_entity, setup_microfacet_material};
pub use prepass::create_prepass_material;
pub use skybox::add_skybox_material_component_for_entity;
pub use tone_mapping::{create_tone_mapping_material, ToneMapping};
pub use vertex_color::VertexColorMaterial;

use crate::{
    geometry::{InstanceFeatureID, InstanceFeatureTypeID, VertexAttributeSet},
    rendering::{
        fre, MaterialShaderInput, RenderAttachmentQuantitySet, RenderPassHints, TextureID,
        UniformBufferable,
    },
    scene::InstanceFeatureManager,
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, stringhash64_newtype, AlignedByteVec, Alignment, Hash64, StringHash64};
use nalgebra::Vector3;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt, mem,
};

/// A color with RGB components.
pub type RGBColor = Vector3<fre>;

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

/// A handle for a material, containing the IDs for the pieces of data holding
/// information about the material.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct MaterialHandle {
    /// The ID of the material's [`MaterialSpecification`](crate::scene::MaterialSpecification).
    material_id: MaterialID,
    /// The ID of the entry for the material's per-instance material properties
    /// in the
    /// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage) (may
    /// be N/A).
    material_property_feature_id: InstanceFeatureID,
    /// The ID of the material's
    /// [`MaterialPropertyTextureSet`](crate::scene::MaterialPropertyTextureSet)
    /// (may represent an empty set).
    material_property_texture_set_id: MaterialPropertyTextureSetID,
}

/// A material description specifying a material's set of required vertex
/// attributes, untextured per-material properties (as instance features),
/// associated render attachments, shader input and optionally fixed material
/// resources.
#[derive(Clone, Debug)]
pub struct MaterialSpecification {
    vertex_attribute_requirements_for_mesh: VertexAttributeSet,
    vertex_attribute_requirements_for_shader: VertexAttributeSet,
    input_render_attachment_quantities: RenderAttachmentQuantitySet,
    output_render_attachment_quantities: RenderAttachmentQuantitySet,
    fixed_resources: Option<FixedMaterialResources>,
    instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
    render_pass_hints: RenderPassHints,
    shader_input: MaterialShaderInput,
}

/// Container for rendering resources for a specific material type that are the
/// same for all uses of the material.
#[derive(Clone, Debug)]
pub struct FixedMaterialResources {
    uniform_bytes: AlignedByteVec,
    uniform_bind_group_layout_entry: wgpu::BindGroupLayoutEntry,
}

/// Specifies a set of textures used for textured material properties.
#[derive(Clone, Debug)]
pub struct MaterialPropertyTextureSet {
    texture_ids: Vec<TextureID>,
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
    /// Creates a new material specification with the given fixed resources and
    /// untextured material property types.
    pub fn new(
        vertex_attribute_requirements_for_mesh: VertexAttributeSet,
        vertex_attribute_requirements_for_shader: VertexAttributeSet,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
        fixed_resources: Option<FixedMaterialResources>,
        instance_feature_type_ids: Vec<InstanceFeatureTypeID>,
        render_pass_hints: RenderPassHints,
        shader_input: MaterialShaderInput,
    ) -> Self {
        Self {
            vertex_attribute_requirements_for_mesh,
            vertex_attribute_requirements_for_shader,
            input_render_attachment_quantities,
            output_render_attachment_quantities,
            fixed_resources,
            instance_feature_type_ids,
            render_pass_hints,
            shader_input,
        }
    }

    /// Returns a [`VertexAttributeSet`] encoding the vertex attributes required
    /// to be available in any mesh using the material.
    pub fn vertex_attribute_requirements_for_mesh(&self) -> VertexAttributeSet {
        self.vertex_attribute_requirements_for_mesh
    }

    /// Returns a [`VertexAttributeSet`] encoding the vertex attributes that
    /// will be used in the material's shader.
    pub fn vertex_attribute_requirements_for_shader(&self) -> VertexAttributeSet {
        self.vertex_attribute_requirements_for_shader
    }

    /// Returns a [`RenderAttachmentQuantitySet`] encoding the quantities whose
    /// render attachment textures are required as input for rendering with the
    /// material.
    pub fn input_render_attachment_quantities(&self) -> RenderAttachmentQuantitySet {
        self.input_render_attachment_quantities
    }

    /// Returns a [`RenderAttachmentQuantitySet`] encoding the quantities whose
    /// render attachment textures are written to when rendering with the
    /// material.
    pub fn output_render_attachment_quantities(&self) -> RenderAttachmentQuantitySet {
        self.output_render_attachment_quantities
    }

    /// Returns a reference to the [`FixedMaterialResources`] of the material,
    /// or [`None`] if the material has no fixed resources.
    pub fn fixed_resources(&self) -> Option<&FixedMaterialResources> {
        self.fixed_resources.as_ref()
    }

    /// Returns the IDs of the material property types used
    /// for the material.
    pub fn instance_feature_type_ids(&self) -> &[InstanceFeatureTypeID] {
        &self.instance_feature_type_ids
    }

    /// Returns the render pass hints for the material.
    pub fn render_pass_hints(&self) -> RenderPassHints {
        self.render_pass_hints
    }

    /// Returns the input required for using the material in a shader.
    pub fn shader_input(&self) -> &MaterialShaderInput {
        &self.shader_input
    }
}

impl FixedMaterialResources {
    /// Binding index for the uniform buffer with fixed material data.
    pub const UNIFORM_BINDING: u32 = 0;

    /// Creates a new container for fixed material resources holding the given
    /// piece of uniform bufferable data.
    pub fn new<U>(resource_uniform: &U) -> Self
    where
        U: UniformBufferable,
    {
        assert_ne!(
            mem::size_of::<U>(),
            0,
            "Tried to create fixed material resources from zero-sized uniform"
        );

        let uniform_data = AlignedByteVec::copied_from_slice(
            Alignment::of::<U>(),
            bytemuck::bytes_of(resource_uniform),
        );

        let uniform_bind_group_layout_entry =
            U::create_bind_group_layout_entry(Self::UNIFORM_BINDING);

        Self {
            uniform_bytes: uniform_data,
            uniform_bind_group_layout_entry,
        }
    }

    /// Returns a byte view of the piece of fixed material data that will reside
    /// in a uniform buffer.
    pub fn uniform_bytes(&self) -> &[u8] {
        &self.uniform_bytes
    }

    /// Returns the bind group layout entry for the fixed material uniform data.
    pub fn uniform_bind_group_layout_entry(&self) -> &wgpu::BindGroupLayoutEntry {
        &self.uniform_bind_group_layout_entry
    }
}

impl MaterialPropertyTextureSet {
    /// Creates a new material property texture set for the textures with the
    /// given IDs.
    ///
    /// # Panics
    /// If the given list of texture IDs is empty.
    pub fn new(texture_ids: Vec<TextureID>) -> Self {
        assert!(!texture_ids.is_empty());
        Self { texture_ids }
    }

    /// Returns the IDs of the textures in the texture set.
    pub fn texture_ids(&self) -> &[TextureID] {
        &self.texture_ids
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

    pub fn register_materials(&mut self, instance_feature_manager: &mut InstanceFeatureManager) {
        instance_feature_manager.register_feature_type::<TexturedMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformDiffuseMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformSpecularMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseUniformSpecularMaterialFeature>();
        instance_feature_manager.register_feature_type::<TexturedParallaxMappingMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseParallaxMappingMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformSpecularParallaxMappingMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseUniformSpecularParallaxMappingMaterialFeature>();
        instance_feature_manager.register_feature_type::<TexturedEmissiveMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformDiffuseEmissiveMaterialFeature>();
        instance_feature_manager.register_feature_type::<UniformSpecularEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseUniformSpecularEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<TexturedParallaxMappingEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseParallaxMappingEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformSpecularParallaxMappingEmissiveMaterialFeature>();
        instance_feature_manager
            .register_feature_type::<UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature>();

        VertexColorMaterial::register(self);
        FixedColorMaterial::register(self, instance_feature_manager);
        FixedTextureMaterial::register(self);
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
                .join(" - ")
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

impl MaterialHandle {
    /// Creates a new handle for a material with the given IDs for the
    /// [`MaterialSpecification`](crate::scene::MaterialSpecification),
    /// per-instance material data and textures (the latter two are optional) .
    pub fn new(
        material_id: MaterialID,
        material_property_feature_id: Option<InstanceFeatureID>,
        material_property_texture_set_id: Option<MaterialPropertyTextureSetID>,
    ) -> Self {
        let material_property_feature_id =
            material_property_feature_id.unwrap_or_else(InstanceFeatureID::not_applicable);
        let material_property_texture_set_id =
            material_property_texture_set_id.unwrap_or_else(MaterialPropertyTextureSetID::empty);
        Self {
            material_id,
            material_property_feature_id,
            material_property_texture_set_id,
        }
    }

    /// Creates a handle that does not represent a valid material.
    pub fn not_applicable() -> Self {
        Self {
            material_id: MaterialID::not_applicable(),
            material_property_feature_id: InstanceFeatureID::not_applicable(),
            material_property_texture_set_id: MaterialPropertyTextureSetID::empty(),
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
    /// the [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage),
    /// or [`None`] if there are no untextured per-instance material properties.
    pub fn material_property_feature_id(&self) -> Option<InstanceFeatureID> {
        if self.material_property_feature_id.is_not_applicable() {
            None
        } else {
            Some(self.material_property_feature_id)
        }
    }

    /// Returns the ID of the material property texture set, or [`None`] if no
    /// material properties are textured.
    pub fn material_property_texture_set_id(&self) -> Option<MaterialPropertyTextureSetID> {
        if self.material_property_texture_set_id.is_empty() {
            None
        } else {
            Some(self.material_property_texture_set_id)
        }
    }

    /// Computes a unique hash for this material handle.
    pub fn compute_hash(&self) -> Hash64 {
        let mut hash = self.material_id.0.hash();

        if !self.material_property_texture_set_id.is_empty() {
            hash = impact_utils::compute_hash_64_of_two_hash_64(
                hash,
                self.material_property_texture_set_id.0.hash(),
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
            if self.material_property_texture_set_id.is_empty() {
                String::new()
            } else {
                format!(", textures: {}", self.material_property_texture_set_id)
            },
        )
    }
}
