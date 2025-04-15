//! Management of voxels for entities.

use crate::{
    engine::Engine,
    impl_InstanceFeature,
    material::{MaterialHandle, MaterialID},
    mesh::MeshID,
    model::{
        InstanceFeature, InstanceFeatureManager, ModelID,
        transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
    },
    physics::{
        inertia::InertialProperties,
        motion::components::{ReferenceFrameComp, VelocityComp},
        rigid_body::{RigidBody, components::RigidBodyComp},
    },
    scene::{
        RenderResourcesDesynchronized, SceneEntityFlags, SceneGraph,
        components::{
            SceneEntityFlagsComp, SceneGraphModelInstanceNodeComp, SceneGraphParentNodeComp,
            UncullableComp,
        },
    },
    voxel::{
        VoxelManager, VoxelObjectID, VoxelObjectManager,
        chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyManager},
        components::{
            GradientNoiseVoxelTypesComp, MultifractalNoiseModificationComp,
            MultiscaleSphereModificationComp, SameVoxelTypeComp, VoxelBoxComp,
            VoxelGradientNoisePatternComp, VoxelObjectComp, VoxelSphereComp, VoxelSphereUnionComp,
        },
        generation::{
            BoxSDFGenerator, GradientNoiseSDFGenerator, GradientNoiseVoxelTypeGenerator,
            MultifractalNoiseSDFModifier, MultiscaleSphereSDFModifier, SDFGenerator, SDFUnion,
            SDFVoxelGenerator, SameVoxelTypeGenerator, SphereSDFGenerator, VoxelTypeGenerator,
        },
        mesh::MeshedChunkedVoxelObject,
        voxel_types::VoxelTypeRegistry,
    },
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::{ComponentArray, SingleInstance},
    setup,
    world::EntityEntry,
};
use impact_utils::hash64;
use std::sync::{LazyLock, RwLock};

