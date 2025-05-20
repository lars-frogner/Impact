//! Management of rigid bodies for entities.

use crate::{
    mesh::{
        MeshRepository,
        components::{
            BoxMeshComp, ConeMeshComp, CylinderMeshComp, HemisphereMeshComp, MeshComp,
            SphereMeshComp,
        },
    },
    physics::{
        fph,
        inertia::InertialProperties,
        motion::components::{ReferenceFrameComp, VelocityComp},
        rigid_body::{
            RigidBody,
            components::{RigidBodyComp, UniformRigidBodyComp},
        },
    },
    scene::components::SceneEntityFlagsComp,
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has a component
/// representing a rigid body, and if so, creates the corresponding rigid body
/// and adds a [`RigidBodyComp`] to the entity.
pub fn setup_rigid_body_for_new_entity(
    mesh_repository: &RwLock<MeshRepository>,
    components: &mut ArchetypeComponentStorage,
) {
    fn execute_setup(
        mut inertial_properties: InertialProperties,
        frame: Option<&ReferenceFrameComp>,
        velocity: Option<&VelocityComp>,
        flags: Option<&SceneEntityFlagsComp>,
    ) -> (
        RigidBodyComp,
        ReferenceFrameComp,
        VelocityComp,
        SceneEntityFlagsComp,
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
        |box_mesh: &BoxMeshComp,
         uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlagsComp>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlagsComp
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
        |cylinder_mesh: &CylinderMeshComp,
         uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlagsComp>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlagsComp
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
        |cone_mesh: &ConeMeshComp,
         uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlagsComp>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlagsComp
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
         flags: Option<&SceneEntityFlagsComp>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlagsComp
        ) {
            let inertial_properties =
                InertialProperties::of_uniform_sphere(uniform_rigid_body.mass_density);
            execute_setup(inertial_properties, frame, velocity, flags)
        },
        [SphereMeshComp],
        ![RigidBodyComp]
    );

    setup!(
        components,
        |uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlagsComp>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlagsComp
        ) {
            let inertial_properties =
                InertialProperties::of_uniform_hemisphere(uniform_rigid_body.mass_density);
            execute_setup(inertial_properties, frame, velocity, flags)
        },
        [HemisphereMeshComp],
        ![RigidBodyComp]
    );

    setup!(
        components,
        |mesh: &MeshComp,
         uniform_rigid_body: &UniformRigidBodyComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         flags: Option<&SceneEntityFlagsComp>|
         -> (
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp,
            SceneEntityFlagsComp
        ) {
            let mesh_repository_readonly = mesh_repository.read().unwrap();
            let triangle_mesh = mesh_repository_readonly
                .get_mesh(mesh.id)
                .expect("Invalid mesh ID when creating rigid body");
            let inertial_properties = InertialProperties::of_uniform_triangle_mesh(
                triangle_mesh,
                uniform_rigid_body.mass_density,
            );
            execute_setup(inertial_properties, frame, velocity, flags)
        },
        ![RigidBodyComp]
    );
}
