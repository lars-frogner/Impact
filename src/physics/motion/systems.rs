//! Tasks representing ECS systems related to motion.

use crate::{
    define_task,
    physics::{
        DrivenAngularVelocityComp, OrientationComp, PhysicsTag, PositionComp, RigidBodyComp,
        Static, VelocityComp,
    },
    world::World,
};
use impact_ecs::query;

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the position of all
    /// entities with velocities by one time step (unless the entity is static
    /// or governed by rigid body physics).
    [pub] AdvancePositions,
    depends_on = [],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing positions"; {
            let time_step_duration = world.simulator().read().unwrap().time_step_duration();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world, |position: &mut PositionComp, velocity: &VelocityComp| {
                    position.0 += velocity.0 * time_step_duration;
                },
                ![Static, RigidBodyComp]
            );
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the orientation of all
    /// entities with driven angluar velocities by one time step (unless the
    /// entity is static or governed by rigid body physics).
    [pub] AdvanceOrientations,
    depends_on = [],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing orientations"; {
            let time_step_duration = world.simulator().read().unwrap().time_step_duration();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world,
                |orientation: &mut OrientationComp,
                 position: &mut PositionComp,
                 driven_angular_velocity: &DrivenAngularVelocityComp|
                {
                    driven_angular_velocity.advance_orientation_and_shift_reference_frame(
                        &mut orientation.0,
                        &mut position.0,
                        time_step_duration
                    );
                },
                ![Static, RigidBodyComp]
            );
            Ok(())
        })
    }
);
