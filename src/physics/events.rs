//! Event handling related to physics.

use crate::{
    gpu::rendering::fre,
    mesh::{
        components::{
            BoxMeshComp, ConeMeshComp, CylinderMeshComp, HemisphereMeshComp, MeshComp,
            SphereMeshComp,
        },
        MeshRepository,
    },
    physics::{
        fph, InertialProperties, PhysicsSimulator, ReferenceFrameComp, RigidBody, RigidBodyComp,
        UniformRigidBodyComp, VelocityComp,
    },
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use std::sync::RwLock;

impl PhysicsSimulator {
    /// Performs any modifications to the physics simulator required to
    /// accommodate a new entity with components represented by the given
    /// component manager, and adds any additional components to the entity's
    /// components.
    pub fn handle_entity_created(
        &self,
        mesh_repository: &RwLock<MeshRepository<fre>>,
        components: &mut ArchetypeComponentStorage,
    ) {
        Self::add_rigid_body_component_for_entity(mesh_repository, components);

        self.rigid_body_force_manager
            .read()
            .unwrap()
            .add_force_components_for_entity(mesh_repository, components);
    }

    /// Performs any modifications required to clean up the physics simulator
    /// when the given entity is removed.
    pub fn handle_entity_removed(&self, entity: &EntityEntry<'_>) {
        Self::remove_rigid_body_for_entity(entity);

        self.rigid_body_force_manager
            .read()
            .unwrap()
            .handle_entity_removed(entity);
    }

    fn add_rigid_body_component_for_entity(
        mesh_repository: &RwLock<MeshRepository<fre>>,
        components: &mut ArchetypeComponentStorage,
    ) {
        fn execute_setup(
            mut inertial_properties: InertialProperties,
            frame: Option<&ReferenceFrameComp>,
            velocity: Option<&VelocityComp>,
        ) -> (RigidBodyComp, ReferenceFrameComp, VelocityComp) {
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
                frame.position,
                frame.orientation,
                frame.scaling,
                &velocity.linear,
                &velocity.angular,
            );

            (RigidBodyComp(rigid_body), frame, velocity)
        }

        setup!(
            components,
            |box_mesh: &BoxMeshComp,
             uniform_rigid_body: &UniformRigidBodyComp,
             frame: Option<&ReferenceFrameComp>,
             velocity: Option<&VelocityComp>|
             -> (RigidBodyComp, ReferenceFrameComp, VelocityComp) {
                let inertial_properties = InertialProperties::of_uniform_box(
                    fph::from(box_mesh.extent_x),
                    fph::from(box_mesh.extent_y),
                    fph::from(box_mesh.extent_z),
                    uniform_rigid_body.mass_density,
                );
                execute_setup(inertial_properties, frame, velocity)
            },
            ![RigidBodyComp]
        );

        setup!(
            components,
            |cylinder_mesh: &CylinderMeshComp,
             uniform_rigid_body: &UniformRigidBodyComp,
             frame: Option<&ReferenceFrameComp>,
             velocity: Option<&VelocityComp>|
             -> (RigidBodyComp, ReferenceFrameComp, VelocityComp) {
                let inertial_properties = InertialProperties::of_uniform_cylinder(
                    fph::from(cylinder_mesh.length),
                    fph::from(cylinder_mesh.diameter),
                    uniform_rigid_body.mass_density,
                );
                execute_setup(inertial_properties, frame, velocity)
            },
            ![RigidBodyComp]
        );

        setup!(
            components,
            |cone_mesh: &ConeMeshComp,
             uniform_rigid_body: &UniformRigidBodyComp,
             frame: Option<&ReferenceFrameComp>,
             velocity: Option<&VelocityComp>|
             -> (RigidBodyComp, ReferenceFrameComp, VelocityComp) {
                let inertial_properties = InertialProperties::of_uniform_cone(
                    fph::from(cone_mesh.length),
                    fph::from(cone_mesh.max_diameter),
                    uniform_rigid_body.mass_density,
                );
                execute_setup(inertial_properties, frame, velocity)
            },
            ![RigidBodyComp]
        );

        setup!(
            components,
            |uniform_rigid_body: &UniformRigidBodyComp,
             frame: Option<&ReferenceFrameComp>,
             velocity: Option<&VelocityComp>|
             -> (RigidBodyComp, ReferenceFrameComp, VelocityComp) {
                let inertial_properties =
                    InertialProperties::of_uniform_sphere(uniform_rigid_body.mass_density);
                execute_setup(inertial_properties, frame, velocity)
            },
            [SphereMeshComp],
            ![RigidBodyComp]
        );

        setup!(
            components,
            |uniform_rigid_body: &UniformRigidBodyComp,
             frame: Option<&ReferenceFrameComp>,
             velocity: Option<&VelocityComp>|
             -> (RigidBodyComp, ReferenceFrameComp, VelocityComp) {
                let inertial_properties =
                    InertialProperties::of_uniform_hemisphere(uniform_rigid_body.mass_density);
                execute_setup(inertial_properties, frame, velocity)
            },
            [HemisphereMeshComp],
            ![RigidBodyComp]
        );

        setup!(
            components,
            |mesh: &MeshComp,
             uniform_rigid_body: &UniformRigidBodyComp,
             frame: Option<&ReferenceFrameComp>,
             velocity: Option<&VelocityComp>|
             -> (RigidBodyComp, ReferenceFrameComp, VelocityComp) {
                let mesh_repository_readonly = mesh_repository.read().unwrap();
                let triangle_mesh = mesh_repository_readonly
                    .get_mesh(mesh.id)
                    .expect("Invalid mesh ID when creating rigid body");
                let inertial_properties = InertialProperties::of_uniform_triangle_mesh(
                    triangle_mesh,
                    uniform_rigid_body.mass_density,
                );
                execute_setup(inertial_properties, frame, velocity)
            },
            ![RigidBodyComp]
        );
    }

    fn remove_rigid_body_for_entity(_entity: &EntityEntry<'_>) {}
}
