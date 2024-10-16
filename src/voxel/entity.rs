//! Management of voxels for entities.

use crate::{
    application::Application,
    impl_InstanceFeature,
    material::{MaterialHandle, MaterialID},
    mesh::MeshID,
    model::{
        transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
        InstanceFeature, InstanceFeatureManager, ModelID,
    },
    physics::{
        fph,
        motion::components::{ReferenceFrameComp, VelocityComp},
    },
    scene::{
        components::{
            SceneGraphModelInstanceNodeComp, SceneGraphNodeComp, SceneGraphParentNodeComp,
            UncullableComp,
        },
        RenderResourcesDesynchronized, SceneGraph,
    },
    voxel::{
        chunks::{disconnection::DisconnectedVoxelObject, ChunkedVoxelObject},
        components::{
            GradientNoiseVoxelTypesComp, MultifractalNoiseModificationComp,
            MultiscaleSphereModificationComp, SameVoxelTypeComp, VoxelBoxComp,
            VoxelGradientNoisePatternComp, VoxelObjectComp, VoxelSphereComp,
        },
        generation::{
            BoxSDFGenerator, GradientNoiseSDFGenerator, GradientNoiseVoxelTypeGenerator,
            MultifractalNoiseSDFModifier, MultiscaleSphereSDFModifier, SDFGenerator,
            SDFVoxelGenerator, SameVoxelTypeGenerator, SphereSDFGenerator, VoxelTypeGenerator,
        },
        mesh::MeshedChunkedVoxelObject,
        voxel_types::VoxelTypeRegistry,
        VoxelManager, VoxelObjectID,
    },
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::{ComponentArray, SingleInstance},
    setup,
    world::{Entity, EntityEntry},
};
use impact_utils::hash64;
use std::sync::{LazyLock, RwLock};

pub static VOXEL_MODEL_ID: LazyLock<ModelID> = LazyLock::new(|| {
    ModelID::for_mesh_and_material(
        MeshID(hash64!("Voxel mesh")),
        MaterialHandle::new(MaterialID(hash64!("Voxel material")), None, None),
    )
});

