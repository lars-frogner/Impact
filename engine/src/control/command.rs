//! Controller commands.

use crate::{
    control::{MotionDirection, MotionState},
    physics::fph,
};
use roc_codegen::roc;

#[roc(prefix = "Engine")]
#[derive(Clone, Debug)]
pub enum ControlCommand {
    SetMotion {
        state: MotionState,
        direction: MotionDirection,
    },
    StopMotion,
    SetMovementSpeed(fph),
}
