//! Materials using a microfacet reflection model.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    geometry::{InstanceFeature, VertexAttributeSet},
    impl_InstanceFeature,
    rendering::{
        fre, InstanceFeatureShaderInput, MaterialPropertyTextureManager, MaterialShaderInput,
        MicrofacetFeatureShaderInput, MicrofacetShadingModel, MicrofacetTextureShaderInput,
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

/// Material using a microfacet model for specular and/or diffuse reflection,
/// with fixed diffuse and specular colors and fixed roughness.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct MicrofacetMaterial {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    roughness: fre,
}

/// Material using a microfacet model for specular and/or diffuse reflection,
/// with textured diffuse colors, fixed specular color and fixed roughness.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct DiffuseTexturedMicrofacetMaterial {
    specular_color: RGBColor,
    roughness: fre,
}

/// Material using a microfacet model for specular and/or diffuse reflection,
/// with textured diffuse and specular colors and fixed roughness.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedMicrofacetMaterial {
    roughness: fre,
}

lazy_static! {
    static ref NO_DIFFUSE_GGX_SPECULAR_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("NoDiffuseGGXSpecularMicrofacetMaterial"));
    static ref LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("LambertianDiffuseGGXSpecularMicrofacetMaterial"));
    static ref GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("GGXDiffuseGGXSpecularMicrofacetMaterial"));
    static ref GGX_DIFFUSE_NO_SPECULAR_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("GGXDiffuseNoSpecularMicrofacetMaterial"));
    static ref DIFFUSE_TEXTURED_LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID: MaterialID =
        MaterialID(hash64!(
            "DiffuseTexturedLambertianDiffuseGGXSpecularMicrofacetMaterial"
        ));
    static ref DIFFUSE_TEXTURED_GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID: MaterialID = MaterialID(
        hash64!("DiffuseTexturedGGXDiffuseGGXSpecularMicrofacetMaterial")
    );
    static ref DIFFUSE_TEXTURED_GGX_DIFFUSE_NO_SPECULAR_MATERIAL_ID: MaterialID = MaterialID(
        hash64!("DiffuseTexturedGGXDiffuseNoSpecularMicrofacetMaterial")
    );
    static ref TEXTURED_LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID: MaterialID = MaterialID(
        hash64!("TexturedLambertianDiffuseGGXSpecularMicrofacetMaterial")
    );
    static ref TEXTURED_GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("TexturedGGXDiffuseGGXSpecularMicrofacetMaterial"));
}

impl MicrofacetMaterial {
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet =
        VertexAttributeSet::POSITION.union(VertexAttributeSet::NORMAL_VECTOR);

    /// Registers this material as a feature type in the given instance feature
    /// manager and adds the material specifications for the variants of the
    /// material to the given material library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        for (model, material_id) in [
            MicrofacetShadingModel::NO_DIFFUSE_GGX_SPECULAR,
            MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
            MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
            MicrofacetShadingModel::GGX_DIFFUSE_NO_SPECULAR,
        ]
        .into_iter()
        .zip(
            [
                *NO_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                *LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                *GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                *GGX_DIFFUSE_NO_SPECULAR_MATERIAL_ID,
            ]
            .into_iter(),
        ) {
            let specification = MaterialSpecification::new(
                Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
                None,
                vec![Self::FEATURE_TYPE_ID],
                MaterialShaderInput::Microfacet((model, None)),
            );
            material_library.add_material_specification(material_id, specification);
        }
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for a variant of this material, and if so, registers the material in the
    /// given instance feature manager and adds the appropriate material
    /// component to the entity.
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
            material_id: MaterialID,
        ) -> MaterialComp {
            let material = MicrofacetMaterial {
                diffuse_color: diffuse_color
                    .map_or_else(RGBColor::zeros, |diffuse_color| diffuse_color.0),
                specular_color: specular_color
                    .map_or_else(RGBColor::zeros, |specular_color| specular_color.0),
                roughness: roughness.map_or(1.0, |roughness| roughness.to_ggx_roughness()),
            };

            let feature_id = instance_feature_manager
                .get_storage_mut::<MicrofacetMaterial>()
                .expect("Missing storage for MicrofacetMaterial features")
                .add_feature(&material);

            MaterialComp::new(material_id, Some(feature_id), None)
        }

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |specular_color: &SpecularColorComp,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                execute_setup(
                    &mut instance_feature_manager,
                    None,
                    Some(specular_color),
                    roughness,
                    *NO_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetSpecularReflection],
            ![MaterialComp, DiffuseColorComp, MicrofacetDiffuseReflection]
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
                    *LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetSpecularReflection],
            ![MaterialComp, MicrofacetDiffuseReflection]
        );

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |diffuse_color: &DiffuseColorComp,
             specular_color: &SpecularColorComp,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                execute_setup(
                    &mut instance_feature_manager,
                    Some(diffuse_color),
                    Some(specular_color),
                    roughness,
                    *GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetDiffuseReflection, MicrofacetSpecularReflection],
            ![MaterialComp]
        );

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |diffuse_color: &DiffuseColorComp, roughness: Option<&RoughnessComp>| -> MaterialComp {
                execute_setup(
                    &mut instance_feature_manager,
                    Some(diffuse_color),
                    None,
                    roughness,
                    *GGX_DIFFUSE_NO_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetDiffuseReflection],
            ![
                MaterialComp,
                SpecularColorComp,
                MicrofacetSpecularReflection
            ]
        );
    }
}

