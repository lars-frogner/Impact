//! Voxel absorption.

use crate::{
    Voxel, VoxelManager, VoxelObjectID, VoxelObjectPhysicsContext,
    chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyUpdater},
    interaction::{
        self, DynamicDisconnectedVoxelObject, NewVoxelObjectEntity, VoxelObjectEntity,
        VoxelObjectInteractionContext, VoxelRemovalOutcome,
    },
    mesh::MeshedChunkedVoxelObject,
    voxel_types::VoxelTypeRegistry,
};
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_containers::HashMap;
use impact_geometry::{CapsuleC, SphereC};
use impact_id::{EntityIDManager, define_entity_id_newtype};
use impact_math::{point::Point3C, transform::Isometry3, vector::Vector3C};
use impact_physics::{
    anchor::{AnchorManager, DynamicRigidBodyAnchor},
    rigid_body::RigidBodyManager,
};
use roc_integration::roc;
use std::mem;

define_entity_id_newtype! {
    /// Identifier for a [`VoxelAbsorbingSphere`].
    [pub] VoxelAbsorbingSphereID
}

define_entity_id_newtype! {
    /// Identifier for a [`VoxelAbsorbingCapsule`].
    [pub] VoxelAbsorbingCapsuleID
}

define_component_type! {
    /// Marks that an entity has a voxel-absorbing sphere identified by a
    /// [`VoxelAbsorbingSphereID`].
    ///
    /// Use [`VoxelAbsorbingSphereID::from_entity_id`] to obtain the absorbing
    /// sphere ID from the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HasVoxelAbsorbingSphere;
}

define_component_type! {
    /// Marks that an entity has a voxel-absorbing capsule identified by a
    /// [`VoxelAbsorbingCapsuleID`].
    ///
    /// Use [`VoxelAbsorbingCapsuleID::from_entity_id`] to obtain the absorbing
    /// capsule ID from the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HasVoxelAbsorbingCapsule;
}

define_setup_type! {
    /// A sphere that instantly absorbs voxels it comes in contact with.
    ///
    /// Does nothing if the entity does not have a
    /// [`impact_geometry::ReferenceFrame`].
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelAbsorbingSphere {
        /// The offset of the sphere in the reference frame of the entity.
        offset: Vector3C,
        /// The radius of the sphere.
        radius: f32,
    }
}

define_setup_type! {
    /// A capsule that instantly absorbs voxels it comes in contact with.
    ///
    /// Does nothing if the entity does not have a
    /// [`impact_geometry::ReferenceFrame`].
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelAbsorbingCapsule {
        /// The offset of the starting point of the capsule's central line segment
        /// in the reference frame of the entity.
        offset_to_segment_start: Vector3C,
        /// The displacement vector from the start to the end of the capsule's
        /// central line segment in the reference frame of the entity.
        segment_vector: Vector3C,
        /// The radius of the capsule.
        radius: f32,
    }
}

/// Manages voxel absorption processes and state.
#[derive(Debug)]
pub struct VoxelAbsorptionManager {
    spheres: HashMap<VoxelAbsorbingSphereID, TrackingVoxelAbsorbingSphere>,
    capsules: HashMap<VoxelAbsorbingCapsuleID, TrackingVoxelAbsorbingCapsule>,
}

#[derive(Clone, Debug)]
pub struct TrackingVoxelAbsorbingSphere {
    pub sphere: VoxelAbsorbingSphere,
    pub tracker: VoxelAbsorptionTracker,
}

#[derive(Clone, Debug)]
pub struct TrackingVoxelAbsorbingCapsule {
    pub capsule: VoxelAbsorbingCapsule,
    pub tracker: VoxelAbsorptionTracker,
}

#[derive(Clone, Debug)]
pub struct VoxelAbsorptionTracker {
    absorbed_voxels_by_type: [AbsorbedVoxels; VoxelTypeRegistry::max_n_voxel_types()],
}

#[derive(Clone, Copy, Debug)]
pub struct AbsorbedVoxels {
    pub count: u32,
    pub volume: f32,
}

