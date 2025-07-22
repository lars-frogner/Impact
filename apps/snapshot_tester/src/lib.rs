//! Snapshot tester for the Impact game engine.

pub mod api;
pub mod scripting;
pub mod testing;

pub use impact::{self, roc_integration};

#[cfg(feature = "roc_codegen")]
pub use impact::component::gather_roc_type_ids_for_all_components;

use anyhow::{Result, bail};
use impact::{
    application::Application,
    command::{EngineCommand, scene::SceneCommand},
    engine::Engine,
    impact_io,
    runtime::{RuntimeConfig, headless::HeadlessConfig},
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use testing::TestScene;

use crate::testing::ComparisonOutcome;

static ENGINE: RwLock<Option<Arc<Engine>>> = RwLock::new(None);

#[derive(Clone, Debug)]
pub struct SnapshotTester {
    test_scenes: Vec<TestScene>,
    config: TestingConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub testing: TestingConfig,
    #[serde(default)]
    pub headless: HeadlessConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    pub engine_config_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestingConfig {
    pub output_dir: PathBuf,
    pub reference_dir: PathBuf,
    #[serde(default)]
    pub only_test_scenes: Option<Vec<TestScene>>,
    pub min_score_to_pass: f64,
}

impl SnapshotTester {
    pub fn new(config: TestingConfig) -> Result<Self> {
        if !config.reference_dir.is_dir() {
            bail!(
                "Missing reference directory: {}",
                config.reference_dir.display()
            );
        }

        fs::create_dir_all(&config.output_dir)?;

        if config.output_dir.canonicalize()? == config.reference_dir.canonicalize()? {
            bail!("Reference and output directories can not be the same");
        }

        let test_scenes = config.only_test_scenes.clone().map_or_else(
            || TestScene::all().to_vec(),
            |mut scenes| {
                scenes.sort();
                scenes.dedup();
                scenes
            },
        );

        Ok(Self {
            test_scenes,
            config,
        })
    }

    fn run_comparisons(&self) -> Result<()> {
        let mut failing_scenes = Vec::new();

        for &scene in &self.test_scenes {
            let output_image_path = scene.append_filename(&self.config.output_dir);
            let reference_image_path = scene.append_filename(&self.config.reference_dir);

            if !reference_image_path.is_file() {
                impact_log::info!(
                    "Skipping {scene} test due to missing reference image at {}",
                    reference_image_path.display()
                );
                continue;
            }

            let outcome = testing::run_comparison(
                scene,
                &output_image_path,
                &reference_image_path,
                self.config.min_score_to_pass,
            )?;

            if outcome == ComparisonOutcome::Different {
                failing_scenes.push(scene.to_string());
            }
        }

        if failing_scenes.is_empty() {
            Ok(())
        } else {
            bail!(
                "Output images differed from their reference: {}",
                failing_scenes.join(", ")
            )
        }
    }
}

impl Application for SnapshotTester {
    fn on_engine_initialized(&self, engine: Arc<Engine>) -> Result<()> {
        if self.test_scenes.is_empty() {
            impact_log::info!("No scenes to test, exiting");
            return engine.execute_command(EngineCommand::Shutdown);
        }

        *ENGINE.write() = Some(engine.clone());

        let first_scene = self.test_scenes[0];

        // Setup first scene
        first_scene.prepare_settings(&engine)?;
        scripting::setup_scene(first_scene)
    }

    fn on_game_loop_iteration_completed(&self, engine: &Engine, iteration: u64) -> Result<()> {
        let iteration = iteration as usize;

        let rendered_scene = self.test_scenes[iteration];

        let output_image_path = rendered_scene.append_filename(&self.config.output_dir);

        // Capture and save screenshot for the scene that was just rendered
        engine.capture_screenshot(Some(&output_image_path))?;

        // Prepare for next scene
        engine.execute_command(EngineCommand::Scene(SceneCommand::Clear))?;
        rendered_scene.restore_settings(engine)?;

        if iteration + 1 == self.test_scenes.len() {
            // All scenes have been rendered
            self.run_comparisons()?;
            return engine.execute_command(EngineCommand::Shutdown);
        }

        let next_scene = self.test_scenes[iteration + 1];

        // Setup next scene
        next_scene.prepare_settings(engine)?;
        scripting::setup_scene(next_scene)
    }
}

impl AppConfig {
    /// Parses the configuration from the RON file at the given path and
    /// resolves any specified paths.
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        let mut config: Self = impact_io::parse_ron_file(file_path)?;
        if let Some(root_path) = file_path.parent() {
            config.resolve_paths(root_path);
        }
        Ok(config)
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.testing.resolve_paths(root_path);
        self.engine_config_path = root_path.join(&self.engine_config_path);
    }
}

impl TestingConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.output_dir = root_path.join(&self.output_dir);
        self.reference_dir = root_path.join(&self.reference_dir);
    }
}
