//! ECS systems related to motion control.

use crate::{
    control::{MotionController, motion::components::MotionControlComp},
    physics::{
        fph,
        motion::components::{ReferenceFrameComp, VelocityComp},
    },
};
use impact_ecs::{query, world::World as ECSWorld};

/// Updates the world-space velocities of all entities controlled by the given
/// motion controller, and advances their positions by the given time step
/// duration when applicable.
pub fn update_motion_of_controlled_entities(
    ecs_world: &ECSWorld,
    motion_controller: &(impl MotionController + ?Sized),
    time_step_duration: fph,
) {
    query!(
        ecs_world,
        |motion_control: &mut MotionControlComp,
         velocity: &mut VelocityComp,
         frame: &mut ReferenceFrameComp| {
            let new_control_velocity =
                motion_controller.compute_control_velocity(&frame.orientation);
            motion_control.apply_new_control_velocity(new_control_velocity, &mut velocity.linear);

            frame.position += velocity.linear * time_step_duration;
        }
    );
}
