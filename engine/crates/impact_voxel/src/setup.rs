//! Setup of voxel objects.

use std::sync::RwLock;

use crate::{
    VoxelManager, VoxelObjectID, VoxelObjectManager,
    chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyManager},
    generation::{
        BoxSDFGenerator, GradientNoiseSDFGenerator, GradientNoiseVoxelTypeGenerator,
        MultifractalNoiseSDFModifier, MultiscaleSphereSDFModifier, SDFGenerator, SDFUnion,
        SDFVoxelGenerator, SameVoxelTypeGenerator, SphereSDFGenerator, VoxelTypeGenerator,
    },
    mesh::MeshedChunkedVoxelObject,
    resource::VOXEL_MODEL_ID,
    voxel_types::{VoxelType, VoxelTypeRegistry},
};
use anyhow::{Result, anyhow, bail};
use bytemuck::{Pod, Zeroable};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_math::Hash32;
use impact_model::{
    InstanceFeature,
    transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
};
use impact_physics::{
    inertia::InertialProperties,
    quantities::Motion,
    rigid_body::{self, DynamicRigidBodyID, RigidBodyManager},
};
use impact_scene::{
    SceneEntityFlags, SceneGraphModelInstanceNodeHandle, SceneGraphParentNodeHandle,
    graph::SceneGraph, model::InstanceFeatureManager,
};
use nalgebra::Vector3;
use roc_integration::roc;

define_setup_type! {
    target = VoxelObjectID;
    /// A voxel type that is the only type present in a voxel object.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SameVoxelType {
        /// The index of the voxel type.
        voxel_type_idx: usize,
    }
}

define_setup_type! {
    target = VoxelObjectID;
    /// A set of voxel types distributed according to a gradient noise pattern.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct GradientNoiseVoxelTypes {
        n_voxel_types: usize,
        voxel_type_name_hashes: [Hash32; GradientNoiseVoxelTypes::VOXEL_TYPE_ARRAY_SIZE],
        noise_frequency: f64,
        voxel_type_frequency: f64,
        pub seed: u64,
    }
}

define_setup_type! {
    target = VoxelObjectID;
    /// A modification to a voxel signed distance field based on unions with a
    /// multiscale sphere grid (<https://iquilezles.org/articles/fbmsdf>/).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct MultiscaleSphereSDFModification {
        pub octaves: usize,
        pub max_scale: f64,
        pub persistence: f64,
        pub inflation: f64,
        pub smoothness: f64,
        pub seed: u64,
    }
}

define_setup_type! {
    target = VoxelObjectID;
    /// A modification of a voxel signed distance field based on multifractal
    /// noise.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct MultifractalNoiseSDFModification {
        pub octaves: usize,
        pub frequency: f64,
        pub lacunarity: f64,
        pub persistence: f64,
        pub amplitude: f64,
        pub seed: u64,
    }
}

define_setup_type! {
    target = VoxelObjectID;
    /// An object made of voxels in a box configuration.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelBox {
        /// The extent of a single voxel.
        pub voxel_extent: f64,
        /// The number of voxels along the box in the x-direction.
        pub extent_x: f64,
        /// The number of voxels along the box in the y-direction.
        pub extent_y: f64,
        /// The number of voxels along the box in the z-direction.
        pub extent_z: f64,
    }
}

define_setup_type! {
    target = VoxelObjectID;
    /// An object made of voxels in a spherical configuration.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelSphere {
        /// The extent of a single voxel.
        pub voxel_extent: f64,
        /// The number of voxels along the radius of the sphere.
        pub radius: f64,
    }
}

define_setup_type! {
    target = VoxelObjectID;
    /// An object made of voxels in a configuration described by the smooth
    /// union of two spheres.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelSphereUnion {
        /// The extent of a single voxel.
        pub voxel_extent: f64,
        /// The number of voxels along the radius of the first sphere.
        pub radius_1: f64,
        /// The number of voxels along the radius of the second sphere.
        pub radius_2: f64,
        /// The offset in number of voxels in each dimension between the centers of
        /// the two spheres.
        pub center_offsets: Vector3<f64>,
        /// The smoothness of the union operation.
        pub smoothness: f64,
    }
}

