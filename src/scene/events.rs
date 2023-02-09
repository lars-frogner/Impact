//! Event handling related to scenes.

use crate::{
    geometry::PerspectiveCamera,
    physics::{OrientationComp, PositionComp},
    scene::{
        self, BlinnPhongMaterial, DiffuseTexturedBlinnPhongMaterial, FixedColorMaterial,
        FixedTextureMaterial, MaterialComp, MeshComp, ModelID, ModelInstanceNodeID, PointLight,
        ScalingComp, Scene, SceneGraphNodeComp, TexturedBlinnPhongMaterial, VertexColorMaterial,
    },
    window::{self, Window},
};
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use nalgebra::{Point3, UnitQuaternion};

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
    /// Performs any modifications to the scene required to accommodate a
    /// new entity with components represented by the given component manager,
    /// and adds any additional components to the entity's components.
    pub fn handle_entity_created(
        &self,
        window: &Window,
        components: &mut ArchetypeComponentStorage,
    ) -> Result<RenderResourcesDesynchronized> {
        let mut desynchronized = RenderResourcesDesynchronized::No;
        self.add_camera_component_for_entity(window, components, &mut desynchronized)?;
        self.add_light_component_for_entity(components, &mut desynchronized);
        self.add_material_component_for_entity(components, &mut desynchronized);
        self.add_model_instance_node_component_for_entity(components, &mut desynchronized);
        Ok(desynchronized)
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

    pub fn handle_window_resized(&self, new_size: (u32, u32)) -> RenderResourcesDesynchronized {
        if let Some(scene_camera) = self.scene_camera().write().unwrap().as_mut() {
            scene_camera.set_aspect_ratio(window::calculate_aspect_ratio(new_size.0, new_size.1));
        }
        RenderResourcesDesynchronized::Yes
    }

    fn add_camera_component_for_entity(
        &self,
        window: &Window,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        PerspectiveCamera::add_camera_to_scene_for_entity(
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
        PointLight::add_point_light_component_for_entity(
            self.scene_camera(),
            self.light_storage(),
            components,
            desynchronized,
        );
    }

    fn add_material_component_for_entity(
        &self,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        VertexColorMaterial::add_material_component_for_entity(components, desynchronized);

        FixedColorMaterial::add_material_component_for_entity(
            self.instance_feature_manager(),
            components,
            desynchronized,
        );

        FixedTextureMaterial::add_material_component_for_entity(
            self.material_library(),
            components,
            desynchronized,
        );

        BlinnPhongMaterial::add_material_component_for_entity(
            self.instance_feature_manager(),
            components,
            desynchronized,
        );

        DiffuseTexturedBlinnPhongMaterial::add_material_component_for_entity(
            self.instance_feature_manager(),
            self.material_library(),
            components,
            desynchronized,
        );

        TexturedBlinnPhongMaterial::add_material_component_for_entity(
            self.instance_feature_manager(),
            self.material_library(),
            components,
            desynchronized,
        );
    }

    fn add_model_instance_node_component_for_entity(
        &self,
        components: &mut ArchetypeComponentStorage,
        _desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            {
                let mesh_repository = self.mesh_repository().read().unwrap();
                let material_library = self.material_library().read().unwrap();
                let mut instance_feature_manager = self.instance_feature_manager().write().unwrap();
                let mut scene_graph = self.scene_graph().write().unwrap();
                let root_node_id = scene_graph.root_node_id();
            },
            components,
            |mesh: &MeshComp,
             material: &MaterialComp,
             position: Option<&PositionComp>,
             orientation: Option<&OrientationComp>,
             scaling: Option<&ScalingComp>|
             -> SceneGraphNodeComp::<ModelInstanceNodeID> {
                let model_id = ModelID::for_mesh_and_material(mesh.id, material.id);
                instance_feature_manager.register_instance(&material_library, model_id);

                let position = position.map_or_else(Point3::origin, |position| position.0.cast());
                let orientation = orientation
                    .map_or_else(UnitQuaternion::identity, |orientation| orientation.0.cast());
                let scaling = scaling.map_or_else(|| 1.0, |scaling| scaling.0);

                let model_to_world_transform =
                    scene::create_model_to_world_transform(position, orientation, scaling);

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
                    .compute_bounding_sphere()
                    .expect("Tried to create renderable entity with empty mesh");

                SceneGraphNodeComp::new(scene_graph.create_model_instance_node(
                    root_node_id,
                    model_to_world_transform,
                    model_id,
                    bounding_sphere,
                    feature_ids,
                ))
            },
            ![SceneGraphNodeComp::<ModelInstanceNodeID>]
        );
    }

    fn remove_camera_for_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        scene::camera::remove_camera_from_scene(
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
        PointLight::remove_light_from_storage(self.light_storage(), entity, desynchronized);
    }

    fn remove_material_features_for_entity(
        &self,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        if let Some(material) = entity.get_component::<MaterialComp>() {
            let feature_id = material.access().feature_id;

            if !feature_id.is_not_applicable() {
                self.instance_feature_manager()
                    .write()
                    .unwrap()
                    .get_storage_mut_for_feature_type_id(feature_id.feature_type_id())
                    .expect("Missing storage for material feature")
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
