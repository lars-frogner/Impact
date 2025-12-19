//! Setup and cleanup of cameras for new and removed entities.

use crate::{lock_order::OrderedRwLock, scene::Scene};
use anyhow::{Result, bail};
use impact_camera::{OrthographicCamera, PerspectiveCamera, setup};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use impact_geometry::ReferenceFrame;
use impact_math::bounds::UpperExclusiveBounds;
use impact_scene::{SceneGraphCameraNodeHandle, SceneGraphParentNodeHandle};
use parking_lot::RwLock;

/// Checks if the entity-to-be with the given components has the required
/// components for a camera, and if so, adds a node for the camera in the
/// [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a
/// [`SceneCamera`](impact_scene::camera::SceneCamera) into the `Scene` and adds
/// a [`SceneGraphCameraNodeHandle`] to the entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_camera_to_scene_for_new_entity(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    add_perspective_camera_to_scene_for_new_entity(scene, components)?;
    add_orthographic_camera_to_scene_for_new_entity(scene, components)
}

/// Checks if the entity-to-be with the given components has the required
/// components for a perspective camera, and if so, adds a node for the camera
/// in the [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a
/// [`SceneCamera`](impact_scene::camera::SceneCamera) into the `Scene` and adds
/// a [`SceneGraphCameraNodeHandle`] to the entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_perspective_camera_to_scene_for_new_entity(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();

            let mut camera_manager = scene.camera_manager().owrite();
            if camera_manager.has_active_camera() {
                bail!("Tried to add camera for entity while another entity still has one")
            }

            let mut scene_graph = scene.scene_graph().owrite();
        },
        components,
        |frame: Option<&ReferenceFrame>,
         camera_props: &setup::PerspectiveCamera,
         parent: Option<&SceneGraphParentNodeHandle>|
         -> SceneGraphCameraNodeHandle {
            let camera = PerspectiveCamera::new(
                camera_manager.camera_context().aspect_ratio,
                camera_props.vertical_field_of_view(),
                UpperExclusiveBounds::new(
                    camera_props.near_distance(),
                    camera_props.far_distance(),
                ),
            );

            let camera_to_parent_transform = frame
                .copied()
                .unwrap_or_default()
                .create_transform_to_parent_space();

            let parent_node_id =
                parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

            let node_id =
                scene_graph.create_camera_node(parent_node_id, camera_to_parent_transform);

            camera_manager.set_active_camera(camera, node_id);

            SceneGraphCameraNodeHandle::new(node_id)
        },
        ![SceneGraphCameraNodeHandle]
    );
    Ok(())
}

/// Checks if the entity-to-be with the given components has the required
/// components for an orthographic camera, and if so, adds a node for the camera
/// in the [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a
/// [`SceneCamera`](impact_scene::camera::SceneCamera) into the `Scene` and adds
/// a [`SceneGraphCameraNodeHandle`] to the entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_orthographic_camera_to_scene_for_new_entity(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();

            let mut camera_manager = scene.camera_manager().owrite();
            if camera_manager.has_active_camera() {
                bail!("Tried to add camera for entity while another entity still has one")
            }

            let mut scene_graph = scene.scene_graph().owrite();
        },
        components,
        |frame: Option<&ReferenceFrame>,
         camera_props: &setup::OrthographicCamera,
         parent: Option<&SceneGraphParentNodeHandle>|
         -> SceneGraphCameraNodeHandle {
            let camera = OrthographicCamera::new(
                camera_manager.camera_context().aspect_ratio,
                camera_props.vertical_field_of_view(),
                UpperExclusiveBounds::new(
                    camera_props.near_distance(),
                    camera_props.far_distance(),
                ),
            );

            let camera_to_parent_transform = frame
                .copied()
                .unwrap_or_default()
                .create_transform_to_parent_space();

            let parent_node_id =
                parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

            let node_id =
                scene_graph.create_camera_node(parent_node_id, camera_to_parent_transform);

            camera_manager.set_active_camera(camera, node_id);

            SceneGraphCameraNodeHandle::new(node_id)
        },
        ![SceneGraphCameraNodeHandle]
    );
    Ok(())
}

/// Checks if the given entity has a [`SceneGraphCameraNodeHandle`], and if so,
/// removes the corresponding camera node from the
/// [`SceneGraph`](impact_scene::graph::SceneGraph) and clears the active camera
/// if appropriate.
pub fn remove_camera_from_scene_for_removed_entity(
    scene: &RwLock<Scene>,
    entity: &EntityEntry<'_>,
) {
    if let Some(node) = entity.get_component::<SceneGraphCameraNodeHandle>() {
        let scene = scene.oread();
        let mut camera_manager = scene.camera_manager().owrite();
        let mut scene_graph = scene.scene_graph().owrite();
        let node_id = node.access().id;
        scene_graph.remove_camera_node(node_id);
        if camera_manager.active_camera_has_node(node_id) {
            camera_manager.clear_active_camera();
        }
    }
}
