//! Setup of voxel objects.

use crate::{
    VoxelObjectID, VoxelObjectManager, VoxelObjectPhysicsContext,
    chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyManager},
    generation::{
        BoxSDFGenerator, GradientNoiseSDFGenerator, GradientNoiseVoxelTypeGenerator,
        MultifractalNoiseSDFModifier, MultiscaleSphereSDFModifier, SDFGenerator, SDFUnion,
        SDFVoxelGenerator, SameVoxelTypeGenerator, SphereSDFGenerator, VoxelTypeGenerator,
    },
    gpu_resource::VOXEL_MODEL_ID,
    mesh::MeshedChunkedVoxelObject,
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
    graph::{FeatureIDSet, SceneGraph},
    model::ModelInstanceManager,
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
        voxel_type_name_hash: Hash32,
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

define_setup_type! {
    target = VoxelObjectID;
    /// A voxel object with dynamic voxels will behave like a dynamic rigid body
    /// and respond to voxel absorption.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DynamicVoxels;
}

/// Template for the voxel types of a voxel object.
#[derive(Clone, Debug)]
pub enum VoxelObjectVoxelTypes {
    Same(SameVoxelType),
    GradientNoise(Box<GradientNoiseVoxelTypes>),
}

/// Template for the shape of a voxel object.
#[derive(Clone, Debug)]
pub enum VoxelObjectShape {
    Box(VoxelBox),
    Sphere(VoxelSphere),
    SphereUnion(VoxelSphereUnion),
    GradientNoisePattern(VoxelGradientNoisePattern),
}

/// A modification to the signed distance field of a voxel object.
#[derive(Copy, Clone, Debug)]
pub enum VoxelObjectSDFModification {
    None,
    MultiscaleSphere(MultiscaleSphereSDFModification),
    MultifractalNoise(MultifractalNoiseSDFModification),
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

