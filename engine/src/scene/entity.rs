//! Management of scene data for entities.

use crate::{
    camera::{self, entity::CameraRenderState},
    light, material, mesh,
    physics::motion::components::ReferenceFrameComp,
    scene::Scene,
    voxel,
};
use anyhow::{Result, anyhow};
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    setup,
    world::{EntityEntry, World as ECSWorld},
};
use impact_gpu::device::GraphicsDevice;
use impact_material::{MaterialTextureProvider, components::MaterialComp};
use impact_mesh::components::TriangleMeshComp;
use impact_scene::components::{
    ParentComp, SceneEntityFlagsComp, SceneGraphGroupComp, SceneGraphGroupNodeComp,
    SceneGraphModelInstanceNodeComp, SceneGraphParentNodeComp, UncullableComp,
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

        voxel::entity::setup_voxel_object_for_new_entity(&self.voxel_manager, components)?;

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

        impact_material::entity::cleanup_material_for_removed_entity(
            self.instance_feature_manager(),
            entity,
            desynchronized,
        );

        impact_light::entity::cleanup_light_for_removed_entity(
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
            |parent: &ParentComp| -> Result<SceneGraphParentNodeComp> {
                let parent_entity = ecs_world
                    .get_entity(parent.entity_id)
                    .ok_or_else(|| anyhow!("Missing parent entity with ID {}", parent.entity_id))?;

                impact_scene::entity::setup_parent_group_node(parent_entity)
            },
            ![SceneGraphParentNodeComp]
        )
    }

    fn add_group_node_component_for_new_entity(&self, components: &mut ArchetypeComponentStorage) {
        setup!(
            {
                let mut scene_graph = self.scene_graph().write().unwrap();
            },
            components,
            |frame: Option<&ReferenceFrameComp>,
             parent: Option<&SceneGraphParentNodeComp>|
             -> SceneGraphGroupNodeComp {
                let group_to_parent_transform = frame
                    .cloned()
                    .unwrap_or_default()
                    .create_transform_to_parent_space();

                impact_scene::entity::setup_group_node(
                    &mut scene_graph,
                    group_to_parent_transform,
                    parent,
                )
            },
            [SceneGraphGroupComp],
            ![SceneGraphGroupNodeComp]
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
            |mesh: &TriangleMeshComp,
             material: &MaterialComp,
             frame: Option<&ReferenceFrameComp>,
             parent: Option<&SceneGraphParentNodeComp>,
             flags: Option<&SceneEntityFlagsComp>|
             -> Result<(SceneGraphModelInstanceNodeComp, SceneEntityFlagsComp)> {
                let model_to_parent_transform = frame
                    .cloned()
                    .unwrap_or_default()
                    .create_transform_to_parent_space();

                let uncullable = components.has_component_type::<UncullableComp>();

                impact_scene::entity::setup_model_instance_node(
                    &mesh_repository,
                    &material_library,
                    &mut instance_feature_manager,
                    &mut scene_graph,
                    model_to_parent_transform,
                    mesh,
                    material,
                    parent,
                    flags,
                    uncullable,
                )
            },
            ![SceneGraphModelInstanceNodeComp]
        )
    }

    fn remove_model_instance_node_for_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut bool,
    ) {
        impact_scene::entity::remove_model_instance_node_for_entity(
            &self.instance_feature_manager,
            &self.scene_graph,
            entity,
            desynchronized,
        );
    }
}
