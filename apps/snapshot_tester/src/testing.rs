//! Snapshot testing.

use anyhow::Result;
use impact::{
    command::ToActiveState,
    engine::Engine,
    gpu::rendering::{
        command::RenderingCommand,
        postprocessing::command::{PostprocessingCommand, ToToneMappingMethod},
    },
    impact_rendering::postprocessing::capturing::dynamic_range_compression::ToneMappingMethod,
    roc_integration::roc,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[roc(parents = "Test")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TestScene {
    AmbientLight,
    OmnidirectionalLight,
    UnidirectionalLight,
    ShadowableOmnidirectionalLight,
    ShadowableUnidirectionalLight,
    ShadowCubeMapping,
    SoftShadowCubeMapping,
    CascadedShadowMapping,
    SoftCascadedShadowMapping,
    AmbientOcclusion,
    Bloom,
    ACESToneMapping,
    KhronosPBRNeutralToneMapping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOutcome {
    Equal,
    Different,
}

impl TestScene {
    pub const fn all() -> [Self; 13] {
        [
            Self::AmbientLight,
            Self::OmnidirectionalLight,
            Self::UnidirectionalLight,
            Self::ShadowableOmnidirectionalLight,
            Self::ShadowableUnidirectionalLight,
            Self::ShadowCubeMapping,
            Self::SoftShadowCubeMapping,
            Self::CascadedShadowMapping,
            Self::SoftCascadedShadowMapping,
            Self::AmbientOcclusion,
            Self::Bloom,
            Self::ACESToneMapping,
            Self::KhronosPBRNeutralToneMapping,
        ]
    }

    pub fn append_filename(&self, root: &Path) -> PathBuf {
        root.join(format!("{self:?}")).with_extension("png")
    }

    pub fn prepare_settings(&self, engine: &Engine) -> Result<()> {
        match self {
            Self::AmbientLight
            | Self::OmnidirectionalLight
            | Self::UnidirectionalLight
            | Self::ShadowableOmnidirectionalLight
            | Self::ShadowableUnidirectionalLight => Ok(()),
            Self::ShadowCubeMapping
            | Self::SoftShadowCubeMapping
            | Self::CascadedShadowMapping
            | Self::SoftCascadedShadowMapping => engine.execute_rendering_command(
                RenderingCommand::SetShadowMapping(ToActiveState::Enabled),
            ),
            Self::AmbientOcclusion => engine.execute_rendering_postprocessing_command(
                PostprocessingCommand::SetAmbientOcclusion(ToActiveState::Enabled),
            ),
            Self::Bloom => engine.execute_rendering_postprocessing_command(
                PostprocessingCommand::SetBloom(ToActiveState::Enabled),
            ),
            Self::ACESToneMapping => engine.execute_rendering_postprocessing_command(
                PostprocessingCommand::SetToneMappingMethod(ToToneMappingMethod::Specific(
                    ToneMappingMethod::ACES,
                )),
            ),
            Self::KhronosPBRNeutralToneMapping => engine.execute_rendering_postprocessing_command(
                PostprocessingCommand::SetToneMappingMethod(ToToneMappingMethod::Specific(
                    ToneMappingMethod::KhronosPBRNeutral,
                )),
            ),
        }
    }

    pub fn restore_settings(&self, engine: &Engine) -> Result<()> {
        match self {
            Self::AmbientLight
            | Self::OmnidirectionalLight
            | Self::UnidirectionalLight
            | Self::ShadowableOmnidirectionalLight
            | Self::ShadowableUnidirectionalLight => Ok(()),
            Self::ShadowCubeMapping
            | Self::SoftShadowCubeMapping
            | Self::CascadedShadowMapping
            | Self::SoftCascadedShadowMapping => engine.execute_rendering_command(
                RenderingCommand::SetShadowMapping(ToActiveState::Disabled),
            ),
            Self::AmbientOcclusion => engine.execute_rendering_postprocessing_command(
                PostprocessingCommand::SetAmbientOcclusion(ToActiveState::Disabled),
            ),
            Self::Bloom => engine.execute_rendering_postprocessing_command(
                PostprocessingCommand::SetBloom(ToActiveState::Disabled),
            ),
            Self::ACESToneMapping | Self::KhronosPBRNeutralToneMapping => engine
                .execute_rendering_postprocessing_command(
                    PostprocessingCommand::SetToneMappingMethod(ToToneMappingMethod::Specific(
                        ToneMappingMethod::None,
                    )),
                ),
        }
    }
}

impl fmt::Display for TestScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub fn run_comparison(
    scene: TestScene,
    output_image_path: &Path,
    reference_image_path: &Path,
    min_score_to_pass: f64,
) -> Result<ComparisonOutcome> {
    let output_image = image::open(output_image_path)?.into_rgb8();
    let reference_image = image::open(reference_image_path)?.into_rgb8();

    let result = image_compare::rgb_hybrid_compare(&output_image, &reference_image)?;

    if result.score >= min_score_to_pass {
        impact_log::info!(
            "Passed {scene} similarity test ({score} >= {min_score_to_pass})",
            score = result.score,
        );
        Ok(ComparisonOutcome::Equal)
    } else {
        let diff_image_path = add_postfix_to_file_name(output_image_path, "_diff");

        impact_log::error!(
            "Failed {scene} similarity test ({score} < {min_score_to_pass}), saving diff image at {diff_path}",
            score = result.score,
            diff_path = diff_image_path.display()
        );

        result.image.to_color_map().save(diff_image_path)?;

        Ok(ComparisonOutcome::Different)
    }
}

fn add_postfix_to_file_name(file_path: &Path, postfix: &str) -> PathBuf {
    let file_stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("File path should have file name");

    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .expect("File path should have file extension");

    file_path.with_file_name(format!("{file_stem}{postfix}.{extension}"))
}
