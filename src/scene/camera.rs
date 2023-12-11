//! Management of cameras.

mod components;
mod orthographic;
mod perspective;

pub use components::{register_camera_components, OrthographicCameraComp, PerspectiveCameraComp};

use crate::{
    geometry::Camera,
    num::Float,
    rendering::fre,
    scene::{CameraNodeID, RenderResourcesDesynchronized, SceneGraph, SceneGraphCameraNodeComp},
};
use impact_ecs::world::EntityEntry;
use nalgebra::Similarity3;
use std::{fmt::Debug, sync::RwLock};

/// Represents a [`Camera`] that has a camera node in a [`SceneGraph`].
#[derive(Debug)]
pub struct SceneCamera<F: Float> {
    camera: Box<dyn Camera<F>>,
    view_transform: Similarity3<F>,
    scene_graph_node_id: CameraNodeID,
}

impl<F: Float> SceneCamera<F> {
    /// Creates a new [`SceneCamera`] representing the given [`Camera`] in the
    /// camera node with the given ID in the [`SceneGraph`].
    pub fn new(camera: impl Camera<F>, scene_graph_node_id: CameraNodeID) -> Self {
        Self {
            camera: Box::new(camera),
            view_transform: Similarity3::identity(),
            scene_graph_node_id,
        }
    }

    /// Returns a reference to the underlying [`Camera`].
    pub fn camera(&self) -> &dyn Camera<F> {
        self.camera.as_ref()
    }

    /// Returns a reference to the camera's view transform.
    pub fn view_transform(&self) -> &Similarity3<F> {
        &self.view_transform
    }

    /// Returns the ID of the [`CameraNode`](crate::scene::graph::CameraNode)
    /// for the camera in the [`SceneGraph`](crate::scene::SceneGraph).
    pub fn scene_graph_node_id(&self) -> CameraNodeID {
        self.scene_graph_node_id
    }

    /// Sets the transform from world space to camera space.
    pub fn set_view_transform(&mut self, view_transform: Similarity3<F>) {
        self.view_transform = view_transform;
    }

    /// Sets the ratio of width to height of the camera's view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: F) {
        self.camera.set_aspect_ratio(aspect_ratio);
    }
}

/// Checks if the given entity has a [`SceneGraphCameraNodeComp`], and if
/// so, removes the corresponding camera node from the given [`SceneGraph`]
/// and sets the content of `scene_camera` to [`None`].
pub fn remove_camera_from_scene(
    scene_graph: &RwLock<SceneGraph<fre>>,
    scene_camera: &RwLock<Option<SceneCamera<fre>>>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    if let Some(node) = entity.get_component::<SceneGraphCameraNodeComp>() {
        let node_id = node.access().id;
        scene_graph.write().unwrap().remove_camera_node(node_id);
        scene_camera.write().unwrap().take();
        desynchronized.set_yes();
    }
}
