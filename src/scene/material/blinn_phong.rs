//! Materials using the Blinn-Phong reflection model.

use crate::{
    geometry::InstanceFeature,
    impl_InstanceFeature_for_VertexBufferable,
    rendering::{self, fre, TextureID, VertexBufferable},
    scene::{
        BlinnPhongComp, DiffuseTexturedBlinnPhongComp, InstanceFeatureManager, MaterialComp,
        MaterialID, MaterialLibrary, MaterialSpecification, RGBColor, ShaderID, ShaderLibrary,
        TexturedBlinnPhongComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ComponentManager, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;

/// Material using the Blinn-Phong reflection model, with
/// fixed ambient, diffuse and specular colors and fixed
/// shininess and alpha.
///
/// This type stores the material's per-instance data that will
/// be sent to the GPU. It implements [`InstanceFeature`], and
/// can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct BlinnPhongMaterial {
    ambient_color: RGBColor,
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    shininess: fre,
    alpha: fre,
}

/// Material using the Blinn-Phong reflection model, with
/// textured diffuse colors, fixed ambient and specular
/// colors and fixed shininess and alpha.
///
/// This type stores the material's per-instance data that will
/// be sent to the GPU. It implements [`InstanceFeature`], and
/// can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct DiffuseTexturedBlinnPhongMaterial {
    ambient_color: RGBColor,
    specular_color: RGBColor,
    shininess: fre,
    alpha: fre,
}

/// Material using the Blinn-Phong reflection model, with
/// textured diffuse and specular colors, fixed ambient
/// color and fixed shininess and alpha.
///
/// This type stores the material's per-instance data that will
/// be sent to the GPU. It implements [`InstanceFeature`], and
/// can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedBlinnPhongMaterial {
    ambient_color: RGBColor,
    shininess: fre,
    alpha: fre,
}

lazy_static! {
    static ref BLINN_PHONG_MATERIAL_ID: MaterialID = MaterialID(hash64!("BlinnPhongMaterial"));
}

impl BlinnPhongMaterial {
    /// Registers this material as a feature type in the given
    /// instance feature manager, prepares a shader for the
    /// material and adds the material specification to the given
    /// material library. Because this material uses no textures,
    /// the same material specification can be used for all
    /// instances using the material.
    pub fn register(
        shader_library: &mut ShaderLibrary,
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        // Construct shader with correct features and get ID (create ShaderBuilder).
        // Shader ID is added to assets if not present.

        let specification = MaterialSpecification::new(
            ShaderID(hash64!("BlinnPhongMaterial")),
            Vec::new(),
            vec![Self::FEATURE_TYPE_ID],
        );
        material_library.add_material_specification(*BLINN_PHONG_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with components represented by the
    /// given component manager has the component for this material, and
    /// if so, registers the material in the given instance feature
    /// manager and adds the appropriate material component to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &mut InstanceFeatureManager,
        component_manager: &mut ComponentManager<'_>,
    ) {
        setup!(
            component_manager,
            |blinn_phong: &BlinnPhongComp| -> MaterialComp {
                let material = Self {
                    ambient_color: blinn_phong.ambient,
                    diffuse_color: blinn_phong.diffuse,
                    specular_color: blinn_phong.specular,
                    shininess: blinn_phong.shininess,
                    alpha: blinn_phong.alpha,
                };

                let feature_id = instance_feature_manager
                    .get_storage_mut::<Self>()
                    .expect("Missing storage for BlinnPhongMaterial features")
                    .add_feature(&material);

                MaterialComp {
                    id: *BLINN_PHONG_MATERIAL_ID,
                    feature_id,
                }
            },
            ![MaterialComp]
        );
    }
}

impl DiffuseTexturedBlinnPhongMaterial {
    /// Registers this material as a feature type in the given
    /// instance feature manager and prepares a shader for the
    /// material. No material specification is created at this
    /// point, because a separate specification will be needed
    /// for every instance that uses a specific texture.
    pub fn register(
        shader_library: &mut ShaderLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        // Construct shader with correct features and get ID (create ShaderBuilder).
        // Shader ID is added to assets if not present.
    }

