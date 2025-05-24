//! Management of camera-related components for entities.

use crate::{
    camera::{
        OrthographicCamera, PerspectiveCamera, SceneCamera,
        components::{OrthographicCameraComp, PerspectiveCameraComp},
    },
    gpu::rendering::RenderingSystem,
    physics::motion::components::ReferenceFrameComp,
    scene::{
        RenderResourcesDesynchronized, SceneGraph,
        components::{SceneGraphCameraNodeComp, SceneGraphParentNodeComp},
    },
    util::bounds::UpperExclusiveBounds,
    window::Window,
};
use anyhow::{Result, bail};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has the required
/// components for a camera, and if so, adds a node for the camera in the
/// given [`SceneGraph`], inserts a [`SceneCamera`] into the given
/// `scene_camera` variable and adds a [`SceneGraphCameraNodeComp`] to the
/// entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_camera_to_scene_for_new_entity(
    window: &Window,
    renderer: &RwLock<RenderingSystem>,
    scene_graph: &RwLock<SceneGraph<f32>>,
    scene_camera: &RwLock<Option<SceneCamera<f32>>>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) -> Result<()> {
    add_perspective_camera_to_scene_for_new_entity(
        window,
        renderer,
        scene_graph,
        scene_camera,
        components,
        desynchronized,
    )?;
    add_orthographic_camera_to_scene_for_new_entity(
        window,
        renderer,
        scene_graph,
        scene_camera,
        components,
        desynchronized,
    )
}

/// Checks if the entity-to-be with the given components has the required
/// components for a perspective camera, and if so, adds a node for the camera
/// in the given [`SceneGraph`], inserts a [`SceneCamera`] into the given
/// `scene_camera` variable and adds a [`SceneGraphCameraNodeComp`] to the
/// entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_perspective_camera_to_scene_for_new_entity(
    window: &Window,
    renderer: &RwLock<RenderingSystem>,
    scene_graph: &RwLock<SceneGraph<f32>>,
    scene_camera: &RwLock<Option<SceneCamera<f32>>>,
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

            let renderer = renderer.read().unwrap();
            let postprocessor = renderer.postprocessor().read().unwrap();
            let mut scene_graph = scene_graph.write().unwrap();
        },
        components,
        |frame: Option<&ReferenceFrameComp>,
         camera_comp: &PerspectiveCameraComp,
         parent: Option<&SceneGraphParentNodeComp>|
         -> SceneGraphCameraNodeComp {
            let camera = PerspectiveCamera::<f32>::new(
                window.aspect_ratio(),
                camera_comp.vertical_field_of_view(),
                UpperExclusiveBounds::new(camera_comp.near_distance(), camera_comp.far_distance()),
            );

            let mut camera_to_parent_transform = frame
                .cloned()
                .unwrap_or_default()
                .create_transform_to_parent_space();

            if camera_to_parent_transform.scaling() != 1.0 {
                log::warn!(
                    "Added camera component to an entity with non-unity scaling:\n\
                     The scaling will be ignored since the view transform is assumed to contain no scaling"
                );
                camera_to_parent_transform.set_scaling(1.0);
            }

            let parent_node_id =
                parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

            let node_id =
                scene_graph.create_camera_node(parent_node_id, camera_to_parent_transform);

            let jittering_enabled = postprocessor.temporal_anti_aliasing_enabled();
            *scene_camera = Some(SceneCamera::new(camera, node_id, jittering_enabled));

            SceneGraphCameraNodeComp::new(node_id)
        },
        ![SceneGraphCameraNodeComp]
    );
    Ok(())
}

/// Checks if the entity-to-be with the given components has the required
/// components for an orthographic camera, and if so, adds a node for the camera
/// in the given [`SceneGraph`], inserts a [`SceneCamera`] into the given
/// `scene_camera` variable and adds a [`SceneGraphCameraNodeComp`] to the
/// entity.
///
/// # Errors
/// Returns an error if the content of `scene_camera` is not [`None`], meaning
/// that the scene already has a camera.
pub fn add_orthographic_camera_to_scene_for_new_entity(
    window: &Window,
    renderer: &RwLock<RenderingSystem>,
    scene_graph: &RwLock<SceneGraph<f32>>,
    scene_camera: &RwLock<Option<SceneCamera<f32>>>,
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

            let renderer = renderer.read().unwrap();
            let postprocessor = renderer.postprocessor().read().unwrap();
            let mut scene_graph = scene_graph.write().unwrap();
        },
        components,
        |frame: Option<&ReferenceFrameComp>,
         camera_comp: &OrthographicCameraComp,
         parent: Option<&SceneGraphParentNodeComp>|
         -> SceneGraphCameraNodeComp {
            let camera = OrthographicCamera::<f32>::new(
                window.aspect_ratio(),
                camera_comp.vertical_field_of_view(),
                UpperExclusiveBounds::new(camera_comp.near_distance(), camera_comp.far_distance()),
            );

            let mut camera_to_parent_transform = frame
                .cloned()
                .unwrap_or_default()
                .create_transform_to_parent_space();

            if camera_to_parent_transform.scaling() != 1.0 {
                log::warn!(
                    "Added camera component to an entity with non-unity scaling:\n\
                     The scaling will be ignored since the view transform is assumed to contain no scaling"
                );
                camera_to_parent_transform.set_scaling(1.0);
            }

            let parent_node_id =
                parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

            let node_id =
                scene_graph.create_camera_node(parent_node_id, camera_to_parent_transform);

            let jittering_enabled = postprocessor.temporal_anti_aliasing_enabled();
            *scene_camera = Some(SceneCamera::new(camera, node_id, jittering_enabled));

            SceneGraphCameraNodeComp::new(node_id)
        },
        ![SceneGraphCameraNodeComp]
    );
    Ok(())
}

/// Checks if the given entity has a [`SceneGraphCameraNodeComp`], and if so,
/// removes the corresponding camera node from the given [`SceneGraph`] and sets
/// the content of `scene_camera` to [`None`].
pub fn remove_camera_from_scene_for_removed_entity(
    scene_graph: &RwLock<SceneGraph<f32>>,
    scene_camera: &RwLock<Option<SceneCamera<f32>>>,
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
