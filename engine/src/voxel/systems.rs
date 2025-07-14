//! ECS systems related to voxels.

use crate::voxel::{
    StagedVoxelObject, VoxelManager, VoxelObjectManager,
    chunks::{
        ChunkedVoxelObject,
        disconnection::DisconnectedVoxelObject,
        inertia::{VoxelObjectInertialPropertyManager, VoxelObjectInertialPropertyUpdater},
    },
    components::{VoxelAbsorbingCapsuleComp, VoxelAbsorbingSphereComp, VoxelObjectComp},
};
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::ComponentArray,
    query,
    world::{EntityID, World as ECSWorld},
};
use impact_geometry::ReferenceFrame;
use impact_physics::{
    fph,
    inertia::InertialProperties,
    quantities::Motion,
    rigid_body::{DynamicRigidBody, DynamicRigidBodyID, RigidBodyManager},
};
use impact_scene::{SceneEntityFlags, SceneGraphParentNodeHandle, graph::SceneGraph};
use nalgebra::{Similarity3, Vector3};

/// Applies each voxel-absorbing sphere and capsule to the affected voxel
/// objects.
pub fn apply_absorption(
    ecs_world: &ECSWorld,
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    scene_graph: &SceneGraph,
    time_step_duration: fph,
) {
    query!(ecs_world, |entity_id: EntityID,
                       voxel_object: &VoxelObjectComp,
                       reference_frame: &mut ReferenceFrame,
                       velocity: &mut Motion,
                       rigid_body_id: &DynamicRigidBodyID,
                       flags: &SceneEntityFlags| {
        if flags.is_disabled() {
            return;
        }

        let (object, inertial_property_manager) = voxel_manager
            .object_manager
            .get_voxel_object_with_inertial_property_manager_mut(voxel_object.voxel_object_id);

        let object = object
            .expect("Missing voxel object for entity with VoxelObjectComp")
            .object_mut();

        let inertial_property_manager = inertial_property_manager
            .expect("Missing inertial property manager for entity with VoxelObjectComp");

        let world_to_voxel_object_transform = reference_frame
            .create_transform_to_parent_space::<f64>()
            .inverse();

        let mut inertial_property_updater = inertial_property_manager.begin_update(
            object.voxel_extent(),
            voxel_manager.type_registry.mass_densities(),
        );

        query!(
            ecs_world,
            |absorbing_sphere: &VoxelAbsorbingSphereComp,
             sphere_frame: &ReferenceFrame,
             flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }

                let sphere_to_world_transform =
                    sphere_frame.create_transform_to_parent_space::<f64>();

                apply_sphere_absorption(
                    time_step_duration,
                    &mut inertial_property_updater,
                    object,
                    &world_to_voxel_object_transform,
                    absorbing_sphere,
                    &sphere_to_world_transform,
                );
            },
            ![SceneGraphParentNodeHandle]
        );

        query!(
            ecs_world,
            |absorbing_sphere: &VoxelAbsorbingSphereComp,
             sphere_frame: &ReferenceFrame,
             parent: &SceneGraphParentNodeHandle,
             flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }

                let parent_node = scene_graph.group_nodes().node(parent.id);

                let sphere_to_world_transform = parent_node.group_to_root_transform().cast()
                    * sphere_frame.create_transform_to_parent_space::<f64>();

                apply_sphere_absorption(
                    time_step_duration,
                    &mut inertial_property_updater,
                    object,
                    &world_to_voxel_object_transform,
                    absorbing_sphere,
                    &sphere_to_world_transform,
                );
            }
        );

        query!(
            ecs_world,
            |absorbing_capsule: &VoxelAbsorbingCapsuleComp,
             capsule_frame: &ReferenceFrame,
             flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }

                let capsule_to_world_transform =
                    capsule_frame.create_transform_to_parent_space::<f64>();

                apply_capsule_absorption(
                    time_step_duration,
                    &mut inertial_property_updater,
                    object,
                    &world_to_voxel_object_transform,
                    absorbing_capsule,
                    &capsule_to_world_transform,
                );
            },
            ![SceneGraphParentNodeHandle]
        );

        query!(
            ecs_world,
            |absorbing_capsule: &VoxelAbsorbingCapsuleComp,
             capsule_frame: &ReferenceFrame,
             parent: &SceneGraphParentNodeHandle,
             flags: &SceneEntityFlags| {
                if flags.is_disabled() {
                    return;
                }

                let parent_node = scene_graph.group_nodes().node(parent.id);

                let capsule_to_world_transform = parent_node.group_to_root_transform().cast()
                    * capsule_frame.create_transform_to_parent_space::<f64>();

                apply_capsule_absorption(
                    time_step_duration,
                    &mut inertial_property_updater,
                    object,
                    &world_to_voxel_object_transform,
                    absorbing_capsule,
                    &capsule_to_world_transform,
                );
            }
        );

        if object.invalidated_mesh_chunk_indices().len() > 0 {
            handle_voxel_object_after_removing_voxels(
                rigid_body_manager,
                voxel_manager,
                ecs_world,
                entity_id,
                voxel_object,
                reference_frame,
                velocity,
                *rigid_body_id,
            );
        }
    });
}

