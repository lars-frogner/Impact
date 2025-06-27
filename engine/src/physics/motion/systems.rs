//! ECS systems related to motion.

use crate::physics::{
    motion::{
        self,
        components::{LogsKineticEnergy, LogsMomentum, ReferenceFrameComp, VelocityComp},
    },
    rigid_body::components::RigidBodyComp,
};
use impact_ecs::{query, world::World as ECSWorld};

/// Logs the kinetic energy of each applicable entity with a [`RigidBodyComp`].
pub fn log_kinetic_energies(ecs_world: &ECSWorld) {
    query!(
        ecs_world,
        |frame: &ReferenceFrameComp, velocity: &VelocityComp, rigid_body: &RigidBodyComp| {
            let translational_kinetic_energy =
                motion::compute_translational_kinetic_energy(rigid_body.0.mass(), &velocity.linear);

            let rotational_kinetic_energy = motion::compute_rotational_kinetic_energy(
                rigid_body.0.inertial_properties(),
                &frame.orientation,
                frame.scaling,
                &velocity.angular,
            );

            let total_kinetic_energy = translational_kinetic_energy + rotational_kinetic_energy;

            impact_log::info!(
                "Body at {{{:.1}, {:.1}, {:.1}}} has kinetic energy {:.2} ({:.2} translational, {:.2} rotational)",
                frame.position.x,
                frame.position.y,
                frame.position.z,
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
            let linear_momentum = rigid_body.0.momentum();
            let angular_momentum = rigid_body.0.angular_momentum();

            impact_log::info!(
                "Body at {{{:.1}, {:.1}, {:.1}}} has linear momentum [{:.2}, {:.2}, {:.2}] and angular momentum [{:.2}, {:.2}, {:.2}]",
                frame.position.x,
                frame.position.y,
                frame.position.z,
                linear_momentum.x,
                linear_momentum.y,
                linear_momentum.z,
                angular_momentum.x,
                angular_momentum.y,
                angular_momentum.z,
            );
        },
        [LogsMomentum]
    );
}
