//! Tasks related to motion.

use crate::{
    define_task,
    physics::{
        motion,
        tasks::{AdvanceSimulation, PhysicsTag},
    },
    world::World,
};

define_task!(
    /// This [`Task`](crate::scheduling::Task) logs the kinetic energy of each
    /// applicable rigid body.
    [pub] LogKineticEnergy,
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Logging kinetic energy"; {
            let ecs_world = world.ecs_world().read().unwrap();
            motion::systems::log_kinetic_energies(&ecs_world);
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
            motion::systems::log_momenta(&ecs_world);
            Ok(())
        })
    }
);
