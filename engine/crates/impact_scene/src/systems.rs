//! ECS systems for scenes.

use crate::{
    CanBeParent, ParentEntity, RemovalBeyondDistance, SceneEntityFlags,
    graph::{SceneGraph, SceneGroupID},
};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_camera::{CameraID, HasCamera};
use impact_containers::HashMap;
use impact_ecs::{
    query,
    world::{EntityStager, World as ECSWorld},
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_id::EntityID;
use impact_intersection::{
    IntersectionManager,
    bounding_volume::{BoundingVolumeID, HasBoundingVolume},
};
use impact_light::{
    AmbientEmission, AmbientLightID, LightManager, OmnidirectionalEmission, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLightID,
    ShadowableUnidirectionalEmission, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLightID,
};
use impact_math::{point::Point3C, transform::Isometry3};
use impact_model::{HasModel, ModelInstanceID};

/// Updates the model transform of each [`SceneGraph`] node representing an
/// entity that also has the
/// [`impact_geometry::ReferenceFrame`] component so that
/// the translational, rotational and scaling parts match the origin offset,
/// position, orientation and scaling. Also updates any flags for the node to
/// match the entity's [`SceneEntityFlags`].
pub fn sync_scene_object_transforms_and_flags(ecs_world: &ECSWorld, scene_graph: &mut SceneGraph) {
    query!(
        ecs_world,
        |entity_id: EntityID, frame: &ReferenceFrame| {
            let node_id = SceneGroupID::from_entity_id(entity_id);
            let group_to_parent_transform = frame.create_transform_to_parent_space();
            scene_graph.set_group_to_parent_transform(node_id, group_to_parent_transform.compact());
        },
        [CanBeParent]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         model_transform: &ModelTransform,
         frame: &ReferenceFrame,
         flags: &SceneEntityFlags| {
            let model_instance_id = ModelInstanceID::from_entity_id(entity_id);

            let model_to_parent_transform = frame.create_transform_to_parent_space()
                * model_transform.create_transform_to_entity_space();

            scene_graph.set_model_to_parent_transform_and_update_flags(
                model_instance_id,
                model_to_parent_transform.compact(),
                *flags,
            );
        },
        [HasModel]
    );

    query!(
        ecs_world,
        |entity_id: EntityID, frame: &ReferenceFrame| {
            let camera_id = CameraID::from_entity_id(entity_id);
            let camera_to_parent_transform = frame.create_transform_to_parent_space();
            scene_graph
                .set_camera_to_parent_transform(camera_id, camera_to_parent_transform.compact());
        },
        [HasCamera]
    );
}

/// Finds entities with a max distance from an anchor and stages them for
/// removal if the distance is exceeded.
pub fn stage_too_remote_entities_for_removal(
    entity_stager: &mut EntityStager,
    ecs_world: &ECSWorld,
) {
    struct Candidate {
        entity_id: EntityID,
        position: Point3C,
        max_dist_squared: f32,
    }

    let arena = ArenaPool::get_arena();

    let mut candidates_by_anchor =
        HashMap::with_capacity_and_hasher_in(0, Default::default(), &arena);

    query!(
        ecs_world,
        |entity_id: EntityID,
         flags: &SceneEntityFlags,
         frame: &ReferenceFrame,
         removal_rule: &RemovalBeyondDistance| {
            if flags.is_disabled() {
                return;
            }
            candidates_by_anchor
                .entry(removal_rule.anchor_id)
                .or_insert_with(|| AVec::new_in(&arena))
                .push(Candidate {
                    entity_id,
                    position: frame.position,
                    max_dist_squared: removal_rule.max_dist_squared(),
                });
        }
    );

    for (anchor_id, candidates) in candidates_by_anchor {
        let anchor_position: Option<Point3C> = ecs_world.get_entity(anchor_id).and_then(|entity| {
            entity
                .get_component::<ReferenceFrame>()
                .map(|entry| entry.access().position)
        });

        if let Some(anchor_position) = anchor_position {
            for Candidate {
                entity_id,
                position,
                max_dist_squared,
            } in candidates
            {
                let displacement = position - anchor_position;
                if displacement.dot(&displacement) > max_dist_squared {
                    log::debug!(
                        "Removing entity {entity_id} exceeding max distance from entity {anchor_id}"
                    );
                    entity_stager.stage_entity_for_removal(entity_id);
                }
            }
        } else {
            // Anchor is gone, so remove all anchored entities
            log::debug!(
                "Removing all entities with a max distance from removed entity {anchor_id}"
            );
            for Candidate { entity_id, .. } in candidates {
                entity_stager.stage_entity_for_removal(entity_id);
            }
        }
    }
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
        |entity_id: EntityID, ambient_emission: &AmbientEmission| {
            impact_light::setup::sync_ambient_light_in_storage(
                light_manager,
                AmbientLightID::from_entity_id(entity_id),
                ambient_emission,
            );
        }
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &OmnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_omnidirectional_light_in_storage(
                light_manager,
                OmnidirectionalLightID::from_entity_id(entity_id),
                view_transform,
                &frame.position.aligned(),
                omnidirectional_emission,
                (*flags).into(),
            );
        },
        ![ParentEntity]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &OmnidirectionalEmission,
         parent: &ParentEntity,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph
                .group_nodes()
                .node(SceneGroupID::from_entity_id(parent.0));

            let group_to_root_transform = parent_group_node.group_to_root_transform().aligned();
            let view_transform = view_transform * group_to_root_transform;

            impact_light::setup::sync_omnidirectional_light_in_storage(
                light_manager,
                OmnidirectionalLightID::from_entity_id(entity_id),
                &view_transform,
                &frame.position.aligned(),
                omnidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_omnidirectional_light_in_storage(
                light_manager,
                ShadowableOmnidirectionalLightID::from_entity_id(entity_id),
                view_transform,
                &frame.position.aligned(),
                omnidirectional_emission,
                (*flags).into(),
            );
        },
        ![ParentEntity]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         parent: &ParentEntity,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph
                .group_nodes()
                .node(SceneGroupID::from_entity_id(parent.0));

            let group_to_root_transform = parent_group_node.group_to_root_transform().aligned();
            let view_transform = view_transform * group_to_root_transform;

            impact_light::setup::sync_shadowable_omnidirectional_light_in_storage(
                light_manager,
                ShadowableOmnidirectionalLightID::from_entity_id(entity_id),
                &view_transform,
                &frame.position.aligned(),
                omnidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         unidirectional_emission: &UnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_unidirectional_light_in_storage(
                light_manager,
                UnidirectionalLightID::from_entity_id(entity_id),
                view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ParentEntity, ReferenceFrame]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         unidirectional_emission: &UnidirectionalEmission,
         parent: &ParentEntity,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph
                .group_nodes()
                .node(SceneGroupID::from_entity_id(parent.0));

            let group_to_root_transform = parent_group_node.group_to_root_transform().aligned();
            let view_transform = view_transform * group_to_root_transform;

            impact_light::setup::sync_unidirectional_light_in_storage(
                light_manager,
                UnidirectionalLightID::from_entity_id(entity_id),
                &view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ReferenceFrame]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         unidirectional_emission: &UnidirectionalEmission,
         frame: &ReferenceFrame,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_unidirectional_light_with_orientation_in_storage(
                light_manager,
                UnidirectionalLightID::from_entity_id(entity_id),
                view_transform,
                &frame.orientation.aligned(),
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ParentEntity]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         unidirectional_emission: &UnidirectionalEmission,
         frame: &ReferenceFrame,
         parent: &ParentEntity,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph
                .group_nodes()
                .node(SceneGroupID::from_entity_id(parent.0));

            let group_to_root_transform = parent_group_node.group_to_root_transform().aligned();
            let view_transform = view_transform * group_to_root_transform;

            impact_light::setup::sync_unidirectional_light_with_orientation_in_storage(
                light_manager,
                UnidirectionalLightID::from_entity_id(entity_id),
                &view_transform,
                &frame.orientation.aligned(),
                unidirectional_emission,
                (*flags).into(),
            );
        }
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_unidirectional_light_in_storage(
                light_manager,
                ShadowableUnidirectionalLightID::from_entity_id(entity_id),
                view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ParentEntity, ReferenceFrame]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         parent: &ParentEntity,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph
                .group_nodes()
                .node(SceneGroupID::from_entity_id(parent.0));

            let group_to_root_transform = parent_group_node.group_to_root_transform().aligned();
            let view_transform = view_transform * group_to_root_transform;

            impact_light::setup::sync_shadowable_unidirectional_light_in_storage(
                light_manager,
                ShadowableUnidirectionalLightID::from_entity_id(entity_id),
                &view_transform,
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ReferenceFrame]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         frame: &ReferenceFrame,
         flags: &SceneEntityFlags| {
            impact_light::setup::sync_shadowable_unidirectional_light_with_orientation_in_storage(
                light_manager,
                ShadowableUnidirectionalLightID::from_entity_id(entity_id),
                view_transform,
                &frame.orientation.aligned(),
                unidirectional_emission,
                (*flags).into(),
            );
        },
        ![ParentEntity]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         frame: &ReferenceFrame,
         parent: &ParentEntity,
         flags: &SceneEntityFlags| {
            let parent_group_node = scene_graph
                .group_nodes()
                .node(SceneGroupID::from_entity_id(parent.0));

            let group_to_root_transform = parent_group_node.group_to_root_transform().aligned();
            let view_transform = view_transform * group_to_root_transform;

            impact_light::setup::sync_shadowable_unidirectional_light_with_orientation_in_storage(
                light_manager,
                ShadowableUnidirectionalLightID::from_entity_id(entity_id),
                &view_transform,
                &frame.orientation.aligned(),
                unidirectional_emission,
                (*flags).into(),
            );
        }
    );
}

