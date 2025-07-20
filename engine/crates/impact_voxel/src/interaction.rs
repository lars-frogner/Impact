//! Interactions with voxel objects.

pub mod absorption;

#[cfg(feature = "ecs")]
pub mod ecs;

use crate::{
    VoxelObjectID, VoxelObjectManager,
    chunks::{
        ChunkedVoxelObject, disconnection::DisconnectedVoxelObject,
        inertia::VoxelObjectInertialPropertyManager,
    },
    voxel_types::VoxelTypeRegistry,
};
use absorption::{VoxelAbsorbingCapsule, VoxelAbsorbingSphere};
use impact_geometry::ModelTransform;
use impact_physics::{
    fph,
    quantities::{AngularVelocity, Orientation, Position, Velocity},
    rigid_body::{DynamicRigidBody, DynamicRigidBodyID},
};
use nalgebra::{Isometry3, Vector3};
use tinyvec::TinyVec;

/// Context trait for handling voxel object interactions in a generic way.
///
/// This trait abstracts the process of gathering entities from the world and
/// handling the lifecycle of voxel objects during interactions like voxel
/// absorption.
pub trait VoxelObjectInteractionContext {
    type EntityID;

    /// Gathers all voxel object entities that may participate in interactions.
    fn gather_voxel_object_entities(
        &mut self,
        entities: &mut Vec<VoxelObjectEntity<Self::EntityID>>,
    );

    /// Gathers all active voxel-absorbing sphere entities.
    fn gather_voxel_absorbing_sphere_entities(
        &mut self,
    ) -> TinyVec<[VoxelAbsorbingSphereEntity; 4]>;

    /// Gathers all active voxel-absorbing capsule entities.
    fn gather_voxel_absorbing_capsule_entities(
        &mut self,
    ) -> TinyVec<[VoxelAbsorbingCapsuleEntity; 4]>;

    /// Called when a new voxel object entity is created, typically from
    /// disconnection.
    fn on_new_voxel_object_entity(&mut self, entity: NewVoxelObjectEntity);

    /// Called when a voxel object becomes empty.
    fn on_empty_voxel_object_entity(&mut self, entity_id: Self::EntityID);
}

#[derive(Debug)]
pub struct VoxelObjectEntity<EntityID> {
    pub entity_id: EntityID,
    pub voxel_object_id: VoxelObjectID,
}

#[derive(Debug)]
pub struct NewVoxelObjectEntity {
    pub voxel_object_id: VoxelObjectID,
    pub rigid_body_id: DynamicRigidBodyID,
}

#[derive(Debug, Default)]
pub struct VoxelAbsorbingSphereEntity {
    pub sphere: VoxelAbsorbingSphere,
    pub sphere_to_world_transform: Isometry3<f64>,
}

#[derive(Debug, Default)]
pub struct VoxelAbsorbingCapsuleEntity {
    pub capsule: VoxelAbsorbingCapsule,
    pub capsule_to_world_transform: Isometry3<f64>,
}

#[derive(Debug)]
struct VoxelRemovalOutcome {
    original_object_empty: bool,
    disconnected_object: Option<DynamicDisconnectedVoxelObject>,
}

#[derive(Debug)]
struct DynamicDisconnectedVoxelObject {
    pub voxel_object: ChunkedVoxelObject,
    pub inertial_property_manager: VoxelObjectInertialPropertyManager,
    pub rigid_body: DynamicRigidBody,
}

/// Synchronizes a voxel object's model transform with its current inertial
/// properties.
///
/// Updates the model transform's offset to match the object's center of mass.
pub fn sync_voxel_object_model_transform_with_inertial_properties(
    voxel_object_manager: &VoxelObjectManager,
    voxel_object_id: VoxelObjectID,
    model_transform: &mut ModelTransform,
) {
    let Some(physics_context) = voxel_object_manager.get_physics_context(voxel_object_id) else {
        return;
    };
    model_transform.offset = physics_context
        .inertial_property_manager
        .derive_center_of_mass()
        .cast();
}

