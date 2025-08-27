//! Headless execution of a [`Runtime`].

use crate::{runtime::Runtime, ui::NoUserInterface};
use anyhow::Result;
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
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            surface_size: (
                NonZeroU32::new(1600).unwrap(),
                NonZeroU32::new(1200).unwrap(),
            ),
        }
    }
}

/// Performs iterations of the game loop in the given runtime until shutdown is
/// requested.
pub fn run_headless(runtime: HeadlessRuntime) -> Result<()> {
    loop {
        runtime.perform_game_loop_iteration()?;

        if runtime.shutdown_requested() {
            impact_log::info!("Shutting down after request");
            runtime.engine().app().on_shutdown()?;
            return Ok(());
        }
    }
}
