//! Voxel absorption.

use crate::{
    VoxelObjectManager, VoxelObjectPhysicsContext,
    chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyUpdater},
    interaction::{
        self, DynamicDisconnectedVoxelObject, NewVoxelObjectEntity, VoxelObjectEntity,
        VoxelObjectInteractionContext, VoxelRemovalOutcome,
    },
    mesh::MeshedChunkedVoxelObject,
    voxel_types::VoxelTypeRegistry,
};
use bytemuck::{Pod, Zeroable};
use impact_geometry::{Capsule, Sphere};
use impact_physics::{fph, rigid_body::RigidBodyManager};
use nalgebra::{Isometry3, Point3, Translation3, Vector3};
use roc_integration::roc;

define_component_type! {
    /// A sphere that absorbs voxels it comes in contact with. The rate of
    /// absorption is highest at the center of the sphere and decreases
    /// quadratically to zero at the full radius.
    ///
    /// Does nothing if the entity does not have a
    /// [`impact_geometry::ReferenceFrame`].
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
    pub struct VoxelAbsorbingSphere {
        /// The offset of the sphere in the reference frame of the entity.
        offset: Vector3<f64>,
        /// The radius of the sphere.
        radius: f64,
        /// The maximum rate of absorption (at the center of the sphere).
        rate: f64,
    }
}

define_component_type! {
    /// A capsule that absorbs voxels it comes in contact with. The rate of
    /// absorption is highest at the central line segment of the capsule and
    /// decreases quadratically to zero at the capsule boundary.
    ///
    /// Does nothing if the entity does not have a
    /// [`impact_geometry::ReferenceFrame`].
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Default,Zeroable, Pod)]
    pub struct VoxelAbsorbingCapsule {
        /// The offset of the starting point of the capsule's central line segment
        /// in the reference frame of the entity.
        offset_to_segment_start: Vector3<f64>,
        /// The displacement vector from the start to the end of the capsule's
        /// central line segment in the reference frame of the entity.
        segment_vector: Vector3<f64>,
        /// The radius of the capsule.
        radius: f64,
        /// The maximum rate of absorption (at the central line segment of the
        /// capsule).
        rate: f64,
    }
}

#[roc]
impl VoxelAbsorbingSphere {
    /// Creates a new [`VoxelAbsorbingSphere`] with the given offset and radius
    /// in the reference frame of the entity and the given maximum absorption
    /// rate (at the center of the sphere).
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    # expect rate >= 0.0
    {
        offset,
        radius,
        rate,
    }"#)]
    pub fn new(offset: Vector3<f64>, radius: f64, rate: f64) -> Self {
        assert!(radius >= 0.0);
        assert!(rate >= 0.0);
        Self {
            offset,
            radius,
            rate,
        }
    }

    /// Returns the sphere in the reference frame of the entity.
    pub fn sphere(&self) -> Sphere<f64> {
        Sphere::new(Point3::from(self.offset), self.radius)
    }

    /// Returns the maximum absorption rate.
    pub fn rate(&self) -> f64 {
        self.rate
    }
}

#[roc]
impl VoxelAbsorbingCapsule {
    /// Creates a new [`VoxelAbsorbingCapsule`] with the given offset to the
    /// start of the capsule's central line segment, displacement from the start
    /// to the end of the line segment and radius, all in the reference frame of
    /// the entity, as well as the given maximum absorption rate (at the central
    /// line segment).
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    # expect rate >= 0.0
    {
        offset_to_segment_start,
        segment_vector,
        radius,
        rate,
    }"#)]
    pub fn new(
        offset_to_segment_start: Vector3<f64>,
        segment_vector: Vector3<f64>,
        radius: f64,
        rate: f64,
    ) -> Self {
        assert!(radius >= 0.0);
        assert!(rate >= 0.0);
        Self {
            offset_to_segment_start,
            segment_vector,
            radius,
            rate,
        }
    }

    /// Returns the capsule in the reference frame of the entity.
    pub fn capsule(&self) -> Capsule<f64> {
        Capsule::new(
            Point3::from(self.offset_to_segment_start),
            self.segment_vector,
            self.radius,
        )
    }

    /// Returns the maximum absorption rate.
    pub fn rate(&self) -> f64 {
        self.rate
    }
}

