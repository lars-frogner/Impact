//! Setup of rigid bodies for new entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager};
use anyhow::{Result, anyhow};
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_id::EntityID;
use impact_mesh::{
    TriangleMeshID,
    setup::{BoxMesh, CapsuleMesh, ConeMesh, CylinderMesh, HemisphereMesh, SphereMesh},
};
use impact_physics::{
    inertia::{InertiaTensor, InertialProperties},
    quantities::Motion,
    rigid_body::{
        self, DynamicRigidBodyID, HasDynamicRigidBody, HasKinematicRigidBody, KinematicRigidBodyID,
        setup::{DynamicRigidBodyInertialProperties, DynamicRigidBodySubstance},
    },
};
use impact_scene::SceneEntityFlags;
use parking_lot::RwLock;

/// Checks if the given entities have the components representing a dynamic or
/// kinematic rigid body, and if so, creates the corresponding rigid bodies and
/// adds the appropriate marker components to the entities.
pub fn setup_rigid_bodies_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    simulator: &RwLock<PhysicsSimulator>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    // Make sure entities with a manually created dynamic rigid body get the
    // correct [`ReferenceFrame`] and [`Motion`] components.
    setup!(
        {
            let simulator = simulator.oread();
            let rigid_body_manager = simulator.rigid_body_manager().oread();
        },
        entities,
        |entity_id: EntityID,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> (ReferenceFrame, Motion) {
            let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
            if let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(rigid_body_id) {
                (rigid_body.reference_frame(), rigid_body.compute_motion())
            } else {
                (
                    frame.copied().unwrap_or_default(),
                    motion.copied().unwrap_or_default(),
                )
            }
        },
        [HasDynamicRigidBody]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         mesh: &BoxMesh,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            HasDynamicRigidBody,
            ModelTransform,
            ReferenceFrame,
            Motion,
            SceneEntityFlags
        )> {
            let flags = flags.copied().unwrap_or_default();

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
                .set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

            rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                inertial_properties,
                frame,
                motion,
            )?;

            Ok((HasDynamicRigidBody, model_transform, frame, motion, flags))
        },
        ![HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         mesh: &CylinderMesh,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            HasDynamicRigidBody,
            ModelTransform,
            ReferenceFrame,
            Motion,
            SceneEntityFlags
        )> {
            let flags = flags.copied().unwrap_or_default();

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
                .set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

            rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                inertial_properties,
                frame,
                motion,
            )?;

            Ok((HasDynamicRigidBody, model_transform, frame, motion, flags))
        },
        ![HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         mesh: &ConeMesh,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            HasDynamicRigidBody,
            ModelTransform,
            ReferenceFrame,
            Motion,
            SceneEntityFlags
        )> {
            let flags = flags.copied().unwrap_or_default();

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
                .set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

            rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                inertial_properties,
                frame,
                motion,
            )?;

            Ok((HasDynamicRigidBody, model_transform, frame, motion, flags))
        },
        ![HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            HasDynamicRigidBody,
            ModelTransform,
            ReferenceFrame,
            Motion,
            SceneEntityFlags
        )> {
            let flags = flags.copied().unwrap_or_default();

            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let radius = 1.0; // The sphere mesh has a radius of 1.0

            let inertial_properties = InertialProperties::of_uniform_sphere(
                radius * model_transform.scale,
                substance.mass_density,
            );

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

            rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                inertial_properties,
                frame,
                motion,
            )?;

            Ok((HasDynamicRigidBody, model_transform, frame, motion, flags))
        },
        [SphereMesh],
        ![HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            HasDynamicRigidBody,
            ModelTransform,
            ReferenceFrame,
            Motion,
            SceneEntityFlags
        )> {
            let flags = flags.copied().unwrap_or_default();

            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let radius = 1.0; // The hemisphere mesh has a radius of 1.0

            let inertial_properties = InertialProperties::of_uniform_hemisphere(
                radius * model_transform.scale,
                substance.mass_density,
            );

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

            rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                inertial_properties,
                frame,
                motion,
            )?;

            Ok((HasDynamicRigidBody, model_transform, frame, motion, flags))
        },
        [HemisphereMesh],
        ![HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         mesh: &CapsuleMesh,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            HasDynamicRigidBody,
            ModelTransform,
            ReferenceFrame,
            Motion,
            SceneEntityFlags
        )> {
            let flags = flags.copied().unwrap_or_default();

            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let inertial_properties = InertialProperties::of_uniform_capsule(
                mesh.segment_length * model_transform.scale,
                mesh.radius * model_transform.scale,
                substance.mass_density,
            );

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

            rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                inertial_properties,
                frame,
                motion,
            )?;

            Ok((HasDynamicRigidBody, model_transform, frame, motion, flags))
        },
        ![HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         inertial_properties: &DynamicRigidBodyInertialProperties,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            HasDynamicRigidBody,
            ModelTransform,
            ReferenceFrame,
            Motion,
            SceneEntityFlags
        )> {
            let flags = flags.copied().unwrap_or_default();

            let mut model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();
            let motion = motion.copied().unwrap_or_default();

            let mass = inertial_properties.mass;
            let center_of_mass = inertial_properties.center_of_mass.aligned();
            let inertia_tensor = inertial_properties.inertia_tensor.aligned();

            let mut inertial_properties = InertialProperties::new(
                mass,
                center_of_mass,
                InertiaTensor::from_matrix(inertia_tensor),
            );
            inertial_properties.scale(model_transform.scale);

            // Offset the model to put the center of mass at the origin of this
            // entity's space
            model_transform
                .set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

            rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                inertial_properties,
                frame,
                motion,
            )?;

            Ok((HasDynamicRigidBody, model_transform, frame, motion, flags))
        },
        ![HasDynamicRigidBody]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         mesh_id: &TriangleMeshID,
         substance: &DynamicRigidBodySubstance,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            HasDynamicRigidBody,
            ModelTransform,
            ReferenceFrame,
            Motion,
            SceneEntityFlags
        )> {
            let flags = flags.copied().unwrap_or_default();

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
                .set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

            rigid_body::setup::setup_dynamic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                inertial_properties,
                frame,
                motion,
            )?;

            Ok((HasDynamicRigidBody, model_transform, frame, motion, flags))
        },
        ![HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         frame: Option<&ReferenceFrame>,
         motion: &Motion,
         flags: Option<&SceneEntityFlags>|
         -> Result<(HasKinematicRigidBody, SceneEntityFlags)> {
            let flags = flags.copied().unwrap_or_default();

            rigid_body::setup::setup_kinematic_rigid_body(
                &mut rigid_body_manager,
                entity_id,
                frame.copied().unwrap_or_default(),
                *motion,
            )?;

            Ok((HasKinematicRigidBody, flags))
        },
        ![HasDynamicRigidBody, HasKinematicRigidBody]
    )?;

    Ok(())
}

pub fn remove_rigid_body_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    if entity.has_component::<HasDynamicRigidBody>() {
        let simulator = simulator.oread();
        let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
        rigid_body_manager.remove_dynamic_rigid_body(rigid_body_id);
    }
    if entity.has_component::<HasKinematicRigidBody>() {
        let simulator = simulator.oread();
        let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        let rigid_body_id = KinematicRigidBodyID::from_entity_id(entity_id);
        rigid_body_manager.remove_kinematic_rigid_body(rigid_body_id);
    }
}
