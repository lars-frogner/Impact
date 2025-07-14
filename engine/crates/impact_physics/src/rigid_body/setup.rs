//! Management of rigid bodies for entities.

use crate::{
    fph,
    inertia::InertialProperties,
    quantities::Motion,
    rigid_body::{
        DynamicRigidBody, DynamicRigidBodyID, KinematicRigidBody, KinematicRigidBodyID,
        RigidBodyManager,
    },
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_geometry::ReferenceFrame;
use impact_math::Float;
use nalgebra::Point3;
use roc_integration::roc;
use simba::scalar::SubsetOf;

define_setup_type! {
    target = DynamicRigidBodyID;
    /// The properties of the substance making up a dynamic rigid body.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DynamicRigidBodySubstance {
        // The mass density of the body's substance.
        pub mass_density: fph,
    }
}

pub fn setup_kinematic_rigid_body(
    rigid_body_manager: &mut RigidBodyManager,
    frame: ReferenceFrame,
    motion: Motion,
) -> KinematicRigidBodyID {
    let rigid_body = KinematicRigidBody::new(
        frame.position,
        frame.orientation,
        motion.linear_velocity,
        motion.angular_velocity,
    );

    rigid_body_manager.add_kinematic_rigid_body(rigid_body)
}

pub fn setup_dynamic_rigid_body(
    rigid_body_manager: &mut RigidBodyManager,
    mut inertial_properties: InertialProperties,
    mut frame: ReferenceFrame,
    motion: Motion,
) -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
    // Scale the mass to be consistent with the initial scale factor. If
    // the scale factor changes later on, we will conserve the mass and
    // only let the scale change the extent of the body.
    inertial_properties.multiply_mass(frame.scaling.powi(3));

    // Use center of mass as new origin, since all free rotation is
    // about the center of mass
    frame.origin_offset = inertial_properties.center_of_mass().coords;

    let rigid_body = DynamicRigidBody::new(
        inertial_properties.mass(),
        *inertial_properties.inertia_tensor(),
        frame.position,
        frame.orientation,
        motion.linear_velocity,
        motion.angular_velocity,
    );

    let rigid_body_id = rigid_body_manager.add_dynamic_rigid_body(rigid_body);

    (rigid_body_id, frame, motion)
}

pub fn setup_dynamic_rigid_body_for_uniform_box(
    rigid_body_manager: &mut RigidBodyManager,
    extent_x: fph,
    extent_y: fph,
    extent_z: fph,
    substance: &DynamicRigidBodySubstance,
    frame: ReferenceFrame,
    motion: Motion,
) -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
    let inertial_properties =
        InertialProperties::of_uniform_box(extent_x, extent_y, extent_z, substance.mass_density);

    setup_dynamic_rigid_body(rigid_body_manager, inertial_properties, frame, motion)
}

pub fn setup_dynamic_rigid_body_for_uniform_cylinder(
    rigid_body_manager: &mut RigidBodyManager,
    length: fph,
    diameter: fph,
    substance: &DynamicRigidBodySubstance,
    frame: ReferenceFrame,
    motion: Motion,
) -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
    let inertial_properties =
        InertialProperties::of_uniform_cylinder(length, diameter, substance.mass_density);

    setup_dynamic_rigid_body(rigid_body_manager, inertial_properties, frame, motion)
}

pub fn setup_dynamic_rigid_body_for_uniform_cone(
    rigid_body_manager: &mut RigidBodyManager,
    length: fph,
    max_diameter: fph,
    substance: &DynamicRigidBodySubstance,
    frame: ReferenceFrame,
    motion: Motion,
) -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
    let inertial_properties =
        InertialProperties::of_uniform_cone(length, max_diameter, substance.mass_density);

    setup_dynamic_rigid_body(rigid_body_manager, inertial_properties, frame, motion)
}

pub fn setup_dynamic_rigid_body_for_uniform_sphere(
    rigid_body_manager: &mut RigidBodyManager,
    substance: &DynamicRigidBodySubstance,
    frame: ReferenceFrame,
    motion: Motion,
) -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
    let inertial_properties = InertialProperties::of_uniform_sphere(substance.mass_density);

    setup_dynamic_rigid_body(rigid_body_manager, inertial_properties, frame, motion)
}

pub fn setup_dynamic_rigid_body_for_uniform_hemisphere(
    rigid_body_manager: &mut RigidBodyManager,
    substance: &DynamicRigidBodySubstance,
    frame: ReferenceFrame,
    motion: Motion,
) -> (DynamicRigidBodyID, ReferenceFrame, Motion) {
    let inertial_properties = InertialProperties::of_uniform_hemisphere(substance.mass_density);

    setup_dynamic_rigid_body(rigid_body_manager, inertial_properties, frame, motion)
}

pub fn setup_dynamic_rigid_body_for_uniform_triangle_mesh<'a, F: Float + SubsetOf<fph>>(
    rigid_body_manager: &mut RigidBodyManager,
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3<F>; 3]>,
    substance: &DynamicRigidBodySubstance,
    frame: ReferenceFrame,
    motion: Motion,
) -> Result<(DynamicRigidBodyID, ReferenceFrame, Motion)> {
    let inertial_properties = InertialProperties::of_uniform_triangle_mesh(
        triangle_vertex_positions,
        substance.mass_density,
    );
    Ok(setup_dynamic_rigid_body(
        rigid_body_manager,
        inertial_properties,
        frame,
        motion,
    ))
}

#[cfg(feature = "ecs")]
pub fn remove_rigid_body_for_entity(
    rigid_body_manager: &std::sync::RwLock<RigidBodyManager>,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    if let Some(rigid_body_id) = entity.get_component::<DynamicRigidBodyID>() {
        rigid_body_manager
            .write()
            .unwrap()
            .remove_dynamic_rigid_body(*rigid_body_id.access());
    }
    if let Some(rigid_body_id) = entity.get_component::<KinematicRigidBodyID>() {
        rigid_body_manager
            .write()
            .unwrap()
            .remove_kinematic_rigid_body(*rigid_body_id.access());
    }
}