define_setup_type! {
    target = VoxelObjectID;
    /// An object made of voxels in a gradient noise pattern.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelGradientNoisePattern {
        /// The extent of a single voxel.
        pub voxel_extent: f64,
        /// The maximum number of voxels in the x-direction.
        pub extent_x: f64,
        /// The maximum number of voxels in the y-direction.
        pub extent_y: f64,
        /// The maximum number of voxels in the z-direction.
        pub extent_z: f64,
        /// The spatial frequency of the noise pattern.
        pub noise_frequency: f64,
        /// The threshold noise value for generating a voxel.
        pub noise_threshold: f64,
        /// The seed for the noise pattern.
        pub seed: u64,
    }
}

#[roc(dependencies=[VoxelType])]
impl SameVoxelType {
    #[roc(body = "{ voxel_type_idx: NativeNum.to_usize(voxel_type) }")]
    pub fn new(voxel_type: VoxelType) -> Self {
        Self {
            voxel_type_idx: voxel_type.idx(),
        }
    }

    pub fn voxel_type(&self) -> VoxelType {
        VoxelType::from_idx(self.voxel_type_idx)
    }
}

#[roc]
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
        n_voxel_types,
        voxel_type_name_hashes,
        noise_frequency,
        voxel_type_frequency,
        seed,
    }
    "#)]
    pub fn new(
        voxel_type_names: &[&str],
        noise_frequency: f64,
        voxel_type_frequency: f64,
        seed: u64,
    ) -> Self {
        let n_voxel_types = voxel_type_names.len();
        assert!(n_voxel_types > 0);
        assert!(n_voxel_types <= VoxelTypeRegistry::max_n_voxel_types());

        let mut voxel_type_name_hashes = [Hash32::zeroed(); Self::VOXEL_TYPE_ARRAY_SIZE];
        for (idx, name) in voxel_type_names.iter().enumerate() {
            voxel_type_name_hashes[idx] = Hash32::from_str(name);
        }

        Self {
            n_voxel_types,
            voxel_type_name_hashes,
            noise_frequency,
            voxel_type_frequency,
            seed,
        }
    }

    pub fn voxel_types(&self, voxel_type_registry: &VoxelTypeRegistry) -> Result<Vec<VoxelType>> {
        let mut voxel_types = Vec::with_capacity(self.n_voxel_types);
        for (idx, &name_hash) in self.voxel_type_name_hashes[..self.n_voxel_types]
            .iter()
            .enumerate()
        {
            voxel_types.push(
                voxel_type_registry
                    .voxel_type_for_name_hash(name_hash)
                    .ok_or_else(|| anyhow!("Missing voxel type for name at index {}", idx))?,
            );
        }
        Ok(voxel_types)
    }

    pub fn noise_frequency(&self) -> f64 {
        self.noise_frequency
    }

    pub fn voxel_type_frequency(&self) -> f64 {
        self.voxel_type_frequency
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }
}

