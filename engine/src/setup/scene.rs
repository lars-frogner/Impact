//! Setup and cleanup of scene data for new and removed entities.

pub mod camera;
pub mod light;
pub mod material;
pub mod mesh;
pub mod voxel;

use crate::scene::Scene;
use anyhow::{Result, anyhow};
use camera::CameraRenderState;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    setup,
    world::{EntityEntry, World as ECSWorld},
};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_gpu::device::GraphicsDevice;
use impact_material::{MaterialHandle, MaterialTextureProvider};
use impact_mesh::TriangleMeshID;
use impact_physics::rigid_body::RigidBodyManager;
use impact_scene::{
    SceneEntityFlags, SceneGraphGroupNodeHandle, SceneGraphModelInstanceNodeHandle,
    SceneGraphParentNodeHandle,
    setup::{Parent, SceneGraphGroup, Uncullable},
};
use std::sync::RwLock;

/// Performs any modifications to the scene required to accommodate new
/// entities with the given components, and adds any additional components to
/// the entities' components (except scene graph components, which are added
/// by calling [`add_new_entities_to_scene_graph`].
pub fn setup_scene_data_for_new_entities(
    scene: &Scene,
    graphics_device: &GraphicsDevice,
    texture_provider: &impl MaterialTextureProvider,
    rigid_body_manager: &RwLock<RigidBodyManager>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    mesh::setup_meshes_for_new_entities(scene.mesh_repository(), components, desynchronized)?;

    light::setup_lights_for_new_entities(
        scene.scene_camera(),
        scene.light_storage(),
        components,
        desynchronized,
    );

    material::setup_materials_for_new_entities(
        graphics_device,
        texture_provider,
        scene.material_library(),
        scene.instance_feature_manager(),
        components,
        desynchronized,
    )?;

    voxel::setup_voxel_objects_for_new_entities(
        rigid_body_manager,
        scene.voxel_manager(),
        components,
    )?;

    mesh::generate_missing_vertex_properties_for_new_entity_meshes(
        scene.mesh_repository(),
        &scene.material_library().read().unwrap(),
        components,
    );

    Ok(())
}

/// Adds the new entities with the given components to the scene graph if
/// required, and adds the corresponding scene graph components to the entities'
/// components.
pub fn add_new_entities_to_scene_graph(
    scene: &Scene,
    ecs_world: &RwLock<ECSWorld>,
    get_render_state: &mut impl FnMut() -> CameraRenderState,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    setup_scene_graph_parent_nodes_for_new_entities(ecs_world, components)?;
    setup_scene_graph_group_nodes_for_new_entities(scene, components);

    camera::add_camera_to_scene_for_new_entity(
        scene.scene_graph(),
        scene.scene_camera(),
        get_render_state,
        components,
        desynchronized,
    )?;

    setup_scene_graph_model_instance_nodes_for_new_entities(scene, components)?;

    voxel::setup_scene_graph_model_instance_nodes_for_new_voxel_object_entities(
        scene.voxel_manager(),
        scene.instance_feature_manager(),
        scene.scene_graph(),
        components,
    )?;

    Ok(())
}

/// Performs any modifications required to clean up the scene when
/// the given entity is removed.
pub fn cleanup_scene_data_for_removed_entity(
    scene: &Scene,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    remove_scene_graph_model_instance_node_for_entity(scene, entity, desynchronized);

    impact_material::setup::cleanup_material_for_removed_entity(
        scene.instance_feature_manager(),
        entity,
        desynchronized,
    );

    impact_light::setup::cleanup_light_for_removed_entity(
        scene.light_storage(),
        entity,
        desynchronized,
    );

    camera::remove_camera_from_scene_for_removed_entity(
        scene.scene_graph(),
        scene.scene_camera(),
        entity,
        desynchronized,
    );

    impact_voxel::setup::cleanup_voxel_object_for_removed_entity(
        scene.voxel_manager(),
        entity,
        desynchronized,
    );
}

fn setup_scene_graph_parent_nodes_for_new_entities(
    ecs_world: &RwLock<ECSWorld>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let ecs_world = ecs_world.read().unwrap();
        },
        components,
        |parent: &Parent| -> Result<SceneGraphParentNodeHandle> {
            let parent_entity = ecs_world
                .get_entity(parent.entity_id)
                .ok_or_else(|| anyhow!("Missing parent entity with ID {}", parent.entity_id))?;

            impact_scene::setup::setup_scene_graph_parent_node(parent_entity)
        },
        ![SceneGraphParentNodeHandle]
    )
}

fn setup_scene_graph_group_nodes_for_new_entities(
    scene: &Scene,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut scene_graph = scene.scene_graph().write().unwrap();
        },
        components,
        |frame: Option<&ReferenceFrame>,
         parent: Option<&SceneGraphParentNodeHandle>|
         -> SceneGraphGroupNodeHandle {
            let frame = frame.copied().unwrap_or_default();

            impact_scene::setup::setup_scene_graph_group_node(
                &mut scene_graph,
                frame.create_transform_to_parent_space(),
                parent,
            )
        },
        [SceneGraphGroup],
        ![SceneGraphGroupNodeHandle]
    );
}

fn setup_scene_graph_model_instance_nodes_for_new_entities(
    scene: &Scene,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let mesh_repository = scene.mesh_repository().read().unwrap();
            let material_library = scene.material_library().read().unwrap();
            let mut instance_feature_manager = scene.instance_feature_manager().write().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();
        },
        components,
        |mesh_id: &TriangleMeshID,
         material: &MaterialHandle,
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
                * model_transform.crate_transform_to_entity_space();

            let uncullable = components.has_component_type::<Uncullable>();

            let (node_handle, flags) = impact_scene::setup::setup_scene_graph_model_instance_node(
                &mesh_repository,
                &material_library,
                &mut instance_feature_manager,
                &mut scene_graph,
                model_to_parent_transform,
                mesh_id,
                material,
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
    scene: &Scene,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    impact_scene::setup::remove_scene_graph_model_instance_node_for_entity(
        scene.instance_feature_manager(),
        scene.scene_graph(),
        entity,
        desynchronized,
    );
}