    fn apply<SD: SDFGenerator>(&self, sdf_generator: SD) -> MultiscaleSphereSDFModifier<SD> {
        MultiscaleSphereSDFModifier::new(
            sdf_generator,
            self.octaves,
            self.max_scale,
            self.persistence,
            self.inflation,
            self.smoothness,
            self.seed,
        )
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

    fn apply<SD: SDFGenerator>(&self, sdf_generator: SD) -> MultifractalNoiseSDFModifier<SD> {
        MultifractalNoiseSDFModifier::new(
            sdf_generator,
            self.octaves,
            self.frequency,
            self.lacunarity,
            self.persistence,
            self.amplitude,
            u32::try_from(self.seed).unwrap(),
        )
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

    fn generator(&self) -> BoxSDFGenerator {
        BoxSDFGenerator::new(self.extents_in_voxels())
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

    fn generator(&self) -> SphereSDFGenerator {
        SphereSDFGenerator::new(self.radius_in_voxels())
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

    fn generator(&self) -> SDFUnion<SphereSDFGenerator, SphereSDFGenerator> {
        let sdf_generator_1 = SphereSDFGenerator::new(self.radius_1_in_voxels());
        let sdf_generator_2 = SphereSDFGenerator::new(self.radius_2_in_voxels());
        SDFUnion::new(
            sdf_generator_1,
            sdf_generator_2,
            self.center_offsets.into(),
            self.smoothness,
        )
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

    fn generator(&self) -> GradientNoiseSDFGenerator {
        GradientNoiseSDFGenerator::new(
            self.extents_in_voxels(),
            self.noise_frequency,
            self.noise_threshold,
            u32::try_from(self.seed).unwrap(),
        )
    }
}

const MAX_MODIFICATIONS: usize = 2;

pub fn gather_modifications(
    multiscale_sphere_modification: Option<&MultiscaleSphereSDFModification>,
    multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>,
) -> [VoxelObjectSDFModification; MAX_MODIFICATIONS] {
    let mut modifications = [VoxelObjectSDFModification::None; MAX_MODIFICATIONS];
    let mut idx = 0;

    if let Some(modification) = multiscale_sphere_modification {
        modifications[idx] = VoxelObjectSDFModification::MultiscaleSphere(*modification);
        idx += 1;
    }

    if let Some(modification) = multifractal_noise_modification {
        modifications[idx] = VoxelObjectSDFModification::MultifractalNoise(*modification);
    }

    modifications
}

pub fn setup_voxel_object(
    voxel_object_manager: &mut VoxelObjectManager,
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_types: VoxelObjectVoxelTypes,
    shape: VoxelObjectShape,
    sdf_modifications: &[VoxelObjectSDFModification; MAX_MODIFICATIONS],
) -> Result<VoxelObjectID> {
    let voxel_object = generate_voxel_object_with_types_shape_and_modifications(
        voxel_type_registry,
        voxel_types,
        shape,
        sdf_modifications,
    )?;

    let meshed_voxel_object = MeshedChunkedVoxelObject::create(voxel_object);

    let voxel_object_id = voxel_object_manager.add_voxel_object(meshed_voxel_object);

    Ok(voxel_object_id)
}

pub fn setup_dynamic_rigid_body_for_voxel_object(
    rigid_body_manager: &mut RigidBodyManager,
    voxel_object_manager: &mut VoxelObjectManager,
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_object_id: VoxelObjectID,
    model_transform: Option<&ModelTransform>,
    frame: Option<&ReferenceFrame>,
    motion: Option<&Motion>,
) -> Result<(DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion)> {
    let voxel_object = voxel_object_manager
        .get_voxel_object(voxel_object_id)
        .ok_or_else(|| anyhow!("Tried to setup dynamic rigid body for missing voxel object"))?;

    let inertial_property_manager = VoxelObjectInertialPropertyManager::initialized_from(
        voxel_object.object(),
        voxel_type_registry.mass_densities(),
    );

    let (rigid_body_id, model_transform, frame, velocity) = setup_rigid_body_for_new_voxel_object(
        rigid_body_manager,
        inertial_property_manager.derive_inertial_properties(),
        model_transform,
        frame,
        motion,
    )?;

    let physics_context = VoxelObjectPhysicsContext {
        inertial_property_manager,
        rigid_body_id,
    };

    voxel_object_manager.add_physics_context_for_voxel_object(voxel_object_id, physics_context);

    Ok((rigid_body_id, model_transform, frame, velocity))
}

pub fn create_model_instance_node_for_voxel_object(
    voxel_object_manager: &VoxelObjectManager,
    model_instance_manager: &mut ModelInstanceManager,
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

    let voxel_object = voxel_object_manager
        .get_voxel_object(*voxel_object_id)
        .ok_or_else(|| anyhow!("Tried to create model instance node for missing voxel object (with ID {voxel_object_id})"))?
        .object();

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
        * model_transform.crate_transform_to_entity_space();

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

    let voxel_object_id_feature_id = model_instance_manager
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
            FeatureIDSet::from_iter([model_view_transform_feature_id, voxel_object_id_feature_id]),
            FeatureIDSet::from_iter([model_light_transform_feature_id, voxel_object_id_feature_id]),
            flags.into(),
        )),
        model_transform,
        flags,
    ))
}

fn generate_voxel_object_with_types_shape_and_modifications(
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_types: VoxelObjectVoxelTypes,
    shape: VoxelObjectShape,
    sdf_modifications: &[VoxelObjectSDFModification; MAX_MODIFICATIONS],
) -> Result<ChunkedVoxelObject> {
    match voxel_types {
        VoxelObjectVoxelTypes::Same(voxel_types) => {
            let voxel_type_generator =
                SameVoxelTypeGenerator::new(voxel_types.voxel_type(voxel_type_registry)?);
            generate_voxel_object_with_shape_and_modifications(
                voxel_type_generator,
                shape,
                sdf_modifications,
            )
        }
        VoxelObjectVoxelTypes::GradientNoise(voxel_types) => {
            let voxel_type_generator = gradient_noise_voxel_type_generator_from_component(
                voxel_type_registry,
                &voxel_types,
            )?;
            generate_voxel_object_with_shape_and_modifications(
                voxel_type_generator,
                shape,
                sdf_modifications,
            )
        }
    }
    .ok_or_else(|| anyhow!("Tried to generate object for empty voxel shape"))
}

fn generate_voxel_object_with_shape_and_modifications(
    voxel_type_generator: impl VoxelTypeGenerator,
    shape: VoxelObjectShape,
    sdf_modifications: &[VoxelObjectSDFModification; MAX_MODIFICATIONS],
) -> Option<ChunkedVoxelObject> {
    match shape {
        VoxelObjectShape::Box(shape) => generate_voxel_object_with_modifications(
            voxel_type_generator,
            shape.voxel_extent,
            shape.generator(),
            sdf_modifications,
        ),
        VoxelObjectShape::Sphere(shape) => generate_voxel_object_with_modifications(
            voxel_type_generator,
            shape.voxel_extent,
            shape.generator(),
            sdf_modifications,
        ),
        VoxelObjectShape::SphereUnion(shape) => generate_voxel_object_with_modifications(
            voxel_type_generator,
            shape.voxel_extent,
            shape.generator(),
            sdf_modifications,
        ),
        VoxelObjectShape::GradientNoisePattern(shape) => generate_voxel_object_with_modifications(
            voxel_type_generator,
            shape.voxel_extent,
            shape.generator(),
            sdf_modifications,
        ),
    }
}

fn generate_voxel_object_with_modifications(
    voxel_type_generator: impl VoxelTypeGenerator,
    voxel_extent: f64,
    sdf_generator: impl SDFGenerator,
    sdf_modifications: &[VoxelObjectSDFModification; MAX_MODIFICATIONS],
) -> Option<ChunkedVoxelObject> {
    match &sdf_modifications[0] {
        VoxelObjectSDFModification::None => {
            generate_voxel_object(voxel_type_generator, voxel_extent, sdf_generator)
        }
        VoxelObjectSDFModification::MultiscaleSphere(modification) => {
            generate_voxel_object_with_modifications_2(
                voxel_type_generator,
                voxel_extent,
                modification.apply(sdf_generator),
                sdf_modifications,
            )
        }
        VoxelObjectSDFModification::MultifractalNoise(modification) => {
            generate_voxel_object_with_modifications_2(
                voxel_type_generator,
                voxel_extent,
                modification.apply(sdf_generator),
                sdf_modifications,
            )
        }
    }
}

fn generate_voxel_object_with_modifications_2(
    voxel_type_generator: impl VoxelTypeGenerator,
    voxel_extent: f64,
    sdf_generator: impl SDFGenerator,
    sdf_modifications: &[VoxelObjectSDFModification; MAX_MODIFICATIONS],
) -> Option<ChunkedVoxelObject> {
    match &sdf_modifications[1] {
        VoxelObjectSDFModification::None => {
            generate_voxel_object(voxel_type_generator, voxel_extent, sdf_generator)
        }
        VoxelObjectSDFModification::MultiscaleSphere(modification) => generate_voxel_object(
            voxel_type_generator,
            voxel_extent,
            modification.apply(sdf_generator),
        ),
        VoxelObjectSDFModification::MultifractalNoise(modification) => generate_voxel_object(
            voxel_type_generator,
            voxel_extent,
            modification.apply(sdf_generator),
        ),
    }
}

fn generate_voxel_object(
    voxel_type_generator: impl VoxelTypeGenerator,
    voxel_extent: f64,
    sdf_generator: impl SDFGenerator,
) -> Option<ChunkedVoxelObject> {
    let generator = SDFVoxelGenerator::new(voxel_extent, sdf_generator, voxel_type_generator);
    ChunkedVoxelObject::generate(&generator)
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

fn gradient_noise_voxel_type_generator_from_component(
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_types: &GradientNoiseVoxelTypes,
) -> Result<GradientNoiseVoxelTypeGenerator> {
    Ok(GradientNoiseVoxelTypeGenerator::new(
        voxel_types.voxel_types(voxel_type_registry)?,
        voxel_types.noise_frequency(),
        voxel_types.voxel_type_frequency(),
        u32::try_from(voxel_types.seed()).unwrap(),
    ))
}
