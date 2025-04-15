//! Interfacing with the application using the engine.

use anyhow::Result;
use std::path::PathBuf;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn engine_config_path(&self) -> PathBuf;

    fn setup_scene(&self) -> Result<()>;
}

#[derive(Clone, Copy, Debug)]
pub struct DummyApp;

impl Application for DummyApp {
    fn engine_config_path(&self) -> PathBuf {
        "".into()
    }

    fn setup_scene(&self) -> Result<()> {
        Ok(())
    }
}
