//! Setup of voxel objects.

use crate::{
    VoxelObjectID, VoxelObjectManager, VoxelObjectPhysicsContext,
    chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyManager},
    generation::{
        ChunkedVoxelGenerator, VoxelGeneratorID,
        sdf::{SDFGraph, SDFNode, SDFNodeID},
        voxel_type::{GradientNoiseVoxelTypeGenerator, SameVoxelTypeGenerator},
    },
    gpu_resource::VOXEL_MODEL_ID,
    mesh::MeshedChunkedVoxelObject,
    voxel_types::{VoxelType, VoxelTypeRegistry},
};
use anyhow::{Result, anyhow, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::Allocator;
use impact_geometry::{AxisAlignedBoxC, ModelTransform, ReferenceFrame};
use impact_id::EntityID;
use impact_intersection::bounding_volume::{BoundingVolumeID, BoundingVolumeManager};
use impact_math::{hash::Hash32, point::Point3C, vector::Vector3C};
use impact_model::{
    InstanceFeature, ModelInstanceID,
    transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
};
use impact_physics::{
    inertia::InertialProperties,
    quantities::Motion,
    rigid_body::{self, RigidBodyManager},
};
use impact_scene::{
    ParentEntity, SceneEntityFlags,
    graph::{FeatureIDSet, SceneGraph, SceneGroupID},
    model::ModelInstanceManager,
};
use roc_integration::roc;

define_setup_type! {
    /// A generated voxel object.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct GeneratedVoxelObject {
        pub generator_id: VoxelGeneratorID,
        pub voxel_extent: f32,
        pub scale_factor: f32,
        pub seed: u64,
    }
}

define_setup_type! {
    /// A voxel type that is the only type present in a voxel object.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SameVoxelType {
        voxel_type_name_hash: Hash32,
    }
}

define_setup_type! {
    /// A set of voxel types distributed according to a gradient noise pattern.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct GradientNoiseVoxelTypes {
        n_voxel_types: u32,
        voxel_type_name_hashes: [Hash32; GradientNoiseVoxelTypes::VOXEL_TYPE_ARRAY_SIZE],
        noise_frequency: f32,
        voxel_type_frequency: f32,
        pub seed: u32,
    }
}

define_setup_type! {
    /// A modification of a voxel signed distance field based on multifractal
    /// noise.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct MultifractalNoiseSDFModification {
        pub octaves: u32,
        pub frequency: f32,
        pub lacunarity: f32,
        pub persistence: f32,
        pub amplitude: f32,
        pub seed: u32,
    }
}

define_setup_type! {
    /// An object made of voxels in a box configuration.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelBox {
        /// The extent of a single voxel.
        pub voxel_extent: f32,
        /// The number of voxels along the box in the x-direction.
        pub extent_x: f32,
        /// The number of voxels along the box in the y-direction.
        pub extent_y: f32,
        /// The number of voxels along the box in the z-direction.
        pub extent_z: f32,
    }
}

define_setup_type! {
    /// An object made of voxels in a spherical configuration.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelSphere {
        /// The extent of a single voxel.
        pub voxel_extent: f32,
        /// The number of voxels along the radius of the sphere.
        pub radius: f32,
    }
}

define_setup_type! {
    /// An object made of voxels in a configuration described by the smooth
    /// union of two spheres.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelSphereUnion {
        /// The extent of a single voxel.
        pub voxel_extent: f32,
        /// The number of voxels along the radius of the first sphere.
        pub radius_1: f32,
        /// The number of voxels along the radius of the second sphere.
        pub radius_2: f32,
        /// The offset in number of voxels in each dimension between the centers of
        /// the two spheres.
        pub center_offsets: Vector3C,
        /// The smoothness of the union operation.
        pub smoothness: f32,
    }
}

define_setup_type! {
    /// A voxel object with dynamic voxels will behave like a dynamic rigid body
    /// and respond to voxel absorption.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DynamicVoxels;
}

#[roc(dependencies = [impact_math::hash::Hash64])]
impl GeneratedVoxelObject {
    #[roc(
        body = "{ generator_id: Hashing.hash_str_64(generator_name), voxel_extent, scale_factor, seed }"
    )]
    pub fn new(generator_name: &str, voxel_extent: f32, scale_factor: f32, seed: u64) -> Self {
        Self {
            generator_id: VoxelGeneratorID::from_name(generator_name),
            voxel_extent,
            scale_factor,
            seed,
        }
    }
}

#[roc]
impl SameVoxelType {
    #[roc(body = "{ voxel_type_name_hash: Hashing.hash_str_32(voxel_type_name) }")]
    pub fn new(voxel_type_name: &str) -> Self {
        Self {
            voxel_type_name_hash: Hash32::from_str(voxel_type_name),
        }
    }

