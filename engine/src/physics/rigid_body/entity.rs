//! Management of rigid bodies for entities.

use crate::physics::{
    fph,
    inertia::InertialProperties,
    motion::components::{ReferenceFrameComp, VelocityComp},
    rigid_body::{
        RigidBody,
        components::{RigidBodyComp, UniformRigidBodyComp},
    },
};
use anyhow::{Result, anyhow};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_mesh::{
    MeshRepository, TriangleMeshID,
    setup::{BoxMesh, ConeMesh, CylinderMesh, HemisphereMesh, SphereMesh},
};
use impact_scene::SceneEntityFlags;
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has a component
/// representing a rigid body, and if so, creates the corresponding rigid body
/// and adds a [`RigidBodyComp`] to the entity.
pub fn setup_rigid_body_for_new_entity(
    mesh_repository: &RwLock<MeshRepository>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    fn execute_setup(
        mut inertial_properties: InertialProperties,
        frame: Option<&ReferenceFrameComp>,
        velocity: Option<&VelocityComp>,
        flags: Option<&SceneEntityFlags>,
    ) -> (
        RigidBodyComp,
        ReferenceFrameComp,
        VelocityComp,
        SceneEntityFlags,
    ) {
        let mut frame = frame.cloned().unwrap_or_default();

        // Scale the mass to be consistent with the initial scale factor. If
        // the scale factor changes later on, we will conserve the mass and
        // only let the scale change the extent of the body.
        inertial_properties.multiply_mass(frame.scaling.powi(3));

        let velocity = velocity.cloned().unwrap_or_default();

        // Use center of mass as new origin, since all free rotation is
        // about the center of mass
        frame.origin_offset = inertial_properties.center_of_mass().coords;

        let rigid_body = RigidBody::new(
            inertial_properties,
            frame.orientation,
            frame.scaling,
            &velocity.linear,
            &velocity.angular,
        );

        (
            RigidBodyComp(rigid_body),
            frame,
            velocity,
            flags.copied().unwrap_or_default(),
        )
    }

    setup!(
        components,
        |box_mesh: &BoxMesh,
         uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlags>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlags
        ) {
            let inertial_properties = InertialProperties::of_uniform_box(
                fph::from(box_mesh.extent_x),
                fph::from(box_mesh.extent_y),
                fph::from(box_mesh.extent_z),
                uniform_rigid_body.mass_density,
            );
            execute_setup(inertial_properties, frame, velocity, flags)
        },
        ![RigidBodyComp]
    );

    setup!(
        components,
        |cylinder_mesh: &CylinderMesh,
         uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlags>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlags
        ) {
            let inertial_properties = InertialProperties::of_uniform_cylinder(
                fph::from(cylinder_mesh.length),
                fph::from(cylinder_mesh.diameter),
                uniform_rigid_body.mass_density,
            );
            execute_setup(inertial_properties, frame, velocity, flags)
        },
        ![RigidBodyComp]
    );

    setup!(
        components,
        |cone_mesh: &ConeMesh,
         uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlags>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlags
        ) {
            let inertial_properties = InertialProperties::of_uniform_cone(
                fph::from(cone_mesh.length),
                fph::from(cone_mesh.max_diameter),
                uniform_rigid_body.mass_density,
            );
            execute_setup(inertial_properties, frame, velocity, flags)
        },
        ![RigidBodyComp]
    );

    setup!(
        components,
        |uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlags>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlags
        ) {
            let inertial_properties =
                InertialProperties::of_uniform_sphere(uniform_rigid_body.mass_density);
            execute_setup(inertial_properties, frame, velocity, flags)
        },
        [SphereMesh],
        ![RigidBodyComp]
    );

    setup!(
        components,
        |uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlags>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlags
        ) {
            let inertial_properties =
                InertialProperties::of_uniform_hemisphere(uniform_rigid_body.mass_density);
            execute_setup(inertial_properties, frame, velocity, flags)
        },
        [HemisphereMesh],
        ![RigidBodyComp]
    );

    setup!(
        components,
        |mesh_id: &TriangleMeshID,
         uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlags
        )> {
            let mesh_repository_readonly = mesh_repository.read().unwrap();
            let triangle_mesh = mesh_repository_readonly
                .get_triangle_mesh(*mesh_id)
                .ok_or_else(|| {
                    anyhow!(
                        "Tried to create rigid body for missing mesh (mesh ID {})",
                        mesh_id
                    )
                })?;
            let inertial_properties = InertialProperties::of_uniform_triangle_mesh(
                triangle_mesh,
                uniform_rigid_body.mass_density,
            );
            Ok(execute_setup(inertial_properties, frame, velocity, flags))
        },
        ![RigidBodyComp]
    )?;

    Ok(())
}
