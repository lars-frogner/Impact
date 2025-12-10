//! Screen capture.

use crate::{lock_order::OrderedRwLock, rendering::RenderingSystem};
use anyhow::Result;
use impact_geometry::projection::CubemapFace;
use impact_light::MAX_SHADOW_MAP_CASCADES;
use impact_rendering::{resource::BasicGPUResources, surface::RenderingSurface};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// Helper for capturing screenshots and related textures.
#[derive(Debug)]
pub struct ScreenCapturer {
    screenshot_save_requested: AtomicBool,
    omnidirectional_light_shadow_map_save_requested: AtomicBool,
    unidirectional_light_shadow_map_save_requested: AtomicBool,
    config: ScreenCaptureConfig,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenCaptureConfig {
    pub output_dir: Option<PathBuf>,
    pub tagging: CaptureTagging,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureTagging {
    #[default]
    Timestamp,
    FrameNumber,
}

impl ScreenCapturer {
    /// Creates a new screen capturer.
    ///
    /// # Panics
    /// When a screenshot is captured, a panic will occur if the width times the
    /// number of bytes per pixel is not a multiple of 256.
    pub fn new(config: ScreenCaptureConfig) -> Self {
        Self {
            screenshot_save_requested: AtomicBool::new(false),
            omnidirectional_light_shadow_map_save_requested: AtomicBool::new(false),
            unidirectional_light_shadow_map_save_requested: AtomicBool::new(false),
            config,
        }
    }

    /// Schedule a screenshot capture for the next
    /// [`Self::save_screenshot_if_requested`] call.
    pub fn request_screenshot_save(&self) {
        self.screenshot_save_requested
            .store(true, Ordering::Release);
    }

    /// Schedule a capture of the omnidirectional light shadow map texture for
    /// the next [`Self::save_omnidirectional_light_shadow_maps_if_requested`]
    /// call.
    pub fn request_omnidirectional_light_shadow_map_save(&self) {
        self.omnidirectional_light_shadow_map_save_requested
            .store(true, Ordering::Release);
    }

    /// Schedule a capture of the unidirectional light shadow map texture for
    /// the next [`Self::save_unidirectional_light_shadow_maps_if_requested`]
    /// call.
    pub fn request_unidirectional_light_shadow_map_save(&self) {
        self.unidirectional_light_shadow_map_save_requested
            .store(true, Ordering::Release);
    }

    /// Checks if a screenshot capture was scheduled with
    /// [`Self::request_screenshot_save`], and if so, captures a screenshot and
    /// saves it as a PNG image at the configured output path.
    pub fn save_screenshot_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
        frame_number: u64,
    ) -> Result<()> {
        if self
            .screenshot_save_requested
            .swap(false, Ordering::Acquire)
        {
            impact_log::info!("Saving screenshot of frame {frame_number}");

            let renderer = renderer.oread();

            let surface_texture = match renderer.rendering_surface() {
                RenderingSurface::Headless(surface) => surface.surface_texture(),
                #[cfg(feature = "window")]
                RenderingSurface::Window(_) => {
                    &renderer
                        .surface_texture_to_present
                        .as_ref()
                        .ok_or_else(|| {
                            anyhow::anyhow!("No unpresented surface to save as screenshot")
                        })?
                        .texture
                }
            };

            let output_path = self
                .config
                .build_output_path(|tag| format!("screenshot_{tag}.png"), frame_number);

            impact_texture::io::save_texture_as_png_file(
                renderer.graphics_device(),
                surface_texture,
                0,
                0,
                true,
                output_path,
            )?;
        }

        Ok(())
    }

    /// Checks if a omnidirectional light shadow map capture was scheduled with
    /// [`Self::request_omnidirectional_light_shadow_map_save`], and if so,
    /// captures the textures and saves them as timestamped PNG files at the
    /// configured output paths.
    pub fn save_omnidirectional_light_shadow_maps_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
        frame_number: u64,
    ) -> Result<()> {
        if self
            .omnidirectional_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            impact_log::info!("Saving omnidirectional light shadow maps for frame {frame_number}");

            let renderer = renderer.oread();
            let render_resource_manager = renderer.render_resource_manager().oread();

            if let Some(light_gpu_resources) = render_resource_manager.light() {
                for (light_idx, texture) in light_gpu_resources
                    .omnidirectional_light_shadow_map_manager()
                    .textures()
                    .iter()
                    .enumerate()
                {
                    for face in CubemapFace::all() {
                        let output_path = self.config.build_output_path(
                            |tag| {
                                format!("omnidirectional_light_{light_idx}_shadow_map_{face:?}_{tag}.png")
                            },
                            frame_number,
                        );

                        texture.save_face_as_png_file(
                            renderer.graphics_device(),
                            face,
                            output_path,
                        )?;
                    }
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    /// Checks if a unidirectional light shadow map capture was scheduled with
    /// [`Self::request_unidirectional_light_shadow_map_save`], and if so,
    /// captures the textures and saves them as timestamped PNG files at the
    /// configured output paths.
    pub fn save_unidirectional_light_shadow_maps_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
        frame_number: u64,
    ) -> Result<()> {
        if self
            .unidirectional_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            impact_log::info!("Saving unidirectional light shadow maps for frame {frame_number}");

            let renderer = renderer.oread();
            let render_resource_manager = renderer.render_resource_manager().oread();

            if let Some(light_gpu_resources) = render_resource_manager.light() {
                for (light_idx, texture) in light_gpu_resources
                    .unidirectional_light_shadow_map_manager()
                    .textures()
                    .iter()
                    .enumerate()
                {
                    for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                        let output_path = self.config.build_output_path(
                            |tag| {
                                format!("unidirectional_light_{light_idx}_shadow_map_{cascade_idx}_{tag}.png")
                            },
                            frame_number,
                        );

                        texture.save_cascade_as_png_file(
                            renderer.graphics_device(),
                            cascade_idx,
                            output_path,
                        )?;
                    }
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

impl Default for ScreenCapturer {
    fn default() -> Self {
        Self::new(ScreenCaptureConfig::default())
    }
}

impl ScreenCaptureConfig {
    fn build_output_path(
        &self,
        tag_filename: impl FnOnce(u128) -> String,
        frame_number: u64,
    ) -> PathBuf {
        let tag = match self.tagging {
            CaptureTagging::Timestamp => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            CaptureTagging::FrameNumber => u128::from(frame_number),
        };

        let filename = tag_filename(tag);

        if let Some(output_dir) = &self.output_dir {
            output_dir.join(filename)
        } else {
            PathBuf::from(filename)
        }
    }
}
