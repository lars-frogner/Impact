//! Materials using the Blinn-Phong reflection model.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    geometry::{InstanceFeature, VertexAttributeSet},
    impl_InstanceFeature,
    rendering::{
        fre, BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput,
        InstanceFeatureShaderInput, MaterialRenderResourceManager, MaterialShaderInput,
    },
    scene::{
        BlinnPhongComp, DiffuseTexturedBlinnPhongComp, InstanceFeatureManager, MaterialComp,
        MaterialID, MaterialLibrary, MaterialSpecification, RGBColor,
        RenderResourcesDesynchronized, TexturedBlinnPhongComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;
use std::sync::RwLock;

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
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet =
        VertexAttributeSet::POSITION.union(VertexAttributeSet::NORMAL_VECTOR);

    const MATERIAL_SHADER_INPUT: MaterialShaderInput = MaterialShaderInput::BlinnPhong(None);

    /// Registers this material as a feature type in the given
    /// instance feature manager and adds the material specification
    /// to the given material library. Because this material uses no
    /// textures, the same material specification can be used for all
    /// instances using the material.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
            Vec::new(),
            vec![Self::FEATURE_TYPE_ID],
            Self::MATERIAL_SHADER_INPUT,
        );
        material_library.add_material_specification(*BLINN_PHONG_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with the given components has the component
    /// for this material, and if so, registers the material in the given
    /// instance feature manager and adds the appropriate material component
    /// to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
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
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet = VertexAttributeSet::POSITION
        .union(VertexAttributeSet::NORMAL_VECTOR)
        .union(VertexAttributeSet::TEXTURE_COORDS);

    const MATERIAL_SHADER_INPUT: MaterialShaderInput =
        MaterialShaderInput::BlinnPhong(Some(BlinnPhongTextureShaderInput {
            diffuse_texture_and_sampler_bindings:
                MaterialRenderResourceManager::get_texture_and_sampler_bindings(0),
            specular_texture_and_sampler_bindings: None,
        }));

    /// Registers this material as a feature type in the given
    /// instance feature manager. No material specification is
    /// created at this point, because a separate specification
    /// will be needed for every instance that uses a specific
    /// texture.
    pub fn register(instance_feature_manager: &mut InstanceFeatureManager) {
        instance_feature_manager.register_feature_type::<Self>();
    }

    /// Checks if the entity-to-be with the given components has the component
    /// for this material, and if so, adds the appropriate material
    /// specification to the material library if not present, registers the
    /// material in the given instance feature manager and adds the appropriate
    /// material component to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        material_library: &RwLock<MaterialLibrary>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
                let mut material_library = material_library.write().unwrap();
            },
            components,
            |blinn_phong: &DiffuseTexturedBlinnPhongComp| -> MaterialComp {
                let texture_ids = [blinn_phong.diffuse];

                let material_id =
                    super::generate_material_id("DiffuseTexturedBlinnPhongComp", &texture_ids);

                // Add a new specification if none with the same material
                // type and textures already exist
                material_library
                    .material_specification_entry(material_id)
                    .or_insert_with(|| {
                        MaterialSpecification::new(
                            Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
                            texture_ids.to_vec(),
                            vec![Self::FEATURE_TYPE_ID],
                            Self::MATERIAL_SHADER_INPUT,
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
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet = VertexAttributeSet::POSITION
        .union(VertexAttributeSet::NORMAL_VECTOR)
        .union(VertexAttributeSet::TEXTURE_COORDS);

    const MATERIAL_SHADER_INPUT: MaterialShaderInput =
        MaterialShaderInput::BlinnPhong(Some(BlinnPhongTextureShaderInput {
            diffuse_texture_and_sampler_bindings:
                MaterialRenderResourceManager::get_texture_and_sampler_bindings(0),
            specular_texture_and_sampler_bindings: Some(
                MaterialRenderResourceManager::get_texture_and_sampler_bindings(1),
            ),
        }));

    /// Registers this material as a feature type in the given
    /// instance feature manager. No material specification is
    /// created at this point, because a separate specification
    /// will be needed for every instance that uses a specific
    /// texture.
    pub fn register(instance_feature_manager: &mut InstanceFeatureManager) {
        instance_feature_manager.register_feature_type::<Self>();
    }

    /// Checks if the entity-to-be with the given components has the component
    /// for this material, and if so, adds the appropriate material specification
    /// to the material library if not present, registers the material in the given
    /// instance feature manager and adds the appropriate material component to the
    /// entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        material_library: &RwLock<MaterialLibrary>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
                let mut material_library = material_library.write().unwrap();
            },
            components,
            |blinn_phong: &TexturedBlinnPhongComp| -> MaterialComp {
                let texture_ids = [blinn_phong.diffuse, blinn_phong.specular];

                let material_id =
                    super::generate_material_id("TexturedBlinnPhongMaterial", &texture_ids);

                // Add a new specification if none with the same material
                // type and textures already exist
                material_library
                    .material_specification_entry(material_id)
                    .or_insert_with(|| {
                        MaterialSpecification::new(
                            Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
                            texture_ids.to_vec(),
                            vec![Self::FEATURE_TYPE_ID],
                            Self::MATERIAL_SHADER_INPUT,
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

impl_InstanceFeature!(
    BlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
        MATERIAL_VERTEX_BINDING_START + 4 => Float32
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        ambient_color_location: MATERIAL_VERTEX_BINDING_START,
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        shininess_location: MATERIAL_VERTEX_BINDING_START + 3,
        alpha_location: MATERIAL_VERTEX_BINDING_START + 4,
    })
);

impl_InstanceFeature!(
    DiffuseTexturedBlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        ambient_color_location: MATERIAL_VERTEX_BINDING_START,
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        shininess_location: MATERIAL_VERTEX_BINDING_START + 2,
        alpha_location: MATERIAL_VERTEX_BINDING_START + 3,
    })
);

impl_InstanceFeature!(
    TexturedBlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        ambient_color_location: MATERIAL_VERTEX_BINDING_START,
        diffuse_color_location: None,
        specular_color_location: None,
        shininess_location: MATERIAL_VERTEX_BINDING_START + 1,
        alpha_location: MATERIAL_VERTEX_BINDING_START + 2,
    })
);
