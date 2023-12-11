//! Management of perspective cameras.

use crate::{
    geometry::{PerspectiveCamera, UpperExclusiveBounds},
    physics::ReferenceFrameComp,
    rendering::fre,
    scene::{
        self, PerspectiveCameraComp, RenderResourcesDesynchronized, SceneCamera, SceneGraph,
        SceneGraphCameraNodeComp, SceneGraphParentNodeComp,
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
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        setup!(
            {
                let mut scene_camera = scene_camera.write().unwrap();
                if scene_camera.is_some() {
                    bail!("Tried to add camera for entity while another entity still has one")
                }

                desynchronized.set_yes();

                let mut scene_graph = scene_graph.write().unwrap();
            },
            components,
            |frame: Option<&ReferenceFrameComp>,
             camera_comp: &PerspectiveCameraComp,
             parent: Option<&SceneGraphParentNodeComp>|
             -> SceneGraphCameraNodeComp {
                let camera = Self::new(
                    window.aspect_ratio(),
                    camera_comp.vertical_field_of_view(),
                    UpperExclusiveBounds::new(
                        camera_comp.near_distance(),
                        camera_comp.far_distance(),
                    ),
                );

                let ReferenceFrameComp {
                    origin_offset,
                    position,
                    orientation,
                    scaling,
                } = frame.cloned().unwrap_or_default();

                if scaling != 1.0 {
                    log::warn!(
                        "Added camera component to an entity with non-unity scaling:\n\
                         The scaling will be ignored since the view transform is assumed to contain no scaling"
                    )
                }

                let camera_to_parent_transform = scene::create_child_to_parent_transform(
                    origin_offset.cast(),
                    position.cast(),
                    orientation.cast(),
                    1.0,
                );

                let parent_node_id =
                    parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

                let node_id =
                    scene_graph.create_camera_node(parent_node_id, camera_to_parent_transform);

                *scene_camera = Some(SceneCamera::new(camera, node_id));

                SceneGraphCameraNodeComp::new(node_id)
            },
            ![SceneGraphCameraNodeComp]
        );
        Ok(())
    }
}
