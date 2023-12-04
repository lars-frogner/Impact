//! Tasks representing ECS systems related to scenes.

use crate::{
    define_task,
    physics::SpatialConfigurationComp,
    rendering::RenderingTag,
    scene::{
        CameraNodeID, DirectionComp, GroupNodeID, LightDirection, ModelInstanceNodeID,
        OmnidirectionalLightComp, SceneGraphNodeComp, SceneGraphParentNodeComp,
        SyncSceneCameraViewTransform, UnidirectionalLightComp, UpdateSceneGroupToWorldTransforms,
    },
    world::World,
};
use impact_ecs::query;
use nalgebra::Similarity3;

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the model transform of
    /// each [`SceneGraph`](crate::scene::SceneGraph) node representing an
    /// entity that also has the [`SpatialConfigurationComp`] component so that
    /// the translational and rotational parts match the origin offset, position
    /// and orientation.
    [pub] SyncSceneObjectTransforms,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing scene graph node transforms"; {
            let ecs_world = world.ecs_world().read().unwrap();
            let scene = world.scene().read().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();

            query!(
                ecs_world, |node: &SceneGraphNodeComp<GroupNodeID>, spatial: &SpatialConfigurationComp| {
                    scene_graph.set_rotation_of_group_to_parent_transform(node.id, spatial.orientation.cast());
                    scene_graph.update_translation_of_group_to_parent_transform(
                        node.id,
                        spatial.origin_offset.cast(),
                        spatial.position.cast()
                    );
                }
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<ModelInstanceNodeID>, spatial: &SpatialConfigurationComp| {
                    scene_graph.set_rotation_of_model_to_parent_transform(node.id, spatial.orientation.cast());
                    scene_graph.update_translation_of_model_to_parent_transform(
                        node.id,
                        spatial.origin_offset.cast(),
                        spatial.position.cast()
                    );
                }
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<CameraNodeID>, spatial: &SpatialConfigurationComp| {
                    scene_graph.set_rotation_of_camera_to_parent_transform(node.id, spatial.orientation.cast());
                    scene_graph.update_translation_of_camera_to_parent_transform(
                        node.id,
                        spatial.origin_offset.cast(),
                        spatial.position.cast()
                    );
                }
            );

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the camera space position
    /// and direction of every applicable light source in the
    /// [`LightStorage`](crate::scene::LightStorage).
    [pub] SyncLightPositionsAndDirectionsInStorage,
    depends_on = [
        UpdateSceneGroupToWorldTransforms,
        SyncSceneCameraViewTransform
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing camera space positions and directions of lights in storage"; {
            let scene = world.scene().read().unwrap();

            let ecs_world = world.ecs_world().read().unwrap();
            let scene_graph = scene.scene_graph().read().unwrap();
            let mut light_storage = scene.light_storage().write().unwrap();

            let view_transform = scene.scene_camera()
                .read()
                .unwrap()
                .as_ref()
                .map_or_else(Similarity3::identity, |scene_camera| {
                    *scene_camera.view_transform()
                });

            query!(
                ecs_world,
                |omnidirectional_light: &OmnidirectionalLightComp,
                 spatial: &SpatialConfigurationComp| {
                    let light_id = omnidirectional_light.id;
                    light_storage
                        .omnidirectional_light_mut(light_id)
                        .set_camera_space_position(view_transform.transform_point(&spatial.position.cast()));
                },
                ![SceneGraphParentNodeComp]
            );

            query!(
                ecs_world,
                |omnidirectional_light: &OmnidirectionalLightComp,
                 spatial: &SpatialConfigurationComp,
                 parent: &SceneGraphParentNodeComp| {
                    let parent_group_node = scene_graph.group_nodes().node(parent.id);

                    let view_transform = view_transform * parent_group_node.group_to_root_transform();

                    let light_id = omnidirectional_light.id;
                    light_storage
                        .omnidirectional_light_mut(light_id)
                        .set_camera_space_position(view_transform.transform_point(&spatial.position.cast()));
                }
            );

            query!(
                ecs_world, |unidirectional_light: &UnidirectionalLightComp, direction: &DirectionComp| {
                    let light_id = unidirectional_light.id;
                    light_storage
                        .unidirectional_light_mut(light_id)
                        .set_camera_space_direction(LightDirection::new_unchecked(view_transform.transform_vector(&direction.0.cast())));
                },
                ![SceneGraphParentNodeComp]
            );

            query!(
                ecs_world, |unidirectional_light: &UnidirectionalLightComp, direction: &DirectionComp, parent: &SceneGraphParentNodeComp| {
                    let parent_group_node = scene_graph.group_nodes().node(parent.id);

                    let view_transform = view_transform * parent_group_node.group_to_root_transform();

                    let light_id = unidirectional_light.id;
                    light_storage
                        .unidirectional_light_mut(light_id)
                        .set_camera_space_direction(LightDirection::new_unchecked(view_transform.transform_vector(&direction.0.cast())));
                }
            );

            Ok(())
        })
    }
);
