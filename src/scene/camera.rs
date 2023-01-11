//! Management of cameras.

use crate::{
    geometry::{Camera, PerspectiveCamera},
    num::Float,
};
use anyhow::{anyhow, Result};
use impact_utils::stringhash64_newtype;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
};

stringhash64_newtype!(
    /// Identifier for specific cameras.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] CameraID
);

/// Repository where [`Camera`]s are stored under a
/// unique [`CameraID`].
#[derive(Debug, Default)]
pub struct CameraRepository<F: Float> {
    /// Cameras using perspective transformations.
    perspective_cameras: HashMap<CameraID, PerspectiveCamera<F>>,
}

impl<F: Float> CameraRepository<F> {
    /// Creates a new empty camera repository.
    pub fn new() -> Self {
        Self {
            perspective_cameras: HashMap::new(),
        }
    }

    /// Returns a trait object representing the [`Camera`] with
    /// the given ID, or [`None`] if the camera is not present.
    pub fn get_camera(&self, camera_id: CameraID) -> Option<&dyn Camera<F>> {
        Some(self.perspective_cameras.get(&camera_id).unwrap())
    }

    /// Returns a mutable trait object representing the [`Camera`]
    /// with the given ID, or [`None`] if the camera is not present.
    pub fn get_camera_mut(&mut self, camera_id: CameraID) -> Option<&mut dyn Camera<F>> {
        Some(self.perspective_cameras.get_mut(&camera_id).unwrap())
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`PerspectiveCamera`]s.
    pub fn perspective_cameras(&self) -> &HashMap<CameraID, PerspectiveCamera<F>> {
        &self.perspective_cameras
    }

    /// Includes the given [`PerspectiveCamera`] in the repository
    /// under the given ID.
    ///
    /// # Errors
    /// Returns an error if a camera with the given ID already
    /// exists. The repository will remain unchanged.
    pub fn add_perspective_camera(
        &mut self,
        camera_id: CameraID,
        camera: PerspectiveCamera<F>,
    ) -> Result<()> {
        match self.perspective_cameras.entry(camera_id) {
            Entry::Vacant(entry) => {
                entry.insert(camera);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Camera {} already present in repository",
                camera_id
            )),
        }
    }

    /// Updates all cameras to have the given aspect ratio.
    pub fn set_aspect_ratios(&mut self, aspect_ratio: F) {
        for camera in self.perspective_cameras.values_mut() {
            camera.set_aspect_ratio(aspect_ratio);
        }
    }
}
