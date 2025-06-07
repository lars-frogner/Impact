pub mod command;
pub mod timing;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentationConfig {
    pub task_timing_enabled: bool,
}

impl Default for InstrumentationConfig {
    fn default() -> Self {
        Self {
            task_timing_enabled: false,
        }
    }
}
