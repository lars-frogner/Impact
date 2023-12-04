//! Tasks representing ECS systems related to motion.

use crate::{
    define_task,
    physics::{
        self, AngularVelocityComp, PhysicsTag, RigidBodyComp, SpatialConfigurationComp, Static,
        VelocityComp,
    },
    scene::{SyncLightPositionsAndDirectionsInStorage, SyncSceneObjectTransforms},
    world::World,
};
use impact_ecs::query;

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the position of all
    /// entities with velocities by one time step (unless the entity is static
    /// or governed by rigid body physics).
    [pub] AdvancePositions,
    depends_on = [
        SyncSceneObjectTransforms,
        SyncLightPositionsAndDirectionsInStorage
    ],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing positions"; {
            let time_step_duration = world.simulator().read().unwrap().scaled_time_step_duration();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world, |spatial: &mut SpatialConfigurationComp, velocity: &VelocityComp| {
                    spatial.position += velocity.0 * time_step_duration;
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
        SyncSceneObjectTransforms,
        SyncLightPositionsAndDirectionsInStorage
    ],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing orientations"; {
            let time_step_duration = world.simulator().read().unwrap().scaled_time_step_duration();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world,
                |spatial: &mut SpatialConfigurationComp,
                 angular_velocity: &AngularVelocityComp|
                {
                    spatial.orientation = physics::advance_orientation(&spatial.orientation, &angular_velocity.0, time_step_duration);
                },
                ![Static, RigidBodyComp]
            );
            Ok(())
        })
    }
);
