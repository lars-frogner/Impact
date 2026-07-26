//! Command utilities.

use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToActiveState {
    Enabled,
    Disabled,
    Opposite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModifiedActiveState {
    pub state: ActiveState,
    pub changed: bool,
}

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveState {
    Enabled,
    Disabled,
}

impl ToActiveState {
    pub fn from_enabled(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }

    pub fn set(self, enabled: &mut bool) -> ModifiedActiveState {
        let was_enabled = *enabled;
        let state = self.apply(enabled);
        let changed = *enabled != was_enabled;
        ModifiedActiveState { state, changed }
    }

    fn apply(self, enabled: &mut bool) -> ActiveState {
        match (self, *enabled) {
            (Self::Enabled, _) | (Self::Opposite, false) => {
                *enabled = true;
                ActiveState::Enabled
            }
            (Self::Disabled, _) | (Self::Opposite, true) => {
                *enabled = false;
                ActiveState::Disabled
            }
        }
    }

    pub fn enabled(&self) -> bool {
        *self == Self::Enabled
    }
}

impl ActiveState {
    pub fn is_enabled(self) -> bool {
        self == Self::Enabled
    }
}
