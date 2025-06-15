//! ECS systems for gizmo management.

use crate::{
    gizmo::{
        GizmoManager, GizmoSet, GizmoType, GizmoVisibility,
        components::GizmosComp,
        model::{
            BOUNDING_SPHERE_GIZMO_MODEL_IDX, LIGHT_SPHERE_GIZMO_MODEL_IDX,
            REFERENCE_FRAME_AXES_GIZMO_MODEL_IDX, SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX,
            SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX,
        },
    },
    light::{
        LightID, LightStorage,
        components::{OmnidirectionalLightComp, ShadowableOmnidirectionalLightComp},
    },
    model::{
        InstanceFeatureManager,
        transform::{InstanceModelViewTransform, InstanceModelViewTransformWithPrevious},
    },
    scene::{
        ModelInstanceNode, ModelInstanceNodeID, SceneGraph,
        components::{SceneEntityFlagsComp, SceneGraphModelInstanceNodeComp},
    },
};
use impact_ecs::{query, world::World as ECSWorld};
use nalgebra::{Similarity3, UnitQuaternion};

pub fn update_visibility_flags_for_gizmo(
    ecs_world: &ECSWorld,
    gizmo_manager: &GizmoManager,
    gizmo: GizmoType,
) {
    let globally_visible = match gizmo_manager.config().visibility(gizmo) {
        GizmoVisibility::Hidden => false,
        GizmoVisibility::VisibleForAll => true,
        GizmoVisibility::VisibleForSelected => {
            return;
        }
    };
    query!(ecs_world, |gizmos: &mut GizmosComp| {
        gizmos.visible_gizmos.set(gizmo.as_set(), globally_visible);
    });
}

/// Finds entities for which each gizmo should be displayed and copies
/// appropriately transformed versions of their model-view transforms to the
/// gizmo's dedicated buffer.
pub fn buffer_transforms_for_gizmos(
    ecs_world: &ECSWorld,
    instance_feature_manager: &mut InstanceFeatureManager,
    scene_graph: &SceneGraph<f32>,
    light_storage: &LightStorage,
    current_frame_count: u32,
) {
    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         model_instance_node: &SceneGraphModelInstanceNodeComp,
         flags: &SceneEntityFlagsComp| {
            if gizmos.visible_gizmos.is_empty() || flags.is_disabled() {
                return;
            }
            buffer_transforms_for_model_instance_gizmos(
                instance_feature_manager,
                scene_graph,
                current_frame_count,
                gizmos.visible_gizmos,
                model_instance_node.id,
            );
        }
    );

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         omnidirectional_light: &OmnidirectionalLightComp,
         flags: &SceneEntityFlagsComp| {
            if !gizmos.visible_gizmos.contains(GizmoSet::LIGHT_SPHERE) || flags.is_disabled() {
                return;
            }
            buffer_transform_for_light_sphere_gizmo(
                instance_feature_manager,
                light_storage,
                omnidirectional_light.id,
                false,
            );
        }
    );
    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         omnidirectional_light: &ShadowableOmnidirectionalLightComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
            if gizmos.visible_gizmos.contains(GizmoSet::LIGHT_SPHERE) {
                buffer_transform_for_light_sphere_gizmo(
                    instance_feature_manager,
                    light_storage,
                    omnidirectional_light.id,
                    true,
                );
            }
            if gizmos
                .visible_gizmos
                .contains(GizmoSet::SHADOW_CUBEMAP_FACES)
            {
                buffer_transforms_for_shadow_cubemap_frusta_gizmo(
                    instance_feature_manager,
                    light_storage,
                    omnidirectional_light.id,
                );
            }
        }
    );
}

fn buffer_transforms_for_model_instance_gizmos(
    instance_feature_manager: &mut InstanceFeatureManager,
    scene_graph: &SceneGraph<f32>,
    current_frame_count: u32,
    visible_gizmos: GizmoSet,
    model_instance_node_id: ModelInstanceNodeID,
) {
    let node = scene_graph
        .model_instance_nodes()
        .node(model_instance_node_id);

    if node.frame_count_when_last_visible() != current_frame_count {
        return;
    }

    let model_view_transform = instance_feature_manager
        .feature::<InstanceModelViewTransformWithPrevious>(node.model_view_transform_feature_id())
        .current;

    for gizmo in GizmoType::all_in_set(visible_gizmos) {
        obtain_transforms_for_model_instance_gizmo(
            instance_feature_manager,
            node,
            model_view_transform,
            gizmo,
        );
    }
}