    /// Checks if the entity-to-be with components represented by the
    /// given component manager has the component for this material, and
    /// if so, adds the appropriate material specification to the material
    /// library if not present, registers the material in the given instance
    /// feature manager and adds the appropriate material component to the
    /// entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &mut InstanceFeatureManager,
        material_library: &mut MaterialLibrary,
        component_manager: &mut ComponentManager<'_>,
    ) {
        setup!(
            component_manager,
            |blinn_phong: &DiffuseTexturedBlinnPhongComp| -> MaterialComp {
                let texture_ids = [blinn_phong.diffuse];

                let material_id =
                    generate_material_id("DiffuseTexturedBlinnPhongComp", &texture_ids);

                // Add a new specification if none with the same material
                // type and textures already exist
                material_library
                    .material_specification_entry(material_id)
                    .or_insert_with(|| {
                        MaterialSpecification::new(
                            ShaderID(hash64!("DiffuseTexturedBlinnPhongComp")),
                            texture_ids.to_vec(),
                            vec![Self::FEATURE_TYPE_ID],
                        )
                    });

                let material = Self {
                    ambient_color: blinn_phong.ambient,
                    specular_color: blinn_phong.specular,
                    shininess: blinn_phong.shininess,
                    alpha: blinn_phong.alpha,
                };

                let feature_id = instance_feature_manager
                    .get_storage_mut::<Self>()
                    .expect("Missing storage for DiffuseTexturedBlinnPhongMaterial features")
                    .add_feature(&material);

                MaterialComp {
                    id: material_id,
                    feature_id,
                }
            },
            ![MaterialComp]
        );
    }
}

impl TexturedBlinnPhongMaterial {
    /// Registers this material as a feature type in the given
    /// instance feature manager and prepares a shader for the
    /// material. No material specification is created at this
    /// point, because a separate specification will be needed
    /// for every instance that uses a specific texture.
    pub fn register(
        shader_library: &mut ShaderLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        // Construct shader with correct features and get ID (create ShaderBuilder).
        // Shader ID is added to assets if not present.
    }

    /// Checks if the entity-to-be with components represented by the
    /// given component manager has the component for this material, and
    /// if so, adds the appropriate material specification to the material
    /// library if not present, registers the material in the given instance
    /// feature manager and adds the appropriate material component to the
    /// entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &mut InstanceFeatureManager,
        material_library: &mut MaterialLibrary,
        component_manager: &mut ComponentManager<'_>,
    ) {
        setup!(
            component_manager,
            |blinn_phong: &TexturedBlinnPhongComp| -> MaterialComp {
                let texture_ids = [blinn_phong.diffuse, blinn_phong.specular];

                let material_id = generate_material_id("TexturedBlinnPhongMaterial", &texture_ids);

                // Add a new specification if none with the same material
                // type and textures already exist
                material_library
                    .material_specification_entry(material_id)
                    .or_insert_with(|| {
                        MaterialSpecification::new(
                            ShaderID(hash64!("TexturedBlinnPhongMaterial")),
                            texture_ids.to_vec(),
                            vec![Self::FEATURE_TYPE_ID],
                        )
                    });

                let material = Self {
                    ambient_color: blinn_phong.ambient,
                    shininess: blinn_phong.shininess,
                    alpha: blinn_phong.alpha,
                };

                let feature_id = instance_feature_manager
                    .get_storage_mut::<Self>()
                    .expect("Missing storage for TexturedBlinnPhongMaterial features")
                    .add_feature(&material);

                MaterialComp {
                    id: material_id,
                    feature_id,
                }
            },
            ![MaterialComp]
        );
    }
}

impl VertexBufferable for BlinnPhongMaterial {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        rendering::create_vertex_buffer_layout_for_vertex::<Self>(
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32, 4 => Float32],
        );
}

impl VertexBufferable for DiffuseTexturedBlinnPhongMaterial {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        rendering::create_vertex_buffer_layout_for_vertex::<Self>(
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32, 3 => Float32],
        );
}

impl VertexBufferable for TexturedBlinnPhongMaterial {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        rendering::create_vertex_buffer_layout_for_vertex::<Self>(
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32],
        );
}

impl_InstanceFeature_for_VertexBufferable!(BlinnPhongMaterial);
impl_InstanceFeature_for_VertexBufferable!(DiffuseTexturedBlinnPhongMaterial);
impl_InstanceFeature_for_VertexBufferable!(TexturedBlinnPhongMaterial);

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
