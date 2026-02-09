//! Setup and cleanup of cameras for new and removed entities.

use crate::{lock_order::OrderedRwLock, scene::Scene};
use anyhow::Result;
use impact_camera::{CameraID, HasCamera, OrthographicCamera, PerspectiveCamera, setup};
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_geometry::ReferenceFrame;
use impact_id::EntityID;
use impact_math::bounds::UpperExclusiveBounds;
use impact_scene::{ParentEntity, graph::SceneGroupID};
use parking_lot::RwLock;

/// Checks if the given entities have the required components for a camera, and
/// if so, adds a node for the camera in the
/// [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a
/// [`SceneCamera`](impact_scene::camera::SceneCamera) into the `Scene` and adds
/// a [`SceneGraphCameraNodeHandle`] to the entity.
pub fn add_camera_to_scene_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    add_perspective_camera_to_scene_for_new_entities(scene, entities)?;
    add_orthographic_camera_to_scene_for_new_entities(scene, entities)
}

/// Checks if the given entities have the required components for a perspective
/// camera, and if so, adds a node for the camera in the
/// [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a
/// [`SceneCamera`](impact_scene::camera::SceneCamera) into the `Scene` and adds
/// a [`SceneGraphCameraNodeHandle`] to the entity.
pub fn add_perspective_camera_to_scene_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let mut camera_manager = scene.camera_manager().owrite();
            let mut scene_graph = scene.scene_graph().owrite();
        },
        entities,
        |entity_id: EntityID,
         frame: Option<&ReferenceFrame>,
         camera_props: &setup::PerspectiveCamera,
         parent: Option<&ParentEntity>|
         -> Result<HasCamera> {
            let frame = frame.copied().unwrap_or_default();

            let camera = PerspectiveCamera::new(
                camera_manager.camera_context().aspect_ratio,
                camera_props.vertical_field_of_view(),
                UpperExclusiveBounds::new(
                    camera_props.near_distance(),
                    camera_props.far_distance(),
                ),
            );

            let camera_to_parent_transform = frame.create_transform_to_parent_space();

            let camera_id = CameraID::from_entity_id(entity_id);
            let parent_group_id = parent.map_or_else(
                || scene_graph.root_node_id(),
                |parent| SceneGroupID::from_entity_id(parent.0),
            );

            scene_graph.create_camera_node(
                parent_group_id,
                camera_id,
                camera_to_parent_transform.compact(),
            )?;

            camera_manager.add_active_camera(camera, camera_id);

            Ok(HasCamera)
        },
        ![HasCamera]
    )
}

/// Checks if the given entities have the required components for an
/// orthographic camera, and if so, adds a node for the camera in the
/// [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a
/// [`SceneCamera`](impact_scene::camera::SceneCamera) into the `Scene` and adds
/// a [`SceneGraphCameraNodeHandle`] to the entity.
pub fn add_orthographic_camera_to_scene_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let mut camera_manager = scene.camera_manager().owrite();
            let mut scene_graph = scene.scene_graph().owrite();
        },
        entities,
        |entity_id: EntityID,
         frame: Option<&ReferenceFrame>,
         camera_props: &setup::OrthographicCamera,
         parent: Option<&ParentEntity>|
         -> Result<HasCamera> {
            let frame = frame.copied().unwrap_or_default();

            let camera = OrthographicCamera::new(
                camera_manager.camera_context().aspect_ratio,
                camera_props.vertical_field_of_view(),
                UpperExclusiveBounds::new(
                    camera_props.near_distance(),
                    camera_props.far_distance(),
                ),
            );

            let camera_to_parent_transform = frame.create_transform_to_parent_space();

            let camera_id = CameraID::from_entity_id(entity_id);
            let parent_group_id = parent.map_or_else(
                || scene_graph.root_node_id(),
                |parent| SceneGroupID::from_entity_id(parent.0),
            );

            scene_graph.create_camera_node(
                parent_group_id,
                camera_id,
                camera_to_parent_transform.compact(),
            )?;

            camera_manager.add_active_camera(camera, camera_id);

            Ok(HasCamera)
        },
        ![HasCamera]
    )
}

/// Checks if the given entity has a [`SceneGraphCameraNodeHandle`], and if so,
/// removes the corresponding camera node from the
/// [`SceneGraph`](impact_scene::graph::SceneGraph) and clears the active camera
/// if appropriate.
pub fn remove_camera_from_scene_for_removed_entity(
    scene: &RwLock<Scene>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    if entity.has_component::<HasCamera>() {
        let scene = scene.oread();
        let mut camera_manager = scene.camera_manager().owrite();
        let mut scene_graph = scene.scene_graph().owrite();
        let camera_id = CameraID::from_entity_id(entity_id);
        scene_graph.remove_camera_node(camera_id);
        if camera_manager.active_camera_has_id(camera_id) {
            camera_manager.clear_active_camera();
        }
    }
}
