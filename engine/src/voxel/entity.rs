//! Management of voxels for entities.

use crate::engine::Engine;
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::{ComponentArray, SingleInstance},
    setup,
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_physics::{
    quantities::Motion,
    rigid_body::{DynamicRigidBodyID, RigidBodyManager},
};
use impact_scene::{
    SceneEntityFlags, SceneGraphModelInstanceNodeHandle, SceneGraphParentNodeHandle,
    graph::SceneGraph, model::InstanceFeatureManager, setup::Uncullable,
};
use impact_voxel::{
    StagedVoxelObject, VoxelManager, VoxelObjectID,
    mesh::MeshedChunkedVoxelObject,
    setup::{
        self, GradientNoiseVoxelTypes, MultifractalNoiseSDFModification,
        MultiscaleSphereSDFModification, SameVoxelType, VoxelBox, VoxelGradientNoisePattern,
        VoxelSphere, VoxelSphereUnion,
    },
};
use std::sync::RwLock;

pub fn setup_voxel_object_for_new_entity(
    rigid_body_manager: &RwLock<RigidBodyManager>,
    voxel_manager: &RwLock<VoxelManager>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mut voxel_manager = voxel_manager.write().unwrap();
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
                &mut voxel_manager,
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mut voxel_manager = voxel_manager.write().unwrap();
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
                &mut voxel_manager,
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mut voxel_manager = voxel_manager.write().unwrap();
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
                &mut voxel_manager,
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mut voxel_manager = voxel_manager.write().unwrap();
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
                &mut voxel_manager,
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mut voxel_manager = voxel_manager.write().unwrap();
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
                &mut voxel_manager,
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mut voxel_manager = voxel_manager.write().unwrap();
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
                &mut voxel_manager,
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mut voxel_manager = voxel_manager.write().unwrap();
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
                &mut voxel_manager,
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mut voxel_manager = voxel_manager.write().unwrap();
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
                &mut voxel_manager,
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

pub fn add_model_instance_node_component_for_new_voxel_object_entity(
    voxel_manager: &RwLock<VoxelManager>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    scene_graph: &RwLock<SceneGraph>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let voxel_manager = voxel_manager.read().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            let mut scene_graph = scene_graph.write().unwrap();
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
                &voxel_manager,
                &mut instance_feature_manager,
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

pub fn handle_staged_voxel_objects(engine: &Engine) -> Result<()> {
    loop {
        let scene = engine.scene().read().unwrap();
        let mut voxel_manager = scene.voxel_manager().write().unwrap();

        if let Some(StagedVoxelObject {
            object,
            inertial_property_manager,
            mut components,
        }) = voxel_manager.object_manager.pop_staged_voxel_object()
        {
            let meshed_voxel_object = MeshedChunkedVoxelObject::create(object);

            let voxel_object_id = voxel_manager
                .object_manager
                .add_voxel_object(meshed_voxel_object);

            if let Some(inertial_property_manager) = inertial_property_manager.clone() {
                voxel_manager
                    .object_manager
                    .add_inertial_property_manager_for_voxel_object(
                        voxel_object_id,
                        inertial_property_manager,
                    );
            }

            components.add_new_component_type(voxel_object_id.into_storage())?;

            // We must release these locks before attempting to create the entity, or we
            // will deadlock
            drop(voxel_manager);
            drop(scene);

            engine.create_entity(SingleInstance::new(components))?;
        } else {
            break;
        }
    }
    Ok(())
}

pub fn handle_emptied_voxel_objects(engine: &Engine) -> Result<()> {
    loop {
        let scene = engine.scene().read().unwrap();
        let mut voxel_manager = scene.voxel_manager().write().unwrap();

        if let Some(entity_id) = voxel_manager.object_manager.pop_empty_voxel_object_entity() {
            // We must release these locks before attempting to remove the entity, or we
            // will deadlock
            drop(voxel_manager);
            drop(scene);

            impact_log::debug!("Removing entity for emptied voxel object");
            engine.remove_entity(entity_id)?;
        } else {
            break;
        }
    }
    Ok(())
}
