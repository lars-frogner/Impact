//! Voxel absorption.

use crate::{
    Voxel, VoxelManager, VoxelObjectID, VoxelObjectManager, VoxelSignedDistance,
    generation::sdf::{Smoothness, hard_sdf_subtraction, sdf_subtraction},
    interaction::{self, RemovedMassFate, VoxelObjectInteractionContext, VoxelRemovalOutcome},
    object::{self, CHUNK_SIZE, VoxelObject, inertia::VoxelObjectInertialPropertyUpdater, sdf},
    voxel_types::VoxelTypeRegistry,
};
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::{arena::ArenaPool, avec};
use impact_containers::HashMap;
use impact_geometry::{CapsuleC, SphereC};
use impact_id::{EntityID, EntityIDManager, define_entity_id_newtype};
use impact_intersection::{IntersectionManager, bounding_volume::BoundingVolumeID};
use impact_math::{
    point::Point3C,
    transform::Isometry3,
    vector::{Vector3, Vector3C},
};
use impact_physics::{
    anchor::AnchorManager,
    rigid_body::{DynamicRigidBodyID, RigidBodyManager},
};
use roc_integration::roc;
use std::{array, ops::Range};
use tinyvec::TinyVec;

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
    absorbing_spheres: HashMap<VoxelAbsorbingSphereID, TrackingVoxelAbsorbingSphere>,
    absorbing_capsules: HashMap<VoxelAbsorbingCapsuleID, TrackingVoxelAbsorbingCapsule>,
    mutual_absorption_processes: HashMap<[EntityID; 2], MutualVoxelAbsorptionProcess>,
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

#[derive(Clone, Copy, Debug)]
pub struct MutualVoxelAbsorptionProcess {
    /// The smoothness to use for the soft SDF subtraction between the objects.
    pub smoothness: Smoothness,
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

    /// Returns the sphere in the reference frame of the entity.
    pub fn sphere(&self) -> SphereC {
        SphereC::new(Point3C::from(self.offset), self.radius)
    }

    /// Returns the sphere of influence in the reference frame of the entity.
    ///
    /// The sphere of influence is slightly larger than the absorbing sphere in
    /// order to keep the SDF well-behaved near the boundary of the absorbed
    /// volume.
    pub fn influence_sphere(&self, voxel_extent: f32) -> SphereC {
        SphereC::new(Point3C::from(self.offset), self.radius + 2.0 * voxel_extent)
    }

