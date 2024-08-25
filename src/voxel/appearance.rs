//! Appearance of individual voxels.

use super::VOXEL_MESH_ID;
use crate::{
    assets::Assets,
    gpu::GraphicsDevice,
    material::{
        self,
        components::{
            MaterialComp, UniformColorComp, UniformRoughnessComp, UniformSpecularReflectanceComp,
        },
        MaterialHandle, MaterialLibrary, RGBColor,
    },
    model::{
        transform::InstanceModelViewTransform, InstanceFeature, InstanceFeatureManager, ModelID,
    },
    voxel::VoxelType,
};
use nalgebra::vector;

/// Descriptor for the appearance of a voxel type.
#[derive(Clone, Debug)]
pub struct VoxelAppearance {
    /// The ID of the single-voxel model.
    pub model_id: ModelID,
    /// The handle for the voxel's material.
    pub material_handle: MaterialHandle,
}

impl VoxelAppearance {
    pub(super) fn setup(
        voxel_type: VoxelType,
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) -> Self {
        let material = setup_voxel_material(
            voxel_type,
            graphics_device,
            assets,
            material_library,
            instance_feature_manager,
        );

        let material_handle = *material.material_handle();

        let model_id = ModelID::for_mesh_and_material(*VOXEL_MESH_ID, material_handle);

        let mut feature_type_ids = Vec::with_capacity(2);

        feature_type_ids.push(InstanceModelViewTransform::FEATURE_TYPE_ID);

        feature_type_ids.extend_from_slice(
            material_library
                .get_material_specification(model_id.material_handle().material_id())
                .expect("Missing material specification for model material")
                .instance_feature_type_ids(),
        );

        instance_feature_manager.register_instance(model_id, &feature_type_ids);

        Self {
            model_id,
            material_handle,
        }
    }
}

fn setup_voxel_material(
    voxel_type: VoxelType,
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
) -> MaterialComp {
    match voxel_type {
        VoxelType::Default => setup_microfacet_material_for_voxel(
            graphics_device,
            assets,
            material_library,
            instance_feature_manager,
            vector![0.5, 0.5, 0.5],
            Some(UniformSpecularReflectanceComp::in_range_of(
                UniformSpecularReflectanceComp::STONE,
                0.5,
            )),
            Some(0.7),
        ),
    }
}

fn setup_microfacet_material_for_voxel(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    albedo: RGBColor,
    specular_reflectance: Option<UniformSpecularReflectanceComp>,
    roughness: Option<f32>,
) -> MaterialComp {
    let roughness = roughness.map(UniformRoughnessComp);

    material::entity::physical::setup_physical_material(
        graphics_device,
        assets,
        material_library,
        instance_feature_manager,
        Some(&UniformColorComp(albedo)),
        None,
        specular_reflectance.as_ref(),
        None,
        roughness.as_ref(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}
