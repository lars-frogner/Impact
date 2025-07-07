//! ECS systems for scenes.

use crate::physics::motion::components::ReferenceFrameComp;
use impact_camera::buffer::BufferableCamera;
use impact_ecs::{query, world::World as ECSWorld};
use impact_light::{
    AmbientEmission, AmbientLightHandle, LightStorage, OmnidirectionalEmission,
    OmnidirectionalLightHandle, ShadowableOmnidirectionalEmission,
    ShadowableOmnidirectionalLightHandle, ShadowableUnidirectionalEmission,
    ShadowableUnidirectionalLightHandle, UnidirectionalEmission, UnidirectionalLightHandle,
};
use impact_scene::{
    SceneEntityFlags, SceneGraphCameraNodeHandle, SceneGraphGroupNodeHandle,
    SceneGraphModelInstanceNodeHandle, SceneGraphParentNodeHandle, camera::SceneCamera,
    graph::SceneGraph,
};
use nalgebra::Similarity3;

/// Updates the model transform of each [`SceneGraph`] node representing an
/// entity that also has the
/// [`crate::physics::motion::components::ReferenceFrameComp`] component so that
/// the translational, rotational and scaling parts match the origin offset,
/// position, orientation and scaling. Also updates any flags for the node to
/// match the entity's [`impact_scene::SceneEntityFlags`].
pub fn sync_scene_object_transforms_and_flags(ecs_world: &ECSWorld, scene_graph: &mut SceneGraph) {
    query!(
        ecs_world,
        |node: &SceneGraphGroupNodeHandle, frame: &ReferenceFrameComp| {
            scene_graph
                .set_group_to_parent_transform(node.id, frame.create_transform_to_parent_space());
        }
    );
    query!(ecs_world, |node: &SceneGraphModelInstanceNodeHandle,
                       frame: &ReferenceFrameComp,
                       flags: &SceneEntityFlags| {
        scene_graph.set_model_to_parent_transform_and_flags(
            node.id,
            frame.create_transform_to_parent_space(),
            (*flags).into(),
        );
    });
    query!(
        ecs_world,
        |node: &SceneGraphCameraNodeHandle, frame: &ReferenceFrameComp| {
            scene_graph
                .set_camera_to_parent_transform(node.id, frame.create_transform_to_parent_space());
        }
    );
}

/// Updates the properties (position, direction, emission, extent and flags) of
/// every light source in the [`LightStorage`].
pub fn sync_lights_in_storage(
    ecs_world: &ECSWorld,
    scene_graph: &SceneGraph,
    scene_camera: Option<&SceneCamera>,
    light_storage: &mut LightStorage,
) {
    let view_transform = scene_camera.map_or_else(Similarity3::identity, |scene_camera| {
        *scene_camera.view_transform()
    });

    query!(
        ecs_world,
        |ambient_light: &AmbientLightHandle, ambient_emission: &AmbientEmission| {
            impact_light::setup::sync_ambient_light_in_storage(
                light_storage,
                ambient_light,
                ambient_emission,
            );
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light: &OmnidirectionalLightHandle,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_omnidirectional_light_in_storage(
                light_storage,
                omnidirectional_light,
                &view_transform,
                &frame.position.cast(),
                omnidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle]
    );

    query!(
        ecs_world,
        |omnidirectional_light: &OmnidirectionalLightHandle,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmission,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_omnidirectional_light_in_storage(
                light_storage,
                omnidirectional_light,
                &view_transform,
                &frame.position.cast(),
                omnidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light: &ShadowableOmnidirectionalLightHandle,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_omnidirectional_light_in_storage(
                light_storage,
                omnidirectional_light,
                &view_transform,
                &frame.position.cast(),
                omnidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle]
    );

    query!(
        ecs_world,
        |omnidirectional_light: &ShadowableOmnidirectionalLightHandle,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_shadowable_omnidirectional_light_in_storage(
                light_storage,
                omnidirectional_light,
                &view_transform,
                &frame.position.cast(),
                omnidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightHandle,
         unidirectional_emission: &UnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_unidirectional_light_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle, ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightHandle,
         unidirectional_emission: &UnidirectionalEmission,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_unidirectional_light_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightHandle,
         unidirectional_emission: &UnidirectionalEmission,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_unidirectional_light_with_orientation_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                &frame.orientation.cast(),
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightHandle,
         unidirectional_emission: &UnidirectionalEmission,
         frame: &ReferenceFrameComp,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_unidirectional_light_with_orientation_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                &frame.orientation.cast(),
                unidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightHandle,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_unidirectional_light_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle, ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightHandle,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_shadowable_unidirectional_light_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightHandle,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_unidirectional_light_with_orientation_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                &frame.orientation.cast(),
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightHandle,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         frame: &ReferenceFrameComp,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_shadowable_unidirectional_light_with_orientation_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                &frame.orientation.cast(),
                unidirectional_emission,
                (*flags).into(),
            );
        }
    );
}
