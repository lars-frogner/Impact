//! Materials using the Blinn-Phong reflection model.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    geometry::{InstanceFeature, VertexAttributeSet},
    impl_InstanceFeature,
    rendering::{
        fre, BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput,
        InstanceFeatureShaderInput, MaterialPropertyTextureManager, MaterialShaderInput,
    },
    scene::{
        DiffuseColorComp, DiffuseTextureComp, InstanceFeatureManager, MaterialComp, MaterialID,
        MaterialLibrary, MaterialPropertyTextureSet, MaterialPropertyTextureSetID,
        MaterialSpecification, MicrofacetDiffuseReflection, MicrofacetSpecularReflection, RGBColor,
        RenderResourcesDesynchronized, RoughnessComp, SpecularColorComp, SpecularTextureComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;
use std::sync::RwLock;

/// Material using the Blinn-Phong reflection model, with fixed diffuse and
/// specular colors and fixed shininess.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct BlinnPhongMaterial {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    shininess: fre,
}

/// Material using the Blinn-Phong reflection model, with textured diffuse
/// colors, fixed specular color and fixed shininess.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct DiffuseTexturedBlinnPhongMaterial {
    specular_color: RGBColor,
    shininess: fre,
}

/// Material using the Blinn-Phong reflection model, with textured diffuse and
/// specular colors and fixed shininess.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedBlinnPhongMaterial {
    shininess: fre,
}

lazy_static! {
    static ref BLINN_PHONG_MATERIAL_ID: MaterialID = MaterialID(hash64!("BlinnPhongMaterial"));
    static ref DIFFUSE_TEXTURED_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("DiffuseTexturedBlinnPhongMaterial"));
    static ref TEXTURED_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("TexturedBlinnPhongMaterial"));
}

impl BlinnPhongMaterial {
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet =
        VertexAttributeSet::POSITION.union(VertexAttributeSet::NORMAL_VECTOR);

    /// Registers this material as a feature type in the given
    /// instance feature manager and adds the material specification
    /// to the given material library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
            None,
            vec![Self::FEATURE_TYPE_ID],
            MaterialShaderInput::BlinnPhong(None),
        );
        material_library.add_material_specification(*BLINN_PHONG_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for this material, and if so, registers the material in the given
    /// instance feature manager and adds the appropriate material component
    /// to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        fn execute_setup(
            instance_feature_manager: &mut InstanceFeatureManager,
            diffuse_color: Option<&DiffuseColorComp>,
            specular_color: Option<&SpecularColorComp>,
            roughness: Option<&RoughnessComp>,
        ) -> MaterialComp {
            let material = BlinnPhongMaterial {
                diffuse_color: diffuse_color
                    .map_or_else(RGBColor::zeros, |diffuse_color| diffuse_color.0),
                specular_color: specular_color
                    .map_or_else(RGBColor::zeros, |specular_color| specular_color.0),
                shininess: roughness.map_or(1.0, |roughness| roughness.to_blinn_phong_shininess()),
            };

            let feature_id = instance_feature_manager
                .get_storage_mut::<BlinnPhongMaterial>()
                .expect("Missing storage for BlinnPhongMaterial features")
                .add_feature(&material);

            MaterialComp::new(*BLINN_PHONG_MATERIAL_ID, Some(feature_id), None)
        }

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |diffuse_color: &DiffuseColorComp,
             specular_color: Option<&SpecularColorComp>,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                execute_setup(
                    &mut instance_feature_manager,
                    Some(diffuse_color),
                    specular_color,
                    roughness,
                )
            },
            ![
                MaterialComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
            ]
        );

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |diffuse_color: Option<&DiffuseColorComp>,
             specular_color: &SpecularColorComp,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                execute_setup(
                    &mut instance_feature_manager,
                    diffuse_color,
                    Some(specular_color),
                    roughness,
                )
            },
            ![
                MaterialComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
            ]
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
                MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
            specular_texture_and_sampler_bindings: None,
        }));

    /// Registers this material as a feature type in the given instance feature
    /// manager and adds the material specification to the given material
    /// library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
            None,
            vec![Self::FEATURE_TYPE_ID],
            Self::MATERIAL_SHADER_INPUT,
        );
        material_library
            .add_material_specification(*DIFFUSE_TEXTURED_BLINN_PHONG_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for this material, and if so, adds the appropriate material property
    /// texture set to the material library if not present, registers the
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
            |diffuse_texture: &DiffuseTextureComp,
             specular_color: Option<&SpecularColorComp>,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                let texture_ids = [diffuse_texture.0];

                let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

                // Add a new texture set if none with the same textures already exist
                material_library
                    .material_property_texture_set_entry(texture_set_id)
                    .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids.to_vec()));

                let material = Self {
                    specular_color: specular_color
                        .map_or_else(RGBColor::zeros, |specular_color| specular_color.0),
                    shininess: roughness
                        .map_or(1.0, |roughness| roughness.to_blinn_phong_shininess()),
                };

                let feature_id = instance_feature_manager
                    .get_storage_mut::<Self>()
                    .expect("Missing storage for DiffuseTexturedBlinnPhongMaterial features")
                    .add_feature(&material);

                MaterialComp::new(
                    *DIFFUSE_TEXTURED_BLINN_PHONG_MATERIAL_ID,
                    Some(feature_id),
                    Some(texture_set_id),
                )
            },
            ![
                MaterialComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
            ]
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
                MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
            specular_texture_and_sampler_bindings: Some(
                MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
            ),
        }));

    /// Registers this material as a feature type in the given instance feature
    /// manager and adds the material specification to the given material
    /// library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
            None,
            vec![Self::FEATURE_TYPE_ID],
            Self::MATERIAL_SHADER_INPUT,
        );
        material_library
            .add_material_specification(*TEXTURED_BLINN_PHONG_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for this material, and if so, adds the appropriate material property
    /// texture set to the material library if not present, registers the
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
            |diffuse_texture: &DiffuseTextureComp,
             specular_texture: &SpecularTextureComp,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                let texture_ids = [diffuse_texture.0, specular_texture.0];

                let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

                // Add a new texture set if none with the same textures already exist
                material_library
                    .material_property_texture_set_entry(texture_set_id)
                    .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids.to_vec()));

                let material = Self {
                    shininess: roughness
                        .map_or(1.0, |roughness| roughness.to_blinn_phong_shininess()),
                };

                let feature_id = instance_feature_manager
                    .get_storage_mut::<Self>()
                    .expect("Missing storage for TexturedBlinnPhongMaterial features")
                    .add_feature(&material);

                MaterialComp::new(
                    *TEXTURED_BLINN_PHONG_MATERIAL_ID,
                    Some(feature_id),
                    Some(texture_set_id),
                )
            },
            ![
                MaterialComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
            ]
        );
    }
}

impl_InstanceFeature!(
    BlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        shininess_location: MATERIAL_VERTEX_BINDING_START + 2,
    })
);

impl_InstanceFeature!(
    DiffuseTexturedBlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        shininess_location: MATERIAL_VERTEX_BINDING_START + 1,
    })
);

impl_InstanceFeature!(
    TexturedBlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        shininess_location: MATERIAL_VERTEX_BINDING_START,
    })
);