    pub fn voxel_type(&self, voxel_type_registry: &VoxelTypeRegistry) -> Result<VoxelType> {
        voxel_type_registry
            .voxel_type_for_name_hash(self.voxel_type_name_hash)
            .ok_or_else(|| anyhow!("Missing voxel type for name in `SameVoxelType`"))
    }

    pub fn create_generator(
        &self,
        voxel_type_registry: &VoxelTypeRegistry,
    ) -> Result<SameVoxelTypeGenerator> {
        Ok(SameVoxelTypeGenerator::new(
            self.voxel_type(voxel_type_registry)?,
        ))
    }
}

#[roc(dependencies=[usize])]
impl GradientNoiseVoxelTypes {
    #[roc(expr = "256")]
    const VOXEL_TYPE_ARRAY_SIZE: usize = VoxelTypeRegistry::max_n_voxel_types().next_power_of_two();

    #[roc(body = r#"
    n_voxel_types = List.len(voxel_type_names)
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect n_voxel_types > 0
    # expect n_voxel_types <= voxel_type_array_size
    unpadded_voxel_type_name_hashes = voxel_type_names |> List.map(Hashing.hash_str_32)
    padding_len = voxel_type_array_size - n_voxel_types
    voxel_type_name_hashes = List.concat(
        unpadded_voxel_type_name_hashes,
        List.repeat(Hashing.hash_str_32(""), padding_len),
    )
    {
        n_voxel_types: Num.to_u32(n_voxel_types),
        voxel_type_name_hashes,
        noise_frequency,
        voxel_type_frequency,
        seed,
    }
    "#)]
    pub fn new(
        voxel_type_names: &[&str],
        noise_frequency: f32,
        voxel_type_frequency: f32,
        seed: u32,
    ) -> Self {
        let n_voxel_types = voxel_type_names.len();
        assert!(n_voxel_types > 0);
        assert!(n_voxel_types <= VoxelTypeRegistry::max_n_voxel_types());

        let mut voxel_type_name_hashes = [Hash32::zeroed(); Self::VOXEL_TYPE_ARRAY_SIZE];
        for (idx, name) in voxel_type_names.iter().enumerate() {
            voxel_type_name_hashes[idx] = Hash32::from_str(name);
        }

        Self {
            n_voxel_types: n_voxel_types as u32,
            voxel_type_name_hashes,
            noise_frequency,
            voxel_type_frequency,
            seed,
        }
    }

    pub fn voxel_types(&self, voxel_type_registry: &VoxelTypeRegistry) -> Result<Vec<VoxelType>> {
        let mut voxel_types = Vec::with_capacity(self.n_voxel_types as usize);
        for (idx, &name_hash) in self.voxel_type_name_hashes[..self.n_voxel_types as usize]
            .iter()
            .enumerate()
        {
            voxel_types.push(
                voxel_type_registry
                    .voxel_type_for_name_hash(name_hash)
                    .ok_or_else(|| {
                        anyhow!(
                            "Missing voxel type for name at index {} in `GradientNoiseVoxelTypes`",
                            idx
                        )
                    })?,
            );
        }
        Ok(voxel_types)
    }

    pub fn noise_frequency(&self) -> f32 {
        self.noise_frequency
    }

    pub fn voxel_type_frequency(&self) -> f32 {
        self.voxel_type_frequency
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn create_generator(
        &self,
        voxel_type_registry: &VoxelTypeRegistry,
    ) -> Result<GradientNoiseVoxelTypeGenerator> {
        Ok(GradientNoiseVoxelTypeGenerator::new(
            self.voxel_types(voxel_type_registry)?,
            self.noise_frequency,
            self.voxel_type_frequency,
            self.seed,
        ))
    }
}

#[roc]
impl MultifractalNoiseSDFModification {
    #[roc(body = r#"
    {
        octaves,
        frequency,
        lacunarity,
        persistence,
        amplitude,
        seed,
    }"#)]
    pub fn new(
        octaves: u32,
        frequency: f32,
        lacunarity: f32,
        persistence: f32,
        amplitude: f32,
        seed: u32,
    ) -> Self {
        Self {
            octaves,
            frequency,
            lacunarity,
            persistence,
            amplitude,
            seed,
        }
    }
}

#[roc]
impl VoxelBox {
    /// Defines a box with the given voxel extent and number of voxels in each
    /// direction.
    ///
    /// # Panics
    /// - If the voxel extent is negative.
    /// - If either of the extents is zero or negative.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect extent_x >= 0.0
    # expect extent_y >= 0.0
    # expect extent_z >= 0.0
    {
        voxel_extent,
        extent_x,
        extent_y,
        extent_z,
    }"#)]
    pub fn new(voxel_extent: f32, extent_x: f32, extent_y: f32, extent_z: f32) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(extent_x >= 0.0);
        assert!(extent_y >= 0.0);
        assert!(extent_z >= 0.0);
        Self {
            voxel_extent,
            extent_x,
            extent_y,
            extent_z,
        }
    }

