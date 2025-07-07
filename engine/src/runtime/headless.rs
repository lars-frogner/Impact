//! Headless execution of a [`Runtime`].

use crate::{runtime::Runtime, ui::NoUserInterface};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

pub type HeadlessRuntime = Runtime<NoUserInterface>;

/// Configuration options for setting up and running the engine headless.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HeadlessConfig {
    /// The width and height of the texture being rendered to in physical
    /// pixels.
    pub surface_size: (NonZeroU32, NonZeroU32),
    /// Actions to perform during headless execution.
    pub actions: HeadlessActions,
    /// When headless execution should terminate.
    pub termination_criterion: TerminationCriterion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessActions {
    /// Whether to save a screenshot of the rendered scene on termination.
    pub save_screenshot_on_exit: bool,
}

/// When headless execution should terminate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TerminationCriterion {
    IterationCountReached { count: u64 },
    ElapsedRealTimeExceeds { seconds: f64 },
    ElapsedSimulationTimeExceeds { time: f64 },
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            surface_size: (
                NonZeroU32::new(1600).unwrap(),
                NonZeroU32::new(1200).unwrap(),
            ),
            actions: HeadlessActions {
                save_screenshot_on_exit: false,
            },
            termination_criterion: TerminationCriterion::IterationCountReached { count: 1 },
        }
    }
}

impl TerminationCriterion {
    fn should_terminate(&self, runtime: &HeadlessRuntime, iteration_count: u64) -> bool {
        match self {
            Self::IterationCountReached { count } => iteration_count == *count,
            Self::ElapsedRealTimeExceeds { seconds } => {
                runtime.game_loop().elapsed_time().as_secs_f64() > *seconds
            }
            Self::ElapsedSimulationTimeExceeds { time } => {
                runtime
                    .engine()
                    .simulator()
                    .read()
                    .unwrap()
                    .current_simulation_time()
                    > *time
            }
        }
    }
}

/// Performs iterations of the game loop in the given runtime until the
/// termination condition is met, performing any specified actions.
pub fn run_headless(
    mut runtime: HeadlessRuntime,
    actions: HeadlessActions,
    termination_criterion: TerminationCriterion,
) -> Result<()> {
    let mut iteration_count = 0;

    while !termination_criterion.should_terminate(&runtime, iteration_count) {
        let result = runtime.perform_game_loop_iteration();
        iteration_count += 1;

        if let Err(errors) = result {
            bail!("A task encountered a fatal error: {errors:?}")
        }
    }

    if actions.save_screenshot_on_exit {
        runtime.engine().request_screenshot_save();
        runtime.engine().save_screenshots()?;
    }

    Ok(())
}
