//! Event handling related to scenes.

use crate::{
    geometry::{OrthographicCamera, PerspectiveCamera, TriangleMesh},
    physics::{OrientationComp, PositionComp},
    scene::{
        self, add_blinn_phong_material_component_for_entity,
        add_microfacet_material_component_for_entity, AmbientLight, FixedColorMaterial,
        FixedTextureMaterial, MaterialComp, MaterialHandle, MeshComp, ModelID, ModelInstanceNodeID,
        OmnidirectionalLight, ParentComp, ScalingComp, Scene, SceneGraphCameraNodeComp,
        SceneGraphGroup, SceneGraphGroupNodeComp, SceneGraphModelInstanceNodeComp,
        SceneGraphNodeComp, SceneGraphParentNodeComp, UnidirectionalLight, VertexColorMaterial,
    },
    window::{self, Window},
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    setup,
    world::{EntityEntry, World as ECSWorld},
};
use nalgebra::{Point3, UnitQuaternion};
use std::sync::RwLock;

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
        ecs_world: &RwLock<ECSWorld>,
        components: &mut ArchetypeComponentStorage,
    ) -> Result<RenderResourcesDesynchronized> {
        let mut desynchronized = RenderResourcesDesynchronized::No;

        self.add_mesh_component_for_entity(components, &mut desynchronized)?;
        self.add_camera_component_for_entity(window, components, &mut desynchronized)?;
        self.add_light_component_for_entity(components, &mut desynchronized);
        self.add_material_component_for_entity(components, &mut desynchronized);

        self.add_parent_group_node_component_for_entity(ecs_world, components);
        self.add_group_node_component_for_entity(components);
        self.add_model_instance_node_component_for_entity(components);

        self.generate_missing_vertex_properties_for_mesh(components);

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

    fn add_mesh_component_for_entity(
        &self,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        TriangleMesh::add_mesh_component_for_entity(
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
        PerspectiveCamera::add_camera_to_scene_for_entity(
            window,
            self.scene_graph(),
            self.scene_camera(),
            components,
            desynchronized,
        )?;
        OrthographicCamera::add_camera_to_scene_for_entity(
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
        AmbientLight::add_ambient_light_component_for_entity(
            self.light_storage(),
            components,
            desynchronized,
        );
        OmnidirectionalLight::add_omnidirectional_light_component_for_entity(
            self.scene_camera(),
            self.light_storage(),
            components,
            desynchronized,
        );
        UnidirectionalLight::add_unidirectional_light_component_for_entity(
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

        add_blinn_phong_material_component_for_entity(
            self.material_library(),
            self.instance_feature_manager(),
            components,
            desynchronized,
        );

        add_microfacet_material_component_for_entity(
            self.material_library(),
            self.instance_feature_manager(),
            components,
            desynchronized,
        );
    }

    fn add_parent_group_node_component_for_entity(
        &self,
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
            ![
                SceneGraphParentNodeComp,
                SceneGraphGroupNodeComp,
                SceneGraphCameraNodeComp,
                SceneGraphModelInstanceNodeComp
            ]
        );
    }

    fn add_group_node_component_for_entity(&self, components: &mut ArchetypeComponentStorage) {
        setup!(
            {
                let mut scene_graph = self.scene_graph().write().unwrap();
            },
            components,
            |position: Option<&PositionComp>,
             orientation: Option<&OrientationComp>,
             scaling: Option<&ScalingComp>,
             parent: Option<&SceneGraphParentNodeComp>|
             -> SceneGraphGroupNodeComp {
                let position = position.map_or_else(Point3::origin, |position| position.0.cast());
                let orientation = orientation
                    .map_or_else(UnitQuaternion::identity, |orientation| orientation.0.cast());
                let scaling = scaling.map_or_else(|| 1.0, |scaling| scaling.0);

                let group_to_parent_transform =
                    scene::create_child_to_parent_transform(position, orientation, scaling);

                let parent_node_id =
                    parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

                SceneGraphNodeComp::new(
                    scene_graph.create_group_node(parent_node_id, group_to_parent_transform),
                )
            },
            [SceneGraphGroup],
            ![
                SceneGraphGroupNodeComp,
                SceneGraphCameraNodeComp,
                SceneGraphModelInstanceNodeComp
            ]
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
             position: Option<&PositionComp>,
             orientation: Option<&OrientationComp>,
             scaling: Option<&ScalingComp>,
             parent: Option<&SceneGraphParentNodeComp>|
             -> SceneGraphModelInstanceNodeComp {
                let model_id = ModelID::for_mesh_and_material(
                    mesh.id,
                    *material.material_handle(),
                    material.prepass_material_handle().cloned(),
                );
                instance_feature_manager.register_instance(&material_library, model_id);

                let position = position.map_or_else(Point3::origin, |position| position.0.cast());
                let orientation = orientation
                    .map_or_else(UnitQuaternion::identity, |orientation| orientation.0.cast());
                let scaling = scaling.map_or_else(|| 1.0, |scaling| scaling.0);

                let model_to_parent_transform =
                    scene::create_child_to_parent_transform(position, orientation, scaling);

                let mut feature_ids = Vec::with_capacity(2);

                // The main material feature comes first, followed by the
                // prepass material feature (this order is also assumed
                // elsewhere)
                if let Some(feature_id) = material.material_handle().material_property_feature_id()
                {
                    feature_ids.push(feature_id);
                }
                if let Some(feature_id) = material
                    .prepass_material_handle()
                    .and_then(MaterialHandle::material_property_feature_id)
                {
                    feature_ids.push(feature_id);
                }

                // Panic on errors since returning an error could leave us
                // in an inconsistent state
                let bounding_sphere = mesh_repository
                    .get_mesh(mesh.id)
                    .expect("Tried to create renderable entity with mesh not present in mesh repository")
                    .compute_bounding_sphere()
                    .expect("Tried to create renderable entity with empty mesh");

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
            ![
                SceneGraphGroupNodeComp,
                SceneGraphCameraNodeComp,
                SceneGraphModelInstanceNodeComp
            ]
        );
    }

    fn generate_missing_vertex_properties_for_mesh(&self, components: &ArchetypeComponentStorage) {
        TriangleMesh::generate_missing_vertex_properties_for_material(
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
        AmbientLight::remove_light_from_storage(self.light_storage(), entity, desynchronized);
        OmnidirectionalLight::remove_light_from_storage(
            self.light_storage(),
            entity,
            desynchronized,
        );
        UnidirectionalLight::remove_light_from_storage(
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