/// Applies each voxel-absorbing sphere and capsule to the affected voxel
/// objects.
pub fn apply_absorption<C>(
    context: &mut C,
    voxel_object_manager: &mut VoxelObjectManager,
    voxel_type_registry: &VoxelTypeRegistry,
    rigid_body_manager: &mut RigidBodyManager,
    time_step_duration: fph,
) where
    C: VoxelObjectInteractionContext,
    <C as VoxelObjectInteractionContext>::EntityID: Clone,
{
    let mut voxel_object_entities = Vec::with_capacity(voxel_object_manager.voxel_object_count());

    context.gather_voxel_object_entities(&mut voxel_object_entities);

    let absorbing_spheres = context.gather_voxel_absorbing_sphere_entities();
    let absorbing_capsules = context.gather_voxel_absorbing_capsule_entities();

    for VoxelObjectEntity {
        entity_id,
        voxel_object_id,
    } in voxel_object_entities
    {
        let Some((voxel_object, physics_context)) =
            voxel_object_manager.get_voxel_object_with_physics_context_mut(voxel_object_id)
        else {
            continue;
        };
        let voxel_object = voxel_object.object_mut();

        let Some(rigid_body) =
            rigid_body_manager.get_dynamic_rigid_body_mut(physics_context.rigid_body_id)
        else {
            impact_log::warn!("Voxel object physics context points to missing dynamic rigid body");
            return;
        };

        let local_center_of_mass = physics_context
            .inertial_property_manager
            .derive_center_of_mass();

        let reference_frame = rigid_body.reference_frame();

        let voxel_object_to_world_transform = reference_frame.create_transform_to_parent_space()
            * Translation3::from(-local_center_of_mass);

        let world_to_voxel_object_transform = voxel_object_to_world_transform.inverse();

        let mut inertial_property_updater = physics_context.inertial_property_manager.begin_update(
            voxel_object.voxel_extent(),
            voxel_type_registry.mass_densities(),
        );

        for absorbing_sphere in &absorbing_spheres {
            apply_sphere_absorption(
                time_step_duration,
                &mut inertial_property_updater,
                voxel_object,
                &world_to_voxel_object_transform,
                &absorbing_sphere.sphere,
                &absorbing_sphere.sphere_to_world_transform,
            );
        }

        for absorbing_capsule in &absorbing_capsules {
            apply_capsule_absorption(
                time_step_duration,
                &mut inertial_property_updater,
                voxel_object,
                &world_to_voxel_object_transform,
                &absorbing_capsule.capsule,
                &absorbing_capsule.capsule_to_world_transform,
            );
        }

        if voxel_object.invalidated_mesh_chunk_indices().len() > 0 {
            let VoxelRemovalOutcome {
                original_object_empty,
                disconnected_object,
            } = interaction::handle_voxel_object_after_removing_voxels(
                voxel_type_registry,
                voxel_object,
                &mut physics_context.inertial_property_manager,
                rigid_body,
                local_center_of_mass,
            );

            if original_object_empty {
                context.on_empty_voxel_object_entity(entity_id.clone());
            }
            if let Some(DynamicDisconnectedVoxelObject {
                voxel_object,
                inertial_property_manager,
                rigid_body,
            }) = disconnected_object
            {
                let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

                let voxel_object_id = voxel_object_manager.add_voxel_object(meshed_voxel_object);

                let rigid_body_id = rigid_body_manager.add_dynamic_rigid_body(rigid_body);

                let physics_context = VoxelObjectPhysicsContext {
                    inertial_property_manager,
                    rigid_body_id,
                };

                voxel_object_manager
                    .add_physics_context_for_voxel_object(voxel_object_id, physics_context);

                context.on_new_disconnected_voxel_object_entity(
                    NewVoxelObjectEntity {
                        voxel_object_id,
                        rigid_body_id,
                    },
                    entity_id,
                );
            }
        }
    }
}

fn apply_sphere_absorption(
    time_step_duration: f64,
    inertial_property_updater: &mut VoxelObjectInertialPropertyUpdater<'_, '_>,
    voxel_object: &mut ChunkedVoxelObject,
    world_to_voxel_object_transform: &Isometry3<f64>,
    absorbing_sphere: &VoxelAbsorbingSphere,
    sphere_to_world_transform: &Isometry3<f64>,
) {
    let sphere_in_voxel_object_space = absorbing_sphere
        .sphere()
        .translated_and_rotated(sphere_to_world_transform)
        .translated_and_rotated(world_to_voxel_object_transform);

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
    world_to_voxel_object_transform: &Isometry3<f64>,
    absorbing_capsule: &VoxelAbsorbingCapsule,
    capsule_to_world_transform: &Isometry3<f64>,
) {
    let capsule_in_voxel_object_space = absorbing_capsule
        .capsule()
        .translated_and_rotated(capsule_to_world_transform)
        .translated_and_rotated(world_to_voxel_object_transform);

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
