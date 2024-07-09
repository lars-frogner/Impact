//! ECS systems related to motion.

use crate::physics::{
    motion::components::{LogsKineticEnergy, ReferenceFrameComp},
    rigid_body::components::RigidBodyComp,
};
use impact_ecs::{query, world::World as ECSWorld};

/// Logs the kinetic energy of each applicable entity with a [`RigidBodyComp`].
pub fn log_kinetic_energies(ecs_world: &ECSWorld) {
    query!(
        ecs_world,
        |frame: &ReferenceFrameComp, rigid_body: &RigidBodyComp| {
            let position = rigid_body.0.position();
            let translational_kinetic_energy = rigid_body.0.compute_translational_kinetic_energy();
            let rotational_kinetic_energy = rigid_body
                .0
                .compute_rotational_kinetic_energy(frame.scaling);
            let total_kinetic_energy = translational_kinetic_energy + rotational_kinetic_energy;
            log::info!(
                "Body at {{{:.1}, {:.1}, {:.1}}} has kinetic energy {:.2} ({:.2} translational, {:.2} rotational)",
                position.x,
                position.y,
                position.z,
                total_kinetic_energy,
                translational_kinetic_energy,
                rotational_kinetic_energy,
            );
        },
        [LogsKineticEnergy]
    );
}

/// Logs the linear and angular momentum of each applicable entity with a
/// [`RigidBodyComp`].
pub fn log_momenta(ecs_world: &ECSWorld) {
    query!(
        ecs_world,
        |frame: &ReferenceFrameComp, rigid_body: &RigidBodyComp| {
            let position = rigid_body.0.position();
            let translational_kinetic_energy = rigid_body.0.compute_translational_kinetic_energy();
            let rotational_kinetic_energy = rigid_body
                .0
                .compute_rotational_kinetic_energy(frame.scaling);
            let total_kinetic_energy = translational_kinetic_energy + rotational_kinetic_energy;
            log::info!(
                "Body at {{{:.1}, {:.1}, {:.1}}} has kinetic energy {:.2} ({:.2} translational, {:.2} rotational)",
                position.x,
                position.y,
                position.z,
                total_kinetic_energy,
                translational_kinetic_energy,
                rotational_kinetic_energy,
            );
        },
        [LogsKineticEnergy]
    );
}
