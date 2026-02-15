//! Setup and cleanup of scene data for new and removed entities.

pub mod bounding_volume;
pub mod camera;
pub mod light;
pub mod material;
pub mod mesh;
pub mod voxel;

use crate::{
    lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager, scene::Scene,
};
use anyhow::Result;
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_id::EntityID;
use impact_material::MaterialID;
use impact_mesh::TriangleMeshID;
use impact_model::HasModel;
use impact_scene::{
    CanBeParent, ParentEntity, SceneEntityFlags, setup::HasIndependentMaterialValues,
};
use parking_lot::RwLock;

/// Performs any modifications to the scene required to accommodate the given
/// new entities, and adds any additional components to the entities' components
/// (except scene graph components, which are added by calling
/// [`add_new_entities_to_scene_graph`].
pub fn setup_scene_data_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    simulator: &RwLock<PhysicsSimulator>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    mesh::setup_meshes_for_new_entities(resource_manager, entities)?;

    light::setup_lights_for_new_entities(scene, entities)?;

    material::setup_materials_for_new_entities(resource_manager, entities)?;

    voxel::setup_voxel_objects_for_new_entities(resource_manager, scene, simulator, entities)?;
    voxel::setup_voxel_interaction_for_new_entities(scene, entities)?;

    bounding_volume::setup_bounding_volumes_for_new_entities(resource_manager, scene, entities)?;

    mesh::generate_missing_vertex_properties_for_new_entity_meshes(resource_manager, entities);

    Ok(())
}

/// Adds the given new entities to the scene graph if required, and adds the
/// corresponding scene graph components to the entities' components.
pub fn add_new_entities_to_scene_graph(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup_scene_graph_group_nodes_for_new_entities(scene, entities)?;

    camera::add_camera_to_scene_for_new_entities(scene, entities)?;

    setup_scene_graph_model_instance_nodes_for_new_entities(resource_manager, scene, entities)?;

    voxel::setup_scene_graph_model_instance_nodes_for_new_voxel_object_entities(scene, entities)?;

    Ok(())
}

/// Performs any modifications required to clean up the scene when
/// the given entity is removed.
pub fn cleanup_scene_data_for_removed_entity(
    scene: &RwLock<Scene>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    remove_scene_graph_model_instance_node_for_entity(scene, entity_id, entity);

    light::cleanup_light_for_removed_entity(scene, entity_id, entity);

    camera::remove_camera_from_scene_for_removed_entity(scene, entity_id, entity);

    voxel::cleanup_voxel_object_for_removed_entity(scene, entity_id, entity);

    bounding_volume::cleanup_bounding_volume_for_removed_entity(scene, entity_id, entity);
}

fn setup_scene_graph_group_nodes_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let mut scene_graph = scene.scene_graph().owrite();
        },
        entities,
        |entity_id: EntityID,
         frame: Option<&ReferenceFrame>,
         parent: Option<&ParentEntity>|
         -> Result<()> {
            let frame = frame.copied().unwrap_or_default();
            let transform_to_parent_space = frame.create_transform_to_parent_space();

            let parent_entity_id = parent.map(|parent| parent.0);

            impact_scene::setup::setup_scene_graph_group_node(
                &mut scene_graph,
                entity_id,
                transform_to_parent_space.compact(),
                parent_entity_id,
            )
        },
        [CanBeParent]
    )
}

fn setup_scene_graph_model_instance_nodes_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let mut scene_graph = scene.scene_graph().owrite();
        },
        entities,
        |entity_id: EntityID,
         mesh_id: &TriangleMeshID,
         material_id: &MaterialID,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         parent: Option<&ParentEntity>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(HasModel, ModelTransform, SceneEntityFlags)> {
            let model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();

            let model_to_parent_transform = frame.create_transform_to_parent_space()
                * model_transform.create_transform_to_entity_space();

            let parent_entity_id = parent.map(|parent| parent.0);

            let has_independent_material_values =
                entities.has_component_type::<HasIndependentMaterialValues>();

            let flags = impact_scene::setup::setup_scene_graph_model_instance_node(
                &resource_manager.materials,
                &mut model_instance_manager,
                &mut scene_graph,
                entity_id,
                model_to_parent_transform.compact(),
                *mesh_id,
                *material_id,
                parent_entity_id,
                flags,
                has_independent_material_values,
            )?;

            Ok((HasModel, model_transform, flags))
        },
        ![HasModel]
    )
}

fn remove_scene_graph_model_instance_node_for_entity(
    scene: &RwLock<Scene>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    if entity.has_component::<HasModel>() {
        let scene = scene.oread();
        let mut model_instance_manager = scene.model_instance_manager().owrite();
        let mut scene_graph = scene.scene_graph().owrite();
        impact_scene::setup::remove_scene_graph_model_instance_node(
            &mut model_instance_manager,
            &mut scene_graph,
            entity_id,
        );
    }
}