    pub fn voxel_extent(&self) -> f32 {
        self.voxel_extent
    }

    pub fn extents_in_voxels(&self) -> [f32; 3] {
        [self.extent_x, self.extent_y, self.extent_z]
    }

    pub fn add<A: Allocator>(&self, graph: &mut SDFGraph<A>) -> SDFNodeID {
        graph.add_node(SDFNode::new_box(self.extents_in_voxels()))
    }
}

#[roc]
impl VoxelSphere {
    /// Defines a sphere with the given voxel extent and number of voxels across
    /// its radius.
    ///
    /// # Panics
    /// - If the voxel extent is negative.
    /// - If the radius zero or negative.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect radius >= 0.0
    {
        voxel_extent,
        radius,
    }"#)]
    pub fn new(voxel_extent: f32, radius: f32) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(radius >= 0.0);
        Self {
            voxel_extent,
            radius,
        }
    }

    pub fn voxel_extent(&self) -> f32 {
        self.voxel_extent
    }

    pub fn radius_in_voxels(&self) -> f32 {
        self.radius
    }

    pub fn add<A: Allocator>(&self, graph: &mut SDFGraph<A>) -> SDFNodeID {
        graph.add_node(SDFNode::new_sphere(self.radius_in_voxels()))
    }
}

#[roc]
impl VoxelSphereUnion {
    /// Defines a sphere union with the given smoothness of the spheres with the
    /// given radii and center offsets (in voxels).
    ///
    /// # Panics
    /// - If the voxel extent is negative.
    /// - If either of the radii is zero or negative.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect radius_1 >= 0.0
    # expect radius_2 >= 0.0
    {
        voxel_extent,
        radius_1,
        radius_2,
        center_offsets,
        smoothness,
    }"#)]
    pub fn new(
        voxel_extent: f32,
        radius_1: f32,
        radius_2: f32,
        center_offsets: Vector3C,
        smoothness: f32,
    ) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(radius_1 >= 0.0);
        assert!(radius_2 >= 0.0);
        Self {
            voxel_extent,
            radius_1,
            radius_2,
            center_offsets,
            smoothness,
        }
    }

    pub fn voxel_extent(&self) -> f32 {
        self.voxel_extent
    }

    pub fn radius_1_in_voxels(&self) -> f32 {
        self.radius_1
    }

    pub fn radius_2_in_voxels(&self) -> f32 {
        self.radius_2
    }

    pub fn add<A: Allocator>(&self, graph: &mut SDFGraph<A>) -> SDFNodeID {
        let center_offsets = self.center_offsets.aligned();
        let sphere_1_id = graph.add_node(SDFNode::new_sphere(self.radius_1_in_voxels()));
        let sphere_2_id = graph.add_node(SDFNode::new_sphere(self.radius_2_in_voxels()));
        let sphere_2_id = graph.add_node(SDFNode::new_translation(sphere_2_id, center_offsets));
        graph.add_node(SDFNode::new_union(
            sphere_1_id,
            sphere_2_id,
            self.smoothness,
        ))
    }
}

pub fn apply_modifications<A: Allocator>(
    graph: &mut SDFGraph<A>,
    node_id: SDFNodeID,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) {
    if let Some(&MultifractalNoiseSDFModification {
        octaves,
        frequency,
        lacunarity,
        persistence,
        amplitude,
        seed,
    }) = multifractal_noise_modification
    {
        graph.add_node(SDFNode::new_multifractal_noise(
            node_id,
            octaves,
            frequency,
            lacunarity,
            persistence,
            amplitude,
            seed,
        ));
    }
}

pub fn setup_voxel_object(
    voxel_object_manager: &mut VoxelObjectManager,
    generator: &impl ChunkedVoxelGenerator,
    entity_id: EntityID,
) -> Result<()> {
    let voxel_object = ChunkedVoxelObject::generate(generator);

    let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

    let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
    voxel_object_manager.add_voxel_object(voxel_object_id, meshed_voxel_object)?;

    Ok(())
}