fn obtain_transforms_for_model_instance_gizmo(
    instance_feature_manager: &mut InstanceFeatureManager,
    node: &ModelInstanceNode<f32>,
    model_view_transform: InstanceModelViewTransform,
    gizmo: GizmoType,
) {
    let models = gizmo.models();
    match gizmo {
        GizmoType::ReferenceFrameAxes => {
            instance_feature_manager.buffer_instance_feature(
                &models[REFERENCE_FRAME_AXES_GIZMO_MODEL_IDX].model_id,
                &model_view_transform,
            );
        }
        GizmoType::BoundingSphere => {
            if let Some(transform) =
                compute_transform_for_bounding_sphere_gizmo(node, model_view_transform)
            {
                instance_feature_manager.buffer_instance_feature(
                    &models[BOUNDING_SPHERE_GIZMO_MODEL_IDX].model_id,
                    &transform,
                );
            }
        }
        GizmoType::LightSphere | GizmoType::ShadowCubemapFaces => {}
    }
}

fn compute_transform_for_bounding_sphere_gizmo(
    node: &ModelInstanceNode<f32>,
    model_view_transform: InstanceModelViewTransform,
) -> Option<InstanceModelViewTransform> {
    let bounding_sphere = node.get_model_bounding_sphere()?;
    let center = bounding_sphere.center();
    let radius = bounding_sphere.radius();

    let bounding_sphere_from_unit_sphere =
        Similarity3::from_parts(center.coords.into(), UnitQuaternion::identity(), radius);

    let model_view_transform: Similarity3<_> = model_view_transform.into();

    Some(InstanceModelViewTransform::from(
        model_view_transform * bounding_sphere_from_unit_sphere,
    ))
}

fn buffer_transform_for_light_sphere_gizmo(
    instance_feature_manager: &mut InstanceFeatureManager,
    light_storage: &LightStorage,
    light_id: LightID,
    is_shadowable: bool,
) {
    let (camera_space_position, max_reach) = if is_shadowable {
        let Some(light) = light_storage.get_shadowable_omnidirectional_light(light_id) else {
            return;
        };
        (*light.camera_space_position(), light.max_reach())
    } else {
        let Some(light) = light_storage.get_omnidirectional_light(light_id) else {
            return;
        };
        (*light.camera_space_position(), light.max_reach())
    };

    let light_sphere_from_unit_sphere = InstanceModelViewTransform {
        translation: camera_space_position.coords,
        scaling: max_reach,
        rotation: UnitQuaternion::identity(),
    };

    instance_feature_manager.buffer_instance_feature(
        &GizmoType::LightSphere.models()[LIGHT_SPHERE_GIZMO_MODEL_IDX].model_id,
        &light_sphere_from_unit_sphere,
    );
}

fn buffer_transforms_for_shadow_cubemap_frusta_gizmo(
    instance_feature_manager: &mut InstanceFeatureManager,
    light_storage: &LightStorage,
    light_id: LightID,
) {
    let Some(light) = light_storage.get_shadowable_omnidirectional_light(light_id) else {
        return;
    };

    let light_space_to_camera_transform = light.create_light_space_to_camera_transform();

    let cubemap_near_plane_transform: InstanceModelViewTransform = light_space_to_camera_transform
        .prepend_scaling(light.near_distance())
        .into();

    instance_feature_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX]
            .model_id,
        &cubemap_near_plane_transform,
    );

    let cubemap_far_plane_transform: InstanceModelViewTransform = light_space_to_camera_transform
        .prepend_scaling(light.far_distance())
        .into();

    instance_feature_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX]
            .model_id,
        &cubemap_far_plane_transform,
    );

    instance_feature_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX]
            .model_id,
        &cubemap_far_plane_transform,
    );
}