#[roc]
impl VoxelAbsorbingSphere {
    /// Creates a new [`VoxelAbsorbingSphere`] with the given offset and radius
    /// in the reference frame of the entity.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    {
        offset,
        radius,
    }"#)]
    pub fn new(offset: Vector3C, radius: f32) -> Self {
        assert!(radius >= 0.0);
        Self { offset, radius }
    }

    /// Returns the sphere of influence in the reference frame of the entity.
    ///
    /// The sphere of influence is slightly larger than the absorbing sphere in
    /// order to keep the SDF well-behaved near the boundary of the absorbed
    /// volume.
    pub fn influence_sphere(&self, voxel_extent: f32) -> SphereC {
        SphereC::new(Point3C::from(self.offset), self.radius + 2.0 * voxel_extent)
    }

    /// Computes the new signed distance for the given voxel inside the sphere
    /// of influence.
    pub fn compute_new_signed_distance(
        &self,
        voxel: &Voxel,
        squared_distance_from_center: f32,
    ) -> f32 {
        let sphere_signed_distance = squared_distance_from_center.sqrt() - self.radius;

        // SDF subtraction
        f32::max(voxel.signed_distance().to_f32(), -sphere_signed_distance)
    }
}

#[roc]
impl VoxelAbsorbingCapsule {
    /// Creates a new [`VoxelAbsorbingCapsule`] with the given offset to the
    /// start of the capsule's central line segment, displacement from the start
    /// to the end of the line segment and radius, all in the reference frame of
    /// the entity.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    {
        offset_to_segment_start,
        segment_vector,
        radius,
    }"#)]
    pub fn new(offset_to_segment_start: Vector3C, segment_vector: Vector3C, radius: f32) -> Self {
        assert!(radius >= 0.0);
        Self {
            offset_to_segment_start,
            segment_vector,
            radius,
        }
    }

    /// Returns the capsule of influence in the reference frame of the entity.
    ///
    /// The capsule of influence is slightly larger than the absorbing capsule
    /// in order to keep the SDF well-behaved near the boundary of the absorbed
    /// volume.
    pub fn influence_capsule(&self, voxel_extent: f32) -> CapsuleC {
        CapsuleC::new(
            Point3C::from(self.offset_to_segment_start),
            self.segment_vector,
            self.radius + 2.0 * voxel_extent,
        )
    }

    /// Computes the new signed distance for the given voxel inside the capsule
    /// of influence.
    pub fn compute_new_signed_distance(
        &self,
        voxel: &Voxel,
        squared_distance_from_segment: f32,
    ) -> f32 {
        let capsule_signed_distance = squared_distance_from_segment.sqrt() - self.radius;

        // SDF subtraction
        f32::max(voxel.signed_distance().to_f32(), -capsule_signed_distance)
    }
}

impl VoxelAbsorptionManager {
    pub fn new() -> Self {
        Self {
            spheres: HashMap::default(),
            capsules: HashMap::default(),
        }
    }

    /// Returns a reference to the [`TrackingVoxelAbsorbingSphere`] with the
    /// given ID, or [`None`] if it does not exist.
    pub fn get_absorbing_sphere(
        &self,
        id: VoxelAbsorbingSphereID,
    ) -> Option<&TrackingVoxelAbsorbingSphere> {
        self.spheres.get(&id)
    }

    /// Returns a mutable reference to the [`TrackingVoxelAbsorbingSphere`] with
    /// the given ID, or [`None`] if it does not exist.
    pub fn get_absorbing_sphere_mut(
        &mut self,
        id: VoxelAbsorbingSphereID,
    ) -> Option<&mut TrackingVoxelAbsorbingSphere> {
        self.spheres.get_mut(&id)
    }

    /// Returns a reference to the [`TrackingVoxelAbsorbingCapsule`] with the
    /// given ID, or [`None`] if it does not exist.
    pub fn get_absorbing_capsule(
        &self,
        id: VoxelAbsorbingCapsuleID,
    ) -> Option<&TrackingVoxelAbsorbingCapsule> {
        self.capsules.get(&id)
    }

    /// Returns a mutable reference to the [`TrackingVoxelAbsorbingCapsule`] with
    /// the given ID, or [`None`] if it does not exist.
    pub fn get_absorbing_capsule_mut(
        &mut self,
        id: VoxelAbsorbingCapsuleID,
    ) -> Option<&mut TrackingVoxelAbsorbingCapsule> {
        self.capsules.get_mut(&id)
    }

