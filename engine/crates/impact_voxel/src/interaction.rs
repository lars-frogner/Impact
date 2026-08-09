//! Interactions with voxel objects.

pub mod absorption;
pub mod fracturing;

#[cfg(feature = "ecs")]
pub mod systems;

use crate::{
    VoxelObjectBufferPool, VoxelObjectID, VoxelObjectManager, VoxelObjectPhysicsContext,
    interaction::fracturing::VoxelFracturingConfig,
    mesh::{MeshedVoxelObject, MeshedVoxelObjectBuffers, VoxelObjectMeshBuffers},
    object::{
        VoxelObject, extraction::ExtractionResult, inertia::VoxelObjectInertialPropertyManager,
    },
    voxel_types::VoxelTypeRegistry,
};
use absorption::VoxelAbsorptionManager;
use anyhow::Result;
use fracturing::VoxelObjectFracturingManager;
use impact_alloc::{AVec, Allocator};
use impact_geometry::{AxisAlignedBoxC, ModelTransform};
use impact_id::EntityID;
use impact_intersection::bounding_volume::{BoundingVolumeID, BoundingVolumeManager};
use impact_math::{
    point::{Point3, Point3C},
    transform::Isometry3,
    vector::Vector3,
};
use impact_physics::{
    anchor::{AnchorManager, DynamicRigidBodyAnchor, DynamicRigidBodyAnchorID},
    quantities::{AngularVelocity, Orientation, Position, Velocity},
    rigid_body::{DynamicRigidBody, DynamicRigidBodyID, RigidBodyManager},
};
use std::mem;
use tinyvec::TinyVec;

/// Context trait for handling voxel object interactions in a generic way.
///
/// This trait abstracts the process of gathering entities from the world and
/// handling the lifecycle of voxel objects during interactions like voxel
/// absorption.
pub trait VoxelObjectInteractionContext {
    /// Whether the entity with the given ID exists.
    fn entity_exists(&self, entity_id: EntityID) -> bool;

    /// Gathers all active voxel-absorbing sphere entities.
    fn gather_voxel_absorbing_sphere_entities(
        &mut self,
    ) -> TinyVec<[VoxelAbsorbingSphereEntity; 4]>;

    /// Gathers all active voxel-absorbing capsule entities.
    fn gather_voxel_absorbing_capsule_entities(
        &mut self,
    ) -> TinyVec<[VoxelAbsorbingCapsuleEntity; 4]>;

    /// Called when a new voxel object entity should be created for an extracted
    /// voxel object.
    fn create_extracted_voxel_object_entity(
        &mut self,
        new_entity_id: EntityID,
        parent_entity_id: EntityID,
    );

    /// Called when new voxel object entities should be created for extracted
    /// voxel objects.
    fn create_extracted_voxel_object_entities(
        &mut self,
        new_entity_ids: Vec<EntityID>,
        parent_entity_id: EntityID,
    );

    /// Called when a voxel object should be removed.
    fn remove_voxel_object_entity(&mut self, entity_id: EntityID);
}