#[roc]
impl MultiscaleSphereSDFModification {
    #[roc(body = r#"
    {
        octaves,
        max_scale,
        persistence,
        inflation,
        smoothness,
        seed,
    }"#)]
    pub fn new(
        octaves: usize,
        max_scale: f64,
        persistence: f64,
        inflation: f64,
        smoothness: f64,
        seed: u64,
    ) -> Self {
        Self {
            octaves,
            max_scale,
            persistence,
            inflation,
            smoothness,
            seed,
        }
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
        octaves: usize,
        frequency: f64,
        lacunarity: f64,
        persistence: f64,
        amplitude: f64,
        seed: u64,
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
    pub fn new(voxel_extent: f64, extent_x: f64, extent_y: f64, extent_z: f64) -> Self {
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

    pub fn extents_in_voxels(&self) -> [f64; 3] {
        [self.extent_x, self.extent_y, self.extent_z]
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
    pub fn new(voxel_extent: f64, radius: f64) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(radius >= 0.0);
        Self {
            voxel_extent,
            radius,
        }
    }

    pub fn radius_in_voxels(&self) -> f64 {
        self.radius
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
        voxel_extent: f64,
        radius_1: f64,
        radius_2: f64,
        center_offsets: Vector3<f64>,
        smoothness: f64,
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

    pub fn radius_1_in_voxels(&self) -> f64 {
        self.radius_1
    }

    pub fn radius_2_in_voxels(&self) -> f64 {
        self.radius_2
    }
}

#[roc]
impl VoxelGradientNoisePattern {
    /// Defines a gradient noise voxel pattern with the given maximum number of
    /// voxels in each direction, spatial noise frequency, noise threshold and
    /// seed.
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
        noise_frequency,
        noise_threshold,
        seed,
    }"#)]
    pub fn new(
        voxel_extent: f64,
        extent_x: f64,
        extent_y: f64,
        extent_z: f64,
        noise_frequency: f64,
        noise_threshold: f64,
        seed: u64,
    ) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(extent_x >= 0.0);
        assert!(extent_y >= 0.0);
        assert!(extent_z >= 0.0);
        Self {
            voxel_extent,
            extent_x,
            extent_y,
            extent_z,
            noise_frequency,
            noise_threshold,
            seed,
        }
    }

    pub fn extents_in_voxels(&self) -> [f64; 3] {
        [self.extent_x, self.extent_y, self.extent_z]
    }
}

pub fn setup_voxel_box_with_same_voxel_type(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    voxel_box: &VoxelBox,
    voxel_type: &SameVoxelType,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> Result<(
    VoxelObjectID,
    DynamicRigidBodyID,
    ModelTransform,
    ReferenceFrame,
    Motion,
)> {
    let sdf_generator = BoxSDFGenerator::new(voxel_box.extents_in_voxels());
    let voxel_type_generator = SameVoxelTypeGenerator::new(voxel_type.voxel_type());

    let voxel_object = generate_voxel_object(
        voxel_box.voxel_extent,
        sdf_generator,
        voxel_type_generator,
        multiscale_sphere_modification,
        multifractal_noise_modification,
    )
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel box"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        &voxel_object,
        voxel_manager.type_registry.mass_densities(),
    );

    let (rigid_body, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let voxel_object_id = mesh_and_store_voxel_object(
        &mut voxel_manager.object_manager,
        voxel_object,
        inertial_property_manager,
    );

    Ok((
        voxel_object_id,
        rigid_body,
        model_transform,
        frame,
        velocity,
    ))
}

pub fn setup_voxel_sphere_with_same_voxel_type(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    voxel_sphere: &VoxelSphere,
    voxel_type: &SameVoxelType,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> Result<(
    VoxelObjectID,
    DynamicRigidBodyID,
    ModelTransform,
    ReferenceFrame,
    Motion,
)> {
    let sdf_generator = SphereSDFGenerator::new(voxel_sphere.radius_in_voxels());
    let voxel_type_generator = SameVoxelTypeGenerator::new(voxel_type.voxel_type());

    let voxel_object = generate_voxel_object(
        voxel_sphere.voxel_extent,
        sdf_generator,
        voxel_type_generator,
        multiscale_sphere_modification,
        multifractal_noise_modification,
    )
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel sphere"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        &voxel_object,
        voxel_manager.type_registry.mass_densities(),
    );

    let (rigid_body, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let voxel_object_id = mesh_and_store_voxel_object(
        &mut voxel_manager.object_manager,
        voxel_object,
        inertial_property_manager,
    );

    Ok((
        voxel_object_id,
        rigid_body,
        model_transform,
        frame,
        velocity,
    ))
}

pub fn setup_voxel_sphere_union_with_same_voxel_type(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    voxel_sphere_union: &VoxelSphereUnion,
    voxel_type: &SameVoxelType,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> Result<(
    VoxelObjectID,
    DynamicRigidBodyID,
    ModelTransform,
    ReferenceFrame,
    Motion,
)> {
    let sdf_generator_1 = SphereSDFGenerator::new(voxel_sphere_union.radius_1_in_voxels());
    let sdf_generator_2 = SphereSDFGenerator::new(voxel_sphere_union.radius_2_in_voxels());
    let sdf_generator = SDFUnion::new(
        sdf_generator_1,
        sdf_generator_2,
        voxel_sphere_union.center_offsets.into(),
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
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel sphere union"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        &voxel_object,
        voxel_manager.type_registry.mass_densities(),
    );

    let (rigid_body, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let voxel_object_id = mesh_and_store_voxel_object(
        &mut voxel_manager.object_manager,
        voxel_object,
        inertial_property_manager,
    );

    Ok((
        voxel_object_id,
        rigid_body,
        model_transform,
        frame,
        velocity,
    ))
}

pub fn setup_voxel_gradient_noise_pattern_with_same_voxel_type(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    voxel_noise_pattern: &VoxelGradientNoisePattern,
    voxel_type: &SameVoxelType,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> Result<(
    VoxelObjectID,
    DynamicRigidBodyID,
    ModelTransform,
    ReferenceFrame,
    Motion,
)> {
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
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel gradient noise pattern"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        &voxel_object,
        voxel_manager.type_registry.mass_densities(),
    );

    let (rigid_body, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let voxel_object_id = mesh_and_store_voxel_object(
        &mut voxel_manager.object_manager,
        voxel_object,
        inertial_property_manager,
    );

    Ok((
        voxel_object_id,
        rigid_body,
        model_transform,
        frame,
        velocity,
    ))
}

pub fn setup_voxel_box_with_gradient_noise_voxel_types(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    voxel_box: &VoxelBox,
    voxel_types: &GradientNoiseVoxelTypes,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> Result<(
    VoxelObjectID,
    DynamicRigidBodyID,
    ModelTransform,
    ReferenceFrame,
    Motion,
)> {
    let sdf_generator = BoxSDFGenerator::new(voxel_box.extents_in_voxels());
    let voxel_type_generator = gradient_noise_voxel_type_generator_from_component(
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
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel box"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        &voxel_object,
        voxel_manager.type_registry.mass_densities(),
    );

    let (rigid_body, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let voxel_object_id = mesh_and_store_voxel_object(
        &mut voxel_manager.object_manager,
        voxel_object,
        inertial_property_manager,
    );

    Ok((
        voxel_object_id,
        rigid_body,
        model_transform,
        frame,
        velocity,
    ))
}

pub fn setup_voxel_sphere_with_gradient_noise_voxel_types(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    voxel_sphere: &VoxelSphere,
    voxel_types: &GradientNoiseVoxelTypes,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> Result<(
    VoxelObjectID,
    DynamicRigidBodyID,
    ModelTransform,
    ReferenceFrame,
    Motion,
)> {
    let sdf_generator = SphereSDFGenerator::new(voxel_sphere.radius_in_voxels());
    let voxel_type_generator = gradient_noise_voxel_type_generator_from_component(
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
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel sphere"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        &voxel_object,
        voxel_manager.type_registry.mass_densities(),
    );

    let (rigid_body, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let voxel_object_id = mesh_and_store_voxel_object(
        &mut voxel_manager.object_manager,
        voxel_object,
        inertial_property_manager,
    );

    Ok((
        voxel_object_id,
        rigid_body,
        model_transform,
        frame,
        velocity,
    ))
}

pub fn setup_voxel_sphere_union_with_gradient_noise_voxel_types(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    voxel_sphere_union: &VoxelSphereUnion,
    voxel_types: &GradientNoiseVoxelTypes,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> Result<(
    VoxelObjectID,
    DynamicRigidBodyID,
    ModelTransform,
    ReferenceFrame,
    Motion,
)> {
    let sdf_generator_1 = SphereSDFGenerator::new(voxel_sphere_union.radius_1_in_voxels());
    let sdf_generator_2 = SphereSDFGenerator::new(voxel_sphere_union.radius_2_in_voxels());
    let sdf_generator = SDFUnion::new(
        sdf_generator_1,
        sdf_generator_2,
        voxel_sphere_union.center_offsets.into(),
        voxel_sphere_union.smoothness,
    );
    let voxel_type_generator = gradient_noise_voxel_type_generator_from_component(
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
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel sphere union"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        &voxel_object,
        voxel_manager.type_registry.mass_densities(),
    );

    let (rigid_body, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let voxel_object_id = mesh_and_store_voxel_object(
        &mut voxel_manager.object_manager,
        voxel_object,
        inertial_property_manager,
    );

    Ok((
        voxel_object_id,
        rigid_body,
        model_transform,
        frame,
        velocity,
    ))
}

pub fn setup_voxel_gradient_noise_pattern_with_gradient_noise_voxel_types(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_manager: &mut VoxelManager,
    voxel_noise_pattern: &VoxelGradientNoisePattern,
    voxel_types: &GradientNoiseVoxelTypes,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> Result<(
    VoxelObjectID,
    DynamicRigidBodyID,
    ModelTransform,
    ReferenceFrame,
    Motion,
)> {
    let sdf_generator = GradientNoiseSDFGenerator::new(
        voxel_noise_pattern.extents_in_voxels(),
        voxel_noise_pattern.noise_frequency,
        voxel_noise_pattern.noise_threshold,
        u32::try_from(voxel_noise_pattern.seed).unwrap(),
    );
    let voxel_type_generator = gradient_noise_voxel_type_generator_from_component(
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
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel gradient noise pattern"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        &voxel_object,
        voxel_manager.type_registry.mass_densities(),
    );

    let (rigid_body, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let voxel_object_id = mesh_and_store_voxel_object(
        &mut voxel_manager.object_manager,
        voxel_object,
        inertial_property_manager,
    );

    Ok((
        voxel_object_id,
        rigid_body,
        model_transform,
        frame,
        velocity,
    ))
}

pub fn create_model_instance_node_for_voxel_object(
    voxel_manager: &VoxelManager,
    instance_feature_manager: &mut InstanceFeatureManager,
    scene_graph: &mut SceneGraph,
    voxel_object_id: &VoxelObjectID,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    parent: Option<&SceneGraphParentNodeHandle>,
    flags: Option<&SceneEntityFlags>,
    uncullable: bool,
) -> Result<(
    SceneGraphModelInstanceNodeHandle,
    ModelTransform,
    SceneEntityFlags,
)> {
    let model_transform = model_transform.copied().unwrap_or_default();
    let frame = frame.copied().unwrap_or_default();
    let flags = flags.copied().unwrap_or_default();

    let voxel_object = voxel_manager
        .object_manager
        .get_voxel_object(*voxel_object_id)
        .ok_or_else(|| anyhow!("Tried to create model instance node for missing voxel object (with ID {voxel_object_id})"))?
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

    let model_to_parent_transform = frame.create_transform_to_parent_space()
        * model_transform.crate_transform_to_entity_space();

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
        .add_feature(voxel_object_id);

    let bounding_sphere = if uncullable {
        // The scene graph will not cull models with no bounding sphere
        None
    } else {
        Some(voxel_object.compute_bounding_sphere())
    };

    let parent_node_id = parent.map_or_else(|| scene_graph.root_node_id(), |parent| parent.id);

    Ok((
        SceneGraphModelInstanceNodeHandle::new(scene_graph.create_model_instance_node(
            parent_node_id,
            model_to_parent_transform,
            model_id,
            bounding_sphere,
            vec![model_view_transform_feature_id, voxel_object_id_feature_id],
            vec![model_light_transform_feature_id, voxel_object_id_feature_id],
            flags.into(),
        )),
        model_transform,
        flags,
    ))
}

/// Removes the voxel object with the given [`VoxelObjectID`]
/// from the [`VoxelManager`].
pub fn cleanup_voxel_object(
    voxel_manager: &RwLock<VoxelManager>,
    voxel_object_id: VoxelObjectID,
    desynchronized: &mut bool,
) {
    voxel_manager
        .write()
        .unwrap()
        .object_manager
        .remove_voxel_object(voxel_object_id);

    *desynchronized = true;
}

/// Checks if the given entity has a [`VoxelObjectID`], and if so, removes the
/// assocated voxel object from the given [`VoxelManager`].
#[cfg(feature = "ecs")]
pub fn cleanup_voxel_object_for_removed_entity(
    voxel_manager: &RwLock<VoxelManager>,
    entity: &impact_ecs::world::EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(voxel_object_id) = entity.get_component::<VoxelObjectID>() {
        cleanup_voxel_object(voxel_manager, *voxel_object_id.access(), desynchronized);
    }
}

fn generate_voxel_object(
    voxel_extent: f64,
    sdf_generator: impl SDFGenerator,
    voxel_type_generator: impl VoxelTypeGenerator,
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
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
    rigid_body_manager: &mut RigidBodyManager,
    inertial_properties: InertialProperties,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
) -> Result<(DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion)> {
    let mut model_transform = model_transform.copied().unwrap_or_default();
    let frame = frame.copied().unwrap_or_default();
    let motion = motion.copied().unwrap_or_default();

    if model_transform.scale != 1.0 {
        bail!("Scaling is not supported for voxel objects");
    }

    // Offset the voxel object model to put the center of mass at the origin of
    // this entity's space
    model_transform.set_offset_after_scaling(inertial_properties.center_of_mass().coords.cast());

    let rigid_body_id = rigid_body::setup::setup_dynamic_rigid_body(
        rigid_body_manager,
        inertial_properties,
        frame,
        motion,
    );

    Ok((rigid_body_id, model_transform, frame, motion))
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

fn gradient_noise_voxel_type_generator_from_component(
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_types: &GradientNoiseVoxelTypes,
) -> GradientNoiseVoxelTypeGenerator {
    GradientNoiseVoxelTypeGenerator::new(
        voxel_types
            .voxel_types(voxel_type_registry)
            .expect("Invalid voxel types"),
        voxel_types.noise_frequency(),
        voxel_types.voxel_type_frequency(),
        u32::try_from(voxel_types.seed()).unwrap(),
    )
}
