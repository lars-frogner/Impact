//! Setup and cleanup of scene data for new and removed entities.

pub mod camera;
pub mod light;
pub mod material;
pub mod mesh;
pub mod voxel;

use crate::{
    lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager, scene::Scene,
};
use anyhow::{Result, anyhow};
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    setup,
    world::{EntityEntry, World as ECSWorld},
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_material::MaterialID;
use impact_mesh::TriangleMeshID;
use impact_scene::{
    SceneEntityFlags, SceneGraphGroupNodeHandle, SceneGraphModelInstanceNodeHandle,
    SceneGraphParentNodeHandle,
    setup::{SceneGraphGroup, SceneParent, Uncullable},
};
use parking_lot::RwLock;

/// Performs any modifications to the scene required to accommodate new
/// entities with the given components, and adds any additional components to
/// the entities' components (except scene graph components, which are added
/// by calling [`add_new_entities_to_scene_graph`].
pub fn setup_scene_data_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    simulator: &RwLock<PhysicsSimulator>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    mesh::setup_meshes_for_new_entities(resource_manager, components)?;

    light::setup_lights_for_new_entities(scene, components);

    material::setup_materials_for_new_entities(resource_manager, components)?;

    voxel::setup_voxel_objects_for_new_entities(resource_manager, scene, simulator, components)?;
    voxel::setup_voxel_interaction_for_new_entities(scene, components);

    mesh::generate_missing_vertex_properties_for_new_entity_meshes(resource_manager, components);

    Ok(())
}

/// Adds the new entities with the given components to the scene graph if
/// required, and adds the corresponding scene graph components to the entities'
/// components.
pub fn add_new_entities_to_scene_graph(
    ecs_world: &RwLock<ECSWorld>,
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup_scene_graph_parent_nodes_for_new_entities(ecs_world, components)?;
    setup_scene_graph_group_nodes_for_new_entities(scene, components);

    camera::add_camera_to_scene_for_new_entity(scene, components)?;

    setup_scene_graph_model_instance_nodes_for_new_entities(resource_manager, scene, components)?;

    voxel::setup_scene_graph_model_instance_nodes_for_new_voxel_object_entities(scene, components)?;

    Ok(())
}

/// Performs any modifications required to clean up the scene when
/// the given entity is removed.
pub fn cleanup_scene_data_for_removed_entity(scene: &RwLock<Scene>, entity: &EntityEntry<'_>) {
    remove_scene_graph_model_instance_node_for_entity(scene, entity);

    light::cleanup_light_for_removed_entity(scene, entity);

    camera::remove_camera_from_scene_for_removed_entity(scene, entity);

    voxel::cleanup_voxel_object_for_removed_entity(scene, entity);
}

fn setup_scene_graph_parent_nodes_for_new_entities(
    ecs_world: &RwLock<ECSWorld>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let ecs_world = ecs_world.oread();
        },
        components,
        |parent: &SceneParent| -> Result<SceneGraphParentNodeHandle> {
            let parent_entity = ecs_world
                .get_entity(parent.entity_id)
                .ok_or_else(|| anyhow!("Missing parent entity with ID {}", parent.entity_id))?;

            impact_scene::setup::setup_scene_graph_parent_node(parent_entity)
        },
        ![SceneGraphParentNodeHandle]
    )
}

fn setup_scene_graph_group_nodes_for_new_entities(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let scene = scene.oread();
            let mut scene_graph = scene.scene_graph().owrite();
        },
        components,
        |frame: Option<&ReferenceFrame>,
         parent: Option<&SceneGraphParentNodeHandle>|
         -> SceneGraphGroupNodeHandle {
            let frame = frame.copied().unwrap_or_default();
            let transform_to_parent_space = frame.create_transform_to_parent_space();

            impact_scene::setup::setup_scene_graph_group_node(
                &mut scene_graph,
                transform_to_parent_space.compact(),
                parent,
            )
        },
        [SceneGraphGroup],
        ![SceneGraphGroupNodeHandle]
    );
}

fn setup_scene_graph_model_instance_nodes_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let mut scene_graph = scene.scene_graph().owrite();
        },
        components,
        |mesh_id: &TriangleMeshID,
         material_id: &MaterialID,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         parent: Option<&SceneGraphParentNodeHandle>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            SceneGraphModelInstanceNodeHandle,
            ModelTransform,
            SceneEntityFlags
        )> {
            let model_transform = model_transform.copied().unwrap_or_default();
            let frame = frame.copied().unwrap_or_default();

            let model_to_parent_transform = frame.create_transform_to_parent_space()
                * model_transform.create_transform_to_entity_space();

            let uncullable = components.has_component_type::<Uncullable>();

            let (node_handle, flags) = impact_scene::setup::setup_scene_graph_model_instance_node(
                &resource_manager.triangle_meshes,
                &resource_manager.materials,
                &mut model_instance_manager,
                &mut scene_graph,
                model_to_parent_transform.compact(),
                *mesh_id,
                *material_id,
                parent,
                flags,
                uncullable,
            )?;

            Ok((node_handle, model_transform, flags))
        },
        ![SceneGraphModelInstanceNodeHandle]
    )
}

fn remove_scene_graph_model_instance_node_for_entity(
    scene: &RwLock<Scene>,
    entity: &EntityEntry<'_>,
) {
    if let Some(node) = entity.get_component::<SceneGraphModelInstanceNodeHandle>() {
        let scene = scene.oread();
        let mut model_instance_manager = scene.model_instance_manager().owrite();
        let mut scene_graph = scene.scene_graph().owrite();
        impact_scene::setup::remove_scene_graph_model_instance_node(
            &mut model_instance_manager,
            &mut scene_graph,
            node.access(),
        );
    }
}