/// Manages voxel interaction processes and state.
#[derive(Debug)]
pub struct VoxelInteractionManager {
    pub absorption_manager: VoxelAbsorptionManager,
    pub fracturing_manager: VoxelObjectFracturingManager,
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct VoxelInteractionConfig {
    pub fracturing: VoxelFracturingConfig,
}

#[derive(Debug, Default)]
pub struct VoxelAbsorbingSphereEntity {
    pub entity_id: EntityID,
    pub sphere_to_world_transform: Option<Isometry3>,
}

#[derive(Debug, Default)]
pub struct VoxelAbsorbingCapsuleEntity {
    pub entity_id: EntityID,
    pub capsule_to_world_transform: Option<Isometry3>,
}

#[derive(Debug)]
struct VoxelObjectUpdateOutcome<A: Allocator> {
    original_object_empty: bool,
    extracted_disconnected_objects: AVec<DynamicExtractedVoxelObject, A>,
    lost_anchors: Anchors,
}

#[derive(Debug)]
struct ExtractedDisconnectedObjects<A: Allocator> {
    objects: AVec<DynamicExtractedVoxelObject, A>,
    /// There could be disconnections even though no disconnected region was big
    /// enough to be extracted into a separate object.
    had_disconnections: bool,
}

/// An extracted voxel object together with the dynamic state it needs to become
/// a standalone voxel object entity.
#[derive(Debug)]
struct DynamicExtractedVoxelObject {
    voxel_object: VoxelObject,
    mesh_buffers: VoxelObjectMeshBuffers,
    inertial_property_manager: VoxelObjectInertialPropertyManager,
    rigid_body: DynamicRigidBody,
    anchors: Anchors,
    origin_offset_in_parent: [usize; 3],
    coordinate_changes: ExtractedVoxelObjectCoordinateChanges,
}

/// A meshed voxel object along with the state required to spawn it as a
/// standalone voxel object entity.
#[derive(Debug)]
struct SpawnableVoxelObject {
    meshed_voxel_object: MeshedVoxelObject,
    inertial_property_manager: VoxelObjectInertialPropertyManager,
    rigid_body: DynamicRigidBody,
    anchors: Anchors,
}

#[derive(Debug)]
struct ExtractedVoxelObjectDynamics {
    pub rigid_body: DynamicRigidBody,
    pub coordinate_changes: ExtractedVoxelObjectCoordinateChanges,
}

#[derive(Clone, Debug)]
struct ExtractedVoxelObjectCoordinateChanges {
    /// The center of mass of the extracted object relative to its origin (in
    /// model space, not world space).
    new_local_center_of_mass: Vector3,
    /// The center of mass of the original object relative to the origin of the
    /// extracted object (in model space, not world space).
    original_local_center_of_mass_relative_to_new_origin: Vector3,
}

type Anchors = TinyVec<[(DynamicRigidBodyAnchorID, Position); 4]>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VoxelRemoval {
    /// Voxels were removed, so the object may have become empty or gained
    /// disconnected regions.
    Occurred(RemovedMassFate),
    /// No voxels were removed, so the object is in a consistent state but have
    /// disconnected regions.
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RemovedMassFate {
    Destroyed,
    Transferred,
}

impl VoxelInteractionManager {
    pub fn new(config: VoxelInteractionConfig) -> Result<Self> {
        Ok(Self {
            absorption_manager: VoxelAbsorptionManager::new(),
            fracturing_manager: VoxelObjectFracturingManager::new(config.fracturing)?,
        })
    }

    /// Returns a reference to the [`VoxelAbsorptionManager`].
    pub fn absorption_manager(&self) -> &VoxelAbsorptionManager {
        &self.absorption_manager
    }

    /// Returns a mutable reference to the [`VoxelAbsorptionManager`].
    pub fn absorption_manager_mut(&mut self) -> &mut VoxelAbsorptionManager {
        &mut self.absorption_manager
    }

    /// Returns a reference to the [`VoxelObjectFracturingManager`].
    pub fn fracturing_manager(&self) -> &VoxelObjectFracturingManager {
        &self.fracturing_manager
    }

    /// Returns a mutable reference to the [`VoxelObjectFracturingManager`].
    pub fn fracturing_manager_mut(&mut self) -> &mut VoxelObjectFracturingManager {
        &mut self.fracturing_manager
    }

    /// Removes all voxel interaction state and frees up all allocated memory.
    pub fn reset_and_free(&mut self) {
        self.absorption_manager.reset_and_free();
        self.fracturing_manager.reset_and_free();
    }
}

/// Synchronizes a voxel object's model transform with its current inertial
/// properties.
///
/// Updates the model transform's offset to match the object's center of mass.
pub fn sync_voxel_object_model_transform_with_inertial_properties(
    voxel_object_manager: &VoxelObjectManager,
    entity_id: EntityID,
    model_transform: &mut ModelTransform,
) {
    let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
    let Some(physics_context) = voxel_object_manager.get_physics_context(voxel_object_id) else {
        return;
    };

    let center_of_mass = physics_context
        .inertial_property_manager
        .derive_center_of_mass();

    model_transform.offset = center_of_mass.compact();
}

/// Updates the voxel object's bounding volume in the bounding volume manager to
/// match the current bounding volume of the object.
pub fn sync_voxel_object_bounding_volume(
    voxel_object_manager: &VoxelObjectManager,
    bounding_volume_manager: &mut BoundingVolumeManager,
    entity_id: EntityID,
) {
    let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
    let Some(voxel_object) = voxel_object_manager.get_voxel_object(voxel_object_id) else {
        return;
    };

    let aabb = if voxel_object.object().contains_only_empty_voxels() {
        AxisAlignedBoxC::new(Point3C::origin(), Point3C::origin())
    } else {
        voxel_object.object().compute_aabb().compact()
    };

    let bounding_volume_id = BoundingVolumeID::from_entity_id(entity_id);
    *bounding_volume_manager
        .get_bounding_volume_mut(bounding_volume_id)
        .expect("Missing bounding volume for voxel object") = aabb;
}

/// Brings a voxel object that has either had voxels removed or is in a
/// consistent state by with potential disconnected parts.
///
/// Any disconnected regions are split off into new independent objects, and the
/// inertial properties, dynamic state and anchors of the original object are
/// updated accordingly.
///
/// `anchors` must contain the anchors of the original object, with points
/// relative to `original_local_center_of_mass`. Anchors that remain attached to
/// the original object are updated to be relative to its new center of mass,
/// while anchors that no longer sit on the object are removed from `anchors`
/// and reported in the outcome.
fn update_modified_or_disconnected_voxel_object<A: Allocator>(
    alloc: A,
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
    voxel_object: &mut VoxelObject,
    inertial_property_manager: &mut VoxelObjectInertialPropertyManager,
    rigid_body: &mut DynamicRigidBody,
    anchors: &mut Anchors,
    original_local_center_of_mass: Vector3,
    voxel_removal: VoxelRemoval,
) -> VoxelObjectUpdateOutcome<A> {
    // Only a removal of voxels could have made the object empty
    if voxel_removal != VoxelRemoval::None && voxel_object.is_effectively_empty() {
        return VoxelObjectUpdateOutcome {
            original_object_empty: true,
            extracted_disconnected_objects: AVec::new_in(alloc),
            lost_anchors: mem::take(anchors),
        };
    }

    // The object may consist of multiple disconnected regions, either because
    // removing voxels divided it or because it was already disconnected. If
    // there are disconnections, we will split off the disconnected regions and
    // make them new independent voxel objects. In the process of splitting off
    // a new object, we will compute the inertial properties of the disconnected
    // region, remove them from the original object and add them to the new
    // object.

    let mut disconnected_objects = extract_all_disconnected_objects(
        alloc,
        voxel_type_registry,
        voxel_object_buffer_pool,
        voxel_object,
        inertial_property_manager,
        rigid_body,
        original_local_center_of_mass,
    );

    if voxel_removal == VoxelRemoval::None && !disconnected_objects.had_disconnections {
        // Nothing was removed and nothing was disconnected, so the object and
        // its dynamic state are still consistent
        return VoxelObjectUpdateOutcome {
            original_object_empty: false,
            extracted_disconnected_objects: disconnected_objects.objects,
            lost_anchors: Anchors::new(),
        };
    }

    if disconnected_objects.objects.is_empty() {
        // If the object had disconnected regions but none were significant
        // enough to produce a new object, we have to re-evaluate the emptiness
        // of the original object since the disconnection attempt could have
        // removed voxels
        let original_object_empty =
            disconnected_objects.had_disconnections && voxel_object.is_effectively_empty();

        let lost_anchors = if original_object_empty {
            // All anchors for the object are lost
            mem::take(anchors)
        } else {
            // There was no disconnected region to carry away momentum, but
            // whether or not we want the removal of voxels in itself to change
            // the momentum of the original object is up to the caller.
            let new_local_center_of_mass = match voxel_removal {
                VoxelRemoval::Occurred(RemovedMassFate::Transferred) => {
                    apply_updated_inertial_properties_to_rigid_body(
                        rigid_body,
                        inertial_property_manager,
                        original_local_center_of_mass,
                    )
                }
                // Any mass lost to the discarded disconnected regions is
                // destroyed
                VoxelRemoval::Occurred(RemovedMassFate::Destroyed) | VoxelRemoval::None => {
                    apply_updated_inertial_properties_to_rigid_body_preserving_momentum(
                        rigid_body,
                        inertial_property_manager,
                        original_local_center_of_mass,
                    )
                }
            };

            update_anchors_for_updated_voxel_object(
                anchors,
                voxel_object,
                &original_local_center_of_mass,
                &new_local_center_of_mass,
            )
        };

        return VoxelObjectUpdateOutcome {
            original_object_empty,
            extracted_disconnected_objects: disconnected_objects.objects,
            lost_anchors,
        };
    }

    // The inertial properties of the original object have changed due to the
    // extraction of disconnected regions, and potentially also due to the
    // original voxel removal. If the object has not become effectively empty
    // due to the splitting we will need them to update its dynamic state.

    let original_object_empty = voxel_object.is_effectively_empty();

    let mut lost_anchors = if original_object_empty {
        // All anchors for the object are lost
        mem::take(anchors)
    } else {
        let new_local_center_of_mass = apply_updated_inertial_properties_to_rigid_body(
            rigid_body,
            inertial_property_manager,
            original_local_center_of_mass,
        );

        update_anchors_for_updated_voxel_object(
            anchors,
            voxel_object,
            &original_local_center_of_mass,
            &new_local_center_of_mass,
        )
    };

    for extracted_object in &mut disconnected_objects.objects {
        let anchors = transfer_anchors_to_extracted_voxel_object(
            &mut lost_anchors,
            &extracted_object.voxel_object,
            &extracted_object.coordinate_changes,
        );
        extracted_object.anchors = anchors;
    }

    VoxelObjectUpdateOutcome {
        original_object_empty,
        extracted_disconnected_objects: disconnected_objects.objects,
        lost_anchors,
    }
}

fn extract_all_disconnected_objects<A: Allocator>(
    alloc: A,
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
    voxel_object: &mut VoxelObject,
    inertial_property_manager: &mut VoxelObjectInertialPropertyManager,
    rigid_body: &mut DynamicRigidBody,
    original_local_center_of_mass: Vector3,
) -> ExtractedDisconnectedObjects<A> {
    let mut objects = AVec::new_in(alloc);

    let mut had_disconnections = false;

    while let Some(disconnected_regions) = voxel_object.find_two_disconnected_regions() {
        had_disconnections = true;

        let mut disconnected_object_inertial_property_manager =
            VoxelObjectInertialPropertyManager::zeroed();

        let mut inertial_property_transferrer = inertial_property_manager.begin_transfer_to(
            &mut disconnected_object_inertial_property_manager,
            voxel_object.voxel_extent(),
            voxel_type_registry.mass_densities(),
        );

        let buffers = voxel_object_buffer_pool.take_or_create_buffers();

        let extraction_result = voxel_object.extract_smallest_region_with_property_transferrer(
            disconnected_regions,
            buffers.object_buffers,
            &mut inertial_property_transferrer,
        );

        match extraction_result {
            ExtractionResult::Extracted(disconnected_voxel_object) => {
                let original_position = rigid_body.position().aligned();
                let orientation = rigid_body.orientation().aligned();
                let original_linear_velocity = rigid_body.compute_velocity();
                let angular_velocity = rigid_body.compute_angular_velocity();

                let disconnected_object_dynamics = determine_extracted_voxel_object_dynamics(
                    &disconnected_voxel_object.voxel_object,
                    disconnected_voxel_object.origin_offset_in_parent,
                    &mut disconnected_object_inertial_property_manager,
                    original_local_center_of_mass,
                    original_position,
                    orientation,
                    original_linear_velocity,
                    angular_velocity,
                );

                objects.push(DynamicExtractedVoxelObject {
                    voxel_object: disconnected_voxel_object.voxel_object,
                    mesh_buffers: buffers.mesh_buffers,
                    inertial_property_manager: disconnected_object_inertial_property_manager,
                    rigid_body: disconnected_object_dynamics.rigid_body,
                    anchors: Anchors::new(),
                    origin_offset_in_parent: disconnected_voxel_object.origin_offset_in_parent,
                    coordinate_changes: disconnected_object_dynamics.coordinate_changes,
                });
            }
            ExtractionResult::NotExtracted(object_buffers) => {
                // Return the buffers to the pool
                voxel_object_buffer_pool.add_buffers(MeshedVoxelObjectBuffers {
                    object_buffers,
                    mesh_buffers: buffers.mesh_buffers,
                });
            }
        }
    }

    ExtractedDisconnectedObjects {
        objects,
        had_disconnections,
    }
}

fn apply_updated_inertial_properties_to_rigid_body(
    rigid_body: &mut DynamicRigidBody,
    new_inertial_property_manager: &VoxelObjectInertialPropertyManager,
    original_local_center_of_mass: Vector3,
) -> Vector3 {
    let original_position = rigid_body.position().aligned();
    let orientation = rigid_body.orientation().aligned();
    let original_linear_velocity = rigid_body.compute_velocity();
    let angular_velocity = rigid_body.compute_angular_velocity();

    let new_inertial_properties = new_inertial_property_manager.derive_inertial_properties();
    let new_local_center_of_mass = new_inertial_properties.center_of_mass().as_vector();

    // We need to know how the center of mass of the original object has
    // changed to update its position and linear velocity. Here we
    // compute the change in the local frame of the object.
    let local_center_of_mass_displacement =
        new_local_center_of_mass - original_local_center_of_mass;

    let world_center_of_mass_displacement =
        orientation.rotate_vector(&local_center_of_mass_displacement);

    // Compute the linear velocity of the new center of mass compared to
    // the old one
    let linear_velocity_change = angular_velocity
        .as_vector()
        .cross(&world_center_of_mass_displacement);

    let new_inertia_tensor = new_inertial_properties.inertia_tensor();
    let new_position = original_position + world_center_of_mass_displacement;
    let new_velocity = original_linear_velocity + linear_velocity_change;

    rigid_body
        .set_inertial_properties(new_inertial_properties.mass(), new_inertia_tensor.compact());

    // The position of the rigid body changes due to the displacement of
    // the center of mass
    rigid_body.set_position(new_position.compact());

    // The momentum of the rigid body must be updated to be consistent
    // with the new mass and linear velocity
    rigid_body.synchronize_momentum(&new_velocity);

    // The angular momentum of the rigid body must be updated to be
    // consistent with the new inertia tensor (the angular velocity is
    // the same for the disconnected object as for the original one)
    rigid_body.synchronize_angular_momentum(&angular_velocity);

    *new_local_center_of_mass
}

fn apply_updated_inertial_properties_to_rigid_body_preserving_momentum(
    rigid_body: &mut DynamicRigidBody,
    new_inertial_property_manager: &VoxelObjectInertialPropertyManager,
    original_local_center_of_mass: Vector3,
) -> Vector3 {
    let original_position = rigid_body.position().aligned();
    let orientation = rigid_body.orientation().aligned();

    let new_inertial_properties = new_inertial_property_manager.derive_inertial_properties();
    let new_local_center_of_mass = new_inertial_properties.center_of_mass().as_vector();

    let local_center_of_mass_displacement =
        new_local_center_of_mass - original_local_center_of_mass;

    let world_center_of_mass_displacement =
        orientation.rotate_vector(&local_center_of_mass_displacement);

    let new_inertia_tensor = new_inertial_properties.inertia_tensor();
    let new_position = original_position + world_center_of_mass_displacement;

    rigid_body.set_position(new_position.compact());

    rigid_body
        .set_inertial_properties(new_inertial_properties.mass(), new_inertia_tensor.compact());

    *new_local_center_of_mass
}

fn spawn_extracted_voxel_object(
    voxel_object_manager: &mut VoxelObjectManager,
    rigid_body_manager: &mut RigidBodyManager,
    anchor_manager: &mut AnchorManager,
    spawnable_object: SpawnableVoxelObject,
    entity_id: EntityID,
) {
    let meshed_voxel_object = spawnable_object.meshed_voxel_object;

    let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
    let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);

    voxel_object_manager
        .add_voxel_object(voxel_object_id, meshed_voxel_object)
        .unwrap();

    rigid_body_manager
        .add_dynamic_rigid_body(rigid_body_id, spawnable_object.rigid_body)
        .unwrap();

    let physics_context = VoxelObjectPhysicsContext {
        inertial_property_manager: spawnable_object.inertial_property_manager,
    };

    voxel_object_manager
        .add_physics_context_for_voxel_object(voxel_object_id, physics_context)
        .unwrap();

    // Update the anchors that have moved from the original object
    // to the extracted object
    for (anchor_id, point) in spawnable_object.anchors {
        anchor_manager.dynamic_mut().replace(
            anchor_id,
            DynamicRigidBodyAnchor {
                rigid_body_id,
                point: point.compact(),
            },
        );
    }
}

fn determine_extracted_voxel_object_dynamics(
    voxel_object: &VoxelObject,
    origin_offset_in_parent: [usize; 3],
    inertial_property_manager: &mut VoxelObjectInertialPropertyManager,
    original_local_center_of_mass: Vector3,
    original_position: Position,
    orientation: Orientation,
    original_linear_velocity: Velocity,
    angular_velocity: AngularVelocity,
) -> ExtractedVoxelObjectDynamics {
    // In terms of physics, extracting part of a voxel object is really just a
    // partitioning of the mass, inertia tensor and linear and angular momentum
    // of the original object into two parts. Since these quantities are
    // additive, any such partitioning of the object is valid regardless of
    // whether the two parts are connected. What happens during extraction is
    // that we change the frame of reference for the extracted part. Instead of
    // expressing the extracted quantities with respect to the center of mass of
    // the original object, we express them with respect to the parts' own
    // centers of mass. We also remove the constraint that the parts must behave
    // as being part of the same rigid body, but this doesn't affect anything at
    // the moment of extraction, only the future evolution. In practice, all we
    // need to do for a part is to assign the properly partitioned mass and
    // inertia tensor properties to its rigid body state, update its position to
    // use its own center of mass, and update its linear velocity to be the
    // velocity of its own center of mass rather than that of the original
    // center of mass.

    // We must compute the center of mass displacement *before* offsetting the
    // origin for `inertial_property_manager`, because after that the new center
    // of mass will not be in the same reference frame as the original one
    let local_center_of_mass_displacement =
        inertial_property_manager.derive_center_of_mass() - original_local_center_of_mass;

    let world_center_of_mass_displacement =
        orientation.rotate_vector(&local_center_of_mass_displacement);

    // Compute the linear velocity of the new center of mass compared to the old
    // one
    let linear_velocity_change = angular_velocity
        .as_vector()
        .cross(&world_center_of_mass_displacement);

    let origin_offset_in_voxel_object_space = Vector3::from(
        origin_offset_in_parent.map(|offset| offset as f32 * voxel_object.voxel_extent()),
    );

    // The inertial properties are assumed defined with respect to the lower
    // corner of the voxel object's voxel grid, so we must offset them from the
    // origin of the original object to the origin of the extracted object
    inertial_property_manager.offset_reference_point_by(&origin_offset_in_voxel_object_space);

    let new_inertial_properties = inertial_property_manager.derive_inertial_properties();
    let new_local_center_of_mass = new_inertial_properties.center_of_mass().as_vector();
    let new_inertia_tensor = new_inertial_properties.inertia_tensor();

    let new_position = original_position + world_center_of_mass_displacement;
    let new_linear_velocity = original_linear_velocity + linear_velocity_change;

    let rigid_body = DynamicRigidBody::new(
        new_inertial_properties.mass(),
        new_inertia_tensor.compact(),
        new_position.compact(),
        orientation.compact(),
        new_linear_velocity.compact(),
        angular_velocity.compact(),
    );

    let coordinate_changes = ExtractedVoxelObjectCoordinateChanges::new(
        original_local_center_of_mass,
        origin_offset_in_voxel_object_space,
        *new_local_center_of_mass,
    );

    ExtractedVoxelObjectDynamics {
        rigid_body,
        coordinate_changes,
    }
}

impl DynamicExtractedVoxelObject {
    fn into_spawnable(self) -> SpawnableVoxelObject {
        SpawnableVoxelObject {
            meshed_voxel_object: MeshedVoxelObject::create(self.mesh_buffers, self.voxel_object),
            inertial_property_manager: self.inertial_property_manager,
            rigid_body: self.rigid_body,
            anchors: self.anchors,
        }
    }
}

impl ExtractedVoxelObjectCoordinateChanges {
    fn new(
        original_local_center_of_mass: Vector3,
        origin_offset_in_voxel_object_space: Vector3,
        new_local_center_of_mass: Vector3,
    ) -> Self {
        let original_local_center_of_mass_relative_to_new_origin =
            original_local_center_of_mass - origin_offset_in_voxel_object_space;
        Self {
            original_local_center_of_mass_relative_to_new_origin,
            new_local_center_of_mass,
        }
    }

