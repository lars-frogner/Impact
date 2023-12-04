//! Management of perspective cameras.

use crate::{
    geometry::{PerspectiveCamera, UpperExclusiveBounds},
    physics::{OrientationComp, PositionComp},
    rendering::fre,
    scene::{
        self, PerspectiveCameraComp, RenderResourcesDesynchronized, ScalingComp, SceneCamera,
        SceneGraph, SceneGraphCameraNodeComp, SceneGraphGroupNodeComp,
        SceneGraphModelInstanceNodeComp, SceneGraphParentNodeComp,
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

                if components.has_component_type::<ScalingComp>() {
                    bail!(
                        "Tried to add both camera and scaling component to the same entity\n\
                         (not allowed because the view transform is assumed to contain no scaling)"
                    )
                }

                desynchronized.set_yes();

                let mut scene_graph = scene_graph.write().unwrap();
            },
            components,
            |position: Option<&PositionComp>,
             orientation: Option<&OrientationComp>,
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

                let PositionComp {
                    origin_offset,
                    position,
                } = position.cloned().unwrap_or_default();
                let orientation = orientation.cloned().unwrap_or_default().0;

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
            ![
                SceneGraphGroupNodeComp,
                SceneGraphCameraNodeComp,
                SceneGraphModelInstanceNodeComp
            ]
        );
        Ok(())
    }
}
