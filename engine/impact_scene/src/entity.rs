//! Management of scene data for entities.

use crate::{
    SceneEntityFlags,
    components::{
        SceneEntityFlagsComp, SceneGraphGroupNodeComp, SceneGraphModelInstanceNodeComp,
        SceneGraphParentNodeComp,
    },
    graph::{NodeTransform, SceneGraph},
    model::{InstanceFeatureManager, ModelID},
};
use anyhow::{Result, anyhow};
use impact_ecs::world::EntityEntry;
use impact_material::{MaterialLibrary, components::MaterialComp};
use impact_mesh::{MeshRepository, components::TriangleMeshComp};
use impact_model::{
    InstanceFeature,
    transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
};
use std::sync::RwLock;

pub fn setup_parent_group_node(parent_entity: EntityEntry<'_>) -> Result<SceneGraphParentNodeComp> {
    let parent_group_node = parent_entity
        .get_component::<SceneGraphGroupNodeComp>()
        .ok_or_else(|| {
            anyhow!(
                "Missing group node component for parent entity with ID {}",
                parent_entity.id()
            )
        })?;

    Ok(SceneGraphParentNodeComp::new(parent_group_node.access().id))
}

pub fn setup_group_node(
    scene_graph: &mut SceneGraph,
    group_to_parent_transform: NodeTransform,
    parent: Option<&SceneGraphParentNodeComp>,
) -> SceneGraphGroupNodeComp {
    let parent_node_id = parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

    SceneGraphGroupNodeComp::new(
        scene_graph.create_group_node(parent_node_id, group_to_parent_transform),
    )
}

pub fn setup_model_instance_node(
    mesh_repository: &MeshRepository,
    material_library: &MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    scene_graph: &mut SceneGraph,
    model_to_parent_transform: NodeTransform,
    mesh: &TriangleMeshComp,
    material: &MaterialComp,
    parent: Option<&SceneGraphParentNodeComp>,
    flags: Option<&SceneEntityFlagsComp>,
    uncullable: bool,
) -> Result<(SceneGraphModelInstanceNodeComp, SceneEntityFlagsComp)> {
    let flags = flags.map_or_else(SceneEntityFlags::empty, |flags| flags.0);

    let model_id = ModelID::for_mesh_and_material(mesh.id, *material.material_handle());

    let bounding_sphere = if uncullable {
        // The scene graph will not cull models with no bounding sphere
        None
    } else {
        Some(
            mesh_repository
                .get_triangle_mesh(mesh.id)
                .ok_or_else(|| {
                    anyhow!(
                        "Tried to create renderable entity with missing mesh (mesh ID {})",
                        mesh.id
                    )
                })?
                .compute_bounding_sphere()
                .ok_or_else(|| {
                    anyhow!(
                        "Tried to create renderable entity with empty mesh (mesh ID {})",
                        mesh.id
                    )
                })?,
        )
    };

    let mut feature_type_ids = Vec::with_capacity(4);

    feature_type_ids.push(InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID);
    feature_type_ids.push(InstanceModelLightTransform::FEATURE_TYPE_ID);

    feature_type_ids.extend_from_slice(
        material_library
            .get_material_specification(model_id.material_handle().material_id())
            .expect("Missing material specification for model material")
            .instance_feature_type_ids(),
    );

    instance_feature_manager.register_instance(model_id, &feature_type_ids);

    let mut feature_ids_for_rendering = Vec::with_capacity(4);

    // Add entries for the model-to-camera and model-to-light transforms
    // for the scene graph to access and modify using the returned IDs
    let model_view_transform_feature_id = instance_feature_manager
        .get_storage_mut::<InstanceModelViewTransformWithPrevious>()
        .expect("Missing storage for InstanceModelViewTransformWithPrevious feature")
        .add_feature(&InstanceModelViewTransformWithPrevious::default());

    let model_light_transform_feature_id = instance_feature_manager
        .get_storage_mut::<InstanceModelLightTransform>()
        .expect("Missing storage for InstanceModelLightTransform feature")
        .add_feature(&InstanceModelLightTransform::default());

    // The first feature is expected to be the model-view transform
    feature_ids_for_rendering.push(model_view_transform_feature_id);

    if let Some(feature_id) = material.material_handle().material_property_feature_id() {
        feature_ids_for_rendering.push(feature_id);
    }

    let feature_ids_for_shadow_mapping = vec![model_light_transform_feature_id];

    let parent_node_id = parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

    Ok((
        SceneGraphModelInstanceNodeComp::new(scene_graph.create_model_instance_node(
            parent_node_id,
            model_to_parent_transform,
            model_id,
            bounding_sphere,
            feature_ids_for_rendering,
            feature_ids_for_shadow_mapping,
            flags.into(),
        )),
        SceneEntityFlagsComp(flags),
    ))
}

pub fn remove_model_instance_node_for_entity(
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    scene_graph: &RwLock<SceneGraph>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(node) = entity.get_component::<SceneGraphModelInstanceNodeComp>() {
        let model_id = scene_graph
            .write()
            .unwrap()
            .remove_model_instance_node(node.access().id);
        instance_feature_manager
            .write()
            .unwrap()
            .unregister_instance(&model_id);
        *desynchronized = true;
    }
}
