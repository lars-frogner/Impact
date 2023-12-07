//! Tasks representing ECS systems related to motion.

use crate::{
    define_task,
    physics::{AdvanceSimulation, LogsKineticEnergy, LogsMomentum, PhysicsTag, RigidBodyComp},
    world::World,
};
use impact_ecs::query;

define_task!(
    /// This [`Task`](crate::scheduling::Task) logs the kinetic energy of each
    /// applicable rigid body.
    [pub] LogKineticEnergy,
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Logging kinetic energy"; {
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world, |rigid_body: &RigidBodyComp| {
                    let position = rigid_body.0.position();
                    let translational_kinetic_energy = rigid_body.0.compute_translational_kinetic_energy();
                    let rotational_kinetic_energy = rigid_body.0.compute_rotational_kinetic_energy();
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
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) logs the linear and angular
    /// momentum of each applicable rigid body.
    [pub] LogMomentum,
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Logging momentum"; {
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world, |rigid_body: &RigidBodyComp| {
                    let position = rigid_body.0.position();
                    let momentum = rigid_body.0.momentum();
                    let angular_momentum = rigid_body.0.angular_momentum();
                    log::info!(
                        "Body at {{{:.1}, {:.1}, {:.1}}} has linear momentum [{:.3}, {:.3}, {:.3}] and angular momentum [{:.3}, {:.3}, {:.3}]",
                        position.x,
                        position.y,
                        position.z,
                        momentum.x,
                        momentum.y,
                        momentum.z,
                        angular_momentum.x,
                        angular_momentum.y,
                        angular_momentum.z,
                    );
                },
                [LogsMomentum]
            );
            Ok(())
        })
    }
);
