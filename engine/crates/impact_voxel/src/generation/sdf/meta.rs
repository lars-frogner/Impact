//! Generation of signed distance fields. This module implements the graph of
//! high-level "meta" SDF nodes that is compiled into the runtime graph of
//! simpler atomic nodes.

pub mod params;

use crate::{
    define_meta_node_params,
    generation::sdf::{SDFGenerator, SDFGeneratorBlockBuffers, SDFGraph, SDFNode, SDFNodeID},
};
use anyhow::{Context, Result, anyhow, bail};
use approx::{abs_diff_eq, abs_diff_ne};
use impact_alloc::{
    AVec, Allocator, Global,
    arena::{ArenaPool, PoolArena},
    avec,
};
use impact_containers::FixedQueue;
use impact_geometry::{Sphere, compute_uniformly_distributed_radial_directions};
use impact_math::{
    angle::{Angle, Degrees},
    consts::f32::TWO_PI,
    point::Point3,
    quaternion::UnitQuaternion,
    splitmix,
    transform::Similarity3,
    vector::{UnitVector3, Vector3},
};
use params::{ContParamSpec, DiscreteParamSpec, ParamRng, create_param_rng};
use rand::{
    Rng,
    distr::{Distribution, Uniform},
    seq::IndexedRandom,
};
use std::{array, borrow::Cow, f32::consts::PI, mem};

#[derive(Clone, Debug)]
pub struct MetaSDFGraph<A: Allocator = Global> {
    nodes: AVec<MetaSDFNode, A>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum MetaSDFNode {
    // Instance primitives
    Points(MetaPoints),
    Spheres(MetaSpheres),
    Capsules(MetaCapsules),
    Boxes(MetaBoxes),

    // Basic instance transforms
    Translation(MetaTranslation),
    Rotation(MetaRotation),
    Scaling(MetaScaling),
    Similarity(MetaSimilarity),

    // Structured instance transforms
    StratifiedGridTransforms(MetaStratifiedGridTransforms),
    SphereSurfaceTransforms(MetaSphereSurfaceTransforms),

    // SDF based instance transforms
    ClosestTranslationToSurface(MetaClosestTranslationToSurface),
    RayTranslationToSurface(MetaRayTranslationToSurface),
    RotationToGradient(MetaRotationToGradient),

    // Filtering
    StochasticSelection(MetaStochasticSelection),

    // SDF from instances
    SDFInstantiation(MetaSDFInstantiation),
    TransformApplication(MetaTransformApplication),

    // SDF modifiers
    MultifractalNoiseSDFModifier(MetaMultifractalNoiseSDFModifier),
    MultiscaleSphereSDFModifier(MetaMultiscaleSphereSDFModifier),

    // SDF combination
    SDFUnion(MetaSDFUnion),
    SDFSubtraction(MetaSDFSubtraction),
    SDFIntersection(MetaSDFIntersection),
    SDFGroupUnion(MetaSDFGroupUnion),
}

pub type MetaSDFNodeID = u32;

#[derive(Clone, Debug)]
enum BuildOperation {
    VisitChildren(MetaSDFNodeID),
    Process(MetaSDFNodeID),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetaNodeBuildState {
    Unvisited,
    ChildrenBeingVisited,
    Resolved,
}

#[derive(Clone, Debug)]
enum MetaSDFNodeOutput<A: Allocator> {
    SingleSDF(Option<SDFNodeID>),
    SDFGroup(AVec<SDFNodeID, A>),
    Instances(AVec<Instance, A>),
}

#[derive(Clone, Debug)]
struct Instance {
    shape: InstanceShape,
    /// Transform from the local space of this instance's shape into this
    /// instance's coordinate space.
    transform: Similarity3,
}

#[derive(Clone, Copy, Debug)]
enum InstanceShape {
    None,
    Sphere(SphereShape),
    Capsule(CapsuleShape),
    Box(BoxShape),
}

#[derive(Clone, Copy, Debug)]
struct SphereShape {
    radius: f32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
}

#[derive(Clone, Copy, Debug)]
struct CapsuleShape {
    segment_length: f32,
    radius: f32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
}

#[derive(Clone, Copy, Debug)]
struct BoxShape {
    extent_x: f32,
    extent_y: f32,
    extent_z: f32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
}

/// A set of instances with no shape, each having an identity transform.
///
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaPoints {
    /// Number of points to generate.
    pub count: u32,
}

/// A set of sphere instances, each having an identity transform.
///
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSpheres {
    /// Sphere radius, in voxels.
    pub radius: ContParamSpec,
    /// Sphere center x-coordinate, in voxels.
    pub center_x: ContParamSpec,
    /// Sphere center y-coordinate, in voxels.
    pub center_y: ContParamSpec,
    /// Sphere center z-coordinate, in voxels.
    pub center_z: ContParamSpec,
    /// Number of spheres to generate.
    pub count: u32,
    /// Seed for generating randomized radius values.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// instances.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaSpheres,
    struct MetaSphereInstanceParams {
        radius: f32,
        center_x: f32,
        center_y: f32,
        center_z: f32,
    }
}

/// A set of vertical capsule instances, each having an identity transform.
///
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaCapsules {
    /// Length between the centers of the spherical caps, in voxels.
    pub segment_length: ContParamSpec,
    /// Radius of the spherical caps, in voxels.
    pub radius: ContParamSpec,
    /// Capsule center x-coordinate, in voxels.
    pub center_x: ContParamSpec,
    /// Capsule center y-coordinate, in voxels.
    pub center_y: ContParamSpec,
    /// Capsule center z-coordinate, in voxels.
    pub center_z: ContParamSpec,
    /// Number of capsules to generate.
    pub count: u32,
    /// Seed for generating randomized segment length and radius values.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// instances.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaCapsules,
    struct MetaCapsuleInstanceParams {
        segment_length: f32,
        radius: f32,
        center_x: f32,
        center_y: f32,
        center_z: f32,
    }
}

/// A set of box instances, each having an identity transform.
///
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaBoxes {
    /// Extent along the x-axis, in voxels.
    pub extent_x: ContParamSpec,
    /// Extent along the y-axis, in voxels.
    pub extent_y: ContParamSpec,
    /// Extent along the z-axis, in voxels.
    pub extent_z: ContParamSpec,
    /// Box center x-coordinate, in voxels.
    pub center_x: ContParamSpec,
    /// Box center y-coordinate, in voxels.
    pub center_y: ContParamSpec,
    /// Box center z-coordinate, in voxels.
    pub center_z: ContParamSpec,
    /// Number of boxes to generate.
    pub count: u32,
    /// Seed for generating randomized extent values.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// instances.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaBoxes,
    struct MetaBoxInstanceParams {
        extent_x: f32,
        extent_y: f32,
        extent_z: f32,
        center_x: f32,
        center_y: f32,
        center_z: f32,
    }
}

/// Translation of one or more instances.
///
/// Input: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaTranslation {
    /// ID of the child instance node to translate.
    pub child_id: MetaSDFNodeID,
    /// Whether to apply the translation before ('Pre') or after ('Post') the
    /// transforms of the input instances.
    pub composition: CompositionMode,
    /// Translation distance along the x-axis, in voxels.
    pub translation_x: ContParamSpec,
    /// Translation distance along the y-axis, in voxels.
    pub translation_y: ContParamSpec,
    /// Translation distance along the z-axis, in voxels.
    pub translation_z: ContParamSpec,
    /// Seed for generating randomized translations.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// instances.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaTranslation,
    struct MetaTranslationParams {
        translation_x: f32,
        translation_y: f32,
        translation_z: f32,
    }
}

/// Rotation of one or more instances.
///
/// Input: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaRotation {
    /// ID of the child instance node to rotate.
    pub child_id: MetaSDFNodeID,
    /// Whether to apply the rotation before ('Pre') or after ('Post') the
    /// transforms of the input instances.
    pub composition: CompositionMode,
    /// Angle away from the y-axis, in degrees.
    pub tilt_angle: ContParamSpec,
    /// Angle from the x-axis in the xz-plane, in degrees.
    pub turn_angle: ContParamSpec,
    /// Additional roll angle around the final rotated axis, in degrees.
    pub roll_angle: ContParamSpec,
    /// Seed for generating randomized rotations.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// instances.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaRotation,
    struct MetaRotationParams {
        tilt_angle: f32,
        turn_angle: f32,
        roll_angle: f32,
    }
}

/// Uniform scaling of one or more instances.
///
/// Input: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaScaling {
    /// ID of the child instance node to scale.
    pub child_id: MetaSDFNodeID,
    /// Whether to apply the scaling before ('Pre') or after ('Post') the
    /// transforms of the input instances.
    pub composition: CompositionMode,
    /// Uniform scale factor.
    pub scaling: ContParamSpec,
    /// Seed for generating randomized scale factors.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// instances.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaScaling,
    struct MetaScalingParams {
        scaling: f32,
    }
}

/// Similarity transformation (scale, rotate, translate) of one or more
/// instances.
///
/// Input: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSimilarity {
    /// ID of the child instance node to apply the similarity transform to.
    pub child_id: MetaSDFNodeID,
    /// Whether to apply the similarity transform before ('Pre') or after
    /// ('Post') the transforms of the input instances.
    pub composition: CompositionMode,
    /// Uniform scale factor.
    pub scale: ContParamSpec,
    /// Angle away from the y-axis, in degrees.
    pub tilt_angle: ContParamSpec,
    /// Angle from the x-axis in the xz-plane, in degrees.
    pub turn_angle: ContParamSpec,
    /// Additional roll angle around the final rotated axis, in degrees.
    pub roll_angle: ContParamSpec,
    /// Translation distance along the x-axis, in voxels.
    pub translation_x: ContParamSpec,
    /// Translation distance along the y-axis, in voxels.
    pub translation_y: ContParamSpec,
    /// Translation distance along the z-axis, in voxels.
    pub translation_z: ContParamSpec,
    /// Seed for generating randomized similarity transforms.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// instances.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaSimilarity,
    struct MetaSimilarityParams {
        scale: f32,
        tilt_angle: f32,
        turn_angle: f32,
        roll_angle: f32,
        translation_x: f32,
        translation_y: f32,
        translation_z: f32,
    }
}

/// Translation of instances from the center of a grid to grid points picked by
/// stratified sampling.
///
/// Input: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaStratifiedGridTransforms {
    /// ID of the child instance node to translate.
    pub child_id: MetaSDFNodeID,
    /// Number of grid cells along the x-axis.
    pub shape_x: DiscreteParamSpec,
    /// Number of grid cells along the y-axis.
    pub shape_y: DiscreteParamSpec,
    /// Number of grid cells along the z-axis.
    pub shape_z: DiscreteParamSpec,
    /// Extent of a grid cell along the x-axis, in voxels.
    pub cell_extent_x: ContParamSpec,
    /// Extent of a grid cell along the y-axis, in voxels.
    pub cell_extent_y: ContParamSpec,
    /// Extent of a grid cell along the z-axis, in voxels.
    pub cell_extent_z: ContParamSpec,
    /// Fraction of a grid cell to randomly displace the points.
    pub jitter_fraction: ContParamSpec,
    /// Seed for random jittering as well as generating randomized parameter
    /// values.
    pub seed: u32,
}

