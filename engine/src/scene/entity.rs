//! Management of scene data for entities.

use crate::{
    camera::{self, entity::CameraRenderState},
    light, material, mesh,
    model::{InstanceFeatureManager, ModelID},
    physics::motion::components::ReferenceFrameComp,
    scene::{
        Scene, SceneEntityFlags, SceneGraph,
        components::{
            ParentComp, SceneEntityFlagsComp, SceneGraphGroupComp, SceneGraphGroupNodeComp,
            SceneGraphModelInstanceNodeComp, SceneGraphParentNodeComp, UncullableComp,
        },
        graph::NodeTransform,
    },
    voxel,
};
use anyhow::{Result, anyhow};
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    setup,
    world::{EntityEntry, World as ECSWorld},
};
use impact_gpu::device::GraphicsDevice;
use impact_material::{MaterialLibrary, MaterialTextureProvider, components::MaterialComp};
use impact_mesh::{MeshRepository, components::TriangleMeshComp};
use impact_model::{
    InstanceFeature,
    transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
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

                setup_parent_group_node(parent_entity)
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

                setup_group_node(&mut scene_graph, group_to_parent_transform, parent)
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

                setup_model_instance_node(
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
        remove_model_instance_node_for_entity(
            &self.instance_feature_manager,
            &self.scene_graph,
            entity,
            desynchronized,
        );
    }
}

fn setup_parent_group_node(parent_entity: EntityEntry<'_>) -> Result<SceneGraphParentNodeComp> {
    let parent_group_node = parent_entity
        .get_component::<SceneGraphGroupNodeComp>()
        .ok_or_else(|| {
            anyhow!(
                "Missing group node component for parent entity with ID {}",
                parent_entity.id()
            )
        })?;

    Ok(SceneGraphParentNodeComp::new(parent_group_node.access().id))
}

fn setup_group_node(
    scene_graph: &mut SceneGraph,
    group_to_parent_transform: NodeTransform,
    parent: Option<&SceneGraphParentNodeComp>,
) -> SceneGraphGroupNodeComp {
    let parent_node_id = parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

    SceneGraphGroupNodeComp::new(
        scene_graph.create_group_node(parent_node_id, group_to_parent_transform),
    )
}

fn setup_model_instance_node(
    mesh_repository: &MeshRepository,
    material_library: &MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    scene_graph: &mut SceneGraph,
    model_to_parent_transform: NodeTransform,
    mesh: &TriangleMeshComp,
    material: &MaterialComp,
    parent: Option<&SceneGraphParentNodeComp>,
    flags: Option<&SceneEntityFlagsComp>,
    uncullable: bool,
) -> Result<(SceneGraphModelInstanceNodeComp, SceneEntityFlagsComp)> {
    let flags = flags.map_or_else(SceneEntityFlags::empty, |flags| flags.0);

    let model_id = ModelID::for_mesh_and_material(mesh.id, *material.material_handle());

    let bounding_sphere = if uncullable {
        // The scene graph will not cull models with no bounding sphere
        None
    } else {
        Some(
            mesh_repository
                .get_triangle_mesh(mesh.id)
                .ok_or_else(|| {
                    anyhow!(
                        "Tried to create renderable entity with missing mesh (mesh ID {})",
                        mesh.id
                    )
                })?
                .compute_bounding_sphere()
                .ok_or_else(|| {
                    anyhow!(
                        "Tried to create renderable entity with empty mesh (mesh ID {})",
                        mesh.id
                    )
                })?,
        )
    };

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

    let mut feature_ids_for_rendering = Vec::with_capacity(4);

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

    // The first feature is expected to be the model-view transform
    feature_ids_for_rendering.push(model_view_transform_feature_id);

    if let Some(feature_id) = material.material_handle().material_property_feature_id() {
        feature_ids_for_rendering.push(feature_id);
    }

    let feature_ids_for_shadow_mapping = vec![model_light_transform_feature_id];

    let parent_node_id = parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

    Ok((
        SceneGraphModelInstanceNodeComp::new(scene_graph.create_model_instance_node(
            parent_node_id,
            model_to_parent_transform,
            model_id,
            bounding_sphere,
            feature_ids_for_rendering,
            feature_ids_for_shadow_mapping,
            flags.into(),
        )),
        SceneEntityFlagsComp(flags),
    ))
}

fn remove_model_instance_node_for_entity(
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    scene_graph: &RwLock<SceneGraph>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(node) = entity.get_component::<SceneGraphModelInstanceNodeComp>() {
        let model_id = scene_graph
            .write()
            .unwrap()
            .remove_model_instance_node(node.access().id);
        instance_feature_manager
            .write()
            .unwrap()
            .unregister_instance(&model_id);
        *desynchronized = true;
    }
}
