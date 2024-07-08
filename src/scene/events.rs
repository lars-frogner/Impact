//! Event handling related to scenes.

use crate::{
    assets::Assets,
    camera,
    gpu::{rendering::fre, GraphicsDevice},
    light,
    material::{self, components::MaterialComp, MaterialHandle},
    mesh::{self, components::MeshComp},
    model::ModelID,
    physics::ReferenceFrameComp,
    scene::{
        ModelInstanceNodeID, ParentComp, Scene, SceneGraphGroupComp, SceneGraphGroupNodeComp,
        SceneGraphModelInstanceNodeComp, SceneGraphNodeComp, SceneGraphParentNodeComp,
        UncullableComp, VoxelManager, VoxelTreeComp, VoxelTreeNodeComp, VoxelTypeComp,
    },
    window::{self, Window},
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    setup,
    world::{EntityEntry, World as ECSWorld},
};
use num_traits::FromPrimitive;
use std::{num::NonZeroU32, sync::RwLock};

/// Indicates whether an event caused the render resources to go out of sync
/// with its source scene data.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderResourcesDesynchronized {
    Yes,
    No,
}

impl RenderResourcesDesynchronized {
    pub fn is_yes(&self) -> bool {
        *self == Self::Yes
    }

    pub fn set_yes(&mut self) {
        *self = Self::Yes;
    }
}

impl Scene {
    /// Performs any modifications to the scene required to accommodate a new
    /// entity with components represented by the given component manager, and
    /// adds any additional components to the entity's components (except scene
    /// graph components, which are added by calling
    /// [`add_entity_to_scene_graph`](Self::add_entity_to_scene_graph)).
    pub fn handle_entity_created(
        &self,
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        self.add_mesh_component_for_entity(components, desynchronized)?;
        self.add_light_component_for_entity(components, desynchronized);
        self.add_material_component_for_entity(graphics_device, assets, components, desynchronized);

        self.add_voxel_tree_component_for_entity(components);

        self.generate_missing_vertex_properties_for_mesh(components);

        Ok(())
    }

    /// Adds the entity to the scene graph if required, and adds the
    /// corresponding scene graph components to the entity.
    pub fn add_entity_to_scene_graph(
        &self,
        window: &Window,
        ecs_world: &RwLock<ECSWorld>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        Self::add_parent_group_node_component_for_entity(ecs_world, components);
        self.add_group_node_component_for_entity(components);
        self.add_camera_component_for_entity(window, components, desynchronized)?;
        self.add_model_instance_node_component_for_entity(components);
        self.add_voxel_tree_node_component_for_entity(components);
        Ok(())
    }

    /// Performs any modifications required to clean up the scene when
    /// the given entity is removed.
    pub fn handle_entity_removed(&self, entity: &EntityEntry<'_>) -> RenderResourcesDesynchronized {
        let mut desynchronized = RenderResourcesDesynchronized::No;

        self.remove_model_instance_node_for_entity(entity, &mut desynchronized);
        self.remove_material_features_for_entity(entity, &mut desynchronized);
        self.remove_light_for_entity(entity, &mut desynchronized);
        self.remove_camera_for_entity(entity, &mut desynchronized);

        desynchronized
    }

    pub fn handle_window_resized(
        &self,
        _old_width: NonZeroU32,
        old_height: NonZeroU32,
        new_width: NonZeroU32,
        new_height: NonZeroU32,
    ) -> RenderResourcesDesynchronized {
        if let Some(scene_camera) = self.scene_camera().write().unwrap().as_mut() {
            scene_camera.set_aspect_ratio(window::calculate_aspect_ratio(new_width, new_height));
        }

        self.voxel_manager()
            .write()
            .unwrap()
            .scale_min_angular_voxel_extent_for_lod(
                fre::from_u32(old_height.into()).unwrap()
                    / fre::from_u32(new_height.into()).unwrap(),
            );

        RenderResourcesDesynchronized::Yes
    }

    fn add_mesh_component_for_entity(
        &self,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        mesh::entity::add_mesh_component_for_entity(
            self.mesh_repository(),
            components,
            desynchronized,
        )
    }

