//! Management of scene data for entities.

use crate::{
    assets::Assets,
    camera,
    gpu::{GraphicsDevice, rendering::RenderingSystem},
    light,
    material::{self, components::MaterialComp},
    mesh::{self, components::MeshComp},
    model::{
        InstanceFeature, ModelID,
        transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
    },
    physics::motion::components::ReferenceFrameComp,
    scene::{
        ModelInstanceNodeID, RenderResourcesDesynchronized, Scene, SceneEntityFlags,
        components::{
            ParentComp, SceneEntityFlagsComp, SceneGraphGroupComp, SceneGraphGroupNodeComp,
            SceneGraphModelInstanceNodeComp, SceneGraphNodeComp, SceneGraphParentNodeComp,
            UncullableComp,
        },
    },
    voxel,
    window::Window,
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    setup,
    world::{EntityEntry, World as ECSWorld},
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
        assets: &Assets,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
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
            assets,
            self.material_library(),
            self.instance_feature_manager(),
            components,
            desynchronized,
        );

        voxel::entity::setup_voxel_object_for_new_entity(&self.voxel_manager, components);

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
        window: &Window,
        renderer: &RwLock<RenderingSystem>,
        ecs_world: &RwLock<ECSWorld>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        Self::add_parent_group_node_component_for_new_entity(ecs_world, components);
        self.add_group_node_component_for_new_entity(components);

        camera::entity::add_camera_to_scene_for_new_entity(
            window,
            renderer,
            self.scene_graph(),
            self.scene_camera(),
            components,
            desynchronized,
        )?;

        self.add_model_instance_node_component_for_new_entity(components);

        voxel::entity::add_model_instance_node_component_for_new_voxel_object_entity(
            self.voxel_manager(),
            self.instance_feature_manager(),
            self.scene_graph(),
            components,
        );

        Ok(())
    }

    /// Performs any modifications required to clean up the scene when
    /// the given entity is removed.
    pub fn perform_cleanup_for_removed_entity(
        &self,
        entity: &EntityEntry<'_>,
    ) -> RenderResourcesDesynchronized {
        let mut desynchronized = RenderResourcesDesynchronized::No;

        self.remove_model_instance_node_for_entity(entity, &mut desynchronized);

        material::entity::cleanup_material_for_removed_entity(
            self.instance_feature_manager(),
            entity,
            &mut desynchronized,
        );

        light::entity::cleanup_light_for_removed_entity(
            self.light_storage(),
            entity,
            &mut desynchronized,
        );

        camera::entity::remove_camera_from_scene_for_removed_entity(
            self.scene_graph(),
            self.scene_camera(),
            entity,
            &mut desynchronized,
        );

        voxel::entity::cleanup_voxel_object_for_removed_entity(
            self.voxel_manager(),
            entity,
            &mut desynchronized,
        );

        desynchronized
    }

    fn add_parent_group_node_component_for_new_entity(
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

    fn add_model_instance_node_component_for_new_entity(
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
             parent: Option<&SceneGraphParentNodeComp>,
             flags: Option<&SceneEntityFlagsComp>|
             -> (SceneGraphModelInstanceNodeComp, SceneEntityFlagsComp) {
                let flags = flags.map_or_else(SceneEntityFlags::empty, |flags| flags.0);

                let model_id = ModelID::for_mesh_and_material(mesh.id, *material.material_handle());

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

                let model_to_parent_transform = frame
                    .cloned()
                    .unwrap_or_default()
                    .create_transform_to_parent_space();

                let mut feature_ids = Vec::with_capacity(4);

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

                // The first two features are expected to be the model-view transform and
                // model-light transforms, respectively
                feature_ids.push(model_view_transform_feature_id);
                feature_ids.push(model_light_transform_feature_id);

                if let Some(feature_id) = material.material_handle().material_property_feature_id()
                {
                    feature_ids.push(feature_id);
                }

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

                (
                    SceneGraphNodeComp::new(scene_graph.create_model_instance_node(
                        parent_node_id,
                        model_to_parent_transform,
                        model_id,
                        bounding_sphere,
                        feature_ids,
                        flags.into(),
                    )),
                    SceneEntityFlagsComp(flags),
                )
            },
            ![SceneGraphModelInstanceNodeComp]
        );
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
                .unregister_instance(&model_id);
            desynchronized.set_yes();
        }
    }
}
