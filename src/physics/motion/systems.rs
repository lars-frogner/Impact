//! Tasks representing ECS systems related to motion.

use crate::{
    define_task,
    physics::{
        self, AngularVelocityComp, OrientationComp, PhysicsTag, PositionComp, Static, VelocityComp,
    },
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
                    position.0 += velocity.0 * time_step_duration;
                },
                ![Static]
            );
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the orientation
    /// of all entities with angluar velocities by one time step.
    [pub] AdvanceOrientations,
    depends_on = [],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing orientations"; {
            let time_step_duration = world.simulator().read().unwrap().time_step_duration();
            let ecs_world = world.ecs_world().read().unwrap();
            query!(
                ecs_world, |orientation: &mut OrientationComp, position: &mut PositionComp, angular_velocity: &AngularVelocityComp| {
                    let new_orientation = physics::advance_orientation(&orientation.0, &angular_velocity.0, time_step_duration);

                    // The position, which is the world space coordinates of the
                    // model space origin, is by default unaffected by the
                    // rotation. This is only correct if the center of rotation
                    // is at the model space origin. If the center of rotation
                    // is somewhere else, the model's reference frame must be
                    // displaced in world space so that the center of rotation
                    // does not move.
                    position.0 += physics::compute_model_origin_shift_from_orientation_change(
                        &orientation.0,
                        &new_orientation,
                        angular_velocity.0.center_of_rotation()
                    );

                    orientation.0 = new_orientation;
                },
                ![Static]
            );
            Ok(())
        })
    }
);
