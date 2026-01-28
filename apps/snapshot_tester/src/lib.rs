//! Snapshot tester for the Impact game engine.

pub mod interface;
pub mod testing;

pub use impact::{self, roc_integration};

#[cfg(feature = "roc_codegen")]
pub use impact::component::gather_roc_type_ids_for_all_components;

use anyhow::{Result, bail};
use impact::{
    engine::Engine,
    impact_io,
    runtime::{RuntimeConfig, headless::HeadlessConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use testing::{ComparisonOutcome, TestScene};

#[derive(Clone, Debug)]
pub struct App {
    test_scenes: Vec<TestScene>,
    config: TestingConfig,
    engine: Option<Arc<Engine>>,
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

impl App {
    pub(crate) fn new(config: TestingConfig) -> Result<Self> {
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
            engine: None,
        })
    }

    pub(crate) fn engine(&self) -> &Engine {
        self.engine
            .as_ref()
            .expect("Tried to use engine before initialization")
    }

    fn run_comparisons(&self) -> Result<()> {
        let mut failing_scenes = Vec::new();

        for (frame_number, &scene) in self.test_scenes.iter().enumerate() {
            let output_image_path = output_image_path(&self.config.output_dir, frame_number);
            let reference_image_path = scene.append_filename(&self.config.reference_dir);

            let renamed_output_image_path = scene.append_filename(&self.config.output_dir);
            fs::rename(&output_image_path, &renamed_output_image_path)?;

            if !reference_image_path.is_file() {
                log::info!(
                    "Skipping {scene} test due to missing reference image at {}",
                    reference_image_path.display()
                );
                continue;
            }

            let outcome = testing::run_comparison(
                scene,
                &renamed_output_image_path,
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

fn output_image_path(dir: &Path, frame_number: usize) -> PathBuf {
    dir.join(format!("screenshot_{frame_number}.png"))
}