    /// Computes the new signed distance for the given voxel inside the sphere.
    /// The sphere radius and distance from center should be specified in
    /// voxels.
    pub fn compute_new_signed_distance(
        sphere_radius: f32,
        voxel: &Voxel,
        squared_distance_from_center: f32,
    ) -> f32 {
        let sphere_signed_distance = squared_distance_from_center.sqrt() - sphere_radius;

        hard_sdf_subtraction(voxel.signed_distance().to_f32(), sphere_signed_distance)
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

    /// Returns the capsule in the reference frame of the entity.
    pub fn capsule(&self) -> CapsuleC {
        CapsuleC::new(
            Point3C::from(self.offset_to_segment_start),
            self.segment_vector,
            self.radius,
        )
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

    /// Computes the new signed distance for the given voxel inside the capsule.
    /// The capsule radius and distance from segment should be specified in
    /// voxels.
    pub fn compute_new_signed_distance(
        capsule_radius: f32,
        voxel: &Voxel,
        squared_distance_from_segment: f32,
    ) -> f32 {
        let capsule_signed_distance = squared_distance_from_segment.sqrt() - capsule_radius;

        hard_sdf_subtraction(voxel.signed_distance().to_f32(), capsule_signed_distance)
    }
}

impl VoxelAbsorptionManager {
    pub fn new() -> Self {
        Self {
            absorbing_spheres: HashMap::default(),
            absorbing_capsules: HashMap::default(),
            mutual_absorption_processes: HashMap::default(),
        }
    }

    /// Returns a reference to the [`TrackingVoxelAbsorbingSphere`] with the
    /// given ID, or [`None`] if it does not exist.
    pub fn get_absorbing_sphere(
        &self,
        id: VoxelAbsorbingSphereID,
    ) -> Option<&TrackingVoxelAbsorbingSphere> {
        self.absorbing_spheres.get(&id)
    }

    /// Returns a mutable reference to the [`TrackingVoxelAbsorbingSphere`] with
    /// the given ID, or [`None`] if it does not exist.
    pub fn get_absorbing_sphere_mut(
        &mut self,
        id: VoxelAbsorbingSphereID,
    ) -> Option<&mut TrackingVoxelAbsorbingSphere> {
        self.absorbing_spheres.get_mut(&id)
    }

    /// Returns a reference to the [`TrackingVoxelAbsorbingCapsule`] with the
    /// given ID, or [`None`] if it does not exist.
    pub fn get_absorbing_capsule(
        &self,
        id: VoxelAbsorbingCapsuleID,
    ) -> Option<&TrackingVoxelAbsorbingCapsule> {
        self.absorbing_capsules.get(&id)
    }

    /// Returns a mutable reference to the [`TrackingVoxelAbsorbingCapsule`] with
    /// the given ID, or [`None`] if it does not exist.
    pub fn get_absorbing_capsule_mut(
        &mut self,
        id: VoxelAbsorbingCapsuleID,
    ) -> Option<&mut TrackingVoxelAbsorbingCapsule> {
        self.absorbing_capsules.get_mut(&id)
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
        if self.absorbing_spheres.contains_key(&id) {
            bail!("A voxel-absorbing sphere with ID {id} already exists");
        }
        self.absorbing_spheres
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
        if self.absorbing_capsules.contains_key(&id) {
            bail!("A voxel-absorbing capsule with ID {id} already exists");
        }
        self.absorbing_capsules
            .insert(id, TrackingVoxelAbsorbingCapsule::new(capsule));
        Ok(())
    }

    /// Initiates a mutual absorption process for the voxel object entities with
    /// the given IDs.
    pub fn initiate_mutual_absorption_process(
        &mut self,
        entity_ids: [EntityID; 2],
        process: MutualVoxelAbsorptionProcess,
    ) {
        self.mutual_absorption_processes
            .insert(Self::sorted_entity_pair(entity_ids), process);
    }

    /// Removes the [`VoxelAbsorbingSphere`] with the given ID from the manager
    /// if it exists.
    pub fn remove_absorbing_sphere(&mut self, id: VoxelAbsorbingSphereID) {
        self.absorbing_spheres.remove(&id);
    }

    /// Removes the [`VoxelAbsorbingCapsule`] with the given ID from the manager
    /// if it exists.
    pub fn remove_absorbing_capsule(&mut self, id: VoxelAbsorbingCapsuleID) {
        self.absorbing_capsules.remove(&id);
    }

    /// Stops any mutual absorption process for the voxel object entities with
    /// the given IDs.
    pub fn stop_mutual_absorption_process(&mut self, entity_ids: [EntityID; 2]) {
        self.mutual_absorption_processes
            .remove(&Self::sorted_entity_pair(entity_ids));
    }

    /// Removes all mutual voxel absorption processes involving the entity with
    /// the given ID.
    pub fn remove_mutual_absorption_processes_involving_entity(&mut self, entity_id: EntityID) {
        self.mutual_absorption_processes
            .retain(|&[entity_a_id, entity_b_id], _| {
                entity_a_id != entity_id && entity_b_id != entity_id
            });
    }

    /// Removes all stored voxel absorbers and processes and frees up all
    /// allocated memory.
    pub fn reset_and_free(&mut self) {
        self.absorbing_spheres = HashMap::default();
        self.absorbing_capsules = HashMap::default();
        self.mutual_absorption_processes = HashMap::default();
    }

    fn sorted_entity_pair(entity_ids: [EntityID; 2]) -> [EntityID; 2] {
        if entity_ids[0].as_u64() <= entity_ids[1].as_u64() {
            entity_ids
        } else {
            [entity_ids[1], entity_ids[0]]
        }
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
    intersection_manager: &IntersectionManager,
    rigid_body_manager: &mut RigidBodyManager,
    anchor_manager: &mut AnchorManager,
) where
    C: VoxelObjectInteractionContext,
{
    let voxel_object_manager = &mut voxel_manager.object_manager;
    let voxel_object_buffer_pool = &mut voxel_manager.object_buffer_pool;
    let voxel_absorption_manager = voxel_manager.interaction_manager.absorption_manager_mut();

    let absorbing_sphere_entities = context.gather_voxel_absorbing_sphere_entities();
    let absorbing_capsule_entities = context.gather_voxel_absorbing_capsule_entities();

    let mut affected_voxel_objects = TinyVec::<[(EntityID, Vector3); 16]>::new();

    for entity in &absorbing_sphere_entities {
        let absorber_id = VoxelAbsorbingSphereID::from_entity_id(entity.entity_id);
        let Some(absorbing_sphere) = voxel_absorption_manager.get_absorbing_sphere_mut(absorber_id)
        else {
            continue;
        };
        absorbing_sphere.tracker.clear_absorbed();

        let Some(sphere_to_world_transform) = &entity.sphere_to_world_transform else {
            continue;
        };

        let sphere = absorbing_sphere
            .sphere
            .sphere()
            .aligned()
            .iso_transformed(sphere_to_world_transform);

        let aabb = sphere.compute_aabb();

        intersection_manager.for_each_bounding_volume_in_axis_aligned_box(
            &aabb,
            |bounding_volume_id, _| {
                let object_entity_id = bounding_volume_id.as_entity_id();
                with_potential_voxel_object(
                    voxel_object_manager,
                    voxel_type_registry,
                    rigid_body_manager,
                    object_entity_id,
                    |inertial_property_updater,
                     voxel_object,
                     local_center_of_mass,
                     world_to_voxel_object_transform| {
                        apply_sphere_absorption(
                            inertial_property_updater,
                            voxel_object,
                            &world_to_voxel_object_transform,
                            absorbing_sphere,
                            sphere_to_world_transform,
                        );
                        if !affected_voxel_objects
                            .iter()
                            .any(|(id, _)| *id == object_entity_id)
                        {
                            affected_voxel_objects.push((object_entity_id, local_center_of_mass));
                        }
                    },
                );
            },
        );
    }

    for entity in &absorbing_capsule_entities {
        let absorber_id = VoxelAbsorbingCapsuleID::from_entity_id(entity.entity_id);
        let Some(absorbing_capsule) =
            voxel_absorption_manager.get_absorbing_capsule_mut(absorber_id)
        else {
            continue;
        };
        absorbing_capsule.tracker.clear_absorbed();

        let Some(capsule_to_world_transform) = &entity.capsule_to_world_transform else {
            continue;
        };

        let capsule = absorbing_capsule
            .capsule
            .capsule()
            .aligned()
            .iso_transformed(capsule_to_world_transform);

        let aabb = capsule.compute_aabb();

        intersection_manager.for_each_bounding_volume_in_axis_aligned_box(
            &aabb,
            |bounding_volume_id, _| {
                let object_entity_id = bounding_volume_id.as_entity_id();
                with_potential_voxel_object(
                    voxel_object_manager,
                    voxel_type_registry,
                    rigid_body_manager,
                    object_entity_id,
                    |inertial_property_updater,
                     voxel_object,
                     local_center_of_mass,
                     world_to_voxel_object_transform| {
                        apply_capsule_absorption(
                            inertial_property_updater,
                            voxel_object,
                            &world_to_voxel_object_transform,
                            absorbing_capsule,
                            capsule_to_world_transform,
                        );
                        if !affected_voxel_objects
                            .iter()
                            .any(|(id, _)| *id == object_entity_id)
                        {
                            affected_voxel_objects.push((object_entity_id, local_center_of_mass));
                        }
                    },
                );
            },
        );
    }

    for (&[entity_a_id, entity_b_id], process) in
        &voxel_absorption_manager.mutual_absorption_processes
    {
        if !intersection_manager.bounding_volumes_intersect(
            BoundingVolumeID::from_entity_id(entity_a_id),
            BoundingVolumeID::from_entity_id(entity_b_id),
        ) {
            continue;
        };

        with_potential_voxel_object_pair(
            voxel_object_manager,
            voxel_type_registry,
            rigid_body_manager,
            entity_a_id,
            entity_b_id,
            |[
                (
                    mut inertial_property_updater_a,
                    voxel_object_a,
                    local_center_of_mass_a,
                    transform_from_world_to_a,
                ),
                (
                    mut inertial_property_updater_b,
                    voxel_object_b,
                    local_center_of_mass_b,
                    transform_from_world_to_b,
                ),
            ]| {
                apply_mutual_absorption(
                    voxel_object_a,
                    voxel_object_b,
                    &mut inertial_property_updater_a,
                    &mut inertial_property_updater_b,
                    &transform_from_world_to_a,
                    &transform_from_world_to_b,
                    process,
                );

                for (entity_id, local_center_of_mass) in [
                    (entity_a_id, local_center_of_mass_a),
                    (entity_b_id, local_center_of_mass_b),
                ] {
                    if !affected_voxel_objects
                        .iter()
                        .any(|(id, _)| *id == entity_id)
                    {
                        affected_voxel_objects.push((entity_id, local_center_of_mass));
                    }
                }
            },
        );
    }

    for (entity_id, original_local_center_of_mass) in affected_voxel_objects {
        let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);

        let (voxel_object, physics_context) = voxel_object_manager
            .get_voxel_object_with_physics_context_mut(voxel_object_id)
            .unwrap();

        let voxel_object = voxel_object.object_mut();

        if voxel_object.invalidated_mesh_chunk_indices().len() > 0 {
            let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
            let rigid_body = rigid_body_manager
                .get_dynamic_rigid_body_mut(rigid_body_id)
                .unwrap();

            // The global connected region information has not been resolved
            // after the voxels were absorbed
            voxel_object.resolve_connected_regions_between_all_chunks();

            let arena = ArenaPool::get_arena();

            let VoxelRemovalOutcome {
                original_object_empty,
                extracted_components_for_disconnected_objects,
                lost_anchors,
            } = interaction::handle_voxel_object_after_removing_voxels(
                &arena,
                anchor_manager,
                voxel_type_registry,
                voxel_object_buffer_pool,
                voxel_object,
                &mut physics_context.inertial_property_manager,
                rigid_body_id,
                rigid_body,
                original_local_center_of_mass,
                RemovedMassFate::Destroyed,
            );

            // All lost anchors not inherited by a disconnected object should be
            // deleted
            for (anchor_id, _) in lost_anchors {
                anchor_manager.dynamic_mut().remove(anchor_id);
            }

            if original_object_empty {
                context.remove_voxel_object_entity(entity_id);
            }

            let disconnected_entity_ids = entity_id_manager
                .provide_id_vec(extracted_components_for_disconnected_objects.len());

            for (disconnected_entity_id, extracted_components) in disconnected_entity_ids
                .iter()
                .copied()
                .zip(extracted_components_for_disconnected_objects)
            {
                interaction::spawn_extracted_voxel_object(
                    voxel_object_manager,
                    rigid_body_manager,
                    anchor_manager,
                    extracted_components,
                    disconnected_entity_id,
                );
            }

            context.create_extracted_voxel_object_entities(disconnected_entity_ids, entity_id);
        }
    }
}

fn with_potential_voxel_object(
    voxel_object_manager: &mut VoxelObjectManager,
    voxel_type_registry: &VoxelTypeRegistry,
    rigid_body_manager: &RigidBodyManager,
    entity_id: EntityID,
    mut f: impl FnMut(
        &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
        &mut VoxelObject,
        Vector3,
        Isometry3,
    ),
) {
    let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);

    let Some((voxel_object, physics_context)) =
        voxel_object_manager.get_voxel_object_with_physics_context_mut(voxel_object_id)
    else {
        return;
    };

    let voxel_object = voxel_object.object_mut();

    let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
    let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(rigid_body_id) else {
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

    f(
        &mut inertial_property_updater,
        voxel_object,
        local_center_of_mass,
        world_to_voxel_object_transform,
    );
}

fn with_potential_voxel_object_pair(
    voxel_object_manager: &mut VoxelObjectManager,
    voxel_type_registry: &VoxelTypeRegistry,
    rigid_body_manager: &RigidBodyManager,
    entity_a_id: EntityID,
    entity_b_id: EntityID,
    mut f: impl FnMut(
        [(
            VoxelObjectInertialPropertyUpdater<'_, '_>,
            &mut VoxelObject,
            Vector3,
            Isometry3,
        ); 2],
    ),
) {
    let voxel_object_a_id = VoxelObjectID::from_entity_id(entity_a_id);
    let voxel_object_b_id = VoxelObjectID::from_entity_id(entity_b_id);

    let Some([(object_a, context_a), (object_b, context_b)]) = voxel_object_manager
        .get_voxel_object_pair_with_physics_contexts_mut(voxel_object_a_id, voxel_object_b_id)
    else {
        return;
    };

    let (Some(body_a), Some(body_b)) = (
        rigid_body_manager.get_dynamic_rigid_body(DynamicRigidBodyID::from_entity_id(entity_a_id)),
        rigid_body_manager.get_dynamic_rigid_body(DynamicRigidBodyID::from_entity_id(entity_b_id)),
    ) else {
        log::warn!("Voxel object physics context points to missing dynamic rigid body");
        return;
    };

    f(
        [(object_a, context_a, body_a), (object_b, context_b, body_b)].map(
            |(voxel_object, physics_context, rigid_body)| {
                let voxel_object = voxel_object.object_mut();

                let local_center_of_mass = physics_context
                    .inertial_property_manager
                    .derive_center_of_mass();

                let voxel_object_to_world_transform = rigid_body
                    .reference_frame()
                    .create_transform_to_parent_space()
                    .applied_to_translation(&(-local_center_of_mass));

                let world_to_voxel_object_transform = voxel_object_to_world_transform.inverted();

                let inertial_property_updater =
                    physics_context.inertial_property_manager.begin_update(
                        voxel_object.voxel_extent(),
                        voxel_type_registry.mass_densities(),
                    );

                (
                    inertial_property_updater,
                    voxel_object,
                    local_center_of_mass,
                    world_to_voxel_object_transform,
                )
            },
        ),
    );
}

fn apply_sphere_absorption(
    inertial_property_updater: &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
    voxel_object: &mut VoxelObject,
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

    let influence_sphere_in_norm_voxel_object_space = influence_sphere
        .iso_transformed(sphere_to_world_transform)
        .iso_transformed(world_to_voxel_object_transform)
        .scaled(voxel_object.inverse_voxel_extent());

    let sphere_radius_in_voxels = absorbing_sphere.radius * voxel_object.inverse_voxel_extent();

    voxel_object.modify_voxels_within_sphere(
        &influence_sphere_in_norm_voxel_object_space,
        &mut |object_voxel_indices, squared_distance_from_center_in_voxels, voxel| {
            let was_empty = voxel.is_empty();

            let new_signed_distance = VoxelAbsorbingSphere::compute_new_signed_distance(
                sphere_radius_in_voxels,
                voxel,
                squared_distance_from_center_in_voxels,
            );

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
    voxel_object: &mut VoxelObject,
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

    let influence_capsule_in_norm_voxel_object_space = influence_capsule
        .iso_transformed(capsule_to_world_transform)
        .iso_transformed(world_to_voxel_object_transform)
        .scaled(voxel_object.inverse_voxel_extent());

    let capsule_radius_in_voxels = absorbing_capsule.radius * voxel_object.inverse_voxel_extent();

    voxel_object.modify_voxels_within_capsule(
        &influence_capsule_in_norm_voxel_object_space,
        &mut |object_voxel_indices, squared_distance_from_segment_in_voxels, voxel| {
            let was_empty = voxel.is_empty();

            let new_signed_distance = VoxelAbsorbingCapsule::compute_new_signed_distance(
                capsule_radius_in_voxels,
                voxel,
                squared_distance_from_segment_in_voxels,
            );

            voxel.set_signed_distance(new_signed_distance, &mut |voxel| {
                if !was_empty {
                    inertial_property_updater.remove_voxel(&object_voxel_indices, *voxel);
                    tracker.register_absorbed_voxel(voxel_volume, *voxel);
                }
            });
        },
    );
}

pub fn apply_mutual_absorption(
    voxel_object_a: &mut VoxelObject,
    voxel_object_b: &mut VoxelObject,
    inertial_property_updater_a: &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
    inertial_property_updater_b: &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
    transform_from_world_to_a: &Isometry3,
    transform_from_world_to_b: &Isometry3,
    process: &MutualVoxelAbsorptionProcess,
) {
    let transform_from_b_to_a = transform_from_world_to_a * transform_from_world_to_b.inverted();

    let Some((intersection_voxel_ranges_in_a, intersection_voxel_ranges_in_b)) =
        VoxelObject::determine_voxel_ranges_encompassing_intersection(
            voxel_object_a,
            voxel_object_b,
            &transform_from_b_to_a,
        )
    else {
        return;
    };

    let voxel_extent_a = voxel_object_a.voxel_extent();
    let voxel_extent_b = voxel_object_b.voxel_extent();
    let inverse_voxel_extent_a = voxel_object_a.inverse_voxel_extent();
    let inverse_voxel_extent_b = voxel_object_b.inverse_voxel_extent();

    let b_dist_to_a = voxel_object_b.voxel_extent() * voxel_object_a.inverse_voxel_extent();
    let a_dist_to_b = voxel_object_a.voxel_extent() * voxel_object_b.inverse_voxel_extent();

    let grid_dimensions_for_a = voxel_object_a
        .chunk_counts()
        .map(|count| count * CHUNK_SIZE);

    let grid_dimensions_for_b = voxel_object_b
        .chunk_counts()
        .map(|count| count * CHUNK_SIZE);

    let snapshot_padding = b_dist_to_a.ceil() as usize;

    let snapshot_grid_ranges: [_; 3] = array::from_fn(|dim| {
        let range = &intersection_voxel_ranges_in_a[dim];
        range.start.saturating_sub(snapshot_padding)
            ..(range.end + snapshot_padding).min(grid_dimensions_for_a[dim])
    });

    let signed_snapshot_grid_ranges = snapshot_grid_ranges
        .clone()
        .map(|range| range.start as isize..range.end as isize);

    let snapshot_voxel_count = snapshot_grid_ranges
        .iter()
        .map(Range::len)
        .product::<usize>();

    let arena = ArenaPool::get_arena();
    let mut snapshot = avec![in &arena; VoxelSignedDistance::MAX_F32; snapshot_voxel_count];

    let get_snapshot_idx = |i: usize, j: usize, k: usize| {
        (i - snapshot_grid_ranges[0].start)
            * snapshot_grid_ranges[1].len()
            * snapshot_grid_ranges[2].len()
            + (j - snapshot_grid_ranges[1].start) * snapshot_grid_ranges[2].len()
            + (k - snapshot_grid_ranges[2].start)
    };

    voxel_object_a.modify_voxels_within_ranges(
        snapshot_grid_ranges.clone(),
        &mut |[i_a, j_a, k_a], voxel_a| {
            if voxel_a.signed_distance().is_maximally_outside() {
                return false;
            }

            let signed_distance_inside_a_in_a = voxel_a.signed_distance().to_f32();

            let snapshot_idx = get_snapshot_idx(i_a, j_a, k_a);
            snapshot[snapshot_idx] = signed_distance_inside_a_in_a;

            let center_in_a = object::voxel_center_position_from_object_voxel_indices(
                voxel_extent_a,
                i_a,
                j_a,
                k_a,
            );

            let center_in_b = inverse_voxel_extent_b
                * transform_from_b_to_a.inverse_transform_point(&center_in_a);

            let signed_distance_inside_b_in_b =
                sdf::sample_voxel_object_sdf(voxel_object_b, &grid_dimensions_for_b, &center_in_b);

            let signed_distance_inside_b = signed_distance_inside_b_in_b * b_dist_to_a;

            let was_empty = voxel_a.is_empty();

            let new_signed_distance = compute_subtracted_signed_distance(
                voxel_a,
                signed_distance_inside_b,
                process.smoothness,
            );

            voxel_a.set_signed_distance(new_signed_distance, &mut |voxel| {
                if !was_empty {
                    inertial_property_updater_a.remove_voxel(&[i_a, j_a, k_a], *voxel);
                }
            });

            true
        },
    );

    voxel_object_b.modify_voxels_within_ranges(
        intersection_voxel_ranges_in_b,
        &mut |[i_b, j_b, k_b], voxel_b| {
            if voxel_b.signed_distance().is_maximally_outside() {
                return false;
            }

            let center_in_b = object::voxel_center_position_from_object_voxel_indices(
                voxel_extent_b,
                i_b,
                j_b,
                k_b,
            );

            let center_in_a =
                inverse_voxel_extent_a * transform_from_b_to_a.transform_point(&center_in_b);

            let lower_corner_position = center_in_a - Vector3::same(0.5);

            let lower_indices = lower_corner_position.as_vector().component_floor();
            let fractional_offset = lower_corner_position.as_vector() - lower_indices;

            let [li, lj, lk] = <[f32; 3]>::from(lower_indices).map(|idx| idx as isize);

            if signed_snapshot_grid_ranges
                .iter()
                .zip([li, lj, lk])
                .any(|(range, idx)| idx < range.start)
                || signed_snapshot_grid_ranges
                    .iter()
                    .zip([li + 1, lj + 1, lk + 1])
                    .any(|(range, idx)| idx >= range.end)
            {
                return false;
            }

            let li = li as usize;
            let lj = lj as usize;
            let lk = lk as usize;

            let sample_dist = |i, j, k| {
                let snapshot_idx = get_snapshot_idx(i, j, k);
                snapshot[snapshot_idx]
            };

            let dists = [
                sample_dist(li, lj, lk),
                sample_dist(li, lj, lk + 1),
                sample_dist(li, lj + 1, lk),
                sample_dist(li, lj + 1, lk + 1),
                sample_dist(li + 1, lj, lk),
                sample_dist(li + 1, lj, lk + 1),
                sample_dist(li + 1, lj + 1, lk),
                sample_dist(li + 1, lj + 1, lk + 1),
            ];

            let signed_distance_inside_a_in_a =
                sdf::evaluate_sdf_from_corner_samples(&dists, &fractional_offset.compact());

            let signed_distance_inside_a = signed_distance_inside_a_in_a * a_dist_to_b;

            let was_empty = voxel_b.is_empty();

            let new_signed_distance = compute_subtracted_signed_distance(
                voxel_b,
                signed_distance_inside_a,
                process.smoothness,
            );

            voxel_b.set_signed_distance(new_signed_distance, &mut |voxel| {
                if !was_empty {
                    inertial_property_updater_b.remove_voxel(&[i_b, j_b, k_b], *voxel);
                }
            });

            true
        },
    );
}

fn compute_subtracted_signed_distance(
    voxel: &Voxel,
    signed_distance_inside_other: f32,
    smoothness: Smoothness,
) -> f32 {
    let signed_distance = voxel.signed_distance().to_f32();

    let intersection_signed_distance = signed_distance.max(signed_distance_inside_other);

    let subtracted_signed_distance =
        sdf_subtraction(signed_distance, intersection_signed_distance, smoothness);

    subtracted_signed_distance
}