impl DiffuseTexturedMicrofacetMaterial {
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet = VertexAttributeSet::POSITION
        .union(VertexAttributeSet::NORMAL_VECTOR)
        .union(VertexAttributeSet::TEXTURE_COORDS);

    /// Registers this material as a feature type in the given instance feature
    /// manager and adds the material specifications for the variants of the
    /// material to the given material library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        for (model, material_id) in [
            MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
            MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
            MicrofacetShadingModel::GGX_DIFFUSE_NO_SPECULAR,
        ]
        .into_iter()
        .zip(
            [
                *DIFFUSE_TEXTURED_LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                *DIFFUSE_TEXTURED_GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                *DIFFUSE_TEXTURED_GGX_DIFFUSE_NO_SPECULAR_MATERIAL_ID,
            ]
            .into_iter(),
        ) {
            let specification = MaterialSpecification::new(
                Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
                None,
                vec![Self::FEATURE_TYPE_ID],
                MaterialShaderInput::Microfacet((
                    model,
                    Some(MicrofacetTextureShaderInput {
                        diffuse_texture_and_sampler_bindings:
                            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                        specular_texture_and_sampler_bindings: None,
                    }),
                )),
            );
            material_library.add_material_specification(material_id, specification);
        }
    }

    /// Checks if the entity-to-be with the given components has the component
    /// for a variant of this material, and if so, adds the appropriate material
    /// property texture set to the material library if not present, registers
    /// the material in the given instance feature manager and adds the
    /// appropriate material component to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        material_library: &RwLock<MaterialLibrary>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        fn execute_setup(
            instance_feature_manager: &mut InstanceFeatureManager,
            material_library: &mut MaterialLibrary,
            diffuse_texture: &DiffuseTextureComp,
            specular_color: Option<&SpecularColorComp>,
            roughness: Option<&RoughnessComp>,
            material_id: MaterialID,
        ) -> MaterialComp {
            let texture_ids = [diffuse_texture.0];

            let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

            // Add a new texture set if none with the same textures already exist
            material_library
                .material_property_texture_set_entry(texture_set_id)
                .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids.to_vec()));

            let material = DiffuseTexturedMicrofacetMaterial {
                specular_color: specular_color
                    .map_or_else(RGBColor::zeros, |specular_color| specular_color.0),
                roughness: roughness.map_or(0.0, |roughness| roughness.0),
            };

            let feature_id = instance_feature_manager
                .get_storage_mut::<DiffuseTexturedMicrofacetMaterial>()
                .expect("Missing storage for DiffuseTexturedMicrofacetMaterial features")
                .add_feature(&material);

            MaterialComp::new(material_id, Some(feature_id), Some(texture_set_id))
        }

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
                let mut material_library = material_library.write().unwrap();
            },
            components,
            |diffuse_texture: &DiffuseTextureComp,
             specular_color: &SpecularColorComp,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                execute_setup(
                    &mut instance_feature_manager,
                    &mut material_library,
                    diffuse_texture,
                    Some(specular_color),
                    roughness,
                    *DIFFUSE_TEXTURED_LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetSpecularReflection],
            ![MaterialComp, MicrofacetDiffuseReflection]
        );

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
                let mut material_library = material_library.write().unwrap();
            },
            components,
            |diffuse_texture: &DiffuseTextureComp,
             specular_color: &SpecularColorComp,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                execute_setup(
                    &mut instance_feature_manager,
                    &mut material_library,
                    diffuse_texture,
                    Some(specular_color),
                    roughness,
                    *DIFFUSE_TEXTURED_GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetDiffuseReflection, MicrofacetSpecularReflection],
            ![MaterialComp]
        );

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
                let mut material_library = material_library.write().unwrap();
            },
            components,
            |diffuse_texture: &DiffuseTextureComp,
             roughness: Option<&RoughnessComp>|
             -> MaterialComp {
                execute_setup(
                    &mut instance_feature_manager,
                    &mut material_library,
                    diffuse_texture,
                    None,
                    roughness,
                    *DIFFUSE_TEXTURED_GGX_DIFFUSE_NO_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetDiffuseReflection],
            ![
                MaterialComp,
                SpecularColorComp,
                MicrofacetSpecularReflection
            ]
        );
    }
}

