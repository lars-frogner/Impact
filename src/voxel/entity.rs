//! Management of voxels for entities.

use crate::{
    gpu::rendering::fre,
    impl_InstanceFeature,
    material::{MaterialHandle, MaterialID},
    mesh::MeshID,
    model::{
        transform::InstanceModelViewTransformWithPrevious, InstanceFeature, InstanceFeatureManager,
        ModelID,
    },
    physics::motion::components::ReferenceFrameComp,
    scene::{
        components::{
            SceneGraphModelInstanceNodeComp, SceneGraphNodeComp, SceneGraphParentNodeComp,
            UncullableComp,
        },
        SceneGraph,
    },
    voxel::{
        chunks::ChunkedVoxelObject,
        components::{
            VoxelBoxComp, VoxelGradientNoisePatternComp, VoxelObjectComp, VoxelSphereComp,
            VoxelTypeComp,
        },
        generation::{
            GradientNoiseVoxelGenerator, UniformBoxVoxelGenerator, UniformSphereVoxelGenerator,
        },
        VoxelManager, VoxelObjectID,
    },
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
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
        |voxel_box: &VoxelBoxComp, voxel_type: &VoxelTypeComp| -> VoxelObjectComp {
            let generator = UniformBoxVoxelGenerator::new(
                voxel_type.voxel_type(),
                voxel_type.voxel_extent(),
                voxel_box.size_x,
                voxel_box.size_y,
                voxel_box.size_z,
            );

            let voxel_object = ChunkedVoxelObject::generate(&generator)
                .expect("Tried to generate object for empty voxel box");

            let voxel_object_id = voxel_manager.add_voxel_object(voxel_object);

            VoxelObjectComp { voxel_object_id }
        },
        ![VoxelObjectComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_sphere: &VoxelSphereComp, voxel_type: &VoxelTypeComp| -> VoxelObjectComp {
            let generator = UniformSphereVoxelGenerator::new(
                voxel_type.voxel_type(),
                voxel_type.voxel_extent(),
                voxel_sphere.n_voxels_across(),
            );

            let voxel_object = ChunkedVoxelObject::generate(&generator)
                .expect("Tried to generate object for empty voxel sphere");

            let voxel_object_id = voxel_manager.add_voxel_object(voxel_object);

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
         voxel_type: &VoxelTypeComp|
         -> VoxelObjectComp {
            let generator = GradientNoiseVoxelGenerator::new(
                voxel_type.voxel_type(),
                voxel_type.voxel_extent(),
                voxel_noise_pattern.size_x,
                voxel_noise_pattern.size_y,
                voxel_noise_pattern.size_z,
                voxel_noise_pattern.noise_frequency,
                voxel_noise_pattern.noise_threshold,
                u32::try_from(voxel_noise_pattern.seed).unwrap(),
            );

            let voxel_object = ChunkedVoxelObject::generate(&generator)
                .expect("Tried to generate object for empty voxel gradient noise pattern");

            let voxel_object_id = voxel_manager.add_voxel_object(voxel_object);

            VoxelObjectComp { voxel_object_id }
        },
        ![VoxelObjectComp]
    );
}

pub fn add_model_instance_node_component_for_new_voxel_object_entity(
    voxel_manager: &RwLock<VoxelManager>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    scene_graph: &RwLock<SceneGraph<fre>>,
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
                .expect("Tried to create model instance node for missing voxel object");

            let model_id = *VOXEL_MODEL_ID;

            instance_feature_manager.register_instance(
                model_id,
                &[
                    InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID,
                    VoxelObjectID::FEATURE_TYPE_ID,
                ],
            );

            let model_to_parent_transform = frame
                .cloned()
                .unwrap_or_default()
                .create_transform_to_parent_space();

            // Add an entry for the model-to-camera transform for the scene
            // graph to access and modify using the returned ID
            let model_view_transform_feature_id = instance_feature_manager
                .get_storage_mut::<InstanceModelViewTransformWithPrevious>()
                .expect("Missing storage for InstanceModelViewTransform feature")
                .add_feature(&InstanceModelViewTransformWithPrevious::default());

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
                vec![model_view_transform_feature_id, voxel_object_id_feature_id],
            ))
        },
        ![SceneGraphModelInstanceNodeComp]
    );
}

impl_InstanceFeature!(
    VoxelObjectID,
    wgpu::vertex_attr_array![
        0 => Uint32,
    ]
);

pub fn register_voxel_feature_types(instance_feature_manager: &mut InstanceFeatureManager) {
    instance_feature_manager.register_feature_type::<VoxelObjectID>();
}
