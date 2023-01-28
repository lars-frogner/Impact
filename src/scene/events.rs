//! Event handling related to scenes.

use crate::{
    physics::{OrientationComp, PositionComp},
    scene::{
        self, BlinnPhongMaterial, CameraComp, CameraNodeID, DiffuseTexturedBlinnPhongMaterial,
        FixedColorMaterial, InstanceFeatureManager, MaterialComp, MeshComp, ModelID,
        ModelInstanceNodeID, Scene, SceneGraphNodeComp, TexturedBlinnPhongMaterial,
        VertexColorMaterial,
    },
};
use impact_ecs::{archetype::ComponentManager, setup, world::EntityEntry};

impl Scene {
    /// Performs any modifications to the scene required to accommodate a
    /// new entity with components represented by the given component manager,
    /// and adds any additional components to the entity's components.
    pub fn handle_entity_created(&self, component_manager: &mut ComponentManager<'_>) {
        let mut instance_feature_manager = self.instance_feature_manager().write().unwrap();
        let mut material_library = self.material_library().write().unwrap();

        VertexColorMaterial::add_material_component_for_entity(component_manager);

        FixedColorMaterial::add_material_component_for_entity(
            &mut instance_feature_manager,
            component_manager,
        );

        BlinnPhongMaterial::add_material_component_for_entity(
            &mut instance_feature_manager,
            component_manager,
        );

        DiffuseTexturedBlinnPhongMaterial::add_material_component_for_entity(
            &mut instance_feature_manager,
            &mut material_library,
            component_manager,
        );

        TexturedBlinnPhongMaterial::add_material_component_for_entity(
            &mut instance_feature_manager,
            &mut material_library,
            component_manager,
        );

        drop(material_library);
        drop(instance_feature_manager);

        self.add_camera_node_component_for_entity(component_manager);
        self.add_model_instance_node_component_for_entity(component_manager);
    }

    /// Performs any modifications required to clean up the scene when
    /// the given entity is removed.
    pub fn handle_entity_removed(&self, entity: &EntityEntry<'_>) {
        self.remove_camera_node_for_entity(entity);
        self.remove_model_instance_node_for_entity(entity);

        let mut instance_feature_manager = self.instance_feature_manager().write().unwrap();
        Self::remove_material_features_for_entity(&mut instance_feature_manager, entity);
        drop(instance_feature_manager);
    }

    fn add_camera_node_component_for_entity(&self, component_manager: &mut ComponentManager<'_>) {
        setup!(
            {
                let mut scene_graph = self.scene_graph().write().unwrap();
                let root_node_id = scene_graph.root_node_id();
            },
            component_manager,
            |camera: &CameraComp,
             position: &PositionComp,
             orientation: &OrientationComp|
             -> SceneGraphNodeComp::<CameraNodeID> {
                let camera_to_world_transform =
                    scene::model_to_world_transform_from_position_and_orientation(
                        position.0.cast(),
                        orientation.0.cast(),
                    );

                let node_id = scene_graph.create_camera_node(
                    root_node_id,
                    camera_to_world_transform,
                    camera.id,
                );

                self.set_active_camera(Some((camera.id, node_id)));

                SceneGraphNodeComp::new(node_id)
            }
        );
    }

    fn add_model_instance_node_component_for_entity(
        &self,
        component_manager: &mut ComponentManager<'_>,
    ) {
        setup!(
            {
                let mesh_repository = self.mesh_repository().read().unwrap();
                let material_library = self.material_library().read().unwrap();
                let mut instance_feature_manager = self.instance_feature_manager().write().unwrap();
                let mut scene_graph = self.scene_graph().write().unwrap();
                let root_node_id = scene_graph.root_node_id();
            },
            component_manager,
            |mesh: &MeshComp,
             material: &MaterialComp,
             position: &PositionComp,
             orientation: &OrientationComp|
             -> SceneGraphNodeComp::<ModelInstanceNodeID> {
                let model_id = ModelID::for_mesh_and_material(mesh.id, material.id);
                instance_feature_manager.register_instance(&material_library, model_id);

                let model_to_world_transform =
                    scene::model_to_world_transform_from_position_and_orientation(
                        position.0.cast(),
                        orientation.0.cast(),
                    );

                let feature_ids = if material.feature_id.is_not_applicable() {
                    Vec::new()
                } else {
                    vec![material.feature_id]
                };

                // Panic on errors since returning an error could leave us
                // in an inconsistent state
                let bounding_sphere = mesh_repository
                    .get_mesh(mesh.id)
                    .expect("Tried to create renderable entity with mesh not present in mesh repository")
                    .bounding_sphere()
                    .expect("Tried to create renderable entity with empty mesh");

                SceneGraphNodeComp::new(scene_graph.create_model_instance_node(
                    root_node_id,
                    model_to_world_transform,
                    model_id,
                    bounding_sphere,
                    feature_ids,
                ))
            }
        );
    }

    fn remove_material_features_for_entity(
        instance_feature_manager: &mut InstanceFeatureManager,
        entity: &EntityEntry<'_>,
    ) {
        if let Some(material) = entity.get_component::<MaterialComp>() {
            let feature_id = material.access().feature_id;

            if !feature_id.is_not_applicable() {
                instance_feature_manager
                    .get_storage_mut_for_feature_type_id(feature_id.feature_type_id())
                    .expect("Missing storage for material feature")
                    .remove_feature(feature_id);
            }
        }
    }

    fn remove_camera_node_for_entity(&self, entity: &EntityEntry<'_>) {
        if let Some(node) = entity.get_component::<SceneGraphNodeComp<CameraNodeID>>() {
            let node_id = node.access().id;

            self.scene_graph()
                .write()
                .unwrap()
                .remove_camera_node(node_id);

            if let Some(active_camera_node_id) = self.get_active_camera_node_id() {
                if active_camera_node_id == node_id {
                    self.set_active_camera(None);
                }
            }
        }
    }

    fn remove_model_instance_node_for_entity(&self, entity: &EntityEntry<'_>) {
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
        }
    }
}
