//! ECS systems for scenes.

use crate::{
    SceneEntityFlags, SceneGraphCameraNodeHandle, SceneGraphGroupNodeHandle,
    SceneGraphModelInstanceNodeHandle, SceneGraphParentNodeHandle, graph::SceneGraph,
};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_light::{
    AmbientEmission, AmbientLightID, LightManager, OmnidirectionalEmission, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLightID,
    ShadowableUnidirectionalEmission, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLightID,
};
use impact_math::transform::Isometry3;

/// Updates the model transform of each [`SceneGraph`] node representing an
/// entity that also has the
/// [`impact_geometry::ReferenceFrame`] component so that
/// the translational, rotational and scaling parts match the origin offset,
/// position, orientation and scaling. Also updates any flags for the node to
/// match the entity's [`SceneEntityFlags`].
pub fn sync_scene_object_transforms_and_flags(ecs_world: &ECSWorld, scene_graph: &mut SceneGraph) {
    query!(ecs_world, |node: &SceneGraphGroupNodeHandle,
                       frame: &ReferenceFrame| {
        scene_graph
            .set_group_to_parent_transform(node.id, frame.create_transform_to_parent_space());
    });

    query!(ecs_world, |node: &SceneGraphModelInstanceNodeHandle,
                       model_transform: &ModelTransform,
                       frame: &ReferenceFrame,
                       flags: &SceneEntityFlags| {
        let model_to_parent_transform = frame.create_transform_to_parent_space()
            * model_transform.crate_transform_to_entity_space();

        scene_graph.set_model_to_parent_transform_and_flags(
            node.id,
            model_to_parent_transform,
            (*flags).into(),
        );
    });

    query!(ecs_world, |node: &SceneGraphCameraNodeHandle,
                       frame: &ReferenceFrame| {
        scene_graph
            .set_camera_to_parent_transform(node.id, frame.create_transform_to_parent_space());
    });
}

/// Updates the properties (position, direction, emission, extent and flags) of
/// every light source in the [`LightManager`].
pub fn sync_lights_in_storage(
    ecs_world: &ECSWorld,
    light_manager: &mut LightManager,
    scene_graph: &SceneGraph,
    view_transform: &Isometry3,
) {
    query!(
        ecs_world,
        |ambient_light_id: &AmbientLightID, ambient_emission: &AmbientEmission| {
            impact_light::setup::sync_ambient_light_in_storage(
                light_manager,
                *ambient_light_id,
                ambient_emission,
            );
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light_id: &OmnidirectionalLightID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &OmnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_omnidirectional_light_in_storage(
                light_manager,
                *omnidirectional_light_id,
                view_transform,
                &frame.position,
                omnidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle]
    );

    query!(
        ecs_world,
        |omnidirectional_light_id: &OmnidirectionalLightID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &OmnidirectionalEmission,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_omnidirectional_light_in_storage(
                light_manager,
                *omnidirectional_light_id,
                &view_transform,
                &frame.position,
                omnidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light_id: &ShadowableOmnidirectionalLightID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_omnidirectional_light_in_storage(
                light_manager,
                *omnidirectional_light_id,
                view_transform,
                &frame.position,
                omnidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle]
    );

    query!(
        ecs_world,
        |omnidirectional_light_id: &ShadowableOmnidirectionalLightID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_shadowable_omnidirectional_light_in_storage(
                light_manager,
                *omnidirectional_light_id,
                &view_transform,
                &frame.position,
                omnidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |unidirectional_light_id: &UnidirectionalLightID,
         unidirectional_emission: &UnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_unidirectional_light_in_storage(
                light_manager,
                *unidirectional_light_id,
                view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle, ReferenceFrame]
    );

    query!(
        ecs_world,
        |unidirectional_light_id: &UnidirectionalLightID,
         unidirectional_emission: &UnidirectionalEmission,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_unidirectional_light_in_storage(
                light_manager,
                *unidirectional_light_id,
                &view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ReferenceFrame]
    );

    query!(
        ecs_world,
        |unidirectional_light_id: &UnidirectionalLightID,
         unidirectional_emission: &UnidirectionalEmission,
         frame: &ReferenceFrame,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_unidirectional_light_with_orientation_in_storage(
                light_manager,
                *unidirectional_light_id,
                view_transform,
                &frame.orientation,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle]
    );

    query!(
        ecs_world,
        |unidirectional_light_id: &UnidirectionalLightID,
         unidirectional_emission: &UnidirectionalEmission,
         frame: &ReferenceFrame,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_unidirectional_light_with_orientation_in_storage(
                light_manager,
                *unidirectional_light_id,
                &view_transform,
                &frame.orientation,
                unidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |unidirectional_light_id: &ShadowableUnidirectionalLightID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_unidirectional_light_in_storage(
                light_manager,
                *unidirectional_light_id,
                view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle, ReferenceFrame]
    );

    query!(
        ecs_world,
        |unidirectional_light_id: &ShadowableUnidirectionalLightID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_shadowable_unidirectional_light_in_storage(
                light_manager,
                *unidirectional_light_id,
                &view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ReferenceFrame]
    );

    query!(
        ecs_world,
        |unidirectional_light_id: &ShadowableUnidirectionalLightID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         frame: &ReferenceFrame,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_unidirectional_light_with_orientation_in_storage(
                light_manager,
                *unidirectional_light_id,
                view_transform,
                &frame.orientation,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![SceneGraphParentNodeHandle]
    );

    query!(
        ecs_world,
        |unidirectional_light_id: &ShadowableUnidirectionalLightID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         frame: &ReferenceFrame,
         parent: &SceneGraphParentNodeHandle,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::setup::sync_shadowable_unidirectional_light_with_orientation_in_storage(
                light_manager,
                *unidirectional_light_id,
                &view_transform,
                &frame.orientation,
                unidirectional_emission,
                (*flags).into(),
            );
        }
    );
}
