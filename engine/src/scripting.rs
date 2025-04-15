//! Interfacing with scripts.

use anyhow::Result;
use std::path::PathBuf;

pub trait Script: Send + Sync + std::fmt::Debug {
    fn app_config_path(&self) -> PathBuf;

    fn setup_scene(&self) -> Result<()>;
}

#[derive(Clone, Copy, Debug)]
pub struct DummyScript;

impl Script for DummyScript {
    fn app_config_path(&self) -> PathBuf {
        "".into()
    }

    fn setup_scene(&self) -> Result<()> {
        Ok(())
    }
}
