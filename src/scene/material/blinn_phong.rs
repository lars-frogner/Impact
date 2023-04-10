//! Materials using the Blinn-Phong reflection model.

use super::{create_prepass_material, MATERIAL_VERTEX_BINDING_START};
use crate::{
    geometry::{InstanceFeature, InstanceFeatureID, VertexAttributeSet},
    impl_InstanceFeature,
    rendering::{
        fre, BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput,
        InstanceFeatureShaderInput, MaterialPropertyTextureManager, MaterialShaderInput,
        RenderAttachmentQuantitySet,
    },
    scene::{
        DiffuseColorComp, DiffuseTextureComp, InstanceFeatureManager, MaterialComp, MaterialHandle,
        MaterialID, MaterialLibrary, MaterialPropertyTextureSet, MaterialPropertyTextureSetID,
        MaterialSpecification, MicrofacetDiffuseReflection, MicrofacetSpecularReflection,
        NormalMapComp, ParallaxMapComp, RGBColor, RenderResourcesDesynchronized, RoughnessComp,
        RoughnessTextureComp, SpecularColorComp, SpecularTextureComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use std::sync::RwLock;

/// Fixed material properties for a Blinn-Phong material with uniform diffuse
/// and specular color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformColorBlinnPhongMaterialFeature {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    shininess: fre,
}

/// Fixed material properties for a Blinn-Phong material with uniform diffuse
/// color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseBlinnPhongMaterialFeature {
    diffuse_color: RGBColor,
    shininess: fre,
}

/// Fixed material properties for a Blinn-Phong material with uniform specular
/// color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularBlinnPhongMaterialFeature {
    specular_color: RGBColor,
    shininess: fre,
}

/// Fixed material properties for a Blinn-Phong material with no uniform color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorBlinnPhongMaterialFeature {
    shininess: fre,
}

/// Checks if the entity-to-be with the given components has the components for
/// a Blinn-Phong material, and if so, adds the material specification to the
/// material library if not already present, adds the appropriate material
/// property texture set to the material library if not already present,
/// registers the material in the instance feature manager and adds the
/// appropriate material component to the entity.
pub fn add_blinn_phong_material_component_for_entity(
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    ambient_color: RGBColor,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_color: &DiffuseColorComp,
         specular_color: Option<&SpecularColorComp>,
         specular_texture: Option<&SpecularTextureComp>,
         roughness: Option<&RoughnessComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                ambient_color,
                Some(diffuse_color),
                specular_color,
                None,
                specular_texture,
                roughness,
                normal_map,
                parallax_map,
            )
        },
        ![
            MaterialComp,
            DiffuseTextureComp,
            RoughnessTextureComp,
            MicrofacetDiffuseReflection,
            MicrofacetSpecularReflection
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_color: Option<&DiffuseColorComp>,
         specular_color: &SpecularColorComp,
         diffuse_texture: Option<&DiffuseTextureComp>,
         roughness: Option<&RoughnessComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                ambient_color,
                diffuse_color,
                Some(specular_color),
                diffuse_texture,
                None,
                roughness,
                normal_map,
                parallax_map,
            )
        },
        ![
            MaterialComp,
            SpecularTextureComp,
            RoughnessTextureComp,
            MicrofacetDiffuseReflection,
            MicrofacetSpecularReflection
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |specular_color: Option<&SpecularColorComp>,
         diffuse_texture: &DiffuseTextureComp,
         specular_texture: Option<&SpecularTextureComp>,
         roughness: Option<&RoughnessComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                ambient_color,
                None,
                specular_color,
                Some(diffuse_texture),
                specular_texture,
                roughness,
                normal_map,
                parallax_map,
            )
        },
        ![
            MaterialComp,
            DiffuseColorComp,
            RoughnessTextureComp,
            MicrofacetDiffuseReflection,
            MicrofacetSpecularReflection
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_color: Option<&DiffuseColorComp>,
         diffuse_texture: Option<&DiffuseTextureComp>,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                ambient_color,
                diffuse_color,
                None,
                diffuse_texture,
                Some(specular_texture),
                roughness,
                normal_map,
                parallax_map,
            )
        },
        ![
            MaterialComp,
            SpecularColorComp,
            RoughnessTextureComp,
            MicrofacetDiffuseReflection,
            MicrofacetSpecularReflection
        ]
    );
}

