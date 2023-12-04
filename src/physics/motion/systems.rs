//! Tasks representing ECS systems related to motion.

use crate::{
    define_task,
    physics::{
        self, AngularVelocityComp, OrientationComp, PhysicsTag, PositionComp, RigidBodyComp,
        Static, VelocityComp,
    },
    scene::{
        SyncLightPositionsAndDirectionsInStorage, SyncSceneObjectTransformsWithOrientations,
        SyncSceneObjectTransformsWithPositions,
    },
    world::World,
};
use impact_ecs::query;

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the position of all
    /// entities with velocities by one time step (unless the entity is static
    /// or governed by rigid body physics).
    [pub] AdvancePositions,
    depends_on = [
        SyncSceneObjectTransformsWithPositions,
        SyncSceneObjectTransformsWithOrientations,
        SyncLightPositionsAndDirectionsInStorage
    ],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing positions"; {
            let time_step_duration = world.simulator().read().unwrap().scaled_time_step_duration();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world, |position: &mut PositionComp, velocity: &VelocityComp| {
                    position.position += velocity.0 * time_step_duration;
                },
                ![Static, RigidBodyComp]
            );
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the orientation of all
    /// entities with angluar velocities by one time step (unless the entity is
    /// static or governed by rigid body physics).
    [pub] AdvanceOrientations,
    depends_on = [
        SyncSceneObjectTransformsWithPositions,
        SyncSceneObjectTransformsWithOrientations,
        SyncLightPositionsAndDirectionsInStorage
    ],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing orientations"; {
            let time_step_duration = world.simulator().read().unwrap().scaled_time_step_duration();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world,
                |orientation: &mut OrientationComp,
                 angular_velocity: &AngularVelocityComp|
                {
                    orientation.0 = physics::advance_orientation(&orientation.0, &angular_velocity.0, time_step_duration);
                },
                ![Static, RigidBodyComp]
            );
            Ok(())
        })
    }
);