use super::StagedVoxelObject;

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
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> (
            VoxelObjectComp,
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp
        ) {
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

            let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
                &voxel_object,
                voxel_manager.type_registry.mass_densities(),
            );

            let (rigid_body, frame, velocity) = setup_rigid_body_for_new_voxel_object(
                inertial_property_manager.derive_inertial_properties(),
                frame,
                velocity,
            );

            let voxel_object_id = mesh_and_store_voxel_object(
                &mut voxel_manager.object_manager,
                voxel_object,
                inertial_property_manager,
            );

            (
                VoxelObjectComp { voxel_object_id },
                rigid_body,
                frame,
                velocity,
            )
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
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> (
            VoxelObjectComp,
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp
        ) {
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

            let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
                &voxel_object,
                voxel_manager.type_registry.mass_densities(),
            );

            let (rigid_body, frame, velocity) = setup_rigid_body_for_new_voxel_object(
                inertial_property_manager.derive_inertial_properties(),
                frame,
                velocity,
            );

            let voxel_object_id = mesh_and_store_voxel_object(
                &mut voxel_manager.object_manager,
                voxel_object,
                inertial_property_manager,
            );

            (
                VoxelObjectComp { voxel_object_id },
                rigid_body,
                frame,
                velocity,
            )
        },
        ![VoxelObjectComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_sphere_union: &VoxelSphereUnionComp,
         voxel_type: &SameVoxelTypeComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> (
            VoxelObjectComp,
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp
        ) {
            let sdf_generator_1 = SphereSDFGenerator::new(voxel_sphere_union.radius_1_in_voxels());
            let sdf_generator_2 = SphereSDFGenerator::new(voxel_sphere_union.radius_2_in_voxels());
            let sdf_generator = SDFUnion::new(
                sdf_generator_1,
                sdf_generator_2,
                voxel_sphere_union.center_offsets,
                voxel_sphere_union.smoothness,
            );
            let voxel_type_generator = SameVoxelTypeGenerator::new(voxel_type.voxel_type());

            let voxel_object = generate_voxel_object(
                voxel_sphere_union.voxel_extent,
                sdf_generator,
                voxel_type_generator,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
            .expect("Tried to generate object for empty voxel sphere");

            let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
                &voxel_object,
                voxel_manager.type_registry.mass_densities(),
            );

            let (rigid_body, frame, velocity) = setup_rigid_body_for_new_voxel_object(
                inertial_property_manager.derive_inertial_properties(),
                frame,
                velocity,
            );

            let voxel_object_id = mesh_and_store_voxel_object(
                &mut voxel_manager.object_manager,
                voxel_object,
                inertial_property_manager,
            );

            (
                VoxelObjectComp { voxel_object_id },
                rigid_body,
                frame,
                velocity,
            )
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
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> (
            VoxelObjectComp,
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp
        ) {
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

            let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
                &voxel_object,
                voxel_manager.type_registry.mass_densities(),
            );

            let (rigid_body, frame, velocity) = setup_rigid_body_for_new_voxel_object(
                inertial_property_manager.derive_inertial_properties(),
                frame,
                velocity,
            );

            let voxel_object_id = mesh_and_store_voxel_object(
                &mut voxel_manager.object_manager,
                voxel_object,
                inertial_property_manager,
            );

            (
                VoxelObjectComp { voxel_object_id },
                rigid_body,
                frame,
                velocity,
            )
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
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> (
            VoxelObjectComp,
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp
        ) {
            let sdf_generator = BoxSDFGenerator::new(voxel_box.extents_in_voxels());
            let voxel_type_generator = GradientNoiseVoxelTypeGenerator::from_component(
                &voxel_manager.type_registry,
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

            let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
                &voxel_object,
                voxel_manager.type_registry.mass_densities(),
            );

            let (rigid_body, frame, velocity) = setup_rigid_body_for_new_voxel_object(
                inertial_property_manager.derive_inertial_properties(),
                frame,
                velocity,
            );

            let voxel_object_id = mesh_and_store_voxel_object(
                &mut voxel_manager.object_manager,
                voxel_object,
                inertial_property_manager,
            );

            (
                VoxelObjectComp { voxel_object_id },
                rigid_body,
                frame,
                velocity,
            )
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
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> (
            VoxelObjectComp,
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp
        ) {
            let sdf_generator = SphereSDFGenerator::new(voxel_sphere.radius_in_voxels());
            let voxel_type_generator = GradientNoiseVoxelTypeGenerator::from_component(
                &voxel_manager.type_registry,
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

            let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
                &voxel_object,
                voxel_manager.type_registry.mass_densities(),
            );

            let (rigid_body, frame, velocity) = setup_rigid_body_for_new_voxel_object(
                inertial_property_manager.derive_inertial_properties(),
                frame,
                velocity,
            );

            let voxel_object_id = mesh_and_store_voxel_object(
                &mut voxel_manager.object_manager,
                voxel_object,
                inertial_property_manager,
            );

            (
                VoxelObjectComp { voxel_object_id },
                rigid_body,
                frame,
                velocity,
            )
        },
        ![VoxelObjectComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_sphere_union: &VoxelSphereUnionComp,
         voxel_types: &GradientNoiseVoxelTypesComp,
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> (
            VoxelObjectComp,
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp
        ) {
            let sdf_generator_1 = SphereSDFGenerator::new(voxel_sphere_union.radius_1_in_voxels());
            let sdf_generator_2 = SphereSDFGenerator::new(voxel_sphere_union.radius_2_in_voxels());
            let sdf_generator = SDFUnion::new(
                sdf_generator_1,
                sdf_generator_2,
                voxel_sphere_union.center_offsets,
                voxel_sphere_union.smoothness,
            );
            let voxel_type_generator = GradientNoiseVoxelTypeGenerator::from_component(
                &voxel_manager.type_registry,
                voxel_types,
            );

            let voxel_object = generate_voxel_object(
                voxel_sphere_union.voxel_extent,
                sdf_generator,
                voxel_type_generator,
                multiscale_sphere_modification,
                multifractal_noise_modification,
            )
            .expect("Tried to generate object for empty voxel sphere union");

            let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
                &voxel_object,
                voxel_manager.type_registry.mass_densities(),
            );

            let (rigid_body, frame, velocity) = setup_rigid_body_for_new_voxel_object(
                inertial_property_manager.derive_inertial_properties(),
                frame,
                velocity,
            );

            let voxel_object_id = mesh_and_store_voxel_object(
                &mut voxel_manager.object_manager,
                voxel_object,
                inertial_property_manager,
            );

            (
                VoxelObjectComp { voxel_object_id },
                rigid_body,
                frame,
                velocity,
            )
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
         frame: Option<&ReferenceFrameComp>,
         velocity: Option<&VelocityComp>,
         multiscale_sphere_modification: Option<&MultiscaleSphereModificationComp>,
         multifractal_noise_modification: Option<&MultifractalNoiseModificationComp>|
         -> (
            VoxelObjectComp,
            RigidBodyComp,
            ReferenceFrameComp,
            VelocityComp
        ) {
            let sdf_generator = GradientNoiseSDFGenerator::new(
                voxel_noise_pattern.extents_in_voxels(),
                voxel_noise_pattern.noise_frequency,
                voxel_noise_pattern.noise_threshold,
                u32::try_from(voxel_noise_pattern.seed).unwrap(),
            );
            let voxel_type_generator = GradientNoiseVoxelTypeGenerator::from_component(
                &voxel_manager.type_registry,
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

            let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
                &voxel_object,
                voxel_manager.type_registry.mass_densities(),
            );

            let (rigid_body, frame, velocity) = setup_rigid_body_for_new_voxel_object(
                inertial_property_manager.derive_inertial_properties(),
                frame,
                velocity,
            );

            let voxel_object_id = mesh_and_store_voxel_object(
                &mut voxel_manager.object_manager,
                voxel_object,
                inertial_property_manager,
            );

            (
                VoxelObjectComp { voxel_object_id },
                rigid_body,
                frame,
                velocity,
            )
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

fn setup_rigid_body_for_new_voxel_object(
    inertial_properties: InertialProperties,
    frame: Option<&ReferenceFrameComp>,
    velocity: Option<&VelocityComp>,
) -> (RigidBodyComp, ReferenceFrameComp, VelocityComp) {
    let mut frame = frame.cloned().unwrap_or_default();

    assert_eq!(
        frame.scaling, 1.0,
        "Scaling is not supported for voxel objects"
    );

    let velocity = velocity.cloned().unwrap_or_default();

    // Use center of mass as new origin, since all free rotation is
    // about the center of mass
    frame.origin_offset = inertial_properties.center_of_mass().coords;

    let rigid_body = RigidBody::new(
        inertial_properties,
        frame.orientation,
        frame.scaling,
        &velocity.linear,
        &velocity.angular,
    );

    (RigidBodyComp(rigid_body), frame, velocity)
}

fn mesh_and_store_voxel_object(
    voxel_object_manager: &mut VoxelObjectManager,
    voxel_object: ChunkedVoxelObject,
    inertial_property_manager: VoxelObjectInertialPropertyManager,
) -> VoxelObjectID {
    let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

    let voxel_object_id = voxel_object_manager.add_voxel_object(meshed_voxel_object);

    voxel_object_manager
        .add_inertial_property_manager_for_voxel_object(voxel_object_id, inertial_property_manager);

    voxel_object_id
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
         parent: Option<&SceneGraphParentNodeComp>,
         flags: Option<&SceneEntityFlagsComp>|
         -> (SceneGraphModelInstanceNodeComp, SceneEntityFlagsComp) {
            let flags = flags.map_or_else(SceneEntityFlags::empty, |flags| flags.0);

            let voxel_object_id = voxel_object.voxel_object_id;

            let voxel_object = voxel_manager
                .object_manager
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

            (
                SceneGraphModelInstanceNodeComp::new(scene_graph.create_model_instance_node(
                    parent_node_id,
                    model_to_parent_transform,
                    model_id,
                    bounding_sphere,
                    vec![
                        model_view_transform_feature_id,
                        model_light_transform_feature_id,
                        voxel_object_id_feature_id,
                    ],
                    flags.into(),
                )),
                SceneEntityFlagsComp(flags),
            )
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
            .object_manager
            .remove_voxel_object(voxel_object.voxel_object_id);

        desynchronized.set_yes();
    }
}

pub fn handle_staged_voxel_objects(engine: &Engine) -> Result<()> {
    loop {
        let scene = engine.scene().read().unwrap();
        let mut voxel_manager = scene.voxel_manager().write().unwrap();

        if let Some(StagedVoxelObject {
            object,
            inertial_property_manager,
            mut components,
        }) = voxel_manager.object_manager.pop_staged_voxel_object()
        {
            let meshed_voxel_object = MeshedChunkedVoxelObject::create(object);

            let voxel_object_id = voxel_manager
                .object_manager
                .add_voxel_object(meshed_voxel_object);

            if let Some(inertial_property_manager) = inertial_property_manager.clone() {
                voxel_manager
                    .object_manager
                    .add_inertial_property_manager_for_voxel_object(
                        voxel_object_id,
                        inertial_property_manager,
                    );
            }

            components
                .add_new_component_type(VoxelObjectComp { voxel_object_id }.into_storage())?;

            // We must release these locks before attempting to create the entity, or we
            // will deadlock
            drop(voxel_manager);
            drop(scene);

            engine.create_entity(SingleInstance::new(components))?;
        } else {
            break;
        }
    }
    Ok(())
}

pub fn handle_emptied_voxel_objects(engine: &Engine) -> Result<()> {
    loop {
        let scene = engine.scene().read().unwrap();
        let mut voxel_manager = scene.voxel_manager().write().unwrap();

        if let Some(entity) = voxel_manager.object_manager.pop_empty_voxel_object_entity() {
            // We must release these locks before attempting to remove the entity, or we
            // will deadlock
            drop(voxel_manager);
            drop(scene);

            log::debug!("Removing entity for emptied voxel object");
            engine.remove_entity(&entity)?;
        } else {
            break;
        }
    }
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
