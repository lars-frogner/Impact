//! Setup and cleanup of cameras for new and removed entities.

use crate::{lock_order::OrderedRwLock, scene::Scene};
use anyhow::{Result, bail};
use impact_camera::{OrthographicCamera, PerspectiveCamera, setup};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use impact_geometry::ReferenceFrame;
use impact_math::UpperExclusiveBounds;
use impact_scene::{SceneGraphCameraNodeHandle, SceneGraphParentNodeHandle, camera::SceneCamera};
use parking_lot::RwLock;

/// Rendering related state needed for camera initialization.
#[derive(Clone, Debug)]
pub struct CameraRenderState {
    /// The aspect ratio of the rendering surface.
    pub aspect_ratio: f32,
    /// Whether the camera should be jittered.
    pub jittering_enabled: bool,
}

/// Checks if the entity-to-be with the given components has the required
/// components for a camera, and if so, adds a node for the camera in the
/// [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a [`SceneCamera`]
/// into the `Scene` and adds a [`SceneGraphCameraNodeHandle`] to the entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_camera_to_scene_for_new_entity(
    scene: &RwLock<Scene>,
    get_render_state: &mut impl FnMut() -> CameraRenderState,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    add_perspective_camera_to_scene_for_new_entity(scene, get_render_state, components)?;
    add_orthographic_camera_to_scene_for_new_entity(scene, get_render_state, components)
}

/// Checks if the entity-to-be with the given components has the required
/// components for a perspective camera, and if so, adds a node for the camera
/// in the [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a
/// [`SceneCamera`] into the `Scene` and adds a [`SceneGraphCameraNodeHandle`]
/// to the entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_perspective_camera_to_scene_for_new_entity(
    scene: &RwLock<Scene>,
    get_render_state: &mut impl FnMut() -> CameraRenderState,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();

            let mut scene_camera = scene.scene_camera().owrite();
            if scene_camera.is_some() {
                bail!("Tried to add camera for entity while another entity still has one")
            }

            let mut scene_graph = scene.scene_graph().owrite();
        },
        components,
        |frame: Option<&ReferenceFrame>,
         camera_props: &setup::PerspectiveCamera,
         parent: Option<&SceneGraphParentNodeHandle>|
         -> SceneGraphCameraNodeHandle {
            let render_state = get_render_state();

            let camera = PerspectiveCamera::<f32>::new(
                render_state.aspect_ratio,
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

            **scene_camera = Some(SceneCamera::new(
                camera,
                node_id,
                render_state.jittering_enabled,
            ));

            SceneGraphCameraNodeHandle::new(node_id)
        },
        ![SceneGraphCameraNodeHandle]
    );
    Ok(())
}

/// Checks if the entity-to-be with the given components has the required
/// components for an orthographic camera, and if so, adds a node for the camera
/// in the [`SceneGraph`](impact_scene::graph::SceneGraph), inserts a
/// [`SceneCamera`] into the `Scene` and adds a [`SceneGraphCameraNodeHandle`]
/// to the entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_orthographic_camera_to_scene_for_new_entity(
    scene: &RwLock<Scene>,
    get_render_state: &mut impl FnMut() -> CameraRenderState,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();

            let mut scene_camera = scene.scene_camera().owrite();
            if scene_camera.is_some() {
                bail!("Tried to add camera for entity while another entity still has one")
            }

            let mut scene_graph = scene.scene_graph().owrite();
        },
        components,
        |frame: Option<&ReferenceFrame>,
         camera_props: &setup::OrthographicCamera,
         parent: Option<&SceneGraphParentNodeHandle>|
         -> SceneGraphCameraNodeHandle {
            let render_state = get_render_state();

            let camera = OrthographicCamera::<f32>::new(
                render_state.aspect_ratio,
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

            **scene_camera = Some(SceneCamera::new(
                camera,
                node_id,
                render_state.jittering_enabled,
            ));

            SceneGraphCameraNodeHandle::new(node_id)
        },
        ![SceneGraphCameraNodeHandle]
    );
    Ok(())
}

/// Checks if the given entity has a [`SceneGraphCameraNodeHandle`], and if so,
/// removes the corresponding camera node from the
/// [`SceneGraph`](impact_scene::graph::SceneGraph) and sets the content of
/// `scene_camera` to [`None`].
pub fn remove_camera_from_scene_for_removed_entity(
    scene: &RwLock<Scene>,
    entity: &EntityEntry<'_>,
) {
    if let Some(node) = entity.get_component::<SceneGraphCameraNodeHandle>() {
        let scene = scene.oread();
        let mut scene_camera = scene.scene_camera().owrite();
        let mut scene_graph = scene.scene_graph().owrite();
        let node_id = node.access().id;
        scene_graph.remove_camera_node(node_id);
        scene_camera.take();
    }
}