    /// Adds the given [`VoxelAbsorbingSphere`] to the manager under the given
    /// ID.
    ///
    /// # Errors
    /// Returns an error if the ID already exists.
    pub fn add_absorbing_sphere(
        &mut self,
        id: VoxelAbsorbingSphereID,
        sphere: VoxelAbsorbingSphere,
    ) -> Result<()> {
        if self.spheres.contains_key(&id) {
            bail!("A voxel-absorbing sphere with ID {id} already exists");
        }
        self.spheres
            .insert(id, TrackingVoxelAbsorbingSphere::new(sphere));
        Ok(())
    }

    /// Adds the given [`VoxelAbsorbingCapsule`] to the manager under the given
    /// ID.
    ///
    /// # Errors
    /// Returns an error if the ID already exists.
    pub fn add_absorbing_capsule(
        &mut self,
        id: VoxelAbsorbingCapsuleID,
        capsule: VoxelAbsorbingCapsule,
    ) -> Result<()> {
        if self.capsules.contains_key(&id) {
            bail!("A voxel-absorbing capsule with ID {id} already exists");
        }
        self.capsules
            .insert(id, TrackingVoxelAbsorbingCapsule::new(capsule));
        Ok(())
    }

    /// Removes the [`VoxelAbsorbingSphere`] with the given ID from the manager
    /// if it exists.
    pub fn remove_absorbing_sphere(&mut self, id: VoxelAbsorbingSphereID) {
        self.spheres.remove(&id);
    }

    /// Removes the [`VoxelAbsorbingCapsule`] with the given ID from the manager
    /// if it exists.
    pub fn remove_absorbing_capsule(&mut self, id: VoxelAbsorbingCapsuleID) {
        self.capsules.remove(&id);
    }

    /// Removes all stored voxel absorbers.
    pub fn remove_all_absorbers(&mut self) {
        self.spheres.clear();
        self.capsules.clear();
    }
}

impl TrackingVoxelAbsorbingSphere {
    pub fn new(sphere: VoxelAbsorbingSphere) -> Self {
        Self {
            sphere,
            tracker: VoxelAbsorptionTracker::new(),
        }
    }
}

impl TrackingVoxelAbsorbingCapsule {
    pub fn new(capsule: VoxelAbsorbingCapsule) -> Self {
        Self {
            capsule,
            tracker: VoxelAbsorptionTracker::new(),
        }
    }
}

impl VoxelAbsorptionTracker {
    pub fn new() -> Self {
        Self {
            absorbed_voxels_by_type: [AbsorbedVoxels::zero();
                VoxelTypeRegistry::max_n_voxel_types()],
        }
    }

    pub fn absorbed_voxels_by_type(
        &self,
    ) -> &[AbsorbedVoxels; VoxelTypeRegistry::max_n_voxel_types()] {
        &self.absorbed_voxels_by_type
    }

    pub fn register_absorbed_voxel(&mut self, voxel_volume: f32, voxel: Voxel) {
        self.absorbed_voxels_by_type[voxel.voxel_type().idx()].add_absorbed_voxel(voxel_volume);
    }

    pub fn clear_absorbed(&mut self) {
        self.absorbed_voxels_by_type.fill(AbsorbedVoxels::zero());
    }
}

impl AbsorbedVoxels {
    pub const fn zero() -> Self {
        Self {
            count: 0,
            volume: 0.0,
        }
    }

    pub fn add_absorbed_voxel(&mut self, voxel_volume: f32) {
        self.count += 1;
        self.volume += voxel_volume;
    }
}

