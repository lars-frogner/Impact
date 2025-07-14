//! Controller commands.

use crate::control::{MotionDirection, MotionState};
use impact_physics::fph;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub enum ControlCommand {
    SetMotion {
        state: MotionState,
        direction: MotionDirection,
    },
    StopMotion,
    SetMovementSpeed(fph),
}

impl PartialEq for ControlCommand {
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

impl Eq for ControlCommand {}