define_meta_node_params! {
    MetaStratifiedGridTransforms,
    struct MetaStratifiedGridParams {
        shape_x: u32,
        shape_y: u32,
        shape_z: u32,
        cell_extent_x: f32,
        cell_extent_y: f32,
        cell_extent_z: f32,
        jitter_fraction: f32,
    }
}

/// Translation of instances from the center to the surface of a sphere, with
/// optional rotations from the y-axis to the radial direction.
///
/// Input: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSphereSurfaceTransforms {
    /// ID of the child instance node to translate.
    pub child_id: MetaSDFNodeID,
    /// Radius of the sphere, in voxels.
    pub radius: ContParamSpec,
    /// Fraction of the regular point spacing to randomly displace the points.
    pub jitter_fraction: ContParamSpec,
    /// Whether to include rotations from the y-axes to the outward or inward
    /// radial direction.
    pub rotation: SphereSurfaceRotation,
    /// Seed for random jittering as well as generating randomized parameter
    /// values.
    pub seed: u32,
}

define_meta_node_params! {
    MetaSphereSurfaceTransforms,
    struct MetaSphereSurfaceParams {
        radius: f32,
        jitter_fraction: f32,
    }
}

/// Translation of the instances in the second input to the closest points on
/// the surface of the SDF in the first input.
///
/// Input 1: `SingleSDF`
/// Input 2: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaClosestTranslationToSurface {
    /// ID of the SDF node whose surface to translate to.
    pub surface_sdf_id: MetaSDFNodeID,
    /// ID of the node containing instances to translate.
    pub subject_id: MetaSDFNodeID,
}

/// Translation of the instances in the second input along their y-axes until a
/// chosen anchor reaches the surface of the SDF in the first input.
///
/// Input 1: `SingleSDF`
/// Input 2: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaRayTranslationToSurface {
    /// ID of the SDF node whose surface to translate to.
    pub surface_sdf_id: MetaSDFNodeID,
    /// ID of the node containing instances to translate.
    pub subject_id: MetaSDFNodeID,
    /// The anchor (origin or shape boundary) that should be translated to the
    /// surface.
    pub anchor: RayTranslationAnchor,
}

/// Rotation of the instances in the second input to make their y-axis align
/// with the gradient of the SDF in the first input.
///
/// Input 1: `SingleSDF`
/// Input 2: `Instances`
/// Output: `Instances`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaRotationToGradient {
    /// ID of the SDF node whose gradient to align with.
    pub gradient_sdf_id: MetaSDFNodeID,
    /// ID of the node containing instances to rotate.
    pub subject_id: MetaSDFNodeID,
}

/// Random selection of SDFs or instances from a group.
///
/// Input: Any
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaStochasticSelection {
    /// ID of the child group node to select from.
    pub child_id: MetaSDFNodeID,
    /// Minimum number of items to select initially.
    pub min_pick_count: u32,
    /// Maximum number of items to select initially.
    pub max_pick_count: u32,
    /// Probability that each of the initially selected items will be kept in
    /// the final selection.
    pub pick_probability: f32,
    /// Seed for random selection.
    pub seed: u32,
}

/// Instantiation of the input instances into SDFs using their shapes and
/// transforms. Instances with no shape produce no SDFs.
///
/// Input: `Instances`
/// Output: `SDFGroup`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFInstantiation {
    /// ID of the node with the instances to convert to SDFs.
    pub child_id: MetaSDFNodeID,
}

/// Application of the transforms of the instances in the second input to the
/// SDFs in the first input (yields all combinations).
///
/// Input 1: `SDFGroup` or `SingleSDF`
/// Input 2: `Instances`
/// Output: `SDFGroup`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaTransformApplication {
    /// ID of the SDF or SDF group node to apply transforms to.
    pub sdf_id: MetaSDFNodeID,
    /// ID of the instance or instance group node with the transforms to apply.
    pub instance_id: MetaSDFNodeID,
}

/// Perturbation of one or more SDFs using a multifractal noise field.
///
/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaMultifractalNoiseSDFModifier {
    /// ID of the child SDF node to modify.
    pub child_id: MetaSDFNodeID,
    /// Number of noise octaves (patterns of increasing frequency) to combine.
    pub octaves: DiscreteParamSpec,
    /// Spatial frequency of the noise pattern in the first octave, in inverse
    /// voxels.
    pub frequency: ContParamSpec,
    /// Noise frequency multiplier between successive octaves.
    pub lacunarity: ContParamSpec,
    /// Noise amplitude multiplier between successive octaves.
    pub persistence: ContParamSpec,
    /// Noise amplitude (max displacement) in the first octave, in voxels.
    pub amplitude: ContParamSpec,
    /// Seed for generating noise and randomized parameter values.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// SDFs.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaMultifractalNoiseSDFModifier,
    struct MetaMultifractalNoiseParams {
        octaves: u32,
        frequency: f32,
        lacunarity: f32,
        persistence: f32,
        amplitude: f32,
    }
}

/// Perturbation of one or more SDFs by intersecting and combining with grids
/// of spheres on multiple scales.
///
/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaMultiscaleSphereSDFModifier {
    /// ID of the child SDF node to modify.
    pub child_id: MetaSDFNodeID,
    /// Number of sphere scales to combine for detail variation.
    pub octaves: DiscreteParamSpec,
    /// Maximum scale of variation in the multiscale pattern, in voxels.
    pub max_scale: ContParamSpec,
    /// Scale multiplier between successive octaves.
    pub persistence: ContParamSpec,
    /// Amount to expand the pattern being modified before intersecting with
    /// spheres, in factors of the max scale.
    pub inflation: ContParamSpec,
    /// Smoothness factor for intersecting spheres with the inflated version of
    /// the pattern being modified.
    pub intersection_smoothness: ContParamSpec,
    /// Smoothness factor for combining the intersected sphere pattern with the
    /// original pattern.
    pub union_smoothness: ContParamSpec,
    /// Seed for generating random sphere radii as well as randomized
    /// parameter values.
    pub seed: u32,
    /// How to sample parameters from distributions when there are multiple
    /// SDFs.
    pub sampling: ParameterSamplingMode,
}

define_meta_node_params! {
    MetaMultiscaleSphereSDFModifier,
    struct MetaMultiscaleSphereParams {
        octaves: u32,
        max_scale: f32,
        persistence: f32,
        inflation: f32,
        intersection_smoothness: f32,
        union_smoothness: f32,
    }
}

/// Smooth union of two SDFs.
///
/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFUnion {
    /// ID of the first SDF node to combine.
    pub child_1_id: MetaSDFNodeID,
    /// ID of the second SDF node to combine.
    pub child_2_id: MetaSDFNodeID,
    /// Smoothness factor for blending the two shapes together.
    pub smoothness: f32,
}

/// Smooth subtraction of the second SDF from the first.
///
/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFSubtraction {
    /// ID of the SDF node to subtract from.
    pub child_1_id: MetaSDFNodeID,
    /// ID of the SDF node to subtract.
    pub child_2_id: MetaSDFNodeID,
    /// Smoothness factor for blending the subtraction operation.
    pub smoothness: f32,
}

/// Smooth intersection of two SDFs.
///
/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFIntersection {
    /// ID of the first SDF node to intersect.
    pub child_1_id: MetaSDFNodeID,
    /// ID of the second SDF node to intersect.
    pub child_2_id: MetaSDFNodeID,
    /// Smoothness factor for blending the intersection operation.
    pub smoothness: f32,
}

/// Smooth union of all SDFs in a group.
///
/// Input: `SDFGroup` or `SingleSDF`
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFGroupUnion {
    /// ID of the SDF group node to union.
    pub child_id: MetaSDFNodeID,
    /// Smoothness factor for blending all the shapes in the group together.
    pub smoothness: f32,
}

/// How to combine the current transformation with the input transformation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionMode {
    /// Apply the current transformation to the subject *after* applying the
    /// input transformation.
    Post,
    /// Apply the current transformation to the subject *before* applying the
    /// input transformation.
    Pre,
}

/// How to sample parameters from distributions when there are multiple
/// instances or SDFs.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterSamplingMode {
    /// Sample the parameters once and use for all instances.
    OnlyOnce,
    /// Sample a new set of parameters for each instance.
    PerInstance,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RayTranslationAnchor {
    /// Place the instance's transform origin on the surface.
    Origin,
    /// Place the boundary of the instance’s shape, treated as if centered at
    /// the instance’s transform origin, on the surface.
    ShapeBoundaryAtOrigin,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SphereSurfaceRotation {
    Identity,
    RadialOutwards,
    RadialInwards,
}

impl MetaSDFGraph<Global> {
    pub fn new() -> Self {
        Self::new_in(Global)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity, Global)
    }
}

