//! Management of scene data for entities.

use crate::{
    camera::{self, entity::CameraRenderState},
    light, material, mesh,
    scene::Scene,
    voxel,
};
use anyhow::{Result, anyhow};
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

impl Scene {
    /// Performs any modifications to the scene required to accommodate a new
    /// entity with the given components, and adds any additional components to
    /// the entity's components (except scene graph components, which are added
    /// by calling [`Self::add_new_entity_to_scene_graph`].
    pub fn perform_setup_for_new_entity(
        &self,
        graphics_device: &GraphicsDevice,
        texture_provider: &impl MaterialTextureProvider,
        rigid_body_manager: &RwLock<RigidBodyManager>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut bool,
    ) -> Result<()> {
        mesh::entity::setup_mesh_for_new_entity(
            self.mesh_repository(),
            components,
            desynchronized,
        )?;

        light::entity::setup_light_for_new_entity(
            self.scene_camera(),
            self.light_storage(),
            components,
            desynchronized,
        );

        material::entity::setup_material_for_new_entity(
            graphics_device,
            texture_provider,
            self.material_library(),
            self.instance_feature_manager(),
            components,
            desynchronized,
        )?;

        voxel::entity::setup_voxel_object_for_new_entity(
            rigid_body_manager,
            self.voxel_manager(),
            components,
        )?;

        mesh::entity::generate_missing_vertex_properties_for_new_entity_mesh(
            self.mesh_repository(),
            &self.material_library().read().unwrap(),
            components,
        );

        Ok(())
    }

    /// Adds the new entity with the given components to the scene graph if
    /// required, and adds the corresponding scene graph components to the
    /// entity's components.
    pub fn add_new_entity_to_scene_graph(
        &self,
        ecs_world: &RwLock<ECSWorld>,
        get_render_state: &mut impl FnMut() -> CameraRenderState,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut bool,
    ) -> Result<()> {
        Self::add_parent_group_node_component_for_new_entity(ecs_world, components)?;
        self.add_group_node_component_for_new_entity(components);

        camera::entity::add_camera_to_scene_for_new_entity(
            self.scene_graph(),
            self.scene_camera(),
            get_render_state,
            components,
            desynchronized,
        )?;

        self.add_model_instance_node_component_for_new_entity(components)?;

        voxel::entity::add_model_instance_node_component_for_new_voxel_object_entity(
            self.voxel_manager(),
            self.instance_feature_manager(),
            self.scene_graph(),
            components,
        )?;

        Ok(())
    }

    /// Performs any modifications required to clean up the scene when
    /// the given entity is removed.
    pub fn perform_cleanup_for_removed_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut bool,
    ) {
        self.remove_model_instance_node_for_entity(entity, desynchronized);

        impact_material::setup::cleanup_material_for_removed_entity(
            self.instance_feature_manager(),
            entity,
            desynchronized,
        );

        impact_light::setup::cleanup_light_for_removed_entity(
            self.light_storage(),
            entity,
            desynchronized,
        );

        camera::entity::remove_camera_from_scene_for_removed_entity(
            self.scene_graph(),
            self.scene_camera(),
            entity,
            desynchronized,
        );

        voxel::entity::cleanup_voxel_object_for_removed_entity(
            self.voxel_manager(),
            entity,
            desynchronized,
        );
    }

    fn add_parent_group_node_component_for_new_entity(
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

                impact_scene::setup::setup_parent_group_node(parent_entity)
            },
            ![SceneGraphParentNodeHandle]
        )
    }

    fn add_group_node_component_for_new_entity(&self, components: &mut ArchetypeComponentStorage) {
        setup!(
            {
                let mut scene_graph = self.scene_graph().write().unwrap();
            },
            components,
            |frame: Option<&ReferenceFrame>,
             parent: Option<&SceneGraphParentNodeHandle>|
             -> SceneGraphGroupNodeHandle {
                let frame = frame.copied().unwrap_or_default();

                impact_scene::setup::setup_group_node(
                    &mut scene_graph,
                    frame.create_transform_to_parent_space(),
                    parent,
                )
            },
            [SceneGraphGroup],
            ![SceneGraphGroupNodeHandle]
        );
    }

    fn add_model_instance_node_component_for_new_entity(
        &self,
        components: &mut ArchetypeComponentStorage,
    ) -> Result<()> {
        setup!(
            {
                let mesh_repository = self.mesh_repository().read().unwrap();
                let material_library = self.material_library().read().unwrap();
                let mut instance_feature_manager = self.instance_feature_manager().write().unwrap();
                let mut scene_graph = self.scene_graph().write().unwrap();
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

                let (node_handle, flags) = impact_scene::setup::setup_model_instance_node(
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

    fn remove_model_instance_node_for_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut bool,
    ) {
        impact_scene::setup::remove_model_instance_node_for_entity(
            &self.instance_feature_manager,
            &self.scene_graph,
            entity,
            desynchronized,
        );
    }
}