    fn add_camera_component_for_entity(
        &self,
        window: &Window,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        camera::entity::add_perspective_camera_to_scene_for_entity(
            window,
            self.scene_graph(),
            self.scene_camera(),
            components,
            desynchronized,
        )?;
        camera::entity::add_orthographic_camera_to_scene_for_entity(
            window,
            self.scene_graph(),
            self.scene_camera(),
            components,
            desynchronized,
        )
    }

    fn add_light_component_for_entity(
        &self,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        light::entity::add_ambient_light_component_for_entity(
            self.light_storage(),
            components,
            desynchronized,
        );
        light::entity::add_omnidirectional_light_component_for_entity(
            self.scene_camera(),
            self.light_storage(),
            components,
            desynchronized,
        );
        light::entity::add_unidirectional_light_component_for_entity(
            self.scene_camera(),
            self.light_storage(),
            components,
            desynchronized,
        );
    }

    fn add_material_component_for_entity(
        &self,
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        material::entity::add_material_component_for_entity(
            graphics_device,
            assets,
            self.material_library(),
            self.instance_feature_manager(),
            components,
            desynchronized,
        );
    }

    fn add_voxel_tree_component_for_entity(&self, components: &mut ArchetypeComponentStorage) {
        VoxelManager::add_voxel_tree_component_for_entity(
            &self.voxel_manager,
            components,
            self.config.voxel_extent,
        );
    }

    fn add_parent_group_node_component_for_entity(
        ecs_world: &RwLock<ECSWorld>,
        components: &mut ArchetypeComponentStorage,
    ) {
        setup!(
            {
                let ecs_world = ecs_world.read().unwrap();
            },
            components,
            |parent: &ParentComp| -> SceneGraphParentNodeComp {
                let parent_entity = ecs_world
                    .get_entity(&parent.entity)
                    .expect("Missing parent entity");

                let parent_group_node = parent_entity
                    .get_component::<SceneGraphGroupNodeComp>()
                    .expect("Missing group node component for parent entity");

                SceneGraphParentNodeComp::new(parent_group_node.access().id)
            },
            ![SceneGraphParentNodeComp]
        );
    }

