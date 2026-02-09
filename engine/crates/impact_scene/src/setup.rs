//! Scene setup.

use crate::{
    SceneEntityFlags,
    graph::{FeatureIDSet, ModelInstanceFlags, SceneGraph, SceneGroupID},
    model::{ModelID, ModelInstanceManager},
};
use anyhow::{Result, anyhow};
use impact_id::EntityID;
use impact_material::{MaterialID, MaterialRegistry};
use impact_math::transform::{Isometry3C, Similarity3C};
use impact_mesh::{TriangleMeshID, TriangleMeshRegistry};
use impact_model::{
    ModelInstanceID,
    transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
};
use tinyvec::TinyVec;

/// Enables the property values of the entity's material to be modified
/// independently.
///
/// This is a [`SetupComponent`](impact_ecs::component::SetupComponent)
/// affecting the entity's initial `SceneEntityFlags` component. It is therefore
/// not kept after entity creation.
#[cfg(feature = "ecs")]
#[roc_integration::roc(parents = "Setup")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod, impact_ecs::SetupComponent)]
pub struct HasIndependentMaterialValues;

/// The entity should never be frustum culled in the [`SceneGraph`].
///
/// This is a [`SetupComponent`](impact_ecs::component::SetupComponent)
/// affecting the entity's initial `SceneEntityFlags` component. It is therefore
/// not kept after entity creation.
#[cfg(feature = "ecs")]
#[roc_integration::roc(parents = "Setup")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod, impact_ecs::SetupComponent)]
pub struct Uncullable;

pub fn setup_scene_graph_group_node(
    scene_graph: &mut SceneGraph,
    entity_id: EntityID,
    group_to_parent_transform: Isometry3C,
    parent_entity_id: Option<EntityID>,
) -> Result<()> {
    let scene_group_id = SceneGroupID::from_entity_id(entity_id);
    let parent_group_id =
        parent_entity_id.map_or_else(|| scene_graph.root_node_id(), SceneGroupID::from_entity_id);

    scene_graph.create_group_node(parent_group_id, scene_group_id, group_to_parent_transform)
}

pub fn setup_scene_graph_model_instance_node(
    mesh_registry: &TriangleMeshRegistry,
    material_registry: &MaterialRegistry,
    model_instance_manager: &mut ModelInstanceManager,
    scene_graph: &mut SceneGraph,
    entity_id: EntityID,
    model_to_parent_transform: Similarity3C,
    mesh_id: TriangleMeshID,
    material_id: MaterialID,
    parent_entity_id: Option<EntityID>,
    flags: Option<&SceneEntityFlags>,
    has_independent_material_values: bool,
    uncullable: bool,
) -> Result<SceneEntityFlags> {
    let flags = flags.copied().unwrap_or_default();

    let model_id = ModelID::for_triangle_mesh_and_material(mesh_id, material_id);

    let bounding_sphere = if uncullable {
        // The scene graph will not cull models with no bounding sphere
        None
    } else {
        Some(
            mesh_registry
                .get(mesh_id)
                .ok_or_else(|| {
                    anyhow!("Tried to create renderable entity with missing mesh: {mesh_id}")
                })?
                .compute_bounding_sphere()
                .ok_or_else(|| {
                    anyhow!("Tried to create renderable entity with empty mesh: {mesh_id}")
                })?,
        )
    };

    let mut feature_type_ids: TinyVec<[_; 4]> = TinyVec::new();
    let mut feature_ids_for_rendering = FeatureIDSet::new();
    let mut feature_ids_for_shadow_mapping = FeatureIDSet::new();

    // Add entries for the model-to-camera and model-to-light transforms
    // for the scene graph to access and modify using the returned IDs
    let model_view_transform_feature_id = model_instance_manager
        .get_storage_mut::<InstanceModelViewTransformWithPrevious>()
        .expect("Missing storage for InstanceModelViewTransformWithPrevious feature")
        .add_feature(&InstanceModelViewTransformWithPrevious::default());

    // The first feature is expected to be the model-view transform
    feature_type_ids.push(model_view_transform_feature_id.feature_type_id());
    feature_ids_for_rendering.push(model_view_transform_feature_id);

    let model_light_transform_feature_id = model_instance_manager
        .get_storage_mut::<InstanceModelLightTransform>()
        .expect("Missing storage for InstanceModelLightTransform feature")
        .add_feature(&InstanceModelLightTransform::default());

    feature_type_ids.push(model_light_transform_feature_id.feature_type_id());
    feature_ids_for_shadow_mapping.push(model_light_transform_feature_id);

    let material_property_values = material_registry
        .get(model_id.material_id())
        .ok_or_else(|| anyhow!("Missing material {} for model", model_id.material_id()))?
        .property_values;

    if let Some(material_property_values_feature_type_id) =
        material_property_values.instance_feature_type_id_if_applicable()
    {
        feature_type_ids.push(material_property_values_feature_type_id);

        if has_independent_material_values
            && let Some(material_property_values_feature_id) =
                material_property_values.add_to_storage(model_instance_manager)
        {
            feature_ids_for_rendering.push(material_property_values_feature_id);
        }
    }

    model_instance_manager.register_instance(model_id, &feature_type_ids);

    let model_instance_id = ModelInstanceID::from_entity_id(entity_id);
    let parent_group_id =
        parent_entity_id.map_or_else(|| scene_graph.root_node_id(), SceneGroupID::from_entity_id);

    let mut model_instance_flags = flags.into();

    if has_independent_material_values {
        model_instance_flags |= ModelInstanceFlags::HAS_INDEPENDENT_MATERIAL_VALUES;
    }

    scene_graph.create_model_instance_node(
        parent_group_id,
        model_instance_id,
        model_to_parent_transform,
        model_id,
        bounding_sphere.map(|sphere| sphere.compact()),
        feature_ids_for_rendering,
        feature_ids_for_shadow_mapping,
        model_instance_flags,
    )?;

    Ok(flags)
}

pub fn remove_scene_graph_model_instance_node(
    model_instance_manager: &mut ModelInstanceManager,
    scene_graph: &mut SceneGraph,
    entity_id: EntityID,
) {
    if let Some(model_id) =
        scene_graph.remove_model_instance_node(ModelInstanceID::from_entity_id(entity_id))
    {
        model_instance_manager.unregister_instance(&model_id);
    }
}