impl TexturedMicrofacetMaterial {
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet = VertexAttributeSet::POSITION
        .union(VertexAttributeSet::NORMAL_VECTOR)
        .union(VertexAttributeSet::TEXTURE_COORDS);

    /// Registers this material as a feature type in the given instance feature
    /// manager and adds the material specifications for the variants of the
    /// material to the given material library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        for (model, material_id) in [
            MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
            MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
        ]
        .into_iter()
        .zip(
            [
                *TEXTURED_LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                *TEXTURED_GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
            ]
            .into_iter(),
        ) {
            let specification = MaterialSpecification::new(
                Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
                None,
                vec![Self::FEATURE_TYPE_ID],
                MaterialShaderInput::Microfacet((
                    model,
                    Some(MicrofacetTextureShaderInput {
                        diffuse_texture_and_sampler_bindings:
                            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                        specular_texture_and_sampler_bindings: Some(
                            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                        ),
                    }),
                )),
            );
            material_library.add_material_specification(material_id, specification);
        }
    }

    /// Checks if the entity-to-be with the given components has the component
    /// for a variant of this material, and if so, adds the appropriate material
    /// property texture set to the material library if not present, registers
    /// the material in the given instance feature manager and adds the
    /// appropriate material component to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        material_library: &RwLock<MaterialLibrary>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        fn execute_setup(
            instance_feature_manager: &mut InstanceFeatureManager,
            material_library: &mut MaterialLibrary,
            diffuse_texture: &DiffuseTextureComp,
            specular_texture: &SpecularTextureComp,
            roughness: Option<&RoughnessComp>,
            material_id: MaterialID,
        ) -> MaterialComp {
            let texture_ids = [diffuse_texture.0, specular_texture.0];

            let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

            // Add a new texture set if none with the same textures already exist
            material_library
                .material_property_texture_set_entry(texture_set_id)
                .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids.to_vec()));

            let material = TexturedMicrofacetMaterial {
                roughness: roughness.map_or(0.0, |roughness| roughness.0),
            };

            let feature_id = instance_feature_manager
                .get_storage_mut::<TexturedMicrofacetMaterial>()
                .expect("Missing storage for TexturedMicrofacetMaterial features")
                .add_feature(&material);

            MaterialComp::new(material_id, Some(feature_id), Some(texture_set_id))
        }

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
                execute_setup(
                    &mut instance_feature_manager,
                    &mut material_library,
                    diffuse_texture,
                    specular_texture,
                    roughness,
                    *TEXTURED_LAMBERTIAN_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetSpecularReflection],
            ![MaterialComp, MicrofacetDiffuseReflection]
        );

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
                execute_setup(
                    &mut instance_feature_manager,
                    &mut material_library,
                    diffuse_texture,
                    specular_texture,
                    roughness,
                    *TEXTURED_GGX_DIFFUSE_GGX_SPECULAR_MATERIAL_ID,
                )
            },
            [MicrofacetDiffuseReflection, MicrofacetSpecularReflection],
            ![MaterialComp]
        );
    }
}

impl_InstanceFeature!(
    MicrofacetMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::MicrofacetMaterial(MicrofacetFeatureShaderInput {
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        roughness_location: MATERIAL_VERTEX_BINDING_START + 2,
    })
);

impl_InstanceFeature!(
    DiffuseTexturedMicrofacetMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::MicrofacetMaterial(MicrofacetFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        roughness_location: MATERIAL_VERTEX_BINDING_START + 1,
    })
);

impl_InstanceFeature!(
    TexturedMicrofacetMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
    ],
    InstanceFeatureShaderInput::MicrofacetMaterial(MicrofacetFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        roughness_location: MATERIAL_VERTEX_BINDING_START,
    })
);
