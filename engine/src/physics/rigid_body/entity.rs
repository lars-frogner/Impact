//! Management of rigid bodies for entities.

use anyhow::{Result, anyhow};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_mesh::{
    MeshRepository, TriangleMeshID,
    setup::{BoxMesh, ConeMesh, CylinderMesh, HemisphereMesh, SphereMesh},
};
use impact_physics::{
    fph,
    inertia::InertialProperties,
    quantities::Motion,
    rigid_body::{
        self, DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager,
        setup::DynamicRigidBodySubstance,
    },
};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has the components
/// representing a dynamic or kinematic rigid body, and if so, creates the
/// corresponding rigid body and adds a [`DynamicRigidBodyID`] or
/// [`KinematicRigidBodyID`] to the entity.
pub fn setup_rigid_body_for_new_entity(
    rigid_body_manager: &RwLock<RigidBodyManager>,
    mesh_repository: &RwLock<MeshRepository>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    // Make sure entities with a manually created dynamic rigid body get the
    // [`ReferenceFrame`] and [`Motion`] components.
    setup!(
        components,
        |frame: Option<&ReferenceFrame>, motion: Option<&Motion>| -> (ReferenceFrame, Motion) {
            (
                frame.copied().unwrap_or_default(),
                motion.copied().unwrap_or_default(),
            )
        },
        [DynamicRigidBodyID]
    );

    setup!(
        {
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
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
                fph::from(mesh.extent_x * model_transform.scale),
                fph::from(mesh.extent_y * model_transform.scale),
                fph::from(mesh.extent_z * model_transform.scale),
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
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
                fph::from(mesh.length * model_transform.scale),
                fph::from(mesh.diameter * model_transform.scale),
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
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
                fph::from(mesh.length * model_transform.scale),
                fph::from(mesh.max_diameter * model_transform.scale),
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
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
                fph::from(radius * model_transform.scale),
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
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
                fph::from(radius * model_transform.scale),
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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
            let mesh_repository = mesh_repository.read().unwrap();
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

            let triangle_mesh = mesh_repository.get_triangle_mesh(*mesh_id).ok_or_else(|| {
                anyhow!(
                    "Tried to create rigid body for missing mesh (mesh ID {})",
                    mesh_id
                )
            })?;

            let mut inertial_properties = InertialProperties::of_uniform_triangle_mesh(
                triangle_mesh.triangle_vertex_positions(),
                substance.mass_density,
            );
            inertial_properties.scale(fph::from(model_transform.scale));

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
            let mut rigid_body_manager = rigid_body_manager.write().unwrap();
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
