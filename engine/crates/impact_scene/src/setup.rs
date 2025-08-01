//! Scene setup.

use crate::{
    SceneEntityFlags, SceneGraphGroupNodeHandle, SceneGraphModelInstanceNodeHandle,
    SceneGraphParentNodeHandle,
    graph::SceneGraph,
    model::{InstanceFeatureManager, ModelID},
};
use anyhow::{Result, anyhow};
use impact_material::{MaterialID, MaterialRegistry};
use impact_mesh::{TriangleMeshID, TriangleMeshRegistry};
use impact_model::transform::{
    InstanceModelLightTransform, InstanceModelViewTransformWithPrevious,
};
use nalgebra::{Isometry3, Similarity3};
use parking_lot::RwLock;

/// A parent entity.
///
/// This is a [`SetupComponent`](impact_ecs::component::SetupComponent) whose
/// purpose is to aid in constructing a `SceneGraphParentNodeHandle` component
/// for an entity. It is therefore not kept after entity creation.
#[cfg(feature = "ecs")]
#[roc_integration::roc(parents = "Setup")]
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod, impact_ecs::SetupComponent)]
pub struct Parent {
    pub entity_id: impact_ecs::world::EntityID,
}

/// The entity has a group node in the [`SceneGraph`](crate::graph::SceneGraph).
///
/// This is a [`SetupComponent`](impact_ecs::component::SetupComponent) whose
/// purpose is to aid in constructing a `SceneGraphGroupNodeHandle` component
/// for an entity. It is therefore not kept after entity creation.
#[cfg(feature = "ecs")]
#[roc_integration::roc(parents = "Setup")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod, impact_ecs::SetupComponent)]
pub struct SceneGraphGroup;

/// The entity should never be frustum culled in the
/// [`SceneGraph`](crate::graph::SceneGraph).
///
/// This is a [`SetupComponent`](impact_ecs::component::SetupComponent) whose
/// purpose is to aid in constructing a `SceneGraphModelInstanceNodeHandle`
/// component for an entity. It is therefore not kept after entity creation.
#[cfg(feature = "ecs")]
#[roc_integration::roc(parents = "Setup")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod, impact_ecs::SetupComponent)]
pub struct Uncullable;

#[cfg(feature = "ecs")]
#[roc_integration::roc]
impl Parent {
    #[roc_integration::roc(body = "{ entity_id: parent }")]
    pub fn new(parent: impact_ecs::world::EntityID) -> Self {
        Self { entity_id: parent }
    }
}

#[cfg(feature = "ecs")]
pub fn setup_scene_graph_parent_node(
    parent_entity: impact_ecs::world::EntityEntry<'_>,
) -> Result<SceneGraphParentNodeHandle> {
    let parent_group_node = parent_entity
        .get_component::<SceneGraphGroupNodeHandle>()
        .ok_or_else(|| {
            anyhow!(
                "Missing group node component for parent entity with ID {}",
                parent_entity.id()
            )
        })?;

    Ok(SceneGraphParentNodeHandle::new(
        parent_group_node.access().id,
    ))
}

pub fn setup_scene_graph_group_node(
    scene_graph: &mut SceneGraph,
    group_to_parent_transform: Isometry3<f32>,
    parent: Option<&SceneGraphParentNodeHandle>,
) -> SceneGraphGroupNodeHandle {
    let parent_node_id = parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

    SceneGraphGroupNodeHandle::new(
        scene_graph.create_group_node(parent_node_id, group_to_parent_transform),
    )
}

pub fn setup_scene_graph_model_instance_node(
    mesh_registry: &TriangleMeshRegistry,
    material_registry: &MaterialRegistry,
    instance_feature_manager: &mut InstanceFeatureManager,
    scene_graph: &mut SceneGraph,
    model_to_parent_transform: Similarity3<f32>,
    mesh_id: TriangleMeshID,
    material_id: MaterialID,
    parent: Option<&SceneGraphParentNodeHandle>,
    flags: Option<&SceneEntityFlags>,
    uncullable: bool,
) -> Result<(SceneGraphModelInstanceNodeHandle, SceneEntityFlags)> {
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

    let mut feature_type_ids = Vec::with_capacity(3);
    let mut feature_ids_for_rendering = Vec::with_capacity(2);
    let mut feature_ids_for_shadow_mapping = Vec::with_capacity(1);

    // Add entries for the model-to-camera and model-to-light transforms
    // for the scene graph to access and modify using the returned IDs
    let model_view_transform_feature_id = instance_feature_manager
        .get_storage_mut::<InstanceModelViewTransformWithPrevious>()
        .expect("Missing storage for InstanceModelViewTransformWithPrevious feature")
        .add_feature(&InstanceModelViewTransformWithPrevious::default());

    // The first feature is expected to be the model-view transform
    feature_type_ids.push(model_view_transform_feature_id.feature_type_id());
    feature_ids_for_rendering.push(model_view_transform_feature_id);

    let model_light_transform_feature_id = instance_feature_manager
        .get_storage_mut::<InstanceModelLightTransform>()
        .expect("Missing storage for InstanceModelLightTransform feature")
        .add_feature(&InstanceModelLightTransform::default());

    feature_type_ids.push(model_light_transform_feature_id.feature_type_id());
    feature_ids_for_shadow_mapping.push(model_light_transform_feature_id);

    if let Some(material_feature_id) = material_registry
        .get(model_id.material_id())
        .ok_or_else(|| anyhow!("Missing material {} for model", model_id.material_id()))?
        .instance_feature_id_if_applicable()
    {
        feature_type_ids.push(material_feature_id.feature_type_id());
        feature_ids_for_rendering.push(material_feature_id);
    }

    instance_feature_manager.register_instance(model_id, &feature_type_ids);

    let parent_node_id = parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

    Ok((
        SceneGraphModelInstanceNodeHandle::new(scene_graph.create_model_instance_node(
            parent_node_id,
            model_to_parent_transform,
            model_id,
            bounding_sphere,
            feature_ids_for_rendering,
            feature_ids_for_shadow_mapping,
            flags.into(),
        )),
        flags,
    ))
}

pub fn remove_scene_graph_model_instance_node(
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    scene_graph: &RwLock<SceneGraph>,
    model_instance_node: &SceneGraphModelInstanceNodeHandle,
    desynchronized: &mut bool,
) {
    let model_id = scene_graph
        .write()
        .remove_model_instance_node(model_instance_node.id);
    instance_feature_manager
        .write()
        .unregister_instance(&model_id);
    *desynchronized = true;
}

#[cfg(feature = "ecs")]
pub fn remove_scene_graph_model_instance_node_for_entity(
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    scene_graph: &RwLock<SceneGraph>,
    entity: &impact_ecs::world::EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(node) = entity.get_component::<SceneGraphModelInstanceNodeHandle>() {
        remove_scene_graph_model_instance_node(
            instance_feature_manager,
            scene_graph,
            node.access(),
            desynchronized,
        );
    }
}