pub fn setup_voxel_object_for_new_entity(
    voxel_manager: &RwLock<VoxelManager>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_box: &VoxelBoxComp,
         voxel_type: &SameVoxelTypeComp,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> VoxelObjectComp {
            let sdf_generator = BoxSDFGenerator::new(voxel_box.extents_in_voxels());
            let voxel_type_generator = SameVoxelTypeGenerator::new(voxel_type.voxel_type());

            let voxel_object = generate_voxel_object(
                voxel_box.voxel_extent,
                sdf_generator,
                voxel_type_generator,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
            .expect("Tried to generate object for empty voxel box");

            let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

            let voxel_object_id = voxel_manager.add_voxel_object(meshed_voxel_object);

            VoxelObjectComp { voxel_object_id }
        },
        ![VoxelObjectComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_sphere: &VoxelSphereComp,
         voxel_type: &SameVoxelTypeComp,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> VoxelObjectComp {
            let sdf_generator = SphereSDFGenerator::new(voxel_sphere.radius_in_voxels());
            let voxel_type_generator = SameVoxelTypeGenerator::new(voxel_type.voxel_type());

            let voxel_object = generate_voxel_object(
                voxel_sphere.voxel_extent,
                sdf_generator,
                voxel_type_generator,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
            .expect("Tried to generate object for empty voxel sphere");

            let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

            let voxel_object_id = voxel_manager.add_voxel_object(meshed_voxel_object);

            VoxelObjectComp { voxel_object_id }
        },
        ![VoxelObjectComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_noise_pattern: &VoxelGradientNoisePatternComp,
         voxel_type: &SameVoxelTypeComp,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> VoxelObjectComp {
            let sdf_generator = GradientNoiseSDFGenerator::new(
                voxel_noise_pattern.extents_in_voxels(),
                voxel_noise_pattern.noise_frequency,
                voxel_noise_pattern.noise_threshold,
                u32::try_from(voxel_noise_pattern.seed).unwrap(),
            );
            let voxel_type_generator = SameVoxelTypeGenerator::new(voxel_type.voxel_type());

            let voxel_object = generate_voxel_object(
                voxel_noise_pattern.voxel_extent,
                sdf_generator,
                voxel_type_generator,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
            .expect("Tried to generate object for empty voxel gradient noise pattern");

            let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

            let voxel_object_id = voxel_manager.add_voxel_object(meshed_voxel_object);

            VoxelObjectComp { voxel_object_id }
        },
        ![VoxelObjectComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_box: &VoxelBoxComp,
         voxel_types: &GradientNoiseVoxelTypesComp,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> VoxelObjectComp {
            let sdf_generator = BoxSDFGenerator::new(voxel_box.extents_in_voxels());
            let voxel_type_generator = GradientNoiseVoxelTypeGenerator::from_component(
                voxel_manager.voxel_type_registry(),
                voxel_types,
            );

            let voxel_object = generate_voxel_object(
                voxel_box.voxel_extent,
                sdf_generator,
                voxel_type_generator,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
            .expect("Tried to generate object for empty voxel box");

            let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

            let voxel_object_id = voxel_manager.add_voxel_object(meshed_voxel_object);

            VoxelObjectComp { voxel_object_id }
        },
        ![VoxelObjectComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_sphere: &VoxelSphereComp,
         voxel_types: &GradientNoiseVoxelTypesComp,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> VoxelObjectComp {
            let sdf_generator = SphereSDFGenerator::new(voxel_sphere.radius_in_voxels());
            let voxel_type_generator = GradientNoiseVoxelTypeGenerator::from_component(
                voxel_manager.voxel_type_registry(),
                voxel_types,
            );

            let voxel_object = generate_voxel_object(
                voxel_sphere.voxel_extent,
                sdf_generator,
                voxel_type_generator,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
            .expect("Tried to generate object for empty voxel sphere");

            let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

            let voxel_object_id = voxel_manager.add_voxel_object(meshed_voxel_object);

            VoxelObjectComp { voxel_object_id }
        },
        ![VoxelObjectComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_noise_pattern: &VoxelGradientNoisePatternComp,
         voxel_types: &GradientNoiseVoxelTypesComp,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> VoxelObjectComp {
            let sdf_generator = GradientNoiseSDFGenerator::new(
                voxel_noise_pattern.extents_in_voxels(),
                voxel_noise_pattern.noise_frequency,
                voxel_noise_pattern.noise_threshold,
                u32::try_from(voxel_noise_pattern.seed).unwrap(),
            );
            let voxel_type_generator = GradientNoiseVoxelTypeGenerator::from_component(
                voxel_manager.voxel_type_registry(),
                voxel_types,
            );

            let voxel_object = generate_voxel_object(
                voxel_noise_pattern.voxel_extent,
                sdf_generator,
                voxel_type_generator,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
            .expect("Tried to generate object for empty voxel gradient noise pattern");

            let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

            let voxel_object_id = voxel_manager.add_voxel_object(meshed_voxel_object);

            VoxelObjectComp { voxel_object_id }
        },
        ![VoxelObjectComp]
    );
}

fn generate_voxel_object(
    voxel_extent: f64,
    sdf_generator: impl SDFGenerator,
    voxel_type_generator: impl VoxelTypeGenerator,
    multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
    multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>,
) -> Option<ChunkedVoxelObject> {
    match (
        multiscale_sphere_modification,
        multifractal_noise_modification,
    ) {
        (Some(multiscale_sphere_modification), Some(multifractal_noise_modification)) => {
            let sdf_generator = MultiscaleSphereSDFModifier::new(
                sdf_generator,
                multiscale_sphere_modification.octaves,
                multiscale_sphere_modification.max_scale,
                multiscale_sphere_modification.persistence,
                multiscale_sphere_modification.inflation,
                multiscale_sphere_modification.smoothness,
                multiscale_sphere_modification.seed,
            );
            let sdf_generator = MultifractalNoiseSDFModifier::new(
                sdf_generator,
                multifractal_noise_modification.octaves,
                multifractal_noise_modification.frequency,
                multifractal_noise_modification.lacunarity,
                multifractal_noise_modification.persistence,
                multifractal_noise_modification.amplitude,
                u32::try_from(multifractal_noise_modification.seed).unwrap(),
            );
            let generator =
                SDFVoxelGenerator::new(voxel_extent, sdf_generator, voxel_type_generator);
            ChunkedVoxelObject::generate(&generator)
        }
        (Some(modification), None) => {
            let sdf_generator = MultiscaleSphereSDFModifier::new(
                sdf_generator,
                modification.octaves,
                modification.max_scale,
                modification.persistence,
                modification.inflation,
                modification.smoothness,
                modification.seed,
            );
            let generator =
                SDFVoxelGenerator::new(voxel_extent, sdf_generator, voxel_type_generator);
            ChunkedVoxelObject::generate(&generator)
        }
        (None, Some(modification)) => {
            let sdf_generator = MultifractalNoiseSDFModifier::new(
                sdf_generator,
                modification.octaves,
                modification.frequency,
                modification.lacunarity,
                modification.persistence,
                modification.amplitude,
                u32::try_from(modification.seed).unwrap(),
            );
            let generator =
                SDFVoxelGenerator::new(voxel_extent, sdf_generator, voxel_type_generator);
            ChunkedVoxelObject::generate(&generator)
        }
        (None, None) => {
            let generator =
                SDFVoxelGenerator::new(voxel_extent, sdf_generator, voxel_type_generator);
            ChunkedVoxelObject::generate(&generator)
        }
    }
}

pub fn add_model_instance_node_component_for_new_voxel_object_entity(
    voxel_manager: &RwLock<VoxelManager>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    scene_graph: &RwLock<SceneGraph<f32>>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let voxel_manager = voxel_manager.read().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            let mut scene_graph = scene_graph.write().unwrap();
        },
        components,
        |voxel_object: &VoxelObjectComp,
         frame: Option<&ReferenceFrameComp>,
         parent: Option<&SceneGraphParentNodeComp>|
         -> SceneGraphModelInstanceNodeComp {
            let voxel_object_id = voxel_object.voxel_object_id;

            let voxel_object = voxel_manager
                .get_voxel_object(voxel_object_id)
                .expect("Tried to create model instance node for missing voxel object")
                .object();

            let model_id = *VOXEL_MODEL_ID;

            instance_feature_manager.register_instance(
                model_id,
                &[
                    InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID,
                    InstanceModelLightTransform::FEATURE_TYPE_ID,
                    VoxelObjectID::FEATURE_TYPE_ID,
                ],
            );

            let model_to_parent_transform = frame
                .cloned()
                .unwrap_or_default()
                .create_transform_to_parent_space();

            // Add entries for the model-to-camera and model-to-light transforms
            // for the scene graph to access and modify using the returned IDs
            let model_view_transform_feature_id = instance_feature_manager
                .get_storage_mut::<InstanceModelViewTransformWithPrevious>()
                .expect("Missing storage for InstanceModelViewTransform feature")
                .add_feature(&InstanceModelViewTransformWithPrevious::default());

            let model_light_transform_feature_id = instance_feature_manager
                .get_storage_mut::<InstanceModelLightTransform>()
                .expect("Missing storage for InstanceModelLightTransform feature")
                .add_feature(&InstanceModelLightTransform::default());

            let voxel_object_id_feature_id = instance_feature_manager
                .get_storage_mut::<VoxelObjectID>()
                .expect("Missing storage for VoxelObjectID feature")
                .add_feature(&voxel_object_id);

            let bounding_sphere = if components.has_component_type::<UncullableComp>() {
                // The scene graph will not cull models with no bounding sphere
                None
            } else {
                Some(voxel_object.compute_bounding_sphere())
            };

            let parent_node_id =
                parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

            SceneGraphNodeComp::new(scene_graph.create_model_instance_node(
                parent_node_id,
                model_to_parent_transform,
                model_id,
                bounding_sphere,
                vec![
                    model_view_transform_feature_id,
                    model_light_transform_feature_id,
                    voxel_object_id_feature_id,
                ],
            ))
        },
        ![SceneGraphModelInstanceNodeComp]
    );
}

/// Checks if the given entity has a [`VoxelObjectComp`], and if so, removes the
/// assocated voxel object from the given [`VoxelManager`].
pub fn cleanup_voxel_object_for_removed_entity(
    voxel_manager: &RwLock<VoxelManager>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    if let Some(voxel_object) = entity.get_component::<VoxelObjectComp>() {
        let voxel_object = voxel_object.access();

        voxel_manager
            .write()
            .unwrap()
            .remove_voxel_object(voxel_object.voxel_object_id);

        desynchronized.set_yes();
    }
}

pub fn handle_emptied_voxel_objects(app: &Application) -> Result<()> {
    loop {
        let scene = app.scene().read().unwrap();
        let mut voxel_manager = scene.voxel_manager().write().unwrap();

        if let Some(entity) = voxel_manager.pop_empty_voxel_object_entity() {
            // We must release these locks before attempting to remove the entity, or we
            // will deadlock
            drop(voxel_manager);
            drop(scene);

            log::debug!("Removing entity for emptied voxel object");
            app.remove_entity(&entity)?;
        } else {
            break;
        }
    }
    Ok(())
}

pub fn handle_disconnected_voxel_objects(app: &Application) -> Result<()> {
    loop {
        let scene = app.scene().read().unwrap();
        let mut voxel_manager = scene.voxel_manager().write().unwrap();

        if let Some((parent_entity, disconnected_object)) =
            voxel_manager.pop_disconnected_voxel_object()
        {
            // We must release these locks before calling the inner function, or we will
            // deadlock
            drop(voxel_manager);
            drop(scene);

            handle_disconnected_voxel_object(app, parent_entity, disconnected_object)?;
        } else {
            break;
        }
    }
    Ok(())
}

fn handle_disconnected_voxel_object(
    app: &Application,
    parent_entity: Entity,
    disconnected_object: DisconnectedVoxelObject,
) -> Result<()> {
    let DisconnectedVoxelObject {
        object,
        origin_offset,
    } = disconnected_object;

    let voxel_extent = object.voxel_extent();
    let meshed_voxel_object = MeshedChunkedVoxelObject::create(object);

    let voxel_object_id = app
        .scene()
        .read()
        .unwrap()
        .voxel_manager()
        .write()
        .unwrap()
        .add_voxel_object(meshed_voxel_object);

    let ecs_world = app.ecs_world().read().unwrap();
    let Some(parent) = ecs_world.get_entity(&parent_entity) else {
        // If the parent entity has been removed, we discard the disconnected object
        return Ok(());
    };

    let mut components = ArchetypeComponentStorage::empty();

    components.add_new_component_type(VoxelObjectComp { voxel_object_id }.into_storage())?;

    if let Some(parent_reference_frame) = parent.get_component() {
        let parent_reference_frame: &ReferenceFrameComp = parent_reference_frame.access();
        let mut reference_frame = *parent_reference_frame;

        let origin_offset_in_voxel_object_space =
            origin_offset.map(|offset| offset as fph * voxel_extent);

        // Offset the reference frame of the new object compared to the frame of the
        // parent object to account for the origin difference
        reference_frame.position += parent_reference_frame
            .create_transform_to_parent_space()
            .transform_vector(&origin_offset_in_voxel_object_space.into());

        components.add_new_component_type(reference_frame.into_storage())?;

        if let Some(parent_velocity) = parent.get_component() {
            let parent_velocity: &VelocityComp = parent_velocity.access();
            // TODO: Handle angular velocity
            let velocity = VelocityComp::linear(parent_velocity.linear);
            components.add_new_component_type(velocity.into_storage())?;
        }
    }

    if let Some(scene_graph_parent) = parent.get_component() {
        let scene_graph_parent: &SceneGraphParentNodeComp = scene_graph_parent.access();
        components.add_new_component_type(scene_graph_parent.into_storage())?;
    }

    // Release all locks before attempting to create entity
    drop(parent);
    drop(ecs_world);

    app.create_entity(SingleInstance::new(components))?;

    Ok(())
}

impl_InstanceFeature!(
    VoxelObjectID,
    wgpu::vertex_attr_array![
        0 => Uint32,
    ]
);

impl GradientNoiseVoxelTypeGenerator {
    fn from_component(
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_types: &GradientNoiseVoxelTypesComp,
    ) -> Self {
        Self::new(
            voxel_types
                .voxel_types(voxel_type_registry)
                .expect("Invalid voxel types"),
            voxel_types.noise_frequency(),
            voxel_types.voxel_type_frequency(),
            u32::try_from(voxel_types.seed()).unwrap(),
        )
    }
}

pub fn register_voxel_feature_types(instance_feature_manager: &mut InstanceFeatureManager) {
    instance_feature_manager.register_feature_type::<VoxelObjectID>();
}
