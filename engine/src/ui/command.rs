//! Commands for controlling the user interface.

use crate::engine::command::ToActiveState;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UICommand {
    Set(ToActiveState),
}
