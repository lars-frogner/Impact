//! Setup of rigid bodies for new entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager};
use anyhow::{Result, anyhow};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_mesh::{
    TriangleMeshID,
    setup::{BoxMesh, ConeMesh, CylinderMesh, HemisphereMesh, SphereMesh},
};
use impact_physics::{
    inertia::InertialProperties,
    quantities::Motion,
    rigid_body::{
        self, DynamicRigidBodyID, KinematicRigidBodyID, setup::DynamicRigidBodySubstance,
    },
};
use parking_lot::RwLock;

/// Checks if the entities-to-be with the given components have the components
/// representing a dynamic or kinematic rigid body, and if so, creates the
/// corresponding rigid bodies and adds the [`DynamicRigidBodyID`]s and/or
/// [`KinematicRigidBodyID`]s to the entities.
pub fn setup_rigid_bodies_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    simulator: &RwLock<PhysicsSimulator>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    // Make sure entities with a manually created dynamic rigid body get the
    // correct [`ReferenceFrame`] and [`Motion`] components.
    setup!(
        {
            let simulator = simulator.oread();
            let rigid_body_manager = simulator.rigid_body_manager().oread();
        },
        components,
        |rigid_body_id: &DynamicRigidBodyID,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (ReferenceFrame, Motion) {
            if let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(*rigid_body_id) {
                (rigid_body.reference_frame(), rigid_body.compute_motion())
            } else {
                (
                    frame.copied().unwrap_or_default(),
                    motion.copied().unwrap_or_default(),
                )
            }
        }
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        components,
        |mesh: &BoxMesh,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion) {
            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let inertial_properties = InertialProperties::of_uniform_box(
                mesh.extent_x * model_transform.scale,
                mesh.extent_y * model_transform.scale,
                mesh.extent_z * model_transform.scale,
                substance.mass_density,
            );

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(inertial_properties.center_of_mass().coords.cast());

            let rigid_body_id = rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                inertial_properties,
                frame,
                motion,
            );

            (rigid_body_id, model_transform, frame, motion)
        },
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        components,
        |mesh: &CylinderMesh,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion) {
            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let inertial_properties = InertialProperties::of_uniform_cylinder(
                mesh.length * model_transform.scale,
                mesh.diameter * model_transform.scale,
                substance.mass_density,
            );

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(inertial_properties.center_of_mass().coords.cast());

            let rigid_body_id = rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                inertial_properties,
                frame,
                motion,
            );

            (rigid_body_id, model_transform, frame, motion)
        },
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        components,
        |mesh: &ConeMesh,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion) {
            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let inertial_properties = InertialProperties::of_uniform_cone(
                mesh.length * model_transform.scale,
                mesh.max_diameter * model_transform.scale,
                substance.mass_density,
            );

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(inertial_properties.center_of_mass().coords.cast());

            let rigid_body_id = rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                inertial_properties,
                frame,
                motion,
            );

            (rigid_body_id, model_transform, frame, motion)
        },
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        components,
        |substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion) {
            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let radius = 0.5; // The sphere mesh has a diameter of 1.0

            let inertial_properties = InertialProperties::of_uniform_sphere(
                radius * model_transform.scale,
                substance.mass_density,
            );

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(inertial_properties.center_of_mass().coords.cast());

            let rigid_body_id = rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                inertial_properties,
                frame,
                motion,
            );

            (rigid_body_id, model_transform, frame, motion)
        },
        [SphereMesh],
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        components,
        |substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion) {
            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let radius = 0.5; // The hemisphere mesh has a diameter of 1.0

            let inertial_properties = InertialProperties::of_uniform_hemisphere(
                radius * model_transform.scale,
                substance.mass_density,
            );

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(inertial_properties.center_of_mass().coords.cast());

            let rigid_body_id = rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                inertial_properties,
                frame,
                motion,
            );

            (rigid_body_id, model_transform, frame, motion)
        },
        [HemisphereMesh],
        ![DynamicRigidBodyID]
    );

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        components,
        |mesh_id: &TriangleMeshID,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> Result<(DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion)> {
            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let triangle_mesh = resource_manager
                .triangle_meshes
                .get(*mesh_id)
                .ok_or_else(|| anyhow!("Tried to create rigid body for missing mesh {mesh_id}"))?;

            let mut inertial_properties = InertialProperties::of_uniform_triangle_mesh(
                triangle_mesh.triangle_vertex_positions(),
                substance.mass_density,
            );
            inertial_properties.scale(model_transform.scale);

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(inertial_properties.center_of_mass().coords.cast());

            let rigid_body_id = rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                inertial_properties,
                frame,
                motion,
            );

            Ok((rigid_body_id, model_transform, frame, motion))
        },
        ![DynamicRigidBodyID]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        components,
        |frame: Option<&ReferenceFrame>, motion: &Motion| -> KinematicRigidBodyID {
            rigid_body::setup::setup_kinematic_rigid_body(
                &mut rigid_body_manager,
                frame.copied().unwrap_or_default(),
                *motion,
            )
        },
        ![DynamicRigidBodyID, KinematicRigidBodyID]
    );

    Ok(())
}

pub fn remove_rigid_body_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity: &EntityEntry<'_>,
) {
    if let Some(rigid_body_id) = entity.get_component::<DynamicRigidBodyID>() {
        let simulator = simulator.oread();
        let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        rigid_body_manager.remove_dynamic_rigid_body(*rigid_body_id.access());
    }
    if let Some(rigid_body_id) = entity.get_component::<KinematicRigidBodyID>() {
        let simulator = simulator.oread();
        let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        rigid_body_manager.remove_kinematic_rigid_body(*rigid_body_id.access());
    }
}