/// Applies each voxel-absorbing sphere and capsule to the affected voxel
/// objects.
pub fn apply_absorption<C>(
    context: &mut C,
    entity_id_manager: &mut EntityIDManager,
    voxel_manager: &mut VoxelManager,
    voxel_type_registry: &VoxelTypeRegistry,
    rigid_body_manager: &mut RigidBodyManager,
    anchor_manager: &mut AnchorManager,
) where
    C: VoxelObjectInteractionContext,
{
    let voxel_object_manager = &mut voxel_manager.object_manager;
    let voxel_absorption_manager = voxel_manager.interaction_manager.absorption_manager_mut();

    let absorbing_sphere_entities = context.gather_voxel_absorbing_sphere_entities();
    let absorbing_capsule_entities = context.gather_voxel_absorbing_capsule_entities();

    let mut enabled_count = 0;

    for entity in &absorbing_sphere_entities {
        let absorber_id = VoxelAbsorbingSphereID::from_entity_id(entity.entity_id);
        if let Some(sphere) = voxel_absorption_manager.get_absorbing_sphere_mut(absorber_id) {
            sphere.tracker.clear_absorbed();

            if entity.sphere_to_world_transform.is_some() {
                enabled_count += 1;
            }
        }
    }
    for entity in &absorbing_capsule_entities {
        let absorber_id = VoxelAbsorbingCapsuleID::from_entity_id(entity.entity_id);
        if let Some(capsule) = voxel_absorption_manager.get_absorbing_capsule_mut(absorber_id) {
            capsule.tracker.clear_absorbed();

            if entity.capsule_to_world_transform.is_some() {
                enabled_count += 1;
            }
        }
    }

    if enabled_count == 0 {
        return;
    }

    let arena = ArenaPool::get_arena_for_capacity(
        voxel_object_manager.voxel_object_count() * mem::size_of::<VoxelObjectEntity>(),
    );
    let mut voxel_object_entities =
        AVec::with_capacity_in(voxel_object_manager.voxel_object_count(), &arena);

    context.gather_voxel_object_entities(&mut voxel_object_entities);

    for VoxelObjectEntity { entity_id } in voxel_object_entities {
        let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
        let Some((voxel_object, physics_context)) =
            voxel_object_manager.get_voxel_object_with_physics_context_mut(voxel_object_id)
        else {
            continue;
        };
        let voxel_object = voxel_object.object_mut();

        let Some(rigid_body) =
            rigid_body_manager.get_dynamic_rigid_body_mut(physics_context.rigid_body_id)
        else {
            log::warn!("Voxel object physics context points to missing dynamic rigid body");
            return;
        };

        let local_center_of_mass = physics_context
            .inertial_property_manager
            .derive_center_of_mass();

        let voxel_object_to_world_transform = rigid_body
            .reference_frame()
            .create_transform_to_parent_space()
            .applied_to_translation(&(-local_center_of_mass));

        let world_to_voxel_object_transform = voxel_object_to_world_transform.inverted();

        let mut inertial_property_updater = physics_context.inertial_property_manager.begin_update(
            voxel_object.voxel_extent(),
            voxel_type_registry.mass_densities(),
        );

        for entity in &absorbing_sphere_entities {
            let Some(sphere_to_world_transform) = &entity.sphere_to_world_transform else {
                continue;
            };
            let absorber_id = VoxelAbsorbingSphereID::from_entity_id(entity.entity_id);
            let Some(tracking_absorbing_sphere) =
                voxel_absorption_manager.get_absorbing_sphere_mut(absorber_id)
            else {
                continue;
            };
            apply_sphere_absorption(
                &mut inertial_property_updater,
                voxel_object,
                &world_to_voxel_object_transform,
                tracking_absorbing_sphere,
                sphere_to_world_transform,
            );
        }

        for entity in &absorbing_capsule_entities {
            let Some(capsule_to_world_transform) = &entity.capsule_to_world_transform else {
                continue;
            };
            let absorber_id = VoxelAbsorbingCapsuleID::from_entity_id(entity.entity_id);
            let Some(tracking_absorbing_capsule) =
                voxel_absorption_manager.get_absorbing_capsule_mut(absorber_id)
            else {
                continue;
            };
            apply_capsule_absorption(
                &mut inertial_property_updater,
                voxel_object,
                &world_to_voxel_object_transform,
                tracking_absorbing_capsule,
                capsule_to_world_transform,
            );
        }

        if voxel_object.invalidated_mesh_chunk_indices().len() > 0 {
            let VoxelRemovalOutcome {
                original_object_empty,
                disconnected_object,
            } = interaction::handle_voxel_object_after_removing_voxels(
                anchor_manager,
                voxel_type_registry,
                voxel_object,
                &mut physics_context.inertial_property_manager,
                physics_context.rigid_body_id,
                rigid_body,
                local_center_of_mass,
            );

            if original_object_empty {
                context.on_empty_voxel_object_entity(entity_id);
            }
            if let Some(DynamicDisconnectedVoxelObject {
                voxel_object: new_voxel_object,
                inertial_property_manager,
                rigid_body: new_rigid_body,
                anchors,
            }) = disconnected_object
            {
                let new_meshed_voxel_object = MeshedChunkedVoxelObject::create(new_voxel_object);

                let new_entity_id = entity_id_manager.provide_id();
                let new_voxel_object_id = VoxelObjectID::from_entity_id(new_entity_id);

                voxel_object_manager
                    .add_voxel_object(new_voxel_object_id, new_meshed_voxel_object)
                    .unwrap();

                let new_rigid_body_id = rigid_body_manager.add_dynamic_rigid_body(new_rigid_body);

                let physics_context = VoxelObjectPhysicsContext {
                    inertial_property_manager,
                    rigid_body_id: new_rigid_body_id,
                };

                voxel_object_manager
                    .add_physics_context_for_voxel_object(new_voxel_object_id, physics_context)
                    .unwrap();

                // Update the anchors that have moved from the original object
                // to the disconnected object
                for (anchor_id, point) in anchors {
                    anchor_manager.dynamic_mut().replace(
                        anchor_id,
                        DynamicRigidBodyAnchor {
                            rigid_body_id: new_rigid_body_id,
                            point: point.compact(),
                        },
                    );
                }

                context.on_new_disconnected_voxel_object_entity(
                    NewVoxelObjectEntity {
                        entity_id: new_entity_id,
                        rigid_body_id: new_rigid_body_id,
                    },
                    entity_id,
                );
            }
        }
    }
}