fn handle_voxel_object_after_removing_voxels(
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_object: &mut ChunkedVoxelObject,
    inertial_property_manager: &mut VoxelObjectInertialPropertyManager,
    rigid_body: &mut DynamicRigidBody,
    original_local_center_of_mass: Vector3<fph>,
) -> VoxelRemovalOutcome {
    if voxel_object.is_effectively_empty() {
        return VoxelRemovalOutcome {
            original_object_empty: true,
            disconnected_object: None,
        };
    }

    // If the object has not become empty, we must resolve the connected region
    // information
    voxel_object.resolve_connected_regions_between_all_chunks();

    // Removing voxels could have divided the object into multiple disconnected
    // regions. If there is a disconnection, we will split off a disconnected
    // region and make it a new independent voxel object. In the process of
    // splitting off the new object, we will compute the inertial properties of
    // the disconnected region, remove them from the original object and add
    // them to the new object.

    let mut disconnected_object_inertial_property_manager =
        VoxelObjectInertialPropertyManager::zeroed();

    let mut inertial_property_transferrer = inertial_property_manager.begin_transfer_to(
        &mut disconnected_object_inertial_property_manager,
        voxel_object.voxel_extent(),
        voxel_type_registry.mass_densities(),
    );

    if let Some(disconnected_voxel_object) = voxel_object
        .split_off_any_disconnected_region_with_property_transferrer(
            &mut inertial_property_transferrer,
        )
    {
        // The inertial properties of the original object have now changed, and
        // if the object has not become effectively empty due to the splitting
        // we will need them to update its dynamic state.

        let original_position = *rigid_body.position();
        let orientation = *rigid_body.orientation();
        let original_linear_velocity = rigid_body.compute_velocity();
        let angular_velocity = rigid_body.compute_angular_velocity();

        let original_object_empty = voxel_object.is_effectively_empty();

        if !original_object_empty {
            let new_inertial_properties = inertial_property_manager.derive_inertial_properties();

            // We need to know how the center of mass of the original object has
            // changed to update its position and linear velocity. Here we
            // compute the change in the local frame of the object.
            let local_center_of_mass_displacement =
                new_inertial_properties.center_of_mass().coords - original_local_center_of_mass;

            let world_center_of_mass_displacement =
                orientation.transform_vector(&local_center_of_mass_displacement);

            // Compute the linear velocity of the new center of mass compared to
            // the old one
            let linear_velocity_change = angular_velocity
                .as_vector()
                .cross(&world_center_of_mass_displacement);

            rigid_body.set_inertial_properties(
                new_inertial_properties.mass(),
                *new_inertial_properties.inertia_tensor(),
            );

            // The position of the rigid body changes due to the displacement of
            // the center of mass
            rigid_body.set_position(original_position + world_center_of_mass_displacement);

            // The momentum of the rigid body must be updated to be consistent
            // with the new mass and linear velocity
            rigid_body.synchronize_momentum(&(original_linear_velocity + linear_velocity_change));

            // The angular momentum of the rigid body must be updated to be
            // consistent with the new inertia tensor (the angular velocity is
            // the same for the disconnected object as for the original one)
            rigid_body.synchronize_angular_momentum(&angular_velocity);
        }

        // We also need to handle the part that was disconnected
        let dynamic_disconnected_object = handle_disconnected_voxel_object(
            disconnected_voxel_object,
            disconnected_object_inertial_property_manager,
            original_local_center_of_mass,
            original_position,
            orientation,
            original_linear_velocity,
            angular_velocity,
        );

        VoxelRemovalOutcome {
            original_object_empty,
            disconnected_object: Some(dynamic_disconnected_object),
        }
    } else {
        // Even though the splitting attempt did not produce a new object, that could
        // just be because the disconnected part was very small. In case this is what
        // happened, we update the physics components to reflect the (small) change in
        // inertial properties.

        let new_inertial_properties = inertial_property_manager.derive_inertial_properties();

        let local_center_of_mass_displacement =
            new_inertial_properties.center_of_mass().coords - original_local_center_of_mass;

        let world_center_of_mass_displacement = rigid_body
            .orientation()
            .transform_vector(&local_center_of_mass_displacement);

        // We don't modify the velocity here, since there was no disconnected object to
        // carry away momentum

        rigid_body.set_position(rigid_body.position() + world_center_of_mass_displacement);

        rigid_body.set_inertial_properties(
            new_inertial_properties.mass(),
            *new_inertial_properties.inertia_tensor(),
        );

        VoxelRemovalOutcome {
            original_object_empty: false,
            disconnected_object: None,
        }
    }
}

fn handle_disconnected_voxel_object(
    disconnected_object: DisconnectedVoxelObject,
    mut inertial_property_manager: VoxelObjectInertialPropertyManager,
    original_local_center_of_mass: Vector3<fph>,
    original_position: Position,
    orientation: Orientation,
    original_linear_velocity: Velocity,
    angular_velocity: AngularVelocity,
) -> DynamicDisconnectedVoxelObject {
    // The disconnection is really just a partitioning of the mass, inertia
    // tensor and linear and angular momentum of the original object into two
    // parts. Since these quantities are additive, any such partitioning of the
    // object is valid regardless of whether the two parts are connected. What
    // happens during a disconnection is that we change the frames of reference
    // for the two parts. Instead of expressing the partitioned quantities with
    // respect to the center of mass of the original object, we express them
    // with respect to the parts' own centers of mass. We also remove the
    // constraint that the parts must behave as being part of the same rigid
    // body, but this doesn't affect anything at the moment of disconnection,
    // only the future evolution. In practice, all we need to do for a part is
    // to assign the properly partitioned mass and inertia tensor properties to
    // its rigid body state, update its position to use its own center of mass,
    // and update its linear velocity to be the velocity of its own center of
    // mass rather than that of the original center of mass.

    // We must compute the center of mass displacement *before* offsetting the
    // origin for `inertial_property_manager`, because after that the new center
    // of mass will not be in the same reference frame as the original one
    let local_center_of_mass_displacement =
        inertial_property_manager.derive_center_of_mass() - original_local_center_of_mass;

    let world_center_of_mass_displacement =
        orientation.transform_vector(&local_center_of_mass_displacement);

    // Compute the linear velocity of the new center of mass compared to the old
    // one
    let linear_velocity_change = angular_velocity
        .as_vector()
        .cross(&world_center_of_mass_displacement);

    let new_position = original_position + world_center_of_mass_displacement;
    let new_linear_velocity = original_linear_velocity + linear_velocity_change;

    let DisconnectedVoxelObject {
        voxel_object,
        origin_offset_in_parent,
    } = disconnected_object;

    let origin_offset_in_voxel_object_space = Vector3::from(
        origin_offset_in_parent.map(|offset| offset as fph * voxel_object.voxel_extent()),
    );

    // The inertial properties are assumed defined with respect to the lower
    // corner of the voxel object's voxel grid, so we must offset them from the
    // origin of the original object to the origin of the disconnected object
    inertial_property_manager.offset_reference_point_by(&origin_offset_in_voxel_object_space);

    let new_inertial_properties = inertial_property_manager.derive_inertial_properties();

    let rigid_body = DynamicRigidBody::new(
        new_inertial_properties.mass(),
        *new_inertial_properties.inertia_tensor(),
        new_position,
        orientation,
        new_linear_velocity,
        angular_velocity,
    );

    DynamicDisconnectedVoxelObject {
        voxel_object,
        inertial_property_manager,
        rigid_body,
    }
}