fn apply_sphere_absorption(
    time_step_duration: f64,
    inertial_property_updater: &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
    voxel_object: &mut ChunkedVoxelObject,
    world_to_voxel_object_transform: &Similarity3<f64>,
    absorbing_sphere: &VoxelAbsorbingSphereComp,
    sphere_to_world_transform: &Similarity3<f64>,
) {
    let sphere_in_voxel_object_space = absorbing_sphere
        .sphere()
        .transformed(sphere_to_world_transform)
        .transformed(world_to_voxel_object_transform);

    let inverse_radius_squared = sphere_in_voxel_object_space.radius_squared().recip();

    let absorption_rate_per_frame = absorbing_sphere.rate() * time_step_duration;

    voxel_object.modify_voxels_within_sphere(
        &sphere_in_voxel_object_space,
        &mut |object_voxel_indices, squared_distance, voxel| {
            let was_empty = voxel.is_empty();

            let signed_distance_delta =
                absorption_rate_per_frame * (1.0 - squared_distance * inverse_radius_squared);

            voxel.increase_signed_distance(signed_distance_delta as f32, &mut |voxel| {
                if !was_empty {
                    inertial_property_updater.remove_voxel(&object_voxel_indices, *voxel);
                }
            });
        },
    );
}

fn apply_capsule_absorption(
    time_step_duration: f64,
    inertial_property_updater: &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
    voxel_object: &mut ChunkedVoxelObject,
    world_to_voxel_object_transform: &Similarity3<f64>,
    absorbing_capsule: &VoxelAbsorbingCapsuleComp,
    capsule_to_world_transform: &Similarity3<f64>,
) {
    let capsule_in_voxel_object_space = absorbing_capsule
        .capsule()
        .transformed(capsule_to_world_transform)
        .transformed(world_to_voxel_object_transform);

    let inverse_radius_squared = capsule_in_voxel_object_space.radius().powi(2).recip();

    let absorption_rate_per_frame = absorbing_capsule.rate() * time_step_duration;

    voxel_object.modify_voxels_within_capsule(
        &capsule_in_voxel_object_space,
        &mut |object_voxel_indices, squared_distance, voxel| {
            let was_empty = voxel.is_empty();

            let signed_distance_delta =
                absorption_rate_per_frame * (1.0 - squared_distance * inverse_radius_squared);

            voxel.increase_signed_distance(signed_distance_delta as f32, &mut |voxel| {
                if !was_empty {
                    inertial_property_updater.remove_voxel(&object_voxel_indices, *voxel);
                }
            });
        },
    );
}

