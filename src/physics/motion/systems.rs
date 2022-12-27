//! Tasks representing ECS systems related to motion.

use crate::{
    define_task,
    physics::{PhysicsTag, PositionComp, VelocityComp},
    world::World,
};
use impact_ecs::query;

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the position
    /// of all entities with velocities by one time step.
    [pub] AdvancePositions,
    depends_on = [],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing positions"; {
            let time_step_duration = world.simulator().read().unwrap().time_step_duration();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world, |position: &mut PositionComp, velocity: &VelocityComp| {
                    position.position += velocity.velocity*time_step_duration;
                }
            );
            Ok(())
        })
    }
);