impl<A: Allocator> MetaSDFGraph<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            nodes: AVec::new_in(alloc),
        }
    }

    pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
        Self {
            nodes: AVec::with_capacity_in(capacity, alloc),
        }
    }

    pub fn add_node(&mut self, node: MetaSDFNode) -> MetaSDFNodeID {
        let id = self.nodes.len().try_into().unwrap();
        self.nodes.push(node);
        id
    }

    pub fn build_in<AG: Allocator>(&self, alloc: AG, seed: u64) -> Result<SDFGraph<AG>> {
        let mut graph = SDFGraph::new_in(alloc);

        if self.nodes.is_empty() {
            return Ok(graph);
        }

        // Estimate capacity based on node count for outputs and processing
        let capacity = self.nodes.len() * (mem::size_of::<MetaSDFNodeOutput<&PoolArena>>() + 128); // Output + overhead per node
        let arena = ArenaPool::get_arena_for_capacity(capacity);

        let mut outputs =
            avec![in &arena; MetaSDFNodeOutput::<&PoolArena>::SingleSDF(None); self.nodes.len()];

        let mut states = avec![in &arena; MetaNodeBuildState::Unvisited; self.nodes.len()];

        let mut stable_seeds = avec![in &arena; 0u64; self.nodes.len()];

        let mut operation_stack = AVec::with_capacity_in(3 * self.nodes.len(), &arena);

        let root_node_id = (self.nodes.len() - 1) as MetaSDFNodeID;
        operation_stack.push(BuildOperation::VisitChildren(root_node_id));

        while let Some(operation) = operation_stack.pop() {
            match operation {
                BuildOperation::VisitChildren(node_id) => {
                    let node_idx = node_id as usize;

                    let state = states
                        .get_mut(node_idx)
                        .ok_or_else(|| anyhow!("Missing meta SDF node {node_id}"))?;

                    match *state {
                        MetaNodeBuildState::Resolved => {
                            // Already resolved via a different parent
                        }
                        MetaNodeBuildState::ChildrenBeingVisited => {
                            // We got back to the same node while visiting its children
                            bail!("Detected cycle in meta SDF node graph")
                        }
                        MetaNodeBuildState::Unvisited => {
                            *state = MetaNodeBuildState::ChildrenBeingVisited;

                            operation_stack.push(BuildOperation::Process(node_id));

                            match &self.nodes[node_idx] {
                                MetaSDFNode::Points(_)
                                | MetaSDFNode::Spheres(_)
                                | MetaSDFNode::Capsules(_)
                                | MetaSDFNode::Boxes(_) => {}
                                MetaSDFNode::Translation(MetaTranslation { child_id, .. })
                                | MetaSDFNode::Rotation(MetaRotation { child_id, .. })
                                | MetaSDFNode::Scaling(MetaScaling { child_id, .. })
                                | MetaSDFNode::Similarity(MetaSimilarity { child_id, .. })
                                | MetaSDFNode::StratifiedGridTransforms(
                                    MetaStratifiedGridTransforms { child_id, .. },
                                )
                                | MetaSDFNode::SphereSurfaceTransforms(
                                    MetaSphereSurfaceTransforms { child_id, .. },
                                )
                                | MetaSDFNode::StochasticSelection(MetaStochasticSelection {
                                    child_id,
                                    ..
                                })
                                | MetaSDFNode::SDFInstantiation(MetaSDFInstantiation {
                                    child_id,
                                })
                                | MetaSDFNode::MultifractalNoiseSDFModifier(
                                    MetaMultifractalNoiseSDFModifier { child_id, .. },
                                )
                                | MetaSDFNode::MultiscaleSphereSDFModifier(
                                    MetaMultiscaleSphereSDFModifier { child_id, .. },
                                )
                                | MetaSDFNode::SDFGroupUnion(MetaSDFGroupUnion {
                                    child_id, ..
                                }) => {
                                    operation_stack.push(BuildOperation::VisitChildren(*child_id));
                                }

                                MetaSDFNode::ClosestTranslationToSurface(
                                    MetaClosestTranslationToSurface {
                                        surface_sdf_id: child_1_id,
                                        subject_id: child_2_id,
                                    },
                                )
                                | MetaSDFNode::RayTranslationToSurface(
                                    MetaRayTranslationToSurface {
                                        surface_sdf_id: child_1_id,
                                        subject_id: child_2_id,
                                        ..
                                    },
                                )
                                | MetaSDFNode::RotationToGradient(MetaRotationToGradient {
                                    gradient_sdf_id: child_1_id,
                                    subject_id: child_2_id,
                                })
                                | MetaSDFNode::TransformApplication(MetaTransformApplication {
                                    sdf_id: child_1_id,
                                    instance_id: child_2_id,
                                })
                                | MetaSDFNode::SDFUnion(MetaSDFUnion {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | MetaSDFNode::SDFSubtraction(MetaSDFSubtraction {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | MetaSDFNode::SDFIntersection(MetaSDFIntersection {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                }) => {
                                    operation_stack
                                        .push(BuildOperation::VisitChildren(*child_2_id));
                                    operation_stack
                                        .push(BuildOperation::VisitChildren(*child_1_id));
                                }
                            }
                        }
                    }
                }
                BuildOperation::Process(node_id) => {
                    let node_idx = node_id as usize;
                    let state = &mut states[node_idx];
                    *state = MetaNodeBuildState::Resolved;

                    let node = &self.nodes[node_idx];

                    let stable_seed = node.obtain_stable_seed(&stable_seeds);
                    stable_seeds[node_idx] = stable_seed;

                    let seed = splitmix::random_u64_from_two_states(seed, stable_seed);

                    outputs[node_idx] = node.resolve(&arena, &mut graph, &outputs, seed)?;
                }
            }
        }

        if let MetaSDFNodeOutput::SingleSDF(atomic_node_id) = &outputs[root_node_id as usize] {
            if let Some(id) = atomic_node_id {
                graph.set_root_node(*id);
            } else {
                return Ok(SDFGraph::new_in(alloc));
            }
        } else {
            bail!("Root meta node must have single SDF output");
        }

        Ok(graph)
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

impl Default for MetaSDFGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "serde")]
impl<A: Allocator> serde::Serialize for MetaSDFGraph<A> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("MetaSDFGraph", 1)?;
        s.serialize_field("nodes", &self.nodes)?;
        s.end()
    }
}

#[cfg(feature = "serde")]
impl<'de, A> serde::Deserialize<'de> for MetaSDFGraph<A>
where
    A: Allocator + Default,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::{fmt, marker::PhantomData};

        struct MetaSDFGraphVisitor<A>(PhantomData<A>);

        impl<'de, A> Visitor<'de> for MetaSDFGraphVisitor<A>
        where
            A: Allocator + Default,
        {
            type Value = MetaSDFGraph<A>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("struct MetaSDFGraph")
            }

            fn visit_map<V>(self, mut map: V) -> Result<MetaSDFGraph<A>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut nodes = None;
                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "nodes" => {
                            if nodes.is_some() {
                                return Err(de::Error::duplicate_field("nodes"));
                            }
                            nodes = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                let nodes = nodes.ok_or_else(|| de::Error::missing_field("nodes"))?;
                Ok(MetaSDFGraph { nodes })
            }
        }

        deserializer.deserialize_struct(
            "MetaSDFGraph",
            &["nodes"],
            MetaSDFGraphVisitor(PhantomData),
        )
    }
}

impl<A: Allocator> MetaSDFNodeOutput<A> {
    fn label(&self) -> &'static str {
        match self {
            Self::SingleSDF(_) => "SingleSDF",
            Self::SDFGroup(_) => "SDFGroup",
            Self::Instances(_) => "Instances",
        }
    }
}

