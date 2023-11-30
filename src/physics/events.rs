//! Event handling related to physics.

use crate::{
    physics::{
        fph, AngularVelocityComp, InertialProperties, OrientationComp, PhysicsSimulator,
        PositionComp, RigidBody, RigidBodyComp, RigidBodyManager, UniformRigidBodyComp,
        VelocityComp,
    },
    rendering::fre,
    scene::{
        BoxMeshComp, ConeMeshComp, CylinderMeshComp, HemisphereMeshComp, MeshComp, MeshRepository,
        ScalingComp, SphereMeshComp,
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
        self.add_rigid_body_component_for_entity(mesh_repository, components);
    }

    /// Performs any modifications required to clean up the physics simulator
    /// when the given entity is removed.
    pub fn handle_entity_removed(&self, entity: &EntityEntry<'_>) {
        self.remove_rigid_body_for_entity(entity);
    }

    fn add_rigid_body_component_for_entity(
        &self,
        mesh_repository: &RwLock<MeshRepository<fre>>,
        components: &mut ArchetypeComponentStorage,
    ) {
        fn execute_setup(
            rigid_body_manager: &RwLock<RigidBodyManager>,
            mut inertial_properties: InertialProperties,
            position: &PositionComp,
            orientation: &OrientationComp,
            velocity: &VelocityComp,
            angular_velocity: &AngularVelocityComp,
            scaling: Option<&ScalingComp>,
        ) -> RigidBodyComp {
            if let Some(scaling) = scaling {
                inertial_properties.scale(scaling.0.into());
            }

            let rigid_body = RigidBody::new(
                inertial_properties,
                position.0,
                orientation.0,
                velocity.0,
                angular_velocity.0,
            );

            let rigid_body_id = rigid_body_manager
                .write()
                .unwrap()
                .include_rigid_body(rigid_body);

            RigidBodyComp { id: rigid_body_id }
        }

        setup!(
            components,
            |box_mesh: &BoxMeshComp,
             uniform_rigid_body: &UniformRigidBodyComp,
             position: &PositionComp,
             orientation: &OrientationComp,
             velocity: &VelocityComp,
             angular_velocity: &AngularVelocityComp,
             scaling: Option<&ScalingComp>|
             -> RigidBodyComp {
                let inertial_properties = InertialProperties::of_uniform_box(
                    box_mesh.extent_x as fph,
                    box_mesh.extent_y as fph,
                    box_mesh.extent_z as fph,
                    uniform_rigid_body.mass_density,
                );
                execute_setup(
                    &self.rigid_body_manager,
                    inertial_properties,
                    position,
                    orientation,
                    velocity,
                    angular_velocity,
                    scaling,
                )
            },
            ![RigidBodyComp]
        );

        setup!(
            components,
            |cylinder_mesh: &CylinderMeshComp,
             uniform_rigid_body: &UniformRigidBodyComp,
             position: &PositionComp,
             orientation: &OrientationComp,
             velocity: &VelocityComp,
             angular_velocity: &AngularVelocityComp,
             scaling: Option<&ScalingComp>|
             -> RigidBodyComp {
                let inertial_properties = InertialProperties::of_uniform_cylinder(
                    cylinder_mesh.length as fph,
                    cylinder_mesh.diameter as fph,
                    uniform_rigid_body.mass_density,
                );
                execute_setup(
                    &self.rigid_body_manager,
                    inertial_properties,
                    position,
                    orientation,
                    velocity,
                    angular_velocity,
                    scaling,
                )
            },
            ![RigidBodyComp]
        );

        setup!(
            components,
            |cone_mesh: &ConeMeshComp,
             uniform_rigid_body: &UniformRigidBodyComp,
             position: &PositionComp,
             orientation: &OrientationComp,
             velocity: &VelocityComp,
             angular_velocity: &AngularVelocityComp,
             scaling: Option<&ScalingComp>|
             -> RigidBodyComp {
                let inertial_properties = InertialProperties::of_uniform_cone(
                    cone_mesh.length as fph,
                    cone_mesh.max_diameter as fph,
                    uniform_rigid_body.mass_density,
                );
                execute_setup(
                    &self.rigid_body_manager,
                    inertial_properties,
                    position,
                    orientation,
                    velocity,
                    angular_velocity,
                    scaling,
                )
            },
            ![RigidBodyComp]
        );

        setup!(
            components,
            |uniform_rigid_body: &UniformRigidBodyComp,
             position: &PositionComp,
             orientation: &OrientationComp,
             velocity: &VelocityComp,
             angular_velocity: &AngularVelocityComp,
             scaling: Option<&ScalingComp>|
             -> RigidBodyComp {
                let inertial_properties =
                    InertialProperties::of_uniform_sphere(uniform_rigid_body.mass_density);
                execute_setup(
                    &self.rigid_body_manager,
                    inertial_properties,
                    position,
                    orientation,
                    velocity,
                    angular_velocity,
                    scaling,
                )
            },
            [SphereMeshComp],
            ![RigidBodyComp]
        );

        setup!(
            components,
            |uniform_rigid_body: &UniformRigidBodyComp,
             position: &PositionComp,
             orientation: &OrientationComp,
             velocity: &VelocityComp,
             angular_velocity: &AngularVelocityComp,
             scaling: Option<&ScalingComp>|
             -> RigidBodyComp {
                let inertial_properties =
                    InertialProperties::of_uniform_hemisphere(uniform_rigid_body.mass_density);
                execute_setup(
                    &self.rigid_body_manager,
                    inertial_properties,
                    position,
                    orientation,
                    velocity,
                    angular_velocity,
                    scaling,
                )
            },
            [HemisphereMeshComp],
            ![RigidBodyComp]
        );

        setup!(
            components,
            |mesh: &MeshComp,
             uniform_rigid_body: &UniformRigidBodyComp,
             position: &PositionComp,
             orientation: &OrientationComp,
             velocity: &VelocityComp,
             angular_velocity: &AngularVelocityComp,
             scaling: Option<&ScalingComp>|
             -> RigidBodyComp {
                let mesh_repository_readonly = mesh_repository.read().unwrap();
                let triangle_mesh = mesh_repository_readonly
                    .get_mesh(mesh.id)
                    .expect("Invalid mesh ID when creating rigid body");
                let inertial_properties = InertialProperties::of_uniform_triangle_mesh(
                    triangle_mesh,
                    uniform_rigid_body.mass_density,
                );
                execute_setup(
                    &self.rigid_body_manager,
                    inertial_properties,
                    position,
                    orientation,
                    velocity,
                    angular_velocity,
                    scaling,
                )
            },
            ![RigidBodyComp]
        );
    }

    fn remove_rigid_body_for_entity(&self, entity: &EntityEntry<'_>) {
        if let Some(rigid_body) = entity.get_component::<RigidBodyComp>() {
            let rigid_body_id = rigid_body.access().id;
            self.rigid_body_manager
                .write()
                .unwrap()
                .remove_rigid_body(rigid_body_id);
        }
    }
}
