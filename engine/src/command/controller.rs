//! Controller commands.

use crate::{command::uils::ToActiveState, engine::Engine, lock_order::OrderedMutex};
use impact_controller::motion::{MotionDirection, MotionState};
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq)]
pub enum ControlCommand {
    SetMotion {
        state: MotionState,
        direction: MotionDirection,
    },
    StopMotion,
    SetMovementSpeed(f32),
}

#[derive(Clone, Debug)]
pub enum ControlAdminCommand {
    SetControls(ToActiveState),
}

pub fn set_motion(engine: &Engine, state: MotionState, direction: MotionDirection) {
    if engine.controls_enabled() {
        if let Some(motion_controller) = engine.motion_controller() {
            log::debug!("Setting motion in direction {direction:?} to {state:?}");
            motion_controller.olock().update_motion(state, direction);
        } else {
            log::trace!("Not setting motion since there is no motion controller");
        }
    } else {
        log::trace!("Not setting motion since controls are disabled");
    }
}

pub fn stop_motion(engine: &Engine) {
    if let Some(motion_controller) = engine.motion_controller() {
        log::info!("Stopping controller motion");
        motion_controller.olock().stop();
    } else {
        log::info!("Not stopping motion since there is no motion controller");
    }
}

pub fn set_movement_speed(engine: &Engine, speed: f32) {
    if let Some(motion_controller) = engine.motion_controller() {
        log::info!("Setting movement speed to {speed:?}");
        motion_controller.olock().set_movement_speed(speed);
    } else {
        log::info!("Not setting movement speed since there is no motion controller");
    }
}
