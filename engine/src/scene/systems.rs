//! ECS systems for scenes.

use crate::physics::motion::components::ReferenceFrameComp;
use impact_camera::buffer::BufferableCamera;
use impact_ecs::{query, world::World as ECSWorld};
use impact_light::{
    LightStorage,
    components::{
        AmbientEmissionComp, AmbientLightComp, OmnidirectionalEmissionComp,
        OmnidirectionalLightComp, ShadowableOmnidirectionalEmissionComp,
        ShadowableOmnidirectionalLightComp, ShadowableUnidirectionalEmissionComp,
        ShadowableUnidirectionalLightComp, UnidirectionalEmissionComp, UnidirectionalLightComp,
    },
};
use impact_scene::{
    camera::SceneCamera,
    components::{
        SceneEntityFlagsComp, SceneGraphCameraNodeComp, SceneGraphGroupNodeComp,
        SceneGraphModelInstanceNodeComp, SceneGraphParentNodeComp,
    },
    graph::SceneGraph,
};
use nalgebra::Similarity3;

/// Updates the model transform of each [`SceneGraph`] node representing an
/// entity that also has the
/// [`crate::physics::motion::components::ReferenceFrameComp`] component so that
/// the translational, rotational and scaling parts match the origin offset,
/// position, orientation and scaling. Also updates any flags for the node to
/// match the entity's [`crate::scene::SceneEntityFlags`].
pub fn sync_scene_object_transforms_and_flags(ecs_world: &ECSWorld, scene_graph: &mut SceneGraph) {
    query!(
        ecs_world,
        |node: &SceneGraphGroupNodeComp, frame: &ReferenceFrameComp| {
            scene_graph
                .set_group_to_parent_transform(node.id, frame.create_transform_to_parent_space());
        }
    );
    query!(
        ecs_world,
        |node: &SceneGraphModelInstanceNodeComp,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlagsComp| {
            scene_graph.set_model_to_parent_transform_and_flags(
                node.id,
                frame.create_transform_to_parent_space(),
                flags.0.into(),
            );
        }
    );
    query!(
        ecs_world,
        |node: &SceneGraphCameraNodeComp, frame: &ReferenceFrameComp| {
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
        |ambient_light: &AmbientLightComp, ambient_emission: &AmbientEmissionComp| {
            impact_light::entity::sync_ambient_light_in_storage(
                light_storage,
                ambient_light,
                ambient_emission,
            );
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light: &OmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmissionComp,
         flags: &SceneEntityFlagsComp| {
            impact_light::entity::sync_omnidirectional_light_in_storage(
                light_storage,
                omnidirectional_light,
                &view_transform,
                &frame.position.cast(),
                omnidirectional_emission,
                flags.0.into(),
            );
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |omnidirectional_light: &OmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::entity::sync_omnidirectional_light_in_storage(
                light_storage,
                omnidirectional_light,
                &view_transform,
                &frame.position.cast(),
                omnidirectional_emission,
                flags.0.into(),
            );
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light: &ShadowableOmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &ShadowableOmnidirectionalEmissionComp,
         flags: &SceneEntityFlagsComp| {
            impact_light::entity::sync_shadowable_omnidirectional_light_in_storage(
                light_storage,
                omnidirectional_light,
                &view_transform,
                &frame.position.cast(),
                omnidirectional_emission,
                flags.0.into(),
            );
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |omnidirectional_light: &ShadowableOmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &ShadowableOmnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::entity::sync_shadowable_omnidirectional_light_in_storage(
                light_storage,
                omnidirectional_light,
                &view_transform,
                &frame.position.cast(),
                omnidirectional_emission,
                flags.0.into(),
            );
        }
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         flags: &SceneEntityFlagsComp| {
            impact_light::entity::sync_unidirectional_light_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                unidirectional_emission,
                flags.0.into(),
            );
        },
        ![SceneGraphParentNodeComp, ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::entity::sync_unidirectional_light_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                unidirectional_emission,
                flags.0.into(),
            );
        },
        ![ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlagsComp| {
            impact_light::entity::sync_unidirectional_light_with_orientation_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                &frame.orientation.cast(),
                unidirectional_emission,
                flags.0.into(),
            );
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::entity::sync_unidirectional_light_with_orientation_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                &frame.orientation.cast(),
                unidirectional_emission,
                flags.0.into(),
            );
        }
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightComp,
         unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         flags: &SceneEntityFlagsComp| {
            impact_light::entity::sync_shadowable_unidirectional_light_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                unidirectional_emission,
                flags.0.into(),
            );
        },
        ![SceneGraphParentNodeComp, ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightComp,
         unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::entity::sync_shadowable_unidirectional_light_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                unidirectional_emission,
                flags.0.into(),
            );
        },
        ![ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightComp,
         unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlagsComp| {
            impact_light::entity::sync_shadowable_unidirectional_light_with_orientation_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                &frame.orientation.cast(),
                unidirectional_emission,
                flags.0.into(),
            );
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightComp,
         unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            impact_light::entity::sync_shadowable_unidirectional_light_with_orientation_in_storage(
                light_storage,
                unidirectional_light,
                &view_transform,
                &frame.orientation.cast(),
                unidirectional_emission,
                flags.0.into(),
            );
        }
    );
}