impl MetaSDFNode {
    /// Combines a node type tag, node seed parameter (for applicable nodes) and
    /// the stable seeds of the child nodes to obtain a stable seed that will
    /// only change due to changes in the seeding, types or topology of the
    /// node's subgraph.
    fn obtain_stable_seed(&self, stable_seeds: &[u64]) -> u64 {
        let leaf = |tag| splitmix::random_u64_from_state(tag);

        let combine_seeded_leaf =
            |tag, seed: &u32| splitmix::random_u64_from_two_states(tag, (*seed).into());

        let combine_unary = |tag, child_id: &MetaSDFNodeID| {
            splitmix::random_u64_from_two_states(tag, stable_seeds[*child_id as usize])
        };

        let combine_seeded_unary = |tag, seed: &u32, child_id: &MetaSDFNodeID| {
            splitmix::random_u64_from_three_states(
                tag,
                (*seed).into(),
                stable_seeds[*child_id as usize],
            )
        };

        let combine_binary = |tag, child_1_id: &MetaSDFNodeID, child_2_id: &MetaSDFNodeID| {
            splitmix::random_u64_from_three_states(
                tag,
                stable_seeds[*child_1_id as usize],
                stable_seeds[*child_2_id as usize],
            )
        };

        let combine_binary_commutative =
            |tag, child_1_id: &MetaSDFNodeID, child_2_id: &MetaSDFNodeID| {
                let s1 = stable_seeds[*child_1_id as usize];
                let s2 = stable_seeds[*child_2_id as usize];
                let (lo, hi) = if s1 <= s2 { (s1, s2) } else { (s2, s1) };
                splitmix::random_u64_from_three_states(tag, lo, hi)
            };

        match self {
            Self::Points(MetaPoints { .. }) => leaf(0x00),
            Self::Spheres(MetaSpheres { seed, .. }) => combine_seeded_leaf(0x01, seed),
            Self::Capsules(MetaCapsules { seed, .. }) => combine_seeded_leaf(0x02, seed),
            Self::Boxes(MetaBoxes { seed, .. }) => combine_seeded_leaf(0x03, seed),
            Self::Translation(MetaTranslation { seed, child_id, .. }) => {
                combine_seeded_unary(0x10, seed, child_id)
            }
            Self::Rotation(MetaRotation { seed, child_id, .. }) => {
                combine_seeded_unary(0x11, seed, child_id)
            }
            Self::Scaling(MetaScaling { seed, child_id, .. }) => {
                combine_seeded_unary(0x12, seed, child_id)
            }
            Self::Similarity(MetaSimilarity { seed, child_id, .. }) => {
                combine_seeded_unary(0x13, seed, child_id)
            }
            Self::StratifiedGridTransforms(MetaStratifiedGridTransforms {
                seed, child_id, ..
            }) => combine_seeded_unary(0x14, seed, child_id),
            Self::SphereSurfaceTransforms(MetaSphereSurfaceTransforms {
                seed, child_id, ..
            }) => combine_seeded_unary(0x15, seed, child_id),
            Self::ClosestTranslationToSurface(MetaClosestTranslationToSurface {
                surface_sdf_id,
                subject_id,
            }) => combine_binary(0x20, surface_sdf_id, subject_id),
            Self::RayTranslationToSurface(MetaRayTranslationToSurface {
                surface_sdf_id,
                subject_id,
                ..
            }) => combine_binary(0x21, surface_sdf_id, subject_id),
            Self::RotationToGradient(MetaRotationToGradient {
                gradient_sdf_id,
                subject_id,
            }) => combine_binary(0x22, gradient_sdf_id, subject_id),
            Self::StochasticSelection(MetaStochasticSelection { seed, child_id, .. }) => {
                combine_seeded_unary(0x30, seed, child_id)
            }
            Self::SDFInstantiation(MetaSDFInstantiation { child_id }) => {
                combine_unary(0x40, child_id)
            }
            Self::TransformApplication(MetaTransformApplication {
                sdf_id,
                instance_id,
            }) => combine_binary(0x41, sdf_id, instance_id),
            Self::MultifractalNoiseSDFModifier(MetaMultifractalNoiseSDFModifier {
                seed,
                child_id,
                ..
            }) => combine_seeded_unary(0x50, seed, child_id),
            Self::MultiscaleSphereSDFModifier(MetaMultiscaleSphereSDFModifier {
                seed,
                child_id,
                ..
            }) => combine_seeded_unary(0x51, seed, child_id),
            Self::SDFUnion(MetaSDFUnion {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary_commutative(0x60, child_1_id, child_2_id),
            Self::SDFSubtraction(MetaSDFSubtraction {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary(0x61, child_1_id, child_2_id),
            Self::SDFIntersection(MetaSDFIntersection {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary_commutative(0x62, child_1_id, child_2_id),
            Self::SDFGroupUnion(MetaSDFGroupUnion { child_id, .. }) => {
                combine_unary(0x63, child_id)
            }
        }
    }

    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<AR>> {
        match self {
            Self::Points(node) => Ok(node.resolve(arena)),
            Self::Spheres(node) => node
                .resolve(arena, seed)
                .context("Failed to resolve Spheres node"),
            Self::Capsules(node) => node
                .resolve(arena, seed)
                .context("Failed to resolve Capsules node"),
            Self::Boxes(node) => node
                .resolve(arena, seed)
                .context("Failed to resolve Boxes node"),
            Self::Translation(node) => node
                .resolve(arena, outputs, seed)
                .context("Failed to resolve Translation node"),
            Self::Rotation(node) => node
                .resolve(arena, outputs, seed)
                .context("Failed to resolve Rotation node"),
            Self::Scaling(node) => node
                .resolve(arena, outputs, seed)
                .context("Failed to resolve Scaling node"),
            Self::Similarity(node) => node
                .resolve(arena, outputs, seed)
                .context("Failed to resolve Similarity node"),
            Self::StratifiedGridTransforms(node) => node
                .resolve(arena, outputs, seed)
                .context("Failed to resolve StratifiedGridTransforms node"),
            Self::SphereSurfaceTransforms(node) => node
                .resolve(arena, outputs, seed)
                .context("Failed to resolve SphereSurfaceTransforms node"),
            Self::ClosestTranslationToSurface(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve ClosestTranslationToSurface node"),
            Self::RayTranslationToSurface(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve RayTranslationToSurface node"),
            Self::RotationToGradient(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve RotationToGradient node"),
            Self::StochasticSelection(node) => Ok(node.resolve(arena, outputs, seed)),
            Self::SDFInstantiation(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve SDFInstantiation node"),
            Self::TransformApplication(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve TransformApplication node"),
            Self::MultifractalNoiseSDFModifier(node) => node
                .resolve(arena, graph, outputs, seed)
                .context("Failed to resolve MultifractalNoiseSDFModifier node"),
            Self::MultiscaleSphereSDFModifier(node) => node
                .resolve(arena, graph, outputs, seed)
                .context("Failed to resolve MultiscaleSphereSDFModifier node"),
            Self::SDFUnion(node) => node
                .resolve(graph, outputs)
                .context("Failed to resolve SDFUnion node"),
            Self::SDFSubtraction(node) => node
                .resolve(graph, outputs)
                .context("Failed to resolve SDFSubtraction node"),
            Self::SDFIntersection(node) => node
                .resolve(graph, outputs)
                .context("Failed to resolve SDFIntersection node"),
            Self::SDFGroupUnion(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve SDFGroupUnion node"),
        }
    }
}

impl Instance {
    fn shapeless(transform: Similarity3) -> Self {
        Self {
            shape: InstanceShape::None,
            transform,
        }
    }

    fn with_transform(&self, transform: Similarity3) -> Self {
        Self {
            shape: self.shape,
            transform,
        }
    }

    fn with_applied_transform(&self, transform: Similarity3) -> Self {
        Self {
            shape: self.shape,
            transform: transform * &self.transform,
        }
    }
}

impl MetaPoints {
    fn resolve<A: Allocator>(&self, arena: A) -> MetaSDFNodeOutput<A> {
        MetaSDFNodeOutput::Instances(
            avec![in arena; Instance::shapeless(Similarity3::identity()); self.count as usize],
        )
    }
}

impl MetaSpheres {
    fn resolve<A: Allocator>(&self, arena: A, seed: u64) -> Result<MetaSDFNodeOutput<A>> {
        let mut rng = create_param_rng(seed);

        let mut instances = AVec::with_capacity_in(self.count as usize, arena);

        let mut params = self.sample_params(&mut rng)?;

        for idx in 0..self.count {
            instances.push(Instance {
                shape: InstanceShape::Sphere(SphereShape {
                    radius: params.radius,
                    center_x: params.center_x,
                    center_y: params.center_y,
                    center_z: params.center_z,
                }),
                transform: Similarity3::identity(),
            });

            if self.sampling == ParameterSamplingMode::PerInstance && idx + 1 < self.count {
                params = self.sample_params(&mut rng)?;
            }
        }

        Ok(MetaSDFNodeOutput::Instances(instances))
    }
}

impl MetaCapsules {
    fn resolve<A: Allocator>(&self, arena: A, seed: u64) -> Result<MetaSDFNodeOutput<A>> {
        let mut rng = create_param_rng(seed);

        let mut instances = AVec::with_capacity_in(self.count as usize, arena);

        let mut params = self.sample_params(&mut rng)?;

        for idx in 0..self.count {
            instances.push(Instance {
                shape: InstanceShape::Capsule(CapsuleShape {
                    segment_length: params.segment_length,
                    radius: params.radius,
                    center_x: params.center_x,
                    center_y: params.center_y,
                    center_z: params.center_z,
                }),
                transform: Similarity3::identity(),
            });

            if self.sampling == ParameterSamplingMode::PerInstance && idx + 1 < self.count {
                params = self.sample_params(&mut rng)?;
            }
        }

        Ok(MetaSDFNodeOutput::Instances(instances))
    }
}

impl MetaBoxes {
    fn resolve<A: Allocator>(&self, arena: A, seed: u64) -> Result<MetaSDFNodeOutput<A>> {
        let mut rng = create_param_rng(seed);

        let mut instances = AVec::with_capacity_in(self.count as usize, arena);

        let mut params = self.sample_params(&mut rng)?;

        for idx in 0..self.count {
            instances.push(Instance {
                shape: InstanceShape::Box(BoxShape {
                    extent_x: params.extent_x,
                    extent_y: params.extent_y,
                    extent_z: params.extent_z,
                    center_x: params.center_x,
                    center_y: params.center_y,
                    center_z: params.center_z,
                }),
                transform: Similarity3::identity(),
            });

            if self.sampling == ParameterSamplingMode::PerInstance && idx + 1 < self.count {
                params = self.sample_params(&mut rng)?;
            }
        }

        Ok(MetaSDFNodeOutput::Instances(instances))
    }
}

impl MetaTranslation {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_instance_op(
            arena,
            "Translation",
            seed,
            &outputs[self.child_id as usize],
            self.sampling,
            |rng| self.sample_params(rng),
            |params, input_instance| {
                let translation = Vector3::new(
                    params.translation_x,
                    params.translation_y,
                    params.translation_z,
                );

                match self.composition {
                    CompositionMode::Post => input_instance
                        .with_transform(input_instance.transform.translated(&translation)),
                    CompositionMode::Pre => input_instance.with_transform(
                        input_instance
                            .transform
                            .applied_to_translation(&translation),
                    ),
                }
            },
        )
    }
}

impl MetaRotation {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_instance_op(
            arena,
            "Rotation",
            seed,
            &outputs[self.child_id as usize],
            self.sampling,
            |rng| self.sample_params(rng),
            |params, input_instance| {
                let rotation = unit_quaternion_from_tilt_turn_roll(
                    Degrees(params.tilt_angle),
                    Degrees(params.turn_angle),
                    Degrees(params.roll_angle),
                );

                match self.composition {
                    CompositionMode::Post => {
                        input_instance.with_transform(input_instance.transform.rotated(&rotation))
                    }
                    CompositionMode::Pre => input_instance
                        .with_transform(input_instance.transform.applied_to_rotation(&rotation)),
                }
            },
        )
    }
}

impl MetaScaling {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_instance_op(
            arena,
            "Scaling",
            seed,
            &outputs[self.child_id as usize],
            self.sampling,
            |rng| self.sample_params(rng),
            |params, input_instance| {
                let scaling = params.scaling.max(f32::EPSILON);

                match self.composition {
                    CompositionMode::Post => {
                        input_instance.with_transform(input_instance.transform.scaled(scaling))
                    }
                    CompositionMode::Pre => input_instance
                        .with_transform(input_instance.transform.applied_to_scaling(scaling)),
                }
            },
        )
    }
}

impl MetaSimilarity {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_instance_op(
            arena,
            "Similarity",
            seed,
            &outputs[self.child_id as usize],
            self.sampling,
            |rng| self.sample_params(rng),
            |params, input_instance| {
                let scaling = params.scale.max(f32::EPSILON);

                let rotation = unit_quaternion_from_tilt_turn_roll(
                    Degrees(params.tilt_angle),
                    Degrees(params.turn_angle),
                    Degrees(params.roll_angle),
                );

                let translation = Vector3::new(
                    params.translation_x,
                    params.translation_y,
                    params.translation_z,
                );

                let transform = Similarity3::from_parts(translation, rotation, scaling);

                match self.composition {
                    CompositionMode::Post => {
                        input_instance.with_transform(transform * &input_instance.transform)
                    }
                    CompositionMode::Pre => {
                        input_instance.with_transform(&input_instance.transform * transform)
                    }
                }
            },
        )
    }
}

impl MetaStratifiedGridTransforms {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        let input_instances = match &outputs[self.child_id as usize] {
            MetaSDFNodeOutput::Instances(instances) => instances,
            child_output => {
                bail!(
                    "StratifiedGridTransforms node expects Instances as input, got {}",
                    child_output.label(),
                );
            }
        };

        if input_instances.is_empty() {
            return Ok(MetaSDFNodeOutput::Instances(AVec::new_in(arena)));
        }

        let instance_count = input_instances.len();

        let mut rng = create_param_rng(seed);

        let MetaStratifiedGridParams {
            shape_x,
            shape_y,
            shape_z,
            cell_extent_x,
            cell_extent_y,
            cell_extent_z,
            jitter_fraction,
        } = self.sample_params(&mut rng)?;

        let shape = [shape_x as usize, shape_y as usize, shape_z as usize];
        let cell_extents = [
            cell_extent_x.max(0.0),
            cell_extent_y.max(0.0),
            cell_extent_z.max(0.0),
        ];
        let jitter_fraction = jitter_fraction.clamp(0.0, 1.0);

        let grid_cell_count = shape[0] * shape[1] * shape[2];

        if grid_cell_count == 0 {
            return Ok(MetaSDFNodeOutput::Instances(input_instances.clone()));
        }

        // Center of the lower corner cell
        let start_pos: [_; 3] = array::from_fn(|i| {
            let cell_extent = cell_extents[i];
            let grid_extent = shape[i] as f32 * cell_extent;
            -0.5 * grid_extent + 0.5 * cell_extent
        });

        let mut output_instances = AVec::with_capacity_in(instance_count, arena);

        let uniform_distr = Uniform::new(-0.5, 0.5).unwrap();

        for (instance_idx, input_instance) in input_instances.iter().enumerate() {
            let grid_cell_idx = (instance_idx * grid_cell_count) / instance_count;

            let i = grid_cell_idx / (shape[1] * shape[2]);
            let j = (grid_cell_idx / shape[2]) % shape[1];
            let k = grid_cell_idx % shape[2];

            let x = start_pos[0] + i as f32 * cell_extents[0];
            let y = start_pos[1] + j as f32 * cell_extents[1];
            let z = start_pos[2] + k as f32 * cell_extents[2];

            let jx = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[0];
            let jy = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[1];
            let jz = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[2];

            let transform = Similarity3::from_parts(
                Vector3::new(x + jx, y + jy, z + jz),
                UnitQuaternion::identity(),
                1.0,
            );

            output_instances.push(input_instance.with_applied_transform(transform));
        }

        Ok(MetaSDFNodeOutput::Instances(output_instances))
    }
}

impl MetaSphereSurfaceTransforms {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        let input_instances = match &outputs[self.child_id as usize] {
            MetaSDFNodeOutput::Instances(instances) => instances,
            child_output => {
                bail!(
                    "SphereSurfaceTransforms node expects Instances as input, got {}",
                    child_output.label(),
                );
            }
        };

        if input_instances.is_empty() {
            return Ok(MetaSDFNodeOutput::Instances(AVec::new_in(arena)));
        }

        let count = input_instances.len();

        let mut rng = create_param_rng(seed);

        let MetaSphereSurfaceParams {
            radius,
            jitter_fraction,
        } = self.sample_params(&mut rng)?;

        let radius = radius.max(0.0);
        let jitter_fraction = jitter_fraction.clamp(0.0, 1.0);

        let max_jitter_angle = Self::compute_max_jitter_angle(count, jitter_fraction);

        let mut output_instances = AVec::with_capacity_in(count, arena);

        for (direction, input_instance) in
            compute_uniformly_distributed_radial_directions(count).zip(input_instances.as_slice())
        {
            let jittered_direction =
                compute_jittered_direction(direction, max_jitter_angle, &mut rng);

            let translation = radius * jittered_direction;

            let rotation = match self.rotation {
                SphereSurfaceRotation::Identity => UnitQuaternion::identity(),
                SphereSurfaceRotation::RadialOutwards => UnitQuaternion::rotation_between_axes(
                    &UnitVector3::unit_y(),
                    &jittered_direction,
                ),
                SphereSurfaceRotation::RadialInwards => UnitQuaternion::rotation_between_axes(
                    &(-UnitVector3::unit_y()),
                    &jittered_direction,
                ),
            };

            let transform = Similarity3::from_parts(translation, rotation, 1.0);

            output_instances.push(input_instance.with_applied_transform(transform));
        }

        Ok(MetaSDFNodeOutput::Instances(output_instances))
    }

    fn compute_max_jitter_angle(count: usize, jitter_fraction: f32) -> f32 {
        let solid_angle_per_transform = 4.0 * PI / (count as f32);

        let max_polar_angle_for_cap_covering_solid_angle = (1.0
            - solid_angle_per_transform / TWO_PI)
            .clamp(-1.0, 1.0)
            .acos();

        let max_jitter_angle = jitter_fraction * max_polar_angle_for_cap_covering_solid_angle;

        // Clamp to avoid very large angles for low counts
        max_jitter_angle.clamp(0.0, 0.5 * PI)
    }
}

impl MetaClosestTranslationToSurface {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        let subject_instances = match &outputs[self.subject_id as usize] {
            MetaSDFNodeOutput::Instances(instances) => instances,
            subject_output => {
                bail!(
                    "ClosestTranslationToSurface node expects Instances as input 2, got {}",
                    subject_output.label(),
                );
            }
        };

        let sdf_node_id = match &outputs[self.surface_sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => {
                return Ok(MetaSDFNodeOutput::Instances(subject_instances.clone()));
            }
            MetaSDFNodeOutput::SingleSDF(Some(sdf_node_id)) => *sdf_node_id,
            child_output => {
                bail!(
                    "ClosestTranslationToSurface node expects SingleSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        let generator = SDFGenerator::new_in(arena, graph.nodes(), sdf_node_id)?;
        let mut buffers = generator.create_buffers_for_block_in(arena);

        let surface_sdf_node_to_parent_transform =
            graph.nodes()[sdf_node_id as usize].node_to_parent_transform();

        let mut compute_translation_to_surface =
            |subject_node_to_parent_transform: &Similarity3| {
                compute_translation_to_closest_point_on_surface(
                    &generator,
                    &mut buffers,
                    &surface_sdf_node_to_parent_transform,
                    subject_node_to_parent_transform,
                    5,
                    0.1,
                )
            };

        let mut translated_subject_instances =
            AVec::with_capacity_in(subject_instances.len(), arena);

        for subject_instance in subject_instances {
            let subject_node_to_parent_transform = &subject_instance.transform;
            let Some(translation_to_surface_in_parent_space) =
                compute_translation_to_surface(subject_node_to_parent_transform)
            else {
                continue;
            };

            let translated_subject_transform = subject_node_to_parent_transform
                .translated(&translation_to_surface_in_parent_space);

            translated_subject_instances
                .push(subject_instance.with_transform(translated_subject_transform));
        }

        Ok(MetaSDFNodeOutput::Instances(translated_subject_instances))
    }
}

impl MetaRayTranslationToSurface {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        let subject_instances = match &outputs[self.subject_id as usize] {
            MetaSDFNodeOutput::Instances(instances) => instances,
            subject_output => {
                bail!(
                    "RayTranslationToSurface node expects Instances as input 2, got {}",
                    subject_output.label(),
                );
            }
        };

        let sdf_node_id = match &outputs[self.surface_sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => {
                return Ok(MetaSDFNodeOutput::Instances(subject_instances.clone()));
            }
            MetaSDFNodeOutput::SingleSDF(Some(sdf_node_id)) => *sdf_node_id,
            child_output => {
                bail!(
                    "RayTranslationToSurface node expects SingleSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        let generator = SDFGenerator::new_in(arena, graph.nodes(), sdf_node_id)?;
        let mut buffers_1x1x1 = generator.create_buffers_for_block_in::<1, _>(arena);
        let mut buffers_2x2x2 = generator.create_buffers_for_block_in::<8, _>(arena);

        let surface_sdf_node_to_parent_transform =
            graph.nodes()[sdf_node_id as usize].node_to_parent_transform();

        let mut compute_translation_to_surface =
            |subject_node_to_parent_transform: &Similarity3, sphere_in_subject_space: &Sphere| {
                compute_spherecast_translation_to_surface(
                    &generator,
                    &mut buffers_1x1x1,
                    &mut buffers_2x2x2,
                    &surface_sdf_node_to_parent_transform,
                    subject_node_to_parent_transform,
                    sphere_in_subject_space,
                    &UnitVector3::unit_y(),
                    128,
                    0.1,
                    0.5,
                )
            };

        // We ignore the center coordinates of the shape, which allows the
        // actual shape to be offset relative to the shape used for intersection
        let sphere_for_shape = |shape: &InstanceShape| match shape {
            InstanceShape::None => Sphere::new(Point3::origin(), 0.0),
            &InstanceShape::Sphere(SphereShape { radius, .. }) => {
                Sphere::new(Point3::origin(), radius)
            }
            &InstanceShape::Capsule(CapsuleShape {
                segment_length,
                radius,
                ..
            }) => Sphere::new(Point3::new(0.0, 0.5 * segment_length, 0.0), radius),
            &InstanceShape::Box(BoxShape {
                extent_x,
                extent_y,
                extent_z,
                ..
            }) => {
                // Use an inscribed sphere rather than the actual box
                let radius = 0.5 * extent_x.min(extent_y).min(extent_z);
                Sphere::new(Point3::new(0.0, 0.5 * extent_y - radius, 0.0), radius)
            }
        };

        let mut translated_subject_instances =
            AVec::with_capacity_in(subject_instances.len(), arena);

        for subject_instance in subject_instances {
            let subject_node_to_parent_transform = &subject_instance.transform;

            let sphere_in_subject_space = match self.anchor {
                RayTranslationAnchor::Origin => Sphere::new(Point3::origin(), 0.0),
                RayTranslationAnchor::ShapeBoundaryAtOrigin => {
                    sphere_for_shape(&subject_instance.shape)
                }
            };

            let Some(translation_to_surface_in_parent_space) = compute_translation_to_surface(
                subject_node_to_parent_transform,
                &sphere_in_subject_space,
            ) else {
                continue;
            };

            let translated_subject_transform = subject_node_to_parent_transform
                .translated(&translation_to_surface_in_parent_space);

            translated_subject_instances
                .push(subject_instance.with_transform(translated_subject_transform));
        }

        Ok(MetaSDFNodeOutput::Instances(translated_subject_instances))
    }
}

impl MetaRotationToGradient {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        let subject_instances = match &outputs[self.subject_id as usize] {
            MetaSDFNodeOutput::Instances(instances) => instances,
            subject_output => {
                bail!(
                    "RotationToGradient node expects Instances as input 2, got {}",
                    subject_output.label(),
                );
            }
        };

        let sdf_node_id = match &outputs[self.gradient_sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => {
                return Ok(MetaSDFNodeOutput::Instances(subject_instances.clone()));
            }
            MetaSDFNodeOutput::SingleSDF(Some(sdf_node_id)) => *sdf_node_id,
            child_output => {
                bail!(
                    "RotationToGradient node expects SingleSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        let generator = SDFGenerator::new_in(arena, graph.nodes(), sdf_node_id)?;
        let mut buffers = generator.create_buffers_for_block_in(arena);

        let gradient_sdf_node_to_parent_transform =
            graph.nodes()[sdf_node_id as usize].node_to_parent_transform();

        let mut compute_rotation_to_gradient = |subject_node_to_parent_transform: &Similarity3| {
            compute_rotation_to_gradient(
                &generator,
                &mut buffers,
                &gradient_sdf_node_to_parent_transform,
                subject_node_to_parent_transform,
            )
        };

        let mut rotated_subject_instances = AVec::with_capacity_in(subject_instances.len(), arena);

        for subject_instance in subject_instances {
            let subject_node_to_parent_transform = &subject_instance.transform;

            let Some(rotation_to_gradient_in_parent_space) =
                compute_rotation_to_gradient(subject_node_to_parent_transform)
            else {
                continue;
            };

            let rotated_subject_transform =
                subject_node_to_parent_transform.rotated(&rotation_to_gradient_in_parent_space);

            rotated_subject_instances
                .push(subject_instance.with_transform(rotated_subject_transform));
        }

        Ok(MetaSDFNodeOutput::Instances(rotated_subject_instances))
    }
}

impl MetaStochasticSelection {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> MetaSDFNodeOutput<A> {
        let mut rng = create_param_rng(seed);

        let pick_count = self.min_pick_count..=self.max_pick_count.max(self.min_pick_count);
        let pick_probability = self.pick_probability.clamp(0.0, 1.0);

        let mut single_is_selected =
            || *pick_count.start() > 0 && rng.random_range(0.0..1.0) < pick_probability;

        match &outputs[self.child_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => MetaSDFNodeOutput::SingleSDF(None),
            MetaSDFNodeOutput::SingleSDF(Some(input_node_id)) => {
                let output_node_id = single_is_selected().then_some(*input_node_id);
                MetaSDFNodeOutput::SingleSDF(output_node_id)
            }
            MetaSDFNodeOutput::SDFGroup(input_node_ids) => {
                let mut output_node_ids = AVec::with_capacity_in(input_node_ids.len(), arena);
                let count = rng.random_range(pick_count.clone());
                for &input_node_id in input_node_ids.choose_multiple(&mut rng, count as usize) {
                    if rng.random_range(0.0..1.0) < pick_probability {
                        output_node_ids.push(input_node_id);
                    }
                }
                MetaSDFNodeOutput::SDFGroup(output_node_ids)
            }
            MetaSDFNodeOutput::Instances(input_instances) => {
                let mut output_instances = AVec::with_capacity_in(input_instances.len(), arena);
                let count = rng.random_range(pick_count.clone());
                for input_instance in input_instances.choose_multiple(&mut rng, count as usize) {
                    if rng.random_range(0.0..1.0) < pick_probability {
                        output_instances.push(input_instance.clone());
                    }
                }
                MetaSDFNodeOutput::Instances(output_instances)
            }
        }
    }
}

impl MetaSDFInstantiation {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        let instances = match &outputs[self.child_id as usize] {
            MetaSDFNodeOutput::Instances(instances) => instances,
            child_output => {
                bail!(
                    "SDFInstantiation node expects Instances as input, got {}",
                    child_output.label(),
                );
            }
        };

        // There could be up to four SDF transform nodes (the primitive,
        // translation, rotation and scaling) per instance
        let mut output_node_ids = AVec::with_capacity_in(4 * instances.len(), arena);

        for instance in instances {
            let (mut output_node_id, center) = match instance.shape {
                InstanceShape::None => {
                    continue;
                }
                InstanceShape::Sphere(SphereShape {
                    radius,
                    center_x,
                    center_y,
                    center_z,
                }) => (
                    graph.add_node(SDFNode::new_sphere(radius)),
                    Point3::new(center_x, center_y, center_z),
                ),
                InstanceShape::Capsule(CapsuleShape {
                    segment_length,
                    radius,
                    center_x,
                    center_y,
                    center_z,
                }) => (
                    graph.add_node(SDFNode::new_capsule(segment_length, radius)),
                    Point3::new(center_x, center_y, center_z),
                ),
                InstanceShape::Box(BoxShape {
                    extent_x,
                    extent_y,
                    extent_z,
                    center_x,
                    center_y,
                    center_z,
                }) => (
                    graph.add_node(SDFNode::new_box([extent_x, extent_y, extent_z])),
                    Point3::new(center_x, center_y, center_z),
                ),
            };

            let transform = &instance.transform;

            let scaling = transform.scaling();
            let rotation = transform.rotation();
            let translation = transform.translation();

            if abs_diff_ne!(&center, &Point3::origin()) {
                output_node_id = graph.add_node(SDFNode::new_translation(
                    output_node_id,
                    *center.as_vector(),
                ));
            }
            if abs_diff_ne!(scaling, 1.0) {
                output_node_id = graph.add_node(SDFNode::new_scaling(output_node_id, scaling));
            }
            if abs_diff_ne!(rotation, &UnitQuaternion::identity()) {
                output_node_id = graph.add_node(SDFNode::new_rotation(output_node_id, *rotation));
            }
            if abs_diff_ne!(translation, &Vector3::zeros()) {
                output_node_id =
                    graph.add_node(SDFNode::new_translation(output_node_id, *translation));
            }

            output_node_ids.push(output_node_id);
        }

        Ok(MetaSDFNodeOutput::SDFGroup(output_node_ids))
    }
}

impl MetaTransformApplication {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        let sdf_node_ids = match &outputs[self.sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(sdf_node_id) => {
                let mut sdf_node_ids = AVec::new_in(arena);
                if let Some(sdf_node_id) = sdf_node_id {
                    sdf_node_ids.push(*sdf_node_id);
                }
                Cow::Owned(sdf_node_ids)
            }
            MetaSDFNodeOutput::SDFGroup(sdf_node_ids) => Cow::Borrowed(sdf_node_ids),
            MetaSDFNodeOutput::Instances(_) => {
                bail!(
                    "TransformApplication node expects SingleSDF or GroupSDF as input 1, got Instances"
                );
            }
        };

        let instances = match &outputs[self.instance_id as usize] {
            MetaSDFNodeOutput::Instances(instances) => instances,
            child_output => {
                bail!(
                    "TransformApplication node expects Instances as input 2, got {}",
                    child_output.label(),
                );
            }
        };

        // There could be up to three SDF transform nodes (translation, rotation
        // and scaling) per (SDF, instance) pair
        let mut output_node_ids =
            AVec::with_capacity_in(3 * sdf_node_ids.len() * instances.len(), arena);

        for &sdf_node_id in sdf_node_ids.as_ref() {
            for instance in instances {
                let transform = &instance.transform;

                let scaling = transform.scaling();
                let rotation = transform.rotation();
                let translation = transform.translation();

                let mut output_node_id = sdf_node_id;

                if abs_diff_ne!(scaling, 1.0) {
                    output_node_id = graph.add_node(SDFNode::new_scaling(output_node_id, scaling));
                }
                if abs_diff_ne!(rotation, &UnitQuaternion::identity()) {
                    output_node_id =
                        graph.add_node(SDFNode::new_rotation(output_node_id, *rotation));
                }
                if abs_diff_ne!(translation, &Vector3::zeros()) {
                    output_node_id =
                        graph.add_node(SDFNode::new_translation(output_node_id, *translation));
                }

                output_node_ids.push(output_node_id);
            }
        }

        Ok(MetaSDFNodeOutput::SDFGroup(output_node_ids))
    }
}

impl MetaMultifractalNoiseSDFModifier {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<AR>> {
        resolve_unary_sdf_op(
            arena,
            graph,
            "MultifractalNoiseSDFModifier",
            seed,
            &outputs[self.child_id as usize],
            self.sampling,
            |rng| Ok((self.sample_params(rng)?, rng.random::<u32>())),
            |(params, seed), input_node_id| {
                SDFNode::new_multifractal_noise(
                    input_node_id,
                    params.octaves,
                    params.frequency,
                    params.lacunarity,
                    params.persistence,
                    params.amplitude,
                    *seed,
                )
            },
        )
    }
}

impl MetaMultiscaleSphereSDFModifier {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<AR>> {
        resolve_unary_sdf_op(
            arena,
            graph,
            "MultiscaleSphereSDFModifier",
            seed,
            &outputs[self.child_id as usize],
            self.sampling,
            |rng| Ok((self.sample_params(rng)?, rng.random::<u32>())),
            |(params, seed), input_node_id| {
                SDFNode::new_multiscale_sphere(
                    input_node_id,
                    params.octaves,
                    params.max_scale,
                    params.persistence,
                    params.inflation,
                    params.intersection_smoothness,
                    params.union_smoothness,
                    *seed,
                )
            },
        )
    }
}

impl MetaSDFUnion {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        let (input_node_1_id, input_node_2_id) = match (
            &outputs[self.child_1_id as usize],
            &outputs[self.child_2_id as usize],
        ) {
            (
                MetaSDFNodeOutput::SingleSDF(input_node_1_id),
                MetaSDFNodeOutput::SingleSDF(input_node_2_id),
            ) => (*input_node_1_id, *input_node_2_id),
            (child_1_output, child_2_output) => {
                bail!(
                    "SDFUnion node expects two SingleSDF inputs, got {} and {}",
                    child_1_output.label(),
                    child_2_output.label()
                );
            }
        };

        match (input_node_1_id, input_node_2_id) {
            (input_node_id, None) | (None, input_node_id) => {
                Ok(MetaSDFNodeOutput::SingleSDF(input_node_id))
            }
            (Some(input_node_1_id), Some(input_node_2_id)) => {
                let output_node_id = graph.add_node(SDFNode::new_union(
                    input_node_1_id,
                    input_node_2_id,
                    self.smoothness.max(0.0),
                ));
                Ok(MetaSDFNodeOutput::SingleSDF(Some(output_node_id)))
            }
        }
    }
}

impl MetaSDFSubtraction {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        let (input_node_1_id, input_node_2_id) = match (
            &outputs[self.child_1_id as usize],
            &outputs[self.child_2_id as usize],
        ) {
            (
                MetaSDFNodeOutput::SingleSDF(input_node_1_id),
                MetaSDFNodeOutput::SingleSDF(input_node_2_id),
            ) => (*input_node_1_id, *input_node_2_id),
            (child_1_output, child_2_output) => {
                bail!(
                    "SDFSubtraction node expects two SingleSDF inputs, got {} and {}",
                    child_1_output.label(),
                    child_2_output.label()
                );
            }
        };

        match (input_node_1_id, input_node_2_id) {
            (None, _) => Ok(MetaSDFNodeOutput::SingleSDF(None)),
            (Some(input_node_id), None) => Ok(MetaSDFNodeOutput::SingleSDF(Some(input_node_id))),
            (Some(input_node_1_id), Some(input_node_2_id)) => {
                let output_node_id = graph.add_node(SDFNode::new_subtraction(
                    input_node_1_id,
                    input_node_2_id,
                    self.smoothness.max(0.0),
                ));
                Ok(MetaSDFNodeOutput::SingleSDF(Some(output_node_id)))
            }
        }
    }
}

impl MetaSDFIntersection {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        let (input_node_1_id, input_node_2_id) = match (
            &outputs[self.child_1_id as usize],
            &outputs[self.child_2_id as usize],
        ) {
            (
                MetaSDFNodeOutput::SingleSDF(input_node_1_id),
                MetaSDFNodeOutput::SingleSDF(input_node_2_id),
            ) => (*input_node_1_id, *input_node_2_id),
            (child_1_output, child_2_output) => {
                bail!(
                    "SDFIntersection node expects two SingleSDF inputs, got {} and {}",
                    child_1_output.label(),
                    child_2_output.label()
                );
            }
        };

        match (input_node_1_id, input_node_2_id) {
            (None, _) | (_, None) => Ok(MetaSDFNodeOutput::SingleSDF(None)),
            (Some(input_node_1_id), Some(input_node_2_id)) => {
                let output_node_id = graph.add_node(SDFNode::new_intersection(
                    input_node_1_id,
                    input_node_2_id,
                    self.smoothness.max(0.0),
                ));
                Ok(MetaSDFNodeOutput::SingleSDF(Some(output_node_id)))
            }
        }
    }
}

impl MetaSDFGroupUnion {
    fn resolve<AR: Allocator, AG: Allocator>(
        &self,
        arena: AR,
        graph: &mut SDFGraph<AG>,
        outputs: &[MetaSDFNodeOutput<AR>],
    ) -> Result<MetaSDFNodeOutput<AR>> {
        match &outputs[self.child_id as usize] {
            MetaSDFNodeOutput::SingleSDF(input_node_id) => {
                Ok(MetaSDFNodeOutput::SingleSDF(*input_node_id))
            }
            MetaSDFNodeOutput::SDFGroup(input_node_ids) => {
                let output_node_id = emit_balanced_binary_tree(
                    arena,
                    input_node_ids,
                    |child_node_1, child_node_2| {
                        graph.add_node(SDFNode::new_union(
                            child_node_1,
                            child_node_2,
                            self.smoothness.max(0.0),
                        ))
                    },
                );
                Ok(MetaSDFNodeOutput::SingleSDF(output_node_id))
            }
            MetaSDFNodeOutput::Instances(_) => {
                bail!("SDFGroupUnion node expects SDFGroup or SingleSDF input, got Instances");
            }
        }
    }
}

impl CompositionMode {
    pub fn try_from_str(variant: &str) -> Result<Self> {
        match variant {
            "Post" => Ok(Self::Post),
            "Pre" => Ok(Self::Pre),
            invalid => Err(anyhow!("Invalid CompositionMode variant: {invalid}")),
        }
    }
}

impl ParameterSamplingMode {
    pub fn try_from_str(variant: &str) -> Result<Self> {
        match variant {
            "Only once" => Ok(Self::OnlyOnce),
            "Per instance" | "Per SDF" => Ok(Self::PerInstance),
            invalid => Err(anyhow!("Invalid ParameterSamplingMode variant: {invalid}")),
        }
    }
}

impl RayTranslationAnchor {
    pub fn try_from_str(variant: &str) -> Result<Self> {
        match variant {
            "Origin" => Ok(Self::Origin),
            "Shape boundary at origin" => Ok(Self::ShapeBoundaryAtOrigin),
            invalid => Err(anyhow!("Invalid CompositionMode variant: {invalid}")),
        }
    }
}

impl SphereSurfaceRotation {
    pub fn try_from_str(variant: &str) -> Result<Self> {
        match variant {
            "Identity" => Ok(Self::Identity),
            "Radial (outwards)" => Ok(Self::RadialOutwards),
            "Radial (inwards)" => Ok(Self::RadialInwards),
            invalid => Err(anyhow!("Invalid SphereSurfaceRotation variant: {invalid}")),
        }
    }
}

fn resolve_unary_instance_op<A: Allocator, P>(
    arena: A,
    name: &str,
    seed: u64,
    child_output: &MetaSDFNodeOutput<A>,
    sampling: ParameterSamplingMode,
    mut sample_params: impl FnMut(&mut ParamRng) -> Result<P>,
    create_instance: impl Fn(&P, &Instance) -> Instance,
) -> Result<MetaSDFNodeOutput<A>> {
    let input_instances = match child_output {
        MetaSDFNodeOutput::Instances(input_instances) => input_instances,
        child_output => {
            bail!(
                "{name} node expects Instances input, got {}",
                child_output.label()
            );
        }
    };

    let mut rng = create_param_rng(seed);

    let instance_count = input_instances.len();

    let mut output_instances = AVec::with_capacity_in(instance_count, arena);

    let mut params = sample_params(&mut rng)?;

    for (idx, input_instance) in input_instances.iter().enumerate() {
        output_instances.push(create_instance(&params, input_instance));

        if sampling == ParameterSamplingMode::PerInstance && idx + 1 < instance_count {
            params = sample_params(&mut rng)?;
        }
    }

    Ok(MetaSDFNodeOutput::Instances(output_instances))
}

fn resolve_unary_sdf_op<AR: Allocator, AG: Allocator, P>(
    arena: AR,
    graph: &mut SDFGraph<AG>,
    name: &str,
    seed: u64,
    child_output: &MetaSDFNodeOutput<AR>,
    sampling: ParameterSamplingMode,
    mut sample_params: impl FnMut(&mut ParamRng) -> Result<P>,
    create_atomic_node: impl Fn(&P, SDFNodeID) -> SDFNode,
) -> Result<MetaSDFNodeOutput<AR>> {
    match child_output {
        MetaSDFNodeOutput::SingleSDF(None) => Ok(MetaSDFNodeOutput::SingleSDF(None)),
        MetaSDFNodeOutput::SingleSDF(Some(input_node_id)) => {
            let mut rng = create_param_rng(seed);
            let params = sample_params(&mut rng)?;

            let output_node_id = graph.add_node(create_atomic_node(&params, *input_node_id));

            Ok(MetaSDFNodeOutput::SingleSDF(Some(output_node_id)))
        }
        MetaSDFNodeOutput::SDFGroup(input_node_ids) => {
            let mut rng = create_param_rng(seed);

            let count = input_node_ids.len();

            let mut output_node_ids = AVec::with_capacity_in(input_node_ids.len(), arena);

            let mut params = sample_params(&mut rng)?;

            for (idx, input_node_id) in input_node_ids.iter().enumerate() {
                let output_node_id = graph.add_node(create_atomic_node(&params, *input_node_id));
                output_node_ids.push(output_node_id);

                if sampling == ParameterSamplingMode::PerInstance && idx + 1 < count {
                    params = sample_params(&mut rng)?;
                }
            }

            Ok(MetaSDFNodeOutput::SDFGroup(output_node_ids))
        }
        child_output => {
            bail!(
                "{name} node expects SingleSDF or SDFGroup input, got {}",
                child_output.label()
            );
        }
    }
}

fn emit_balanced_binary_tree<A, N>(
    arena: A,
    leaf_nodes: &[N],
    mut create_parent_node: impl FnMut(N, N) -> N,
) -> Option<N>
where
    A: Allocator,
    N: Copy,
{
    let mut queue = FixedQueue::new_full_in(arena, leaf_nodes);

    while queue.len() > 1 {
        let child_node_1 = queue.pop_front().unwrap();
        let child_node_2 = queue.pop_front().unwrap();
        let parent_node = create_parent_node(child_node_1, child_node_2);
        queue.push_back(parent_node);
    }

    queue.pop_front()
}

fn compute_translation_to_closest_point_on_surface<A: Allocator>(
    generator: &SDFGenerator<A>,
    buffers: &mut SDFGeneratorBlockBuffers<8, A>,
    surface_sdf_node_to_parent_transform: &Similarity3,
    subject_node_to_parent_transform: &Similarity3,
    max_iterations: u32,
    max_distance_from_surface: f32,
) -> Option<Vector3> {
    // The basis for this computation is that the surface node (for which we
    // sample the SDF) and the subject node (which we will translate) have the
    // *same* parent space. In other words, we assume that no additional
    // transforms will be applied to either of the nodes before they are
    // combined with a binary operator.

    // We need to determine the position of the subject node's domain center in
    // the space of the surface node, since this is where we will begin to
    // sample the SDF. The center of the subject node's domain in its own space
    // is the origin, and we start by transforming that to the (common) parent
    // space.
    let subject_center_in_parent_space =
        subject_node_to_parent_transform.transform_point(&Point3::origin());

    // We can now transform it from the common parent space to the space of the
    // surface node
    let subject_center_in_surface_sdf_space = surface_sdf_node_to_parent_transform
        .inverse_transform_point(&subject_center_in_parent_space);

    // To find the closest point on the surface (where the signed distance is
    // zero), we use the Newton-Raphson method

    let mut sampling_position = subject_center_in_surface_sdf_space;
    let mut iteration_count = 0;

    while iteration_count < max_iterations {
        // We will sample a block of 2x2x2 signed distance values centered at
        // the sampling position. The samples are one voxel apart, so the lower
        // samples will be offset by 0.5 voxels down from the sampling position.
        let (signed_distance, gradient) =
            sample_signed_distance_with_gradient(generator, buffers, &sampling_position);

        let gradient_norm_squared = gradient.norm_squared();

        // If the gradient for some reason is zero, we can't determine the
        // direction towards the surface, so we abort
        if abs_diff_eq!(gradient_norm_squared, 0.0, epsilon = 1e-8) {
            return None;
        }

        // Newton-Raphson step
        sampling_position += (-signed_distance / gradient_norm_squared) * gradient;

        if signed_distance.abs() <= max_distance_from_surface {
            break;
        }

        iteration_count += 1;
    }

    let translation_to_surface_in_surface_sdf_space =
        sampling_position - subject_center_in_surface_sdf_space;

    // We are still in the space of the surface node, but when applying the
    // translation to the subject node we need the translation to be in the
    // parent space
    let translation_to_surface_in_parent_space = surface_sdf_node_to_parent_transform
        .transform_vector(&translation_to_surface_in_surface_sdf_space);

    Some(translation_to_surface_in_parent_space)
}

fn compute_rotation_to_gradient<A: Allocator>(
    generator: &SDFGenerator<A>,
    buffers: &mut SDFGeneratorBlockBuffers<8, A>,
    gradient_sdf_node_to_parent_transform: &Similarity3,
    subject_node_to_parent_transform: &Similarity3,
) -> Option<UnitQuaternion> {
    // The basis for this computation is that the gradient node (for which we
    // sample the SDF) and the subject node (which we will rotate) have the
    // *same* parent space. In other words, we assume that no additional
    // transforms will be applied to either of the nodes before they are
    // combined with a binary operator.

    // We need to determine the position of the subject node's domain center in
    // the space of the gradient node, since this is where we will sample the
    // SDF. The center of the subject node's domain in its own space is
    // the origin, and we start by transforming that to the (common) parent
    // space.
    let subject_center_in_parent_space =
        subject_node_to_parent_transform.transform_point(&Point3::origin());

    // We can now transform it from the common parent space to the space of the
    // gradient node and sample the gradient
    let subject_center_in_gradient_sdf_space = gradient_sdf_node_to_parent_transform
        .inverse_transform_point(&subject_center_in_parent_space);

    let (_, gradient) = sample_signed_distance_with_gradient(
        generator,
        buffers,
        &subject_center_in_gradient_sdf_space,
    );

    // The rotation will be from the subject's y-axis to the gradient. When
    // applying the rotation to the subject node we need the rotation to be in
    // the parent space, so we transform both the y-axis and gradient to that
    // space.

    let subject_y_axis_in_parent_space =
        subject_node_to_parent_transform.transform_vector(&Vector3::unit_y());

    let gradient_in_parent_space =
        gradient_sdf_node_to_parent_transform.transform_vector(&gradient);

    // If the source (y-axis) or destination (gradient) vector has length zero,
    // we can't determine the rotation, so we abort
    let y_axis = UnitVector3::normalized_from_if_above(subject_y_axis_in_parent_space, 1e-8)?;
    let gradient_direction = UnitVector3::normalized_from_if_above(gradient_in_parent_space, 1e-8)?;

    Some(UnitQuaternion::rotation_between_axes(
        &y_axis,
        &gradient_direction,
    ))
}

fn compute_spherecast_translation_to_surface<A: Allocator>(
    generator: &SDFGenerator<A>,
    buffers_1x1x1: &mut SDFGeneratorBlockBuffers<1, A>,
    buffers_2x2x2: &mut SDFGeneratorBlockBuffers<8, A>,
    surface_sdf_node_to_parent_transform: &Similarity3,
    subject_node_to_parent_transform: &Similarity3,
    sphere_in_subject_space: &Sphere,
    direction_in_subject_space: &UnitVector3,
    max_steps: u32,
    tolerance: f32,
    safety_factor: f32,
) -> Option<Vector3> {
    // The basis for this computation is that the surface node (for which we
    // sample the SDF) and the subject node (where the sphere is defined) have
    // the *same* parent space. In other words, we assume that no additional
    // transforms will be applied to either of the nodes before they are
    // combined with a binary operator.

    // We need to determine the position of the sphere's center in the space of
    // the surface node, since this is where we will begin to sample the SDF. We
    // start by transforming the center from the subject node space to the
    // (common) parent space.
    let sphere_center_in_parent_space =
        subject_node_to_parent_transform.transform_point(sphere_in_subject_space.center());

    // Same for radius and direction
    let sphere_radius_in_parent_space =
        subject_node_to_parent_transform.scaling() * sphere_in_subject_space.radius();

    let direction_in_parent_space =
        subject_node_to_parent_transform.transform_vector(direction_in_subject_space);

    // We can now transform the sphere and direction from the common parent
    // space to the space of the surface node
    let sphere_center_in_surface_sdf_space = surface_sdf_node_to_parent_transform
        .inverse_transform_point(&sphere_center_in_parent_space);

    let sphere_radius_in_surface_space =
        surface_sdf_node_to_parent_transform.scaling().recip() * sphere_radius_in_parent_space;

    let sphere_in_surface_space = Sphere::new(
        sphere_center_in_surface_sdf_space,
        sphere_radius_in_surface_space,
    );

    let direction_in_surface_sdf_space = UnitVector3::normalized_from_if_above(
        surface_sdf_node_to_parent_transform.inverse_transform_vector(&direction_in_parent_space),
        1e-8,
    )?;

    let translation_to_surface_in_surface_sdf_space =
        compute_spherecast_translation_to_surface_same_space(
            generator,
            buffers_1x1x1,
            buffers_2x2x2,
            &sphere_in_surface_space,
            &direction_in_surface_sdf_space,
            max_steps,
            tolerance,
            safety_factor,
        )?;

    // We are still in the space of the surface node, but when applying the
    // translation to the subject node we need the translation to be in the
    // parent space
    let translation_to_surface_in_parent_space = surface_sdf_node_to_parent_transform
        .transform_vector(&translation_to_surface_in_surface_sdf_space);

    Some(translation_to_surface_in_parent_space)
}

fn compute_spherecast_translation_to_surface_same_space<A: Allocator>(
    generator: &SDFGenerator<A>,
    buffers_1x1x1: &mut SDFGeneratorBlockBuffers<1, A>,
    buffers_2x2x2: &mut SDFGeneratorBlockBuffers<8, A>,
    sphere: &Sphere,
    direction: &UnitVector3,
    max_steps: u32,
    tolerance: f32,
    safety_factor: f32,
) -> Option<Vector3> {
    assert!(safety_factor > 0.0 && safety_factor <= 1.0);

    let ray_origin = sphere.center();
    let ray_direction = direction;

    let domain = generator.domain();

    let (ray_domain_intersection_start, ray_domain_intersection_end) =
        domain.find_ray_intersection(ray_origin, ray_direction)?;

    // We want the front of the sphere, not the center, to start at the domain
    // boundary
    let start_distance_along_ray = ray_domain_intersection_start - sphere.radius();
    let max_distance_along_ray = ray_domain_intersection_end;

    let mut center_distance_along_ray = start_distance_along_ray;
    let mut sampling_position = ray_origin + center_distance_along_ray * ray_direction;

    // To determine where the sphere hits the surface, we need to find the point
    // along the ray where the signed distance of the point on the sphere
    // closest to the surface is zero
    let mut smallest_signed_distance = compute_smallest_signed_distance_on_sphere(
        generator,
        buffers_1x1x1,
        buffers_2x2x2,
        sphere.radius(),
        &sampling_position,
    )?;

    if smallest_signed_distance < 0.0 {
        // The sphere is already penetrating the surface. We treat that as a
        // miss.
        return None;
    }

    let mut smallest_distance = smallest_signed_distance.abs();

    let mut step_count = 0;
    let mut crossed_surface = false;

    while smallest_distance > tolerance {
        step_count += 1;

        // If we reach max iterations, we assume that we are fairly close to the
        // surface if we have crossed it (we are stepping back and forth across
        // it without getting close enough), otherwise we assume we missed and
        // give up
        if step_count >= max_steps {
            if crossed_surface {
                break;
            } else {
                return None;
            }
        }

        // If the SDF was exact, the surface couldn't possibly be closer to the
        // sphere boundary than `smallest_distance`, so we could safely step
        // that far without overshooting. However, since the distances may be
        // inaccurate due to things like smoothing or perturbing with noise, we
        // shorten the stepping distance by a safety factor.
        center_distance_along_ray += smallest_signed_distance * safety_factor;

        if !crossed_surface && smallest_signed_distance.is_sign_negative() {
            crossed_surface = true;
        }

        // We have exited the SDF domain, so the ray didn't hit the surface
        if center_distance_along_ray > max_distance_along_ray
            || center_distance_along_ray < start_distance_along_ray
        {
            return None;
        }

        sampling_position = ray_origin + center_distance_along_ray * ray_direction;

        smallest_signed_distance = compute_smallest_signed_distance_on_sphere(
            generator,
            buffers_1x1x1,
            buffers_2x2x2,
            sphere.radius(),
            &sampling_position,
        )?;
        smallest_distance = smallest_signed_distance.abs();
    }

    let translation_to_surface = sampling_position - ray_origin;

    Some(translation_to_surface)
}

fn compute_smallest_signed_distance_on_sphere<A: Allocator>(
    generator: &SDFGenerator<A>,
    buffers_1x1x1: &mut SDFGeneratorBlockBuffers<1, A>,
    buffers_2x2x2: &mut SDFGeneratorBlockBuffers<8, A>,
    radius: f32,
    position: &Point3,
) -> Option<f32> {
    let closest_point_on_sphere = if abs_diff_ne!(radius, 0.0) {
        let (_, gradient) =
            sample_signed_distance_with_gradient(generator, buffers_2x2x2, position);
        let gradient_direction = UnitVector3::normalized_from_if_above(gradient, 1e-8)?;

        position - radius * gradient_direction
    } else {
        *position
    };

    let signed_distance =
        generator.compute_signed_distance(buffers_1x1x1, &closest_point_on_sphere);

    Some(signed_distance)
}

fn sample_signed_distance_with_gradient<A: Allocator>(
    generator: &SDFGenerator<A>,
    buffers: &mut SDFGeneratorBlockBuffers<8, A>,
    sampling_position: &Point3,
) -> (f32, Vector3) {
    const SAMPLE_BLOCK_SIZE: usize = 2;

    let block_origin = sampling_position - Vector3::same(0.5 * (SAMPLE_BLOCK_SIZE - 1) as f32);

    generator.compute_signed_distances_for_block_preserving_gradients::<SAMPLE_BLOCK_SIZE, _>(
        buffers,
        &block_origin,
    );

    let sampled_signed_distances = buffers.final_signed_distances();

    let signed_distance = compute_center_value_of_2x2x2_samples(sampled_signed_distances);
    let gradient = compute_gradient_from_2x2x2_samples(sampled_signed_distances);

    (signed_distance, gradient)
}

/// Takes 2x2x2 signed distances (column-major order) sampled one voxel width
/// apart and estimates the value at the center of the sampled block by taking
/// their average.
#[inline]
fn compute_center_value_of_2x2x2_samples(signed_distances: &[f32; 8]) -> f32 {
    signed_distances.iter().sum::<f32>() * 0.125
}

/// Takes 2x2x2 signed distances (column-major order) sampled one voxel width
/// apart and estimates the gradient at the center of the sampled block by
/// calculating the analytic gradient of the trilinear interpolation of the
/// samples at the center.
#[inline]
fn compute_gradient_from_2x2x2_samples(signed_distances: &[f32; 8]) -> Vector3 {
    let &[d000, d001, d010, d011, d100, d101, d110, d111] = signed_distances;
    0.25 * Vector3::new(
        (d100 + d110 + d101 + d111) - (d000 + d010 + d001 + d011),
        (d010 + d110 + d011 + d111) - (d000 + d100 + d001 + d101),
        (d001 + d101 + d011 + d111) - (d000 + d100 + d010 + d110),
    )
}

fn compute_jittered_direction(
    direction: UnitVector3,
    max_jitter_angle: f32,
    rng: &mut ParamRng,
) -> UnitVector3 {
    assert!(max_jitter_angle >= 0.0);
    if abs_diff_eq!(max_jitter_angle, 0.0) {
        return direction;
    }

    let angle = rng.random_range(0.0..=max_jitter_angle);

    let mut axis = Vector3::new(
        rng.random_range(-1.0..=1.0),
        rng.random_range(-1.0..=1.0),
        rng.random_range(-1.0..=1.0),
    );

    // Retain only the component perpendicular to `direction`
    axis -= axis.dot(&direction) * direction;

    let axis = UnitVector3::normalized_from_if_above(axis, 1e-8).unwrap_or_else(|| {
        // `axis` was either zero or parallel to `direction`, so we pick an
        // arbitrary non-parallel axis
        let axis = if direction.z().abs() < 0.9 {
            UnitVector3::unit_z()
        } else {
            UnitVector3::unit_x()
        };
        let axis = axis.as_vector() - axis.dot(&direction) * direction;
        UnitVector3::normalized_from(axis)
    });

    let rotation = UnitQuaternion::from_axis_angle(&axis, angle);

    rotation.rotate_unit_vector(&direction)
}

fn unit_quaternion_from_tilt_turn_roll(
    tilt_angle: Degrees<f32>,
    turn_angle: Degrees<f32>,
    roll_angle: Degrees<f32>,
) -> UnitQuaternion {
    let polar_angle = tilt_angle.radians();
    let azimuthal_angle = turn_angle.radians();
    let roll_angle = roll_angle.radians();

    let (sin_polar_angle, cos_polar_angle) = polar_angle.sin_cos();
    let (sin_azimuthal_angle, cos_azimuthal_angle) = azimuthal_angle.sin_cos();

    let direction = UnitVector3::unchecked_from(Vector3::new(
        sin_polar_angle * cos_azimuthal_angle,
        cos_polar_angle,
        sin_polar_angle * sin_azimuthal_angle,
    ));

    let rotation_without_roll =
        UnitQuaternion::rotation_between_axes(&UnitVector3::unit_y(), &direction);

    let roll_rotation = UnitQuaternion::from_axis_angle(&direction, roll_angle);

    roll_rotation * rotation_without_roll
}
