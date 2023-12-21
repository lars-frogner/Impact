//! Tasks representing ECS systems related to scenes.

use crate::{
    define_task,
    physics::ReferenceFrameComp,
    rendering::RenderingTag,
    scene::{
        AmbientLightComp, CameraNodeID, DirectionComp, GroupNodeID, LightDirection,
        ModelInstanceNodeID, OmnidirectionalLightComp, RadianceComp, SceneGraphNodeComp,
        SceneGraphParentNodeComp, SyncSceneCameraViewTransform, UnidirectionalLightComp,
        UpdateSceneGroupToWorldTransforms, VoxelTreeNodeComp,
    },
    world::World,
};
use impact_ecs::query;
use nalgebra::Similarity3;

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the model transform of
    /// each [`SceneGraph`](crate::scene::SceneGraph) node representing an
    /// entity that also has the [`ReferenceFrameComp`] component so that the
    /// translational, rotational and scaling parts match the origin offset,
    /// position, orientation and scaling.
    [pub] SyncSceneObjectTransforms,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing scene graph node transforms"; {
            let ecs_world = world.ecs_world().read().unwrap();
            let scene = world.scene().read().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();

            query!(
                ecs_world, |node: &SceneGraphNodeComp<GroupNodeID>, frame: &ReferenceFrameComp| {
                    scene_graph.set_group_to_parent_transform(
                        node.id,
                        frame.create_transform_to_parent_space(),
                    );
                }
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<ModelInstanceNodeID>, frame: &ReferenceFrameComp| {
                    scene_graph.set_model_to_parent_transform(
                        node.id,
                        frame.create_transform_to_parent_space(),
                    );
                }
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<CameraNodeID>, frame: &ReferenceFrameComp| {
                    scene_graph.set_camera_to_parent_transform(
                        node.id,
                        frame.create_transform_to_parent_space(),
                    );
                }
            );
            query!(
                ecs_world, |voxel_tree_node: &VoxelTreeNodeComp, frame: &ReferenceFrameComp| {
                    scene_graph.set_group_to_parent_transform(
                        voxel_tree_node.group_node_id,
                        frame.create_transform_to_parent_space(),
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
                 frame: &ReferenceFrameComp| {
                    let light_id = omnidirectional_light.id;
                    light_storage
                        .omnidirectional_light_mut(light_id)
                        .set_camera_space_position(view_transform.transform_point(&frame.position.cast()));
                },
                ![SceneGraphParentNodeComp]
            );

            query!(
                ecs_world,
                |omnidirectional_light: &OmnidirectionalLightComp,
                 frame: &ReferenceFrameComp,
                 parent: &SceneGraphParentNodeComp| {
                    let parent_group_node = scene_graph.group_nodes().node(parent.id);

                    let view_transform = view_transform * parent_group_node.group_to_root_transform();

                    let light_id = omnidirectional_light.id;
                    light_storage
                        .omnidirectional_light_mut(light_id)
                        .set_camera_space_position(view_transform.transform_point(&frame.position.cast()));
                }
            );

            query!(
                ecs_world, |unidirectional_light: &UnidirectionalLightComp, direction: &DirectionComp| {
                    let light_id = unidirectional_light.id;
                    light_storage
                        .unidirectional_light_mut(light_id)
                        .set_camera_space_direction(
                            LightDirection::new_unchecked(view_transform.transform_vector(&direction.0.cast()))
                        );
                },
                ![SceneGraphParentNodeComp, ReferenceFrameComp]
            );

            query!(
                ecs_world,
                |unidirectional_light: &UnidirectionalLightComp,
                 direction: &DirectionComp,
                 parent: &SceneGraphParentNodeComp| {
                    let parent_group_node = scene_graph.group_nodes().node(parent.id);

                    let view_transform = view_transform * parent_group_node.group_to_root_transform();

                    let light_id = unidirectional_light.id;
                    light_storage
                        .unidirectional_light_mut(light_id)
                        .set_camera_space_direction(
                            LightDirection::new_unchecked(view_transform.transform_vector(&direction.0.cast()))
                        );
                },
                ![ReferenceFrameComp]
            );

            query!(
                ecs_world,
                |unidirectional_light: &UnidirectionalLightComp,
                 direction: &DirectionComp,
                 frame: &ReferenceFrameComp| {
                    let light_id = unidirectional_light.id;

                    let world_direction = frame.orientation.transform_vector(&direction.0.cast());

                    light_storage
                        .unidirectional_light_mut(light_id)
                        .set_camera_space_direction(
                            LightDirection::new_unchecked(view_transform.transform_vector(&world_direction.cast()))
                        );
                },
                ![SceneGraphParentNodeComp]
            );

            query!(
                ecs_world,
                |unidirectional_light: &UnidirectionalLightComp,
                 direction: &DirectionComp,
                 frame: &ReferenceFrameComp,
                 parent: &SceneGraphParentNodeComp| {
                    let parent_group_node = scene_graph.group_nodes().node(parent.id);

                    let view_transform = view_transform * parent_group_node.group_to_root_transform();
                    let world_direction = frame.orientation.transform_vector(&direction.0.cast());

                    let light_id = unidirectional_light.id;
                    light_storage
                        .unidirectional_light_mut(light_id)
                        .set_camera_space_direction(
                            LightDirection::new_unchecked(view_transform.transform_vector(&world_direction.cast()))
                        );
                }
            );

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the radiance or
    /// irradiance of every applicable light source in the
    /// [`LightStorage`](crate::scene::LightStorage).
    [pub] SyncLightRadiancesInStorage,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing radiance of lights in storage"; {
            let scene = world.scene().read().unwrap();

            let ecs_world = world.ecs_world().read().unwrap();
            let mut light_storage = scene.light_storage().write().unwrap();

            query!(
                ecs_world, |radiance: &RadianceComp, ambient_light: &AmbientLightComp| {
                    light_storage
                        .ambient_light_mut(ambient_light.id)
                        .set_radiance(radiance.0);
                }
            );

            query!(
                ecs_world,
                |radiance: &RadianceComp, omnidirectional_light: &OmnidirectionalLightComp| {
                    light_storage
                        .omnidirectional_light_mut(omnidirectional_light.id)
                        .set_radiance(radiance.0);
                }
            );

            query!(
                ecs_world, |radiance: &RadianceComp, unidirectional_light: &UnidirectionalLightComp| {
                    light_storage
                        .unidirectional_light_mut(unidirectional_light.id)
                        .set_radiance(radiance.0);
                }
            );

            Ok(())
        })
    }
);