pub fn add_bounding_volumes_to_hierarchy(
    ecs_world: &ECSWorld,
    intersection_manager: &mut IntersectionManager,
    scene_graph: &SceneGraph,
) {
    query!(
        ecs_world,
        |entity_id: EntityID,
         model_transform: &ModelTransform,
         frame: &ReferenceFrame,
         flags: &SceneEntityFlags| {
            if flags.is_disabled() {
                return;
            }

            let model_to_world_transform = frame.create_transform_to_parent_space()
                * model_transform.create_transform_to_entity_space();

            let bounding_volume_id = BoundingVolumeID::from_entity_id(entity_id);
            if let Err(err) = intersection_manager
                .add_bounding_volume_to_hierarchy(bounding_volume_id, &model_to_world_transform)
            {
                log::error!("Failed to add bounding volume to hierarchy: {err}");
            }
        },
        [HasBoundingVolume],
        ![ParentEntity]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         model_transform: &ModelTransform,
         frame: &ReferenceFrame,
         parent: &ParentEntity,
         flags: &SceneEntityFlags| {
            if flags.is_disabled() {
                return;
            }

            let parent_group_node = scene_graph
                .group_nodes()
                .node(SceneGroupID::from_entity_id(parent.0));

            let model_to_world_transform = (parent_group_node.group_to_root_transform().aligned()
                * frame.create_transform_to_parent_space())
                * model_transform.create_transform_to_entity_space();

            let bounding_volume_id = BoundingVolumeID::from_entity_id(entity_id);
            if let Err(err) = intersection_manager
                .add_bounding_volume_to_hierarchy(bounding_volume_id, &model_to_world_transform)
            {
                log::error!("Failed to add bounding volume to hierarchy: {err}");
            }
        },
        [HasBoundingVolume]
    );
}