fn execute_material_setup(
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    ambient_color: RGBColor,
    diffuse_color: Option<&DiffuseColorComp>,
    specular_color: Option<&SpecularColorComp>,
    diffuse_texture: Option<&DiffuseTextureComp>,
    specular_texture: Option<&SpecularTextureComp>,
    roughness: Option<&RoughnessComp>,
    normal_map: Option<&NormalMapComp>,
    parallax_map: Option<&ParallaxMapComp>,
) -> MaterialComp {
    let mut material_name_parts = Vec::new();

    let shininess = roughness.map_or(1.0, |roughness| roughness.to_blinn_phong_shininess());

    let (feature_type_id, feature_id) = match (diffuse_color, specular_color) {
        (Some(diffuse_color), Some(specular_color)) => {
            material_name_parts.push("UniformDiffuseUniformSpecular");

            (
                UniformColorBlinnPhongMaterialFeature::FEATURE_TYPE_ID,
                UniformColorBlinnPhongMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    specular_color,
                    shininess,
                ),
            )
        }
        (Some(diffuse_color), None) => {
            material_name_parts.push("UniformDiffuse");

            (
                UniformDiffuseBlinnPhongMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseBlinnPhongMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    shininess,
                ),
            )
        }
        (None, Some(specular_color)) => {
            material_name_parts.push("UniformSpecular");

            (
                UniformSpecularBlinnPhongMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularBlinnPhongMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_color,
                    shininess,
                ),
            )
        }
        (None, None) => (
            TexturedColorBlinnPhongMaterialFeature::FEATURE_TYPE_ID,
            TexturedColorBlinnPhongMaterialFeature::add_feature(
                instance_feature_manager,
                shininess,
            ),
        ),
    };

    let mut vertex_attribute_requirements_for_shader =
        VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR;
    let mut vertex_attribute_requirements_for_mesh = vertex_attribute_requirements_for_shader;

    let mut texture_shader_input = BlinnPhongTextureShaderInput {
        diffuse_texture_and_sampler_bindings: None,
        specular_texture_and_sampler_bindings: None,
    };

    let mut texture_ids = Vec::with_capacity(4);

    if let Some(diffuse_texture) = diffuse_texture {
        assert!(
            diffuse_color.is_none(),
            "Tried to create Blinn-Phong material with both uniform and textured diffuse color"
        );

        material_name_parts.push("TexturedDiffuse");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.diffuse_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(diffuse_texture.0);
    }

    if let Some(specular_texture) = specular_texture {
        assert!(
            specular_color.is_none(),
            "Tried to create Blinn-Phong material with both uniform and textured specular color"
        );

        material_name_parts.push("TexturedSpecular");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.specular_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(specular_texture.0);
    }

    let (prepass_material_handle, input_render_attachment_quantities) = create_prepass_material(
        instance_feature_manager,
        material_library,
        ambient_color,
        normal_map,
        parallax_map,
    );

    if normal_map.is_some() || parallax_map.is_some() {
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TANGENT_SPACE_QUATERNION;
    }

    if input_render_attachment_quantities.contains(RenderAttachmentQuantitySet::POSITION) {
        vertex_attribute_requirements_for_shader -= VertexAttributeSet::POSITION;
    }

    if input_render_attachment_quantities.contains(RenderAttachmentQuantitySet::NORMAL_VECTOR) {
        vertex_attribute_requirements_for_shader -= VertexAttributeSet::NORMAL_VECTOR;
    }

    if input_render_attachment_quantities.contains(RenderAttachmentQuantitySet::TEXTURE_COORDS) {
        vertex_attribute_requirements_for_shader -= VertexAttributeSet::TEXTURE_COORDS;
    }

    let material_id = MaterialID(hash64!(format!(
        "{}{}BlinnPhongMaterial",
        material_name_parts.join(""),
        input_render_attachment_quantities,
    )));

    // Add material specification unless a specification for the same material exists
    material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            MaterialSpecification::new(
                vertex_attribute_requirements_for_mesh,
                vertex_attribute_requirements_for_shader,
                input_render_attachment_quantities,
                RenderAttachmentQuantitySet::empty(),
                None,
                vec![feature_type_id],
                MaterialShaderInput::BlinnPhong(texture_shader_input),
            )
        });

    let texture_set_id = if !texture_ids.is_empty() {
        let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

        // Add a new texture set if none with the same textures already exist
        material_library
            .material_property_texture_set_entry(texture_set_id)
            .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids));

        Some(texture_set_id)
    } else {
        None
    };

    MaterialComp::new(
        MaterialHandle::new(material_id, Some(feature_id), texture_set_id),
        Some(prepass_material_handle),
    )
}

impl UniformColorBlinnPhongMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        specular_color: &SpecularColorComp,
        shininess: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformColorBlinnPhongMaterial features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                specular_color: specular_color.0,
                shininess,
            })
    }
}

impl UniformDiffuseBlinnPhongMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        shininess: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseBlinnPhongMaterial features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                shininess,
            })
    }
}

impl UniformSpecularBlinnPhongMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_color: &SpecularColorComp,
        shininess: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularBlinnPhongMaterial features")
            .add_feature(&Self {
                specular_color: specular_color.0,
                shininess,
            })
    }
}

impl TexturedColorBlinnPhongMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        shininess: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedColorBlinnPhongMaterial features")
            .add_feature(&Self { shininess })
    }
}

impl_InstanceFeature!(
    UniformColorBlinnPhongMaterialFeature,
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
    UniformDiffuseBlinnPhongMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: None,
        shininess_location: MATERIAL_VERTEX_BINDING_START + 1,
    })
);

impl_InstanceFeature!(
    UniformSpecularBlinnPhongMaterialFeature,
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
    TexturedColorBlinnPhongMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        shininess_location: MATERIAL_VERTEX_BINDING_START,
    })
);
