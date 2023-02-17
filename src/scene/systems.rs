//! Tasks representing ECS systems related to scenes.

use crate::{
    define_task,
    physics::{AdvanceOrientations, AdvancePositions, OrientationComp, PositionComp, Static},
    rendering::RenderingTag,
    scene::{
        CameraNodeID, DirectionComp, DirectionalLightComp, GroupNodeID, LightDirection,
        ModelInstanceNodeID, PointLightComp, SceneGraphNodeComp, SyncSceneCameraViewTransform,
    },
    world::World,
};
use impact_ecs::query;
use nalgebra::{Similarity3, Translation3};

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the model transform
    /// of each [`SceneGraph`](crate::scene::SceneGraph) node representing
    /// an entity that also has the [`PositionComp`] component so that the
    /// translational part matches the position.
    [pub] SyncSceneObjectTransformsWithPositions,
    depends_on = [AdvancePositions],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing scene graph node transforms with positions"; {
            let ecs_world = world.ecs_world().read().unwrap();
            let scene = world.scene().read().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();

            query!(
                ecs_world, |node: &SceneGraphNodeComp<GroupNodeID>, position: &PositionComp| {
                    scene_graph.set_translation_of_group_to_parent_transform(node.id, Translation3::from(position.0.cast()));
                },
                ![Static]
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<ModelInstanceNodeID>, position: &PositionComp| {
                    scene_graph.set_translation_of_model_to_parent_transform(node.id, Translation3::from(position.0.cast()));
                },
                ![Static]
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<CameraNodeID>, position: &PositionComp| {
                    scene_graph.set_translation_of_camera_to_parent_transform(node.id, Translation3::from(position.0.cast()));
                },
                ![Static]
            );

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the model transform
    /// of each [`SceneGraph`](crate::scene::SceneGraph) node representing
    /// an entity that also has the [`PositionComp`] component so that the
    /// rotational part matches the orientation.
    [pub] SyncSceneObjectTransformsWithOrientations,
    depends_on = [AdvanceOrientations],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing scene graph node transforms with orientations"; {
            let ecs_world = world.ecs_world().read().unwrap();
            let scene = world.scene().read().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();

            query!(
                ecs_world, |node: &SceneGraphNodeComp<GroupNodeID>, orientation: &OrientationComp| {
                    scene_graph.set_rotation_of_group_to_parent_transform(node.id, orientation.0.cast());
                },
                ![Static]
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<ModelInstanceNodeID>, orientation: &OrientationComp| {
                    scene_graph.set_rotation_of_model_to_parent_transform(node.id, orientation.0.cast());
                },
                ![Static]
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<CameraNodeID>, orientation: &OrientationComp| {
                    scene_graph.set_rotation_of_camera_to_parent_transform(node.id, orientation.0.cast());
                },
                ![Static]
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
    depends_on = [SyncSceneCameraViewTransform],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing camera space positions and directions of lights in storage"; {
            let scene = world.scene().read().unwrap();

            let ecs_world = world.ecs_world().read().unwrap();
            let mut light_storage = scene.light_storage().write().unwrap();

            let view_transform = scene.scene_camera()
                .read()
                .unwrap()
                .as_ref()
                .map_or_else(Similarity3::identity, |scene_camera| {
                    *scene_camera.view_transform()
                });

            query!(
                ecs_world, |point_light: &PointLightComp, position: &PositionComp| {
                    let light_id = point_light.id;
                    light_storage
                        .point_light_mut(light_id)
                        .set_camera_space_position(view_transform.transform_point(&position.0.cast()));
                }
            );

            query!(
                ecs_world, |directional_light: &DirectionalLightComp, direction: &DirectionComp| {
                    let light_id = directional_light.id;
                    light_storage
                        .directional_light_mut(light_id)
                        .set_camera_space_direction(LightDirection::new_unchecked(view_transform.transform_vector(&direction.0.cast())));
                }
            );


            Ok(())
        })
    }
);