    /// Transforms a vector given relative to the center of mass of the original
    /// object to be relative to the origin of the new object.
    fn change_from_original_com_frame_to_new_origin_frame(&self, point: &Point3) -> Point3 {
        point + self.original_local_center_of_mass_relative_to_new_origin
    }

    /// Transforms a vector given relative to the center of mass of the original
    /// object to be relative to the origin of the new object.
    fn change_from_new_origin_frame_to_new_com_frame(&self, point: &Point3) -> Point3 {
        point - self.new_local_center_of_mass
    }
}

fn get_all_rigid_body_anchors(
    anchor_manager: &AnchorManager,
    rigid_body_id: DynamicRigidBodyID,
) -> Anchors {
    anchor_manager
        .dynamic()
        .anchors_for_body(rigid_body_id)
        .map(|(id, point)| (id, point.aligned()))
        .collect()
}

fn apply_anchors_to_rigid_body(
    anchor_manager: &mut AnchorManager,
    rigid_body_id: DynamicRigidBodyID,
    anchors: &Anchors,
) {
    anchor_manager.dynamic_mut().for_each_body_anchor_mut(
        rigid_body_id,
        &mut |anchor_id, compact_anchor_point| {
            if let Some((_, anchor_point)) = anchors.iter().find(|(id, _)| *id == anchor_id) {
                *compact_anchor_point = anchor_point.compact();
            }
        },
    );
}

/// Removes the anchors that are no longer attached to the given voxel object
/// from `anchors` and returns them, and makes the remaining anchor points
/// relative to the new center of mass of the object.
fn update_anchors_for_updated_voxel_object(
    anchors: &mut Anchors,
    voxel_object: &VoxelObject,
    original_local_center_of_mass: &Vector3,
    new_local_center_of_mass: &Vector3,
) -> Anchors {
    let mut lost_anchors = Anchors::new();

    // Iterate in reverse order so we can swap remove anchors without
    // invalidating unchecked ones
    for idx in (0..anchors.len()).rev() {
        let (anchor_id, anchor_point) = anchors[idx];

        // The anchor point is relative to the original center of mass, so we
        // add that to get it relative to the origin of the voxel grid
        let local_anchor = anchor_point + original_local_center_of_mass;

        if voxel_object
            .get_voxel_at_coords_if_occupied(local_anchor.as_vector())
            .is_some()
        {
            // The anchor is still attached to the original object, so now we
            // correct it to be relative to the new center of mass
            anchors[idx].1 = local_anchor - new_local_center_of_mass;
        } else {
            // The anchor is no longer attached to the original object, so we
            // hold on to it to check if it is anchored to a disconnected object
            // or to remove it
            lost_anchors.push((anchor_id, anchor_point));
            anchors.swap_remove(idx);
        }
    }

    lost_anchors
}

fn transfer_anchors_to_extracted_voxel_object(
    remaining_lost_anchors: &mut Anchors,
    extracted_object: &VoxelObject,
    coordinate_changes: &ExtractedVoxelObjectCoordinateChanges,
) -> Anchors {
    let mut extracted_anchors = Anchors::new();

    // Iterate in reverse order so we can swap remove anchors without
    // invalidating unchecked ones
    for idx in (0..remaining_lost_anchors.len()).rev() {
        let (anchor_id, anchor_point) = &remaining_lost_anchors[idx];

        // Make anchor point relative to the origin of the extracted object for
        // querying which voxel it sits on
        let local_anchor =
            coordinate_changes.change_from_original_com_frame_to_new_origin_frame(anchor_point);

        if extracted_object
            .get_voxel_at_coords_if_occupied(local_anchor.as_vector())
            .is_some()
        {
            // The anchor is attached to the extracted object, so we specify the
            // point relative to the the center of mass so it can be assigned to
            // the object
            let new_anchor_point =
                coordinate_changes.change_from_new_origin_frame_to_new_com_frame(&local_anchor);

            extracted_anchors.push((*anchor_id, new_anchor_point));

            remaining_lost_anchors.swap_remove(idx);
        }
    }

    extracted_anchors
}
