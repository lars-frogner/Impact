//! Controller commands.

use crate::engine::Engine;
use impact_controller::motion::{MotionDirection, MotionState};
use impact_physics::fph;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub enum ControllerCommand {
    SetMotion {
        state: MotionState,
        direction: MotionDirection,
    },
    StopMotion,
    SetMovementSpeed(fph),
}

impl PartialEq for ControllerCommand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::SetMotion {
                    state: state_a,
                    direction: direction_a,
                },
                Self::SetMotion {
                    state: state_b,
                    direction: direction_b,
                },
            ) => state_a == state_b && direction_a == direction_b,
            (Self::StopMotion, Self::StopMotion) => true,
            (Self::SetMovementSpeed(a), Self::SetMovementSpeed(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

impl Eq for ControllerCommand {}

pub fn set_motion(engine: &Engine, state: MotionState, direction: MotionDirection) {
    if engine.controls_enabled() {
        if let Some(motion_controller) = engine.motion_controller() {
            impact_log::debug!("Setting motion in direction {direction:?} to {state:?}");
            motion_controller
                .lock()
                .unwrap()
                .update_motion(state, direction);
        } else {
            impact_log::info!("Not setting motion since there is no motion controller");
        }
    } else {
        impact_log::info!("Not setting motion since controls are disabled");
    }
}

pub fn stop_motion(engine: &Engine) {
    if let Some(motion_controller) = engine.motion_controller() {
        impact_log::info!("Stopping controller motion");
        motion_controller.lock().unwrap().stop();
    } else {
        impact_log::info!("Not stopping motion since there is no motion controller");
    }
}

pub fn set_movement_speed(engine: &Engine, speed: fph) {
    if let Some(motion_controller) = engine.motion_controller() {
        impact_log::info!("Setting movement speed to {speed:?}");
        motion_controller.lock().unwrap().set_movement_speed(speed);
    } else {
        impact_log::info!("Not setting movement speed since there is no motion controller");
    }
}
