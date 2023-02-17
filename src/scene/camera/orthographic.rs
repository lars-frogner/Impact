//! Management of orthographic cameras.

use crate::{
    geometry::{OrthographicCamera, UpperExclusiveBounds},
    physics::{OrientationComp, PositionComp},
    rendering::fre,
    scene::{
        self, OrthographicCameraComp, RenderResourcesDesynchronized, ScalingComp, SceneCamera,
        SceneGraph, SceneGraphCameraNodeComp,
    },
    window::Window,
};
use anyhow::{bail, Result};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use nalgebra::{Point3, UnitQuaternion};
use std::sync::RwLock;

impl OrthographicCamera<fre> {
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
                let root_node_id = scene_graph.root_node_id();
            },
            components,
            |position: Option<&PositionComp>,
             orientation: Option<&OrientationComp>,
             camera_comp: &OrthographicCameraComp|
             -> SceneGraphCameraNodeComp {
                let camera = Self::new(
                    window.aspect_ratio(),
                    camera_comp.vertical_field_of_view(),
                    UpperExclusiveBounds::new(
                        camera_comp.near_distance(),
                        camera_comp.far_distance(),
                    ),
                );

                let position = position.map_or_else(Point3::origin, |position| position.0.cast());
                let orientation = orientation
                    .map_or_else(UnitQuaternion::identity, |orientation| orientation.0.cast());

                let camera_to_world_transform =
                    scene::create_model_to_world_transform(position, orientation, 1.0);

                let node_id =
                    scene_graph.create_camera_node(root_node_id, camera_to_world_transform);

                *scene_camera = Some(SceneCamera::new(camera, node_id));

                SceneGraphCameraNodeComp::new(node_id)
            }
        );
        Ok(())
    }
}