pub fn setup_dynamic_rigid_body_for_voxel_object(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_object_manager: &mut VoxelObjectManager,
    voxel_type_registry: &VoxelTypeRegistry,
    entity_id: EntityID,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
) -> Result<(ModelTransform, ReferenceFrame, Motion)> {
    let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
    let voxel_object = voxel_object_manager
        .get_voxel_object(voxel_object_id)
        .ok_or_else(|| anyhow!("Tried to setup dynamic rigid body for missing voxel object"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        voxel_object.object(),
        voxel_type_registry.mass_densities(),
    );

    let (model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        entity_id,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let physics_context = VoxelObjectPhysicsContext {
        inertial_property_manager,
    };

    voxel_object_manager.add_physics_context_for_voxel_object(voxel_object_id, physics_context)?;

    Ok((model_transform, frame, velocity))
}

pub fn create_model_instance_node_for_voxel_object(
    model_instance_manager: &mut ModelInstanceManager,
    scene_graph: &mut SceneGraph,
    entity_id: EntityID,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    parent_entity_id: Option<&ParentEntity>,
    flags: Option<&SceneEntityFlags>,
) -> Result<(ModelTransform, SceneEntityFlags)> {
    let model_transform = model_transform.copied().unwrap_or_default();
    let frame = frame.copied().unwrap_or_default();
    let flags = flags.copied().unwrap_or_default();

    let model_id = *VOXEL_MODEL_ID;

    model_instance_manager.register_instance(
        model_id,
        &[
            InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID,
            InstanceModelLightTransform::FEATURE_TYPE_ID,
            VoxelObjectID::FEATURE_TYPE_ID,
        ],
    );

    let model_to_parent_transform = frame.create_transform_to_parent_space()
        * model_transform.create_transform_to_entity_space();

    // Add entries for the model-to-camera and model-to-light transforms
    // for the scene graph to access and modify using the returned IDs
    let model_view_transform_feature_id = model_instance_manager
        .get_storage_mut::<InstanceModelViewTransformWithPrevious>()
        .expect("Missing storage for InstanceModelViewTransform feature")
        .add_feature(&InstanceModelViewTransformWithPrevious::default());

    let model_light_transform_feature_id = model_instance_manager
        .get_storage_mut::<InstanceModelLightTransform>()
        .expect("Missing storage for InstanceModelLightTransform feature")
        .add_feature(&InstanceModelLightTransform::default());

    let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);

    let voxel_object_id_feature_id = model_instance_manager
        .get_storage_mut::<VoxelObjectID>()
        .expect("Missing storage for VoxelObjectID feature")
        .add_feature(&voxel_object_id);

    let model_instance_id = ModelInstanceID::from_entity_id(entity_id);
    let parent_group_id = parent_entity_id.map_or_else(
        || scene_graph.root_node_id(),
        |parent| SceneGroupID::from_entity_id(parent.0),
    );

    scene_graph.create_model_instance_node(
        parent_group_id,
        model_instance_id,
        model_to_parent_transform.compact(),
        model_id,
        FeatureIDSet::from_iter([model_view_transform_feature_id, voxel_object_id_feature_id]),
        FeatureIDSet::from_iter([model_light_transform_feature_id, voxel_object_id_feature_id]),
        flags.into(),
    )?;

    Ok((model_transform, flags))
}

pub fn setup_bounding_volume_for_voxel_object(
    voxel_object_manager: &VoxelObjectManager,
    bounding_volume_manager: &mut BoundingVolumeManager,
    entity_id: EntityID,
) -> Result<()> {
    let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
    let voxel_object = voxel_object_manager
        .get_voxel_object(voxel_object_id)
        .ok_or_else(|| anyhow!("Tried to create bounding volume for missing voxel object (with ID {voxel_object_id})"))?
        .object();

    let aabb = if voxel_object.contains_only_empty_voxels() {
        AxisAlignedBoxC::new(Point3C::origin(), Point3C::origin())
    } else {
        voxel_object.compute_aabb().compact()
    };

    let bounding_volume_id = BoundingVolumeID::from_entity_id(entity_id);
    bounding_volume_manager.insert_bounding_volume(bounding_volume_id, aabb)
}

fn setup_rigid_body_for_new_voxel_object(
    rigid_body_manager: &mut RigidBodyManager,
    entity_id: EntityID,
    inertial_properties: InertialProperties,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
) -> Result<(ModelTransform, ReferenceFrame, Motion)> {
    let mut model_transform = model_transform.copied().unwrap_or_default();
    let frame = frame.copied().unwrap_or_default();
    let motion = motion.copied().unwrap_or_default();

    if model_transform.scale != 1.0 {
        bail!("Scaling is not supported for voxel objects");
    }

    // Offset the voxel object model to put the center of mass at the origin of
    // this entity's space
    model_transform.set_offset_after_scaling(*inertial_properties.center_of_mass().as_vector());

    rigid_body::setup::setup_dynamic_rigid_body(
        rigid_body_manager,
        entity_id,
        inertial_properties,
        frame,
        motion,
    )?;

    Ok((model_transform, frame, motion))
}