fn apply_sphere_absorption(
    inertial_property_updater: &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
    voxel_object: &mut ChunkedVoxelObject,
    world_to_voxel_object_transform: &Isometry3,
    tracking_absorbing_sphere: &mut TrackingVoxelAbsorbingSphere,
    sphere_to_world_transform: &Isometry3,
) {
    let absorbing_sphere = &tracking_absorbing_sphere.sphere;
    let tracker = &mut tracking_absorbing_sphere.tracker;

    let voxel_volume = voxel_object.voxel_extent().powi(3);

    let influence_sphere = absorbing_sphere
        .influence_sphere(voxel_object.voxel_extent())
        .aligned();

    let influence_sphere_in_voxel_object_space = influence_sphere
        .iso_transformed(sphere_to_world_transform)
        .iso_transformed(world_to_voxel_object_transform);

    voxel_object.modify_voxels_within_sphere(
        &influence_sphere_in_voxel_object_space,
        &mut |object_voxel_indices, squared_distance_from_center, voxel| {
            let was_empty = voxel.is_empty();

            let new_signed_distance =
                absorbing_sphere.compute_new_signed_distance(voxel, squared_distance_from_center);

            voxel.set_signed_distance(new_signed_distance, &mut |voxel| {
                if !was_empty {
                    inertial_property_updater.remove_voxel(&object_voxel_indices, *voxel);
                    tracker.register_absorbed_voxel(voxel_volume, *voxel);
                }
            });
        },
    );
}

fn apply_capsule_absorption(
    inertial_property_updater: &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
    voxel_object: &mut ChunkedVoxelObject,
    world_to_voxel_object_transform: &Isometry3,
    tracking_absorbing_capsule: &mut TrackingVoxelAbsorbingCapsule,
    capsule_to_world_transform: &Isometry3,
) {
    let absorbing_capsule = &tracking_absorbing_capsule.capsule;
    let tracker = &mut tracking_absorbing_capsule.tracker;

    let voxel_volume = voxel_object.voxel_extent().powi(3);

    let influence_capsule = absorbing_capsule
        .influence_capsule(voxel_object.voxel_extent())
        .aligned();

    let influence_capsule_in_voxel_object_space = influence_capsule
        .iso_transformed(capsule_to_world_transform)
        .iso_transformed(world_to_voxel_object_transform);

    voxel_object.modify_voxels_within_capsule(
        &influence_capsule_in_voxel_object_space,
        &mut |object_voxel_indices, squared_distance_from_segment, voxel| {
            let was_empty = voxel.is_empty();

            let new_signed_distance =
                absorbing_capsule.compute_new_signed_distance(voxel, squared_distance_from_segment);

            voxel.set_signed_distance(new_signed_distance, &mut |voxel| {
                if !was_empty {
                    inertial_property_updater.remove_voxel(&object_voxel_indices, *voxel);
                    tracker.register_absorbed_voxel(voxel_volume, *voxel);
                }
            });
        },
    );
}
