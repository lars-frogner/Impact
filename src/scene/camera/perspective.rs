//! Management of perspective cameras.

use crate::{
    geometry::{PerspectiveCamera, UpperExclusiveBounds},
    physics::{OrientationComp, PositionComp},
    rendering::fre,
    scene::{
        self, PerspectiveCameraComp, RenderResourcesDesynchronized, SceneCamera, SceneGraph,
        SceneGraphCameraNodeComp,
    },
    window::Window,
};
use anyhow::{bail, Result};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use std::sync::RwLock;

impl PerspectiveCamera<fre> {
    /// Checks if the entity-to-be with the given components has the required
    /// components for this camera type, and if so, adds a node for the camera
    /// in the given [`SceneGraph`], inserts a [`SceneCamera`] into the given
    /// `scene_camera` variable and adds a [`SceneGraphCameraNodeComp`] to the
    /// entity.
    ///
    /// # Errors
    /// Returns an error if the content of `scene_camera` is not [`None`],
    /// meaning that the scene already has a camera.
    pub fn add_camera_to_scene_for_entity(
        window: &Window,
        scene_graph: &RwLock<SceneGraph<fre>>,
        scene_camera: &RwLock<Option<SceneCamera<fre>>>,
        components: &mut ArchetypeComponentStorage,
    ) -> Result<()> {
        setup!(
            {
                let mut scene_camera = scene_camera.write().unwrap();
                if scene_camera.is_some() {
                    bail!("Tried to add camera for entity while another entity still has one")
                }


                let mut scene_graph = scene_graph.write().unwrap();
                let root_node_id = scene_graph.root_node_id();
            },
            components,
            |position: &PositionComp,
             orientation: &OrientationComp,
             camera_comp: &PerspectiveCameraComp|
             -> SceneGraphCameraNodeComp {
                let camera = Self::new(
                    window.aspect_ratio(),
                    camera_comp.vertical_field_of_view(),
                    UpperExclusiveBounds::new(
                        camera_comp.near_distance(),
                        camera_comp.far_distance(),
                    ),
                );

                let camera_to_world_transform =
                    scene::model_to_world_transform_from_position_and_orientation(
                        position.0.cast(),
                        orientation.0.cast(),
                    );

                let node_id =
                    scene_graph.create_camera_node(root_node_id, camera_to_world_transform);

                *scene_camera = Some(SceneCamera::new(camera, node_id));

                SceneGraphCameraNodeComp::new(node_id)
            }
        );
        Ok(())
    }
}
