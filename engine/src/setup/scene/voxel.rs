//! Management of voxels for entities.

use crate::resource::ResourceManager;
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_physics::{
    quantities::Motion,
    rigid_body::{DynamicRigidBodyID, RigidBodyManager},
};
use impact_scene::{
    SceneEntityFlags, SceneGraphModelInstanceNodeHandle, SceneGraphParentNodeHandle,
    graph::SceneGraph, model::ModelInstanceManager, setup::Uncullable,
};
use impact_voxel::{
    VoxelObjectID, VoxelObjectManager,
    setup::{
        self, GradientNoiseVoxelTypes, MultifractalNoiseSDFModification,
        MultiscaleSphereSDFModification, SameVoxelType, VoxelBox, VoxelGradientNoisePattern,
        VoxelSphere, VoxelSphereUnion,
    },
};
use parking_lot::RwLock;

pub fn setup_voxel_objects_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    rigid_body_manager: &RwLock<RigidBodyManager>,
    voxel_object_manager: &RwLock<VoxelObjectManager>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    // Make sure entities that have manually created voxel object and physics
    // context get a model transform component with the center of mass offset
    setup!(
        {
            let voxel_object_manager = voxel_object_manager.read();
        },
        components,
        |voxel_object_id: &VoxelObjectID,
         model_transform: Option<&ModelTransform>|
         -> ModelTransform {
            if let Some(physics_context) =
                voxel_object_manager.get_physics_context(*voxel_object_id)
            {
                ModelTransform::with_offset(
                    physics_context
                        .inertial_property_manager
                        .derive_center_of_mass()
                        .cast(),
                )
            } else {
                model_transform.copied().unwrap_or_default()
            }
        }
    );

    setup!(
        {
            let resource_manager = resource_manager.read();
            let mut rigid_body_manager = rigid_body_manager.write();
            let mut voxel_object_manager = voxel_object_manager.write();
        },
        components,
        |voxel_box: &VoxelBox,
         voxel_type: &SameVoxelType,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<(
            VoxelObjectID,
            DynamicRigidBodyID,
            ModelTransform,
            ReferenceFrame,
            Motion
        )> {
            setup::setup_voxel_box_with_same_voxel_type(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                voxel_box,
                voxel_type,
                model_transform,
                frame,
                motion,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.read();
            let mut rigid_body_manager = rigid_body_manager.write();
            let mut voxel_object_manager = voxel_object_manager.write();
        },
        components,
        |voxel_sphere: &VoxelSphere,
         voxel_type: &SameVoxelType,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<(
            VoxelObjectID,
            DynamicRigidBodyID,
            ModelTransform,
            ReferenceFrame,
            Motion
        )> {
            setup::setup_voxel_sphere_with_same_voxel_type(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                voxel_sphere,
                voxel_type,
                model_transform,
                frame,
                motion,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.read();
            let mut rigid_body_manager = rigid_body_manager.write();
            let mut voxel_object_manager = voxel_object_manager.write();
        },
        components,
        |voxel_sphere_union: &VoxelSphereUnion,
         voxel_type: &SameVoxelType,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<(
            VoxelObjectID,
            DynamicRigidBodyID,
            ModelTransform,
            ReferenceFrame,
            Motion
        )> {
            setup::setup_voxel_sphere_union_with_same_voxel_type(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                voxel_sphere_union,
                voxel_type,
                model_transform,
                frame,
                motion,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.read();
            let mut rigid_body_manager = rigid_body_manager.write();
            let mut voxel_object_manager = voxel_object_manager.write();
        },
        components,
        |voxel_noise_pattern: &VoxelGradientNoisePattern,
         voxel_type: &SameVoxelType,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<(
            VoxelObjectID,
            DynamicRigidBodyID,
            ModelTransform,
            ReferenceFrame,
            Motion
        )> {
            setup::setup_voxel_gradient_noise_pattern_with_same_voxel_type(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                voxel_noise_pattern,
                voxel_type,
                model_transform,
                frame,
                motion,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.read();
            let mut rigid_body_manager = rigid_body_manager.write();
            let mut voxel_object_manager = voxel_object_manager.write();
        },
        components,
        |voxel_box: &VoxelBox,
         voxel_types: &GradientNoiseVoxelTypes,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<(
            VoxelObjectID,
            DynamicRigidBodyID,
            ModelTransform,
            ReferenceFrame,
            Motion
        )> {
            setup::setup_voxel_box_with_gradient_noise_voxel_types(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                voxel_box,
                voxel_types,
                model_transform,
                frame,
                motion,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.read();
            let mut rigid_body_manager = rigid_body_manager.write();
            let mut voxel_object_manager = voxel_object_manager.write();
        },
        components,
        |voxel_sphere: &VoxelSphere,
         voxel_types: &GradientNoiseVoxelTypes,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<(
            VoxelObjectID,
            DynamicRigidBodyID,
            ModelTransform,
            ReferenceFrame,
            Motion
        )> {
            setup::setup_voxel_sphere_with_gradient_noise_voxel_types(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                voxel_sphere,
                voxel_types,
                model_transform,
                frame,
                motion,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.read();
            let mut rigid_body_manager = rigid_body_manager.write();
            let mut voxel_object_manager = voxel_object_manager.write();
        },
        components,
        |voxel_sphere_union: &VoxelSphereUnion,
         voxel_types: &GradientNoiseVoxelTypes,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<(
            VoxelObjectID,
            DynamicRigidBodyID,
            ModelTransform,
            ReferenceFrame,
            Motion
        )> {
            setup::setup_voxel_sphere_union_with_gradient_noise_voxel_types(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                voxel_sphere_union,
                voxel_types,
                model_transform,
                frame,
                motion,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.read();
            let mut rigid_body_manager = rigid_body_manager.write();
            let mut voxel_object_manager = voxel_object_manager.write();
        },
        components,
        |voxel_noise_pattern: &VoxelGradientNoisePattern,
         voxel_types: &GradientNoiseVoxelTypes,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<(
            VoxelObjectID,
            DynamicRigidBodyID,
            ModelTransform,
            ReferenceFrame,
            Motion
        )> {
            setup::setup_voxel_gradient_noise_pattern_with_gradient_noise_voxel_types(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                voxel_noise_pattern,
                voxel_types,
                model_transform,
                frame,
                motion,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
        },
        ![VoxelObjectID]
    )?;

    Ok(())
}

pub fn setup_scene_graph_model_instance_nodes_for_new_voxel_object_entities(
    voxel_object_manager: &RwLock<VoxelObjectManager>,
    model_instance_manager: &RwLock<ModelInstanceManager>,
    scene_graph: &RwLock<SceneGraph>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let voxel_object_manager = voxel_object_manager.read();
            let mut model_instance_manager = model_instance_manager.write();
            let mut scene_graph = scene_graph.write();
        },
        components,
        |voxel_object_id: &VoxelObjectID,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         parent: Option<&SceneGraphParentNodeHandle>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            SceneGraphModelInstanceNodeHandle,
            ModelTransform,
            SceneEntityFlags
        )> {
            setup::create_model_instance_node_for_voxel_object(
                &voxel_object_manager,
                &mut model_instance_manager,
                &mut scene_graph,
                voxel_object_id,
                model_transform,
                frame,
                parent,
                flags,
                components.has_component_type::<Uncullable>(),
            )
        },
        ![SceneGraphModelInstanceNodeHandle]
    )
}