fn handle_voxel_object_after_removing_voxels(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    ecs_world: &ECSWorld,
    entity_id: EntityID,
    voxel_object: &VoxelObjectComp,
    reference_frame: &mut ReferenceFrame,
    motion: &mut Motion,
    rigid_body_id: DynamicRigidBodyID,
) {
    let (object, inertial_property_manager) = voxel_manager
        .object_manager
        .get_voxel_object_with_inertial_property_manager_mut(voxel_object.voxel_object_id);

    let object = object
        .expect("Missing voxel object for entity with VoxelObjectComp")
        .object_mut();

    let rigid_body = rigid_body_manager
        .get_dynamic_rigid_body_mut(rigid_body_id)
        .expect("Missing dynamic rigid body for entity with VoxelObjectComp");

    let inertial_property_manager = inertial_property_manager
        .expect("Missing inertial property manager for entity with VoxelObjectComp");

    if object.is_effectively_empty() {
        impact_log::debug!("Marked voxel object as empty");
        voxel_manager
            .object_manager
            .mark_voxel_object_as_empty_for_entity(entity_id);
        return;
    }

    // If the object has not become empty, we must resolve the connected region
    // information
    object.resolve_connected_regions_between_all_chunks();

    // Removing voxels could have divided the object into multiple disconnected
    // regions. If there is a disconnection, we will split off a disconnected region
    // and make it a new independent voxel object. In the process of splitting off
    // the new object, we will compute the inertial properties of the disconnected
    // region, remove them from the original object and add them to the new object.

    let mut disconnected_object_inertial_property_manager =
        VoxelObjectInertialPropertyManager::zeroed();

    let mut inertial_property_transferrer = inertial_property_manager.begin_transfer_to(
        &mut disconnected_object_inertial_property_manager,
        object.voxel_extent(),
        voxel_manager.type_registry.mass_densities(),
    );

    if let Some(disconnected_voxel_object) = object
        .split_off_any_disconnected_region_with_property_transferrer(
            &mut inertial_property_transferrer,
        )
    {
        let original_reference_frame = *reference_frame;
        let original_motion = *motion;
        let original_rigid_body = *rigid_body;

        // The inertial properties of the original object have now changed, and if the
        // object has not become effectively empty due to the splitting we will need
        // them to update the physics-related components of the object
        let new_inertial_properties = inertial_property_manager.derive_inertial_properties();

        if object.is_effectively_empty() {
            impact_log::debug!("Marked voxel object as empty");
            voxel_manager
                .object_manager
                .mark_voxel_object_as_empty_for_entity(entity_id);
        } else {
            // We need to know how the center of mass of the original object has changed to
            // update its linear velocity. Here we compute the change in the local frame of
            // the object.
            let local_center_of_mass_displacement = new_inertial_properties.center_of_mass().coords
                - original_reference_frame.origin_offset;

            update_physics_components_after_disconnection(
                reference_frame,
                motion,
                rigid_body,
                new_inertial_properties,
                &local_center_of_mass_displacement,
            );
        }

        // We also need to handle the part that was disconnected
        handle_disconnected_voxel_object(
            rigid_body_manager,
            &mut voxel_manager.object_manager,
            ecs_world,
            entity_id,
            original_reference_frame,
            original_motion,
            original_rigid_body,
            disconnected_voxel_object,
            disconnected_object_inertial_property_manager,
        );
    } else {
        // Even though the splitting attempt did not produce a new object, that could
        // just be because the disconnected part was very small. In case this is what
        // happened, we update the physics components to reflect the (small) change in
        // inertial properties.
        update_physics_components_after_voxel_removal_without_disconnection(
            reference_frame,
            rigid_body,
            inertial_property_manager.derive_inertial_properties(),
        );
    }
}

#[allow(clippy::large_types_passed_by_value)]
fn handle_disconnected_voxel_object(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_object_manager: &mut VoxelObjectManager,
    ecs_world: &ECSWorld,
    parent_entity_id: EntityID,
    original_reference_frame: ReferenceFrame,
    original_motion: Motion,
    original_rigid_body: DynamicRigidBody,
    object: DisconnectedVoxelObject,
    mut inertial_property_manager: VoxelObjectInertialPropertyManager,
) {
    let mut reference_frame = original_reference_frame;
    let mut motion = original_motion;
    let mut rigid_body = original_rigid_body;

    // We must compute the center of mass displacement *before* offsetting the
    // origin for `inertial_property_manager`, because after that the new center of
    // mass will not be in the same reference frame as the original one
    let local_center_of_mass_displacement =
        inertial_property_manager.derive_center_of_mass() - original_reference_frame.origin_offset;

    let DisconnectedVoxelObject {
        object,
        origin_offset_in_parent: origin_offset,
    } = object;

    let origin_offset_in_voxel_object_space =
        Vector3::from(origin_offset.map(|offset| offset as fph * object.voxel_extent()));

    // The inertial properties are assumed defined with respect to the lower corner
    // of the voxel object's voxel grid, so we must offset them from the origin of
    // the original object to the origin of the disconnected object
    inertial_property_manager.offset_reference_point_by(&origin_offset_in_voxel_object_space);

    // Similarly, we must offset the reference frame of the new object compared to
    // the frame of the parent object to account for the origin difference
    reference_frame.position += reference_frame
        .create_transform_to_parent_space()
        .transform_vector(&origin_offset_in_voxel_object_space);

    update_physics_components_after_disconnection(
        &mut reference_frame,
        &mut motion,
        &mut rigid_body,
        inertial_property_manager.derive_inertial_properties(),
        &local_center_of_mass_displacement,
    );

    let rigid_body_id = rigid_body_manager.add_dynamic_rigid_body(rigid_body);

    let mut components =
        ArchetypeComponentStorage::try_from_view((&reference_frame, &motion, &rigid_body_id))
            .unwrap();

    add_additional_parent_components_for_disconnected_object(
        ecs_world,
        parent_entity_id,
        &mut components,
    );

    // Stage the object for being added to the scene as an entity by a separate task
    voxel_object_manager.stage_new_voxel_object(StagedVoxelObject {
        object,
        inertial_property_manager: Some(inertial_property_manager),
        components,
    });
}