    fn add_group_node_component_for_entity(&self, components: &mut ArchetypeComponentStorage) {
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

                let parent_node_id =
                    parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

                SceneGraphNodeComp::new(
                    scene_graph.create_group_node(parent_node_id, group_to_parent_transform),
                )
            },
            [SceneGraphGroupComp],
            ![SceneGraphGroupNodeComp]
        );
    }

    fn add_model_instance_node_component_for_entity(
        &self,
        components: &mut ArchetypeComponentStorage,
    ) {
        setup!(
            {
                let mesh_repository = self.mesh_repository().read().unwrap();
                let material_library = self.material_library().read().unwrap();
                let mut instance_feature_manager = self.instance_feature_manager().write().unwrap();
                let mut scene_graph = self.scene_graph().write().unwrap();
            },
            components,
            |mesh: &MeshComp,
             material: &MaterialComp,
             frame: Option<&ReferenceFrameComp>,
             parent: Option<&SceneGraphParentNodeComp>|
             -> SceneGraphModelInstanceNodeComp {
                let model_id = ModelID::for_mesh_and_material(
                    mesh.id,
                    *material.material_handle(),
                    material.prepass_material_handle().cloned(),
                );
                instance_feature_manager.register_instance(&material_library, model_id);

                let model_to_parent_transform = frame
                    .cloned()
                    .unwrap_or_default()
                    .create_transform_to_parent_space();

                let feature_ids = match (
                    material.material_handle().material_property_feature_id(),
                    material
                        .prepass_material_handle()
                        .and_then(MaterialHandle::material_property_feature_id),
                ) {
                    (None, None) => Vec::new(),
                    (Some(feature_id), None) | (None, Some(feature_id)) => {
                        vec![feature_id]
                    }
                    (Some(feature_id), Some(prepass_feature_id)) => {
                        assert_eq!(
                            prepass_feature_id, feature_id,
                            "Prepass material must use the same feature as main material"
                        );
                        vec![feature_id]
                    }
                };

                let bounding_sphere = if components.has_component_type::<UncullableComp>() {
                    // The scene graph will not cull models with no bounding sphere
                    None
                } else {
                    // Panic on errors since returning an error could leave us
                    // in an inconsistent state
                    Some(mesh_repository
                        .get_mesh(mesh.id)
                        .expect("Tried to create renderable entity with mesh not present in mesh repository")
                        .compute_bounding_sphere()
                        .expect("Tried to create renderable entity with empty mesh"))
                };

                let parent_node_id =
                    parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

                SceneGraphNodeComp::new(scene_graph.create_model_instance_node(
                    parent_node_id,
                    model_to_parent_transform,
                    model_id,
                    bounding_sphere,
                    feature_ids,
                ))
            },
            ![SceneGraphModelInstanceNodeComp]
        );
    }

    fn add_voxel_tree_node_component_for_entity(&self, components: &mut ArchetypeComponentStorage) {
        setup!(
            {
                let voxel_manager = self.voxel_manager().read().unwrap();
                let mut scene_graph = self.scene_graph().write().unwrap();
            },
            components,
            |voxel_tree: &VoxelTreeComp,
             voxel_type: &VoxelTypeComp,
             frame: Option<&ReferenceFrameComp>,
             parent: Option<&SceneGraphParentNodeComp>|
             -> VoxelTreeNodeComp {
                let voxel_tree_id = voxel_tree.voxel_tree_id;
                let voxel_tree = voxel_manager
                    .get_voxel_tree(voxel_tree_id)
                    .expect(
                    "Tried to create voxel tree node entity with voxel tree not present in voxel manager",
                );

                let voxel_tree_to_parent_transform = frame
                    .cloned()
                    .unwrap_or_default()
                    .create_transform_to_parent_space();

                let voxel_tree_bounding_sphere = voxel_tree.compute_bounding_sphere(0);

                let appearance = voxel_manager.voxel_appearance(voxel_type.voxel_type());

                let parent_node_id =
                    parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

                let group_node_id =
                    scene_graph.create_group_node(parent_node_id, voxel_tree_to_parent_transform);

                let voxel_tree_node_id = scene_graph.create_voxel_tree_node(
                    group_node_id,
                    appearance.model_id,
                    voxel_tree_id,
                    voxel_tree_bounding_sphere,
                );

                VoxelTreeNodeComp::new(voxel_tree_id, group_node_id, voxel_tree_node_id)
            },
            ![VoxelTreeNodeComp]
        );
    }

    fn generate_missing_vertex_properties_for_mesh(&self, components: &ArchetypeComponentStorage) {
        mesh::entity::generate_missing_vertex_properties_for_material(
            self.mesh_repository(),
            &self.material_library().read().unwrap(),
            components,
        );
    }

    fn remove_camera_for_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        camera::entity::remove_camera_from_scene(
            self.scene_graph(),
            self.scene_camera(),
            entity,
            desynchronized,
        );
    }

    fn remove_light_for_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        light::entity::remove_ambient_light_from_storage_for_entity(
            self.light_storage(),
            entity,
            desynchronized,
        );
        light::entity::remove_omnidirectional_light_from_storage_for_entity(
            self.light_storage(),
            entity,
            desynchronized,
        );
        light::entity::remove_unidirectional_light_from_storage_for_entity(
            self.light_storage(),
            entity,
            desynchronized,
        );
    }

    fn remove_material_features_for_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        if let Some(material) = entity.get_component::<MaterialComp>() {
            let material = material.access();

            if let Some(feature_id) = material.material_handle().material_property_feature_id() {
                self.instance_feature_manager()
                    .write()
                    .unwrap()
                    .get_storage_mut_for_feature_type_id(feature_id.feature_type_id())
                    .expect("Missing storage for material feature")
                    .remove_feature(feature_id);
                desynchronized.set_yes();
            }

            if let Some(feature_id) = material
                .prepass_material_handle()
                .and_then(MaterialHandle::material_property_feature_id)
            {
                self.instance_feature_manager()
                    .write()
                    .unwrap()
                    .get_storage_mut_for_feature_type_id(feature_id.feature_type_id())
                    .expect("Missing storage for prepass material feature")
                    .remove_feature(feature_id);
                desynchronized.set_yes();
            }
        }
    }

    fn remove_model_instance_node_for_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        if let Some(node) = entity.get_component::<SceneGraphNodeComp<ModelInstanceNodeID>>() {
            let model_id = self
                .scene_graph()
                .write()
                .unwrap()
                .remove_model_instance_node(node.access().id);
            self.instance_feature_manager()
                .write()
                .unwrap()
                .unregister_instance(model_id);
            desynchronized.set_yes();
        }
    }
}
