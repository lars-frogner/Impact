//! Controller commands.

use crate::{
    control::{MotionDirection, MotionState},
    physics::fph,
};
use roc_codegen::roc;

#[roc(parents = "Command")]
#[derive(Clone, Debug)]
pub enum ControlCommand {
    SetMotion {
        state: MotionState,
        direction: MotionDirection,
    },
    StopMotion,
    SetMovementSpeed(fph),
}