fn add_additional_parent_components_for_disconnected_object(
    ecs_world: &ECSWorld,
    parent_entity_id: EntityID,
    components: &mut ArchetypeComponentStorage,
) {
    let parent = ecs_world
        .get_entity(parent_entity_id)
        .expect("Missing parent entity for disconnected voxel object");

    if let Some(scene_graph_parent) = parent.get_component() {
        let scene_graph_parent: &SceneGraphParentNodeHandle = scene_graph_parent.access();
        components
            .add_new_component_type(scene_graph_parent.into_storage())
            .unwrap();
    };
}

fn update_physics_components_after_disconnection(
    reference_frame: &mut ReferenceFrame,
    motion: &mut Motion,
    rigid_body: &mut DynamicRigidBody,
    new_inertial_properties: InertialProperties,
    local_center_of_mass_displacement: &Vector3<fph>,
) {
    // The disconnection is really just a partitioning of the mass, inertia tensor
    // and linear and angular momentum of the original object into two parts. Since
    // these quantities are additive, any such partitioning of the object is valid
    // regardless of whether the two parts are connected. What happens during a
    // disconnection is that we change the frames of reference for the two parts.
    // Instead of expressing the partitioned quantities with respect to the center
    // of mass of the original object, we express them with respect to the parts'
    // own center of mass. We also remove the constraint that the parts must behave
    // as being part of the same rigid body, but this doesn't affect anything at
    // the moment of disconnection, only the future evolution. In practice, all we
    // need to do for a part is to assign the properly partitioned mass and inertia
    // tensor properties to its rigid body component, update its reference frame
    // component to use its own center of mass, and update the velocity component to
    // be the velocity of its own center of mass rather than that of the original
    // center of mass.

    let world_center_of_mass_displacement = reference_frame
        .orientation
        .transform_vector(local_center_of_mass_displacement);

    // Compute the linear velocity of the new center of mass compared to the old one
    let linear_velocity_change = motion
        .angular_velocity
        .as_vector()
        .cross(&world_center_of_mass_displacement);

    // Assign new center of mass
    reference_frame.update_origin_offset_while_preserving_position(
        new_inertial_properties.center_of_mass().coords,
    );

    // Transform velocity to new center of mass
    motion.linear_velocity += linear_velocity_change;

    rigid_body.set_inertial_properties(
        new_inertial_properties.mass(),
        *new_inertial_properties.inertia_tensor(),
    );

    // The position of the rigid body (the world space position of its center of
    // mass) changed when we assigned it a new center of mass
    rigid_body.set_position(reference_frame.position);

    // The momentum of the rigid body must be updated to be consistent with the new
    // mass and linear velocity
    rigid_body.synchronize_momentum(&motion.linear_velocity);

    // The angular momentum of the rigid body must be updated to be consistent with
    // the new inertia tensor (the angular velocity is the same for the disconnected
    // object as for the original one)
    rigid_body.synchronize_angular_momentum(&motion.angular_velocity);
}

fn update_physics_components_after_voxel_removal_without_disconnection(
    reference_frame: &mut ReferenceFrame,
    rigid_body: &mut DynamicRigidBody,
    new_inertial_properties: InertialProperties,
) {
    // We don't modify the velocity here, since there was no disconnected object to
    // carry away momentum

    reference_frame.update_origin_offset_while_preserving_position(
        new_inertial_properties.center_of_mass().coords,
    );

    rigid_body.set_position(reference_frame.position);

    rigid_body.set_inertial_properties(
        new_inertial_properties.mass(),
        *new_inertial_properties.inertia_tensor(),
    );
}
