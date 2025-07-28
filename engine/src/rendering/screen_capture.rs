//! Screen capture.

use crate::rendering::RenderingSystem;
use anyhow::Result;
use impact_geometry::CubemapFace;
use impact_light::MAX_SHADOW_MAP_CASCADES;
use impact_rendering::{resource::BasicGPUResources, surface::RenderingSurface};
use parking_lot::RwLock;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// Helper for capturing screenshots and related textures.
#[derive(Debug)]
pub struct ScreenCapturer {
    screenshot_save_requested: AtomicBool,
    omnidirectional_light_shadow_map_save_requested: AtomicBool,
    unidirectional_light_shadow_map_save_requested: AtomicBool,
}

impl ScreenCapturer {
    /// Creates a new screen capturer.
    ///
    /// # Panics
    /// When a screenshot is captured, a panic will occur if the width times the
    /// number of bytes per pixel is not a multiple of 256.
    pub fn new() -> Self {
        Self {
            screenshot_save_requested: AtomicBool::new(false),
            omnidirectional_light_shadow_map_save_requested: AtomicBool::new(false),
            unidirectional_light_shadow_map_save_requested: AtomicBool::new(false),
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
    /// saves it as a PNG image to the specified output path, or, if not
    /// specified, as a timestamped PNG file in the current directory.
    pub fn save_screenshot_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
        output_path: Option<&Path>,
    ) -> Result<()> {
        if self
            .screenshot_save_requested
            .swap(false, Ordering::Acquire)
        {
            let renderer = renderer.read();

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

            let timestamped_filename = PathBuf::from(format!(
                "screenshot_{}.png",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ));

            let output_path = output_path.unwrap_or(timestamped_filename.as_path());

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
    /// captures the textures and saves them as timestamped PNG files in the
    /// current directory.
    pub fn save_omnidirectional_light_shadow_maps_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
    ) -> Result<()> {
        if self
            .omnidirectional_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            let renderer = renderer.read();

            let render_resource_manager = renderer.render_resource_manager().read();

            if let Some(light_buffer_manager) = render_resource_manager.get_light_buffer_manager() {
                for (light_idx, texture) in light_buffer_manager
                    .omnidirectional_light_shadow_map_manager()
                    .textures()
                    .iter()
                    .enumerate()
                {
                    for face in CubemapFace::all() {
                        texture.save_face_as_png_file(
                            renderer.graphics_device(),
                            face,
                            format!(
                                "omnidirectional_light_{}_shadow_map_{:?}_{}.png",
                                light_idx,
                                face,
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            ),
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
    /// captures the textures and saves them as timestamped PNG files in the
    /// current directory.
    pub fn save_unidirectional_light_shadow_maps_if_requested(
        &self,
        renderer: &RwLock<RenderingSystem>,
    ) -> Result<()> {
        if self
            .unidirectional_light_shadow_map_save_requested
            .swap(false, Ordering::Acquire)
        {
            let renderer = renderer.read();

            let render_resource_manager = renderer.render_resource_manager().read();

            if let Some(light_buffer_manager) = render_resource_manager.get_light_buffer_manager() {
                for (light_idx, texture) in light_buffer_manager
                    .unidirectional_light_shadow_map_manager()
                    .textures()
                    .iter()
                    .enumerate()
                {
                    for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                        texture.save_cascade_as_png_file(
                            renderer.graphics_device(),
                            cascade_idx,
                            format!(
                                "unidirectional_light_{}_shadow_map_{}_{}.png",
                                light_idx,
                                cascade_idx,
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            ),
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
        Self::new()
    }
}
