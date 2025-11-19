//! Generation of signed distance fields. This module implements the graph of
//! high-level "meta" SDF nodes that is compiled into the runtime graph of
//! simpler atomic nodes.

pub mod params;

use crate::{
    define_meta_node_params,
    generation::sdf::{
        SDFGenerator, SDFGeneratorBlockBuffers, SDFGraph, SDFNode, SDFNodeID,
        meta::params::ParamScratch,
    },
};
use allocator_api2::{
    alloc::{Allocator, Global},
    vec::Vec as AVec,
};
use anyhow::{Context, Result, anyhow, bail};
use approx::{abs_diff_eq, abs_diff_ne};
use impact_containers::FixedQueue;
use impact_geometry::{compute_uniformly_distributed_radial_directions, rotation_between_axes};
use impact_math::splitmix;
use nalgebra::{
    Point3, Similarity, Similarity3, Translation3, UnitQuaternion, UnitVector3, Vector3, vector,
};
use params::{ContParamSpec, DiscreteParamSpec, ParamRng, create_param_rng};
use rand::{
    Rng,
    distr::{Distribution, Uniform},
    seq::IndexedRandom,
};
use std::{array, borrow::Cow, f32::consts::PI};

#[derive(Clone, Debug)]
pub struct MetaSDFGraph<A: Allocator = Global> {
    seed: u64,
    nodes: AVec<MetaSDFNode, A>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum MetaSDFNode {
    // SDF primitives
    BoxSDF(MetaBoxSDF),
    SphereSDF(MetaSphereSDF),
    CapsuleSDF(MetaCapsuleSDF),
    GradientNoiseSDF(MetaGradientNoiseSDF),

    // SDF transforms
    SDFTranslation(MetaSDFTranslation),
    SDFRotation(MetaSDFRotation),
    SDFScaling(MetaSDFScaling),

    // SDF modifiers
    MultifractalNoiseSDFModifier(MetaMultifractalNoiseSDFModifier),
    MultiscaleSphereSDFModifier(MetaMultiscaleSphereSDFModifier),

    // SDF combination
    SDFUnion(MetaSDFUnion),
    SDFSubtraction(MetaSDFSubtraction),
    SDFIntersection(MetaSDFIntersection),
    SDFGroupUnion(MetaSDFGroupUnion),

    // Transform primitives
    StratifiedGridTransforms(MetaStratifiedGridTransforms),
    SphereSurfaceTransforms(MetaSphereSurfaceTransforms),

    // Transform operations
    TransformTranslation(MetaTransformTranslation),
    TransformRotation(MetaTransformRotation),
    TransformScaling(MetaTransformScaling),

    // SDF/transform operations
    ClosestTranslationToSurface(MetaClosestTranslationToSurface),
    RayTranslationToSurface(MetaRayTranslationToSurface),
    RotationToGradient(MetaRotationToGradient),

    // Transform application
    TransformApplication(MetaTransformApplication),

    // Filtering
    StochasticSelection(MetaStochasticSelection),
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

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum MetaSDFNodeOutput<A: Allocator> {
    SingleSDF(Option<SDFNodeID>),
    SDFGroup(AVec<SDFNodeID, A>),
    /// Transforms are from the local space of the node being transformed to the
    /// space of the node that applies the transform.
    SingleTransform(Option<Similarity3<f32>>),
    TransformGroup(AVec<Similarity3<f32>, A>),
}

/// A box-shaped SDF.
///
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaBoxSDF {
    /// Extent of the box along the x-axis, in voxels.
    pub extent_x: ContParamSpec,
    /// Extent of the box along the y-axis, in voxels.
    pub extent_y: ContParamSpec,
    /// Extent of the box along the z-axis, in voxels.
    pub extent_z: ContParamSpec,
    /// Seed for sampling random extent values.
    pub seed: u32,
}

define_meta_node_params! {
    MetaBoxSDF,
    struct MetaBoxParams {
        extent_x: f32,
        extent_y: f32,
        extent_z: f32,
    }
}

/// A sphere-shaped SDF.
///
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSphereSDF {
    /// Radius of the sphere, in voxels.
    pub radius: ContParamSpec,
    /// Seed for selecting a radius within the specified range.
    pub seed: u32,
}

define_meta_node_params! {
    MetaSphereSDF,
    struct MetaSphereParams {
        radius: f32,
    }
}

/// A vertical capsule-shaped SDF.
///
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaCapsuleSDF {
    /// Length between the centers of the spherical caps, in voxels.
    pub segment_length: ContParamSpec,
    /// Radius of the spherical caps, in voxels.
    pub radius: ContParamSpec,
    /// Seed for selecting a segment length and radius within the specified
    /// ranges.
    pub seed: u32,
}

define_meta_node_params! {
    MetaCapsuleSDF,
    struct MetaCapsuleParams {
        segment_length: f32,
        radius: f32,
    }
}

/// An SDF generated from thresholding a gradient noise field.
///
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaGradientNoiseSDF {
    /// Extent of the noise field along the x-axis, in voxels.
    pub extent_x: ContParamSpec,
    /// Extent of the noise field along the y-axis, in voxels.
    pub extent_y: ContParamSpec,
    /// Extent of the noise field along the z-axis, in voxels.
    pub extent_z: ContParamSpec,
    /// Spatial frequency of the noise pattern, in inverse voxels.
    pub noise_frequency: ContParamSpec,
    /// Minimum noise value (they range from -1 to 1) for a voxel to be
    /// considered inside the object.
    pub noise_threshold: ContParamSpec,
    /// Seed for generating noise and selecting parameter values within the
    /// specified ranges.
    pub seed: u32,
}

define_meta_node_params! {
    MetaGradientNoiseSDF,
    struct MetaGradientNoiseParams {
        extent_x: f32,
        extent_y: f32,
        extent_z: f32,
        noise_frequency: f32,
        noise_threshold: f32,
    }
}

/// Translation of one or more SDFs.
///
/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFTranslation {
    /// ID of the child SDF node to transform.
    pub child_id: MetaSDFNodeID,
    /// Translation distance along the x-axis, in voxels.
    pub translation_x: ContParamSpec,
    /// Translation distance along the y-axis, in voxels.
    pub translation_y: ContParamSpec,
    /// Translation distance along the z-axis, in voxels.
    pub translation_z: ContParamSpec,
    /// Seed for selecting a translation within the specified ranges.
    pub seed: u32,
}

define_meta_node_params! {
    MetaSDFTranslation,
    struct MetaSDFTranslationParams {
        translation_x: f32,
        translation_y: f32,
        translation_z: f32,
    }
}

/// Rotation of one or more SDFs.
///
/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFRotation {
    /// ID of the child SDF node to transform.
    pub child_id: MetaSDFNodeID,
    /// Rotation angle around the x-axis, in radians.
    pub roll: ContParamSpec,
    /// Rotation angle around the y-axis, in radians.
    pub pitch: ContParamSpec,
    /// Rotation angle around the z-axis, in radians.
    pub yaw: ContParamSpec,
    /// Seed for selecting a rotation within the specified ranges.
    pub seed: u32,
}

define_meta_node_params! {
    MetaSDFRotation,
    struct MetaSDFRotationParams {
        roll: f32,
        pitch: f32,
        yaw: f32,
    }
}

/// Uniform scaling of one or more SDFs.
///
/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFScaling {
    /// ID of the child SDF node to transform.
    pub child_id: MetaSDFNodeID,
    /// Uniform scale factor.
    pub scaling: ContParamSpec,
    /// Seed for selecting a scale factor within the specified range.
    pub seed: u32,
}

define_meta_node_params! {
    MetaSDFScaling,
    struct MetaSDFScalingParams {
        scaling: f32,
    }
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
    /// Noise amplitude (max distransform) in the first octave, in voxels.
    pub amplitude: ContParamSpec,
    /// Seed for generating noise and selecting parameter values within the
    /// specified ranges.
    pub seed: u32,
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
    /// Seed for generating random sphere radii as well as selecting parameter
    /// values within the specified ranges.
    pub seed: u32,
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

/// Transforms with translations from the center of a grid to grid points picked
/// by stratified sampling.
///
/// Output: `TransformGroup`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaStratifiedGridTransforms {
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
    /// Number of points generated within each grid cell.
    pub points_per_grid_cell: DiscreteParamSpec,
    /// Fraction of a grid cell to randomly displace the points.
    pub jitter_fraction: ContParamSpec,
    /// Seed for random jittering as well as selecting parameter values within
    /// the specified ranges.
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
        points_per_grid_cell: u32,
        jitter_fraction: f32,
    }
}

/// Transforms with translations from the center to the surface of a sphere and
/// optional rotations from the y-axis to the radial direction.
///
/// Output: `TransformGroup`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSphereSurfaceTransforms {
    /// Number of transforms to generate.
    pub count: DiscreteParamSpec,
    /// Radius of the sphere, in voxels.
    pub radius: ContParamSpec,
    /// Fraction of the regular point spacing to randomly displace the points.
    pub jitter_fraction: ContParamSpec,
    /// Whether to include rotations from the y-axes to the outward radial
    /// direction.
    pub rotation: SphereSurfaceRotation,
    /// Seed for random jittering as well as selecting parameter values within
    /// the specified ranges.
    pub seed: u32,
}

define_meta_node_params! {
    MetaSphereSurfaceTransforms,
    struct MetaSphereSurfaceParams {
        count: u32,
        radius: f32,
        jitter_fraction: f32,
    }
}

/// Translation of one or more transforms.
///
/// Input: `TransformGroup` or `SingleTransform`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaTransformTranslation {
    /// ID of the child transform node to translate.
    pub child_id: MetaSDFNodeID,
    /// Whether to apply the translation before ('Pre') or after ('Post') the
    /// input transforms.
    pub composition: CompositionMode,
    /// Translation distance along the x-axis, in voxels.
    pub translation_x: ContParamSpec,
    /// Translation distance along the y-axis, in voxels.
    pub translation_y: ContParamSpec,
    /// Translation distance along the z-axis, in voxels.
    pub translation_z: ContParamSpec,
    /// Seed for selecting a translation within the specified ranges.
    pub seed: u32,
}

define_meta_node_params! {
    MetaTransformTranslation,
    struct MetaTransformTranslationParams {
        translation_x: f32,
        translation_y: f32,
        translation_z: f32,
    }
}

/// Rotation of one or more transforms.
///
/// Input: `TransformGroup` or `SingleTransform`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaTransformRotation {
    /// ID of the child transform node to rotate.
    pub child_id: MetaSDFNodeID,
    /// Whether to apply the rotation before ('Pre') or after ('Post') the
    /// input transforms.
    pub composition: CompositionMode,
    /// Rotation angle around the x-axis, in radians.
    pub roll: ContParamSpec,
    /// Rotation angle around the y-axis, in radians.
    pub pitch: ContParamSpec,
    /// Rotation angle around the z-axis, in radians.
    pub yaw: ContParamSpec,
    /// Seed for selecting a rotation within the specified ranges.
    pub seed: u32,
}

define_meta_node_params! {
    MetaTransformRotation,
    struct MetaTransformRotationParams {
        roll: f32,
        pitch: f32,
        yaw: f32,
    }
}

/// Uniform scaling of one or more transforms.
///
/// Input: `TransformGroup` or `SingleTransform`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaTransformScaling {
    /// ID of the child transform node to scale.
    pub child_id: MetaSDFNodeID,
    /// Whether to apply the scaling before ('Pre') or after ('Post') the
    /// input transforms.
    pub composition: CompositionMode,
    /// Uniform scale factor.
    pub scaling: ContParamSpec,
    /// Seed for selecting a scale factor within the specified range.
    pub seed: u32,
}

define_meta_node_params! {
    MetaTransformScaling,
    struct MetaTransformScalingParams {
        scaling: f32,
    }
}

/// Translation of the SDFs or transforms in the second input to the closest
/// points on the surface of the SDF in the first input.
///
/// Input 1: `SingleSDF`
/// Input 2: Any
/// Output: Same as input 2
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaClosestTranslationToSurface {
    /// ID of the SDF node whose surface to translate to.
    pub surface_sdf_id: MetaSDFNodeID,
    /// ID of the node containing SDFs or transforms to translate.
    pub subject_id: MetaSDFNodeID,
}

/// Translation of the SDFs or transforms in the second input to the
/// intersection of their y-axes with the surface of the SDF in the first input.
///
/// Input 1: `SingleSDF`
/// Input 2: Any
/// Output: Same as input 2
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaRayTranslationToSurface {
    /// ID of the SDF node whose surface to translate to.
    pub surface_sdf_id: MetaSDFNodeID,
    /// ID of the node containing SDFs or transforms to translate.
    pub subject_id: MetaSDFNodeID,
}

/// Rotation of the SDFs or transforms in the second input to make their y-axis
/// align with the gradient of the SDF in the first input.
///
/// Input 1: `SingleSDF`
/// Input 2: Any
/// Output: Same as input 2
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaRotationToGradient {
    /// ID of the SDF node whose gradient to align with.
    pub gradient_sdf_id: MetaSDFNodeID,
    /// ID of the node containing SDFs or transforms to rotate.
    pub subject_id: MetaSDFNodeID,
}

/// Application of the transforms in the second input to the SDFs in the first
/// input (yields all combinations).
///
/// Input 1: `SDFGroup` or `SingleSDF`
/// Input 2: `TransformGroup` or `SingleTransform`
/// Output: `SDFGroup`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaTransformApplication {
    /// ID of the SDF or SDF group node to scatter.
    pub sdf_id: MetaSDFNodeID,
    /// ID of the transform or transform group node to apply.
    pub transform_id: MetaSDFNodeID,
}

/// Random selection of SDFs or transforms from a group.
///
/// Input: `SDFGroup` or `TransformGroup`
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

/// How to combine the current transformation with the input transformation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionMode {
    /// Apply the current transformation to the subject *before* applying the
    /// input transformation.
    Pre,
    /// Apply the current transformation to the subject *after* applying the
    /// input transformation.
    Post,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SphereSurfaceRotation {
    Identity,
    Radial,
}

impl<A: Allocator> MetaSDFGraph<A> {
    pub fn new_in(alloc: A, seed: u64) -> Self {
        Self {
            seed,
            nodes: AVec::new_in(alloc),
        }
    }

    pub fn with_capacity_in(capacity: usize, alloc: A, seed: u64) -> Self {
        Self {
            seed,
            nodes: AVec::with_capacity_in(capacity, alloc),
        }
    }

    pub fn add_node(&mut self, node: MetaSDFNode) -> MetaSDFNodeID {
        let id = self.nodes.len().try_into().unwrap();
        self.nodes.push(node);
        id
    }

    pub fn build<AR>(&self, arena: AR) -> Result<SDFGraph<AR>>
    where
        AR: Allocator + Copy,
    {
        let mut graph = SDFGraph::new_in(arena);

        if self.nodes.is_empty() {
            return Ok(graph);
        }

        let mut outputs = AVec::new_in(arena);
        outputs.resize(self.nodes.len(), MetaSDFNodeOutput::<AR>::SingleSDF(None));

        let mut states = AVec::new_in(arena);
        states.resize(self.nodes.len(), MetaNodeBuildState::Unvisited);

        let mut stable_seeds = AVec::new_in(arena);
        stable_seeds.resize(self.nodes.len(), 0u64);

        let mut param_scratch = ParamScratch::new_in(arena);

        let mut operation_stack = AVec::with_capacity_in(3 * self.nodes.len(), arena);

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
                                MetaSDFNode::BoxSDF(_)
                                | MetaSDFNode::SphereSDF(_)
                                | MetaSDFNode::CapsuleSDF(_)
                                | MetaSDFNode::GradientNoiseSDF(_)
                                | MetaSDFNode::StratifiedGridTransforms(_)
                                | MetaSDFNode::SphereSurfaceTransforms(_) => {}
                                MetaSDFNode::SDFTranslation(MetaSDFTranslation {
                                    child_id,
                                    ..
                                })
                                | MetaSDFNode::SDFRotation(MetaSDFRotation { child_id, .. })
                                | MetaSDFNode::SDFScaling(MetaSDFScaling { child_id, .. })
                                | MetaSDFNode::MultifractalNoiseSDFModifier(
                                    MetaMultifractalNoiseSDFModifier { child_id, .. },
                                )
                                | MetaSDFNode::MultiscaleSphereSDFModifier(
                                    MetaMultiscaleSphereSDFModifier { child_id, .. },
                                )
                                | MetaSDFNode::SDFGroupUnion(MetaSDFGroupUnion {
                                    child_id, ..
                                })
                                | MetaSDFNode::TransformTranslation(MetaTransformTranslation {
                                    child_id,
                                    ..
                                })
                                | MetaSDFNode::TransformRotation(MetaTransformRotation {
                                    child_id,
                                    ..
                                })
                                | MetaSDFNode::TransformScaling(MetaTransformScaling {
                                    child_id,
                                    ..
                                })
                                | MetaSDFNode::StochasticSelection(MetaStochasticSelection {
                                    child_id,
                                    ..
                                }) => {
                                    operation_stack.push(BuildOperation::VisitChildren(*child_id));
                                }
                                MetaSDFNode::SDFUnion(MetaSDFUnion {
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
                                })
                                | MetaSDFNode::TransformApplication(MetaTransformApplication {
                                    sdf_id: child_1_id,
                                    transform_id: child_2_id,
                                })
                                | MetaSDFNode::ClosestTranslationToSurface(
                                    MetaClosestTranslationToSurface {
                                        surface_sdf_id: child_1_id,
                                        subject_id: child_2_id,
                                    },
                                )
                                | MetaSDFNode::RayTranslationToSurface(
                                    MetaRayTranslationToSurface {
                                        surface_sdf_id: child_1_id,
                                        subject_id: child_2_id,
                                    },
                                )
                                | MetaSDFNode::RotationToGradient(MetaRotationToGradient {
                                    gradient_sdf_id: child_1_id,
                                    subject_id: child_2_id,
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

                    let seed = splitmix::random_u64_from_two_states(self.seed, stable_seed);

                    outputs[node_idx] =
                        node.resolve(arena, &mut param_scratch, &mut graph, &outputs, seed)?;
                }
            }
        }

        if let MetaSDFNodeOutput::SingleSDF(atomic_node_id) = &outputs[root_node_id as usize] {
            if let Some(id) = atomic_node_id {
                graph.set_root_node(*id);
            } else {
                return Ok(SDFGraph::new_in(arena));
            }
        } else {
            bail!("Root meta node must have single SDF output");
        }

        Ok(graph)
    }
}

impl<A: Allocator> MetaSDFNodeOutput<A> {
    fn label(&self) -> &'static str {
        match self {
            Self::SingleSDF(_) => "SingleSDF",
            Self::SDFGroup(_) => "SDFGroup",
            Self::SingleTransform(_) => "SingleTransform",
            Self::TransformGroup(_) => "TransformGroup",
        }
    }
}

impl MetaSDFNode {
    /// Combines a node type tag, node seed parameter (for applicable nodes) and
    /// the stable seeds of the child nodes to obtain a stable seed that will
    /// only change due to changes in the seeding, types or topology of the
    /// node's subgraph.
    fn obtain_stable_seed(&self, stable_seeds: &[u64]) -> u64 {
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
            Self::BoxSDF(MetaBoxSDF { seed, .. }) => combine_seeded_leaf(0x01, seed),
            Self::SphereSDF(MetaSphereSDF { seed, .. }) => combine_seeded_leaf(0x02, seed),
            Self::CapsuleSDF(MetaCapsuleSDF { seed, .. }) => combine_seeded_leaf(0x03, seed),
            Self::GradientNoiseSDF(MetaGradientNoiseSDF { seed, .. }) => {
                combine_seeded_leaf(0x04, seed)
            }
            Self::SDFTranslation(MetaSDFTranslation { seed, child_id, .. }) => {
                combine_seeded_unary(0x10, seed, child_id)
            }
            Self::SDFRotation(MetaSDFRotation { seed, child_id, .. }) => {
                combine_seeded_unary(0x11, seed, child_id)
            }
            Self::SDFScaling(MetaSDFScaling { seed, child_id, .. }) => {
                combine_seeded_unary(0x12, seed, child_id)
            }
            Self::MultifractalNoiseSDFModifier(MetaMultifractalNoiseSDFModifier {
                seed,
                child_id,
                ..
            }) => combine_seeded_unary(0x20, seed, child_id),
            Self::MultiscaleSphereSDFModifier(MetaMultiscaleSphereSDFModifier {
                seed,
                child_id,
                ..
            }) => combine_seeded_unary(0x21, seed, child_id),
            Self::SDFUnion(MetaSDFUnion {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary_commutative(0x30, child_1_id, child_2_id),
            Self::SDFSubtraction(MetaSDFSubtraction {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary(0x31, child_1_id, child_2_id),
            Self::SDFIntersection(MetaSDFIntersection {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary_commutative(0x32, child_1_id, child_2_id),
            Self::SDFGroupUnion(MetaSDFGroupUnion { child_id, .. }) => {
                combine_unary(0x33, child_id)
            }
            Self::StratifiedGridTransforms(MetaStratifiedGridTransforms { seed, .. }) => {
                combine_seeded_leaf(0x40, seed)
            }
            Self::SphereSurfaceTransforms(MetaSphereSurfaceTransforms { seed, .. }) => {
                combine_seeded_leaf(0x41, seed)
            }
            Self::TransformTranslation(MetaTransformTranslation { seed, child_id, .. }) => {
                combine_seeded_unary(0x50, seed, child_id)
            }
            Self::TransformRotation(MetaTransformRotation { seed, child_id, .. }) => {
                combine_seeded_unary(0x51, seed, child_id)
            }
            Self::TransformScaling(MetaTransformScaling { seed, child_id, .. }) => {
                combine_seeded_unary(0x52, seed, child_id)
            }
            Self::ClosestTranslationToSurface(MetaClosestTranslationToSurface {
                surface_sdf_id,
                subject_id,
            }) => combine_binary(0x60, surface_sdf_id, subject_id),
            Self::RayTranslationToSurface(MetaRayTranslationToSurface {
                surface_sdf_id,
                subject_id,
            }) => combine_binary(0x61, surface_sdf_id, subject_id),
            Self::RotationToGradient(MetaRotationToGradient {
                gradient_sdf_id,
                subject_id,
            }) => combine_binary(0x62, gradient_sdf_id, subject_id),
            Self::TransformApplication(MetaTransformApplication {
                sdf_id,
                transform_id,
            }) => combine_binary(0x70, sdf_id, transform_id),
            Self::StochasticSelection(MetaStochasticSelection { seed, child_id, .. }) => {
                combine_seeded_unary(0x80, seed, child_id)
            }
        }
    }

    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        match self {
            Self::BoxSDF(node) => node
                .resolve(param_scratch, graph, seed)
                .context("Failed to resolve BoxSDF node"),
            Self::SphereSDF(node) => node
                .resolve(param_scratch, graph, seed)
                .context("Failed to resolve SphereSDF node"),
            Self::CapsuleSDF(node) => node
                .resolve(param_scratch, graph, seed)
                .context("Failed to resolve CapsuleSDF node"),
            Self::GradientNoiseSDF(node) => node
                .resolve(param_scratch, graph, seed)
                .context("Failed to resolve GradientNoiseSDF node"),
            Self::SDFTranslation(node) => node
                .resolve(arena, param_scratch, graph, outputs, seed)
                .context("Failed to resolve SDFTranslation node"),
            Self::SDFRotation(node) => node
                .resolve(arena, param_scratch, graph, outputs, seed)
                .context("Failed to resolve SDFRotation node"),
            Self::SDFScaling(node) => node
                .resolve(arena, param_scratch, graph, outputs, seed)
                .context("Failed to resolve SDFScaling node"),
            Self::MultifractalNoiseSDFModifier(node) => node
                .resolve(arena, param_scratch, graph, outputs, seed)
                .context("Failed to resolve MultifractalNoiseSDFModifier node"),
            Self::MultiscaleSphereSDFModifier(node) => node
                .resolve(arena, param_scratch, graph, outputs, seed)
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
            Self::StratifiedGridTransforms(node) => node
                .resolve(param_scratch, arena, seed)
                .context("Failed to resolve StratifiedGridTransforms node"),
            Self::SphereSurfaceTransforms(node) => node
                .resolve(param_scratch, arena, seed)
                .context("Failed to resolve SphereSurfaceTransforms node"),
            Self::TransformTranslation(node) => node
                .resolve(arena, param_scratch, outputs, seed)
                .context("Failed to resolve TransformTranslation node"),
            Self::TransformRotation(node) => node
                .resolve(arena, param_scratch, outputs, seed)
                .context("Failed to resolve TransformRotation node"),
            Self::TransformScaling(node) => node
                .resolve(arena, param_scratch, outputs, seed)
                .context("Failed to resolve TransformScaling node"),
            Self::ClosestTranslationToSurface(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve ClosestTranslationToSurface node"),
            Self::RayTranslationToSurface(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve RayTranslationToSurface node"),
            Self::RotationToGradient(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve RotationToGradient node"),
            Self::TransformApplication(node) => node
                .resolve(arena, graph, outputs)
                .context("Failed to resolve TransformApplication node"),
            Self::StochasticSelection(node) => Ok(node.resolve(arena, outputs, seed)),
        }
    }
}

impl MetaBoxSDF {
    fn resolve<A>(
        &self,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_param_rng(seed);

        let MetaBoxParams {
            extent_x,
            extent_y,
            extent_z,
        } = self.sample_params(param_scratch, &mut rng)?;

        let extents = [extent_x.max(0.0), extent_y.max(0.0), extent_z.max(0.0)];

        let node_id = graph.add_node(SDFNode::new_box(extents));

        Ok(MetaSDFNodeOutput::SingleSDF(Some(node_id)))
    }
}

impl MetaSphereSDF {
    fn resolve<A>(
        &self,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_param_rng(seed);

        let MetaSphereParams { radius } = self.sample_params(param_scratch, &mut rng)?;

        let radius = radius.max(0.0);

        let node_id = graph.add_node(SDFNode::new_sphere(radius));

        Ok(MetaSDFNodeOutput::SingleSDF(Some(node_id)))
    }
}

impl MetaCapsuleSDF {
    fn resolve<A>(
        &self,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_param_rng(seed);

        let MetaCapsuleParams {
            segment_length,
            radius,
        } = self.sample_params(param_scratch, &mut rng)?;

        let segment_length = segment_length.max(0.0);
        let radius = radius.max(0.0);

        let node_id = graph.add_node(SDFNode::new_capsule(segment_length, radius));

        Ok(MetaSDFNodeOutput::SingleSDF(Some(node_id)))
    }
}

impl MetaGradientNoiseSDF {
    fn resolve<A>(
        &self,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_param_rng(seed);

        let MetaGradientNoiseParams {
            extent_x,
            extent_y,
            extent_z,
            noise_frequency,
            noise_threshold,
        } = self.sample_params(param_scratch, &mut rng)?;

        let extents = [extent_x.max(0.0), extent_y.max(0.0), extent_z.max(0.0)];

        let seed = rng.random();

        let node_id = graph.add_node(SDFNode::new_gradient_noise(
            extents,
            noise_frequency,
            noise_threshold,
            seed,
        ));

        Ok(MetaSDFNodeOutput::SingleSDF(Some(node_id)))
    }
}

impl MetaSDFTranslation {
    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        resolve_unary_sdf_op(
            arena,
            graph,
            "SDFTranslation",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let MetaSDFTranslationParams {
                    translation_x,
                    translation_y,
                    translation_z,
                } = self.sample_params(param_scratch, rng)?;

                let translation = vector![translation_x, translation_y, translation_z,];

                Ok(SDFNode::new_translation(input_node_id, translation))
            },
        )
    }
}

impl MetaSDFRotation {
    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        resolve_unary_sdf_op(
            arena,
            graph,
            "SDFRotation",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let MetaSDFRotationParams { roll, pitch, yaw } =
                    self.sample_params(param_scratch, rng)?;

                Ok(SDFNode::new_rotation(
                    input_node_id,
                    UnitQuaternion::from_euler_angles(roll, pitch, yaw),
                ))
            },
        )
    }
}

impl MetaSDFScaling {
    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        resolve_unary_sdf_op(
            arena,
            graph,
            "SDFScaling",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let MetaSDFScalingParams { scaling } = self.sample_params(param_scratch, rng)?;

                let scaling = scaling.max(f32::EPSILON);

                Ok(SDFNode::new_scaling(input_node_id, scaling))
            },
        )
    }
}

impl MetaMultifractalNoiseSDFModifier {
    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        resolve_unary_sdf_op(
            arena,
            graph,
            "MultifractalNoiseSDFModifier",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let MetaMultifractalNoiseParams {
                    octaves,
                    frequency,
                    lacunarity,
                    persistence,
                    amplitude,
                } = self.sample_params(param_scratch, rng)?;

                let seed = rng.random();

                Ok(SDFNode::new_multifractal_noise(
                    input_node_id,
                    octaves,
                    frequency,
                    lacunarity,
                    persistence,
                    amplitude,
                    seed,
                ))
            },
        )
    }
}

impl MetaMultiscaleSphereSDFModifier {
    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        resolve_unary_sdf_op(
            arena,
            graph,
            "MultiscaleSphereSDFModifier",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let MetaMultiscaleSphereParams {
                    octaves,
                    max_scale,
                    persistence,
                    inflation,
                    intersection_smoothness,
                    union_smoothness,
                } = self.sample_params(param_scratch, rng)?;

                let seed = rng.random();

                Ok(SDFNode::new_multiscale_sphere(
                    input_node_id,
                    octaves,
                    max_scale,
                    persistence,
                    inflation,
                    intersection_smoothness,
                    union_smoothness,
                    seed,
                ))
            },
        )
    }
}

impl MetaSDFUnion {
    fn resolve<A>(
        &self,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
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
    fn resolve<A>(
        &self,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
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
    fn resolve<A>(
        &self,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
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
    fn resolve<A>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
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
            child_output => {
                bail!(
                    "SDFGroupUnion node expects SDFGroup or SingleSDF input, got {}",
                    child_output.label()
                );
            }
        }
    }
}

impl MetaStratifiedGridTransforms {
    fn resolve<A>(
        &self,
        param_scratch: &mut ParamScratch<A>,
        arena: A,
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_param_rng(seed);

        let MetaStratifiedGridParams {
            shape_x,
            shape_y,
            shape_z,
            cell_extent_x,
            cell_extent_y,
            cell_extent_z,
            points_per_grid_cell,
            jitter_fraction,
        } = self.sample_params(param_scratch, &mut rng)?;

        let shape = [shape_x, shape_y, shape_z];
        let cell_extents = [
            cell_extent_x.max(0.0),
            cell_extent_y.max(0.0),
            cell_extent_z.max(0.0),
        ];
        let jitter_fraction = jitter_fraction.clamp(0.0, 1.0);

        let grid_cell_count = (shape[0] as usize) * (shape[1] as usize) * (shape[2] as usize);
        let point_count = grid_cell_count * points_per_grid_cell as usize;

        if point_count == 0 {
            return Ok(MetaSDFNodeOutput::TransformGroup(AVec::new_in(arena)));
        }

        let grid_center: [_; 3] = array::from_fn(|i| {
            let cell_extent = cell_extents[i];
            let grid_extent = shape[i] as f32 * cell_extent;
            0.5 * grid_extent
        });

        // Center of the lower corner cell
        let start_pos: [_; 3] = array::from_fn(|i| {
            let cell_extent = cell_extents[i];
            let grid_extent = shape[i] as f32 * cell_extent;
            -0.5 * grid_extent + 0.5 * cell_extent
        });

        let mut translations = AVec::with_capacity_in(point_count, arena);

        let uniform_distr = Uniform::new(-0.5, 0.5).unwrap();

        for i in 0..shape[0] {
            let x = start_pos[0] + i as f32 * cell_extents[0] - grid_center[0];
            for j in 0..shape[1] {
                let y = start_pos[1] + j as f32 * cell_extents[1] - grid_center[1];
                for k in 0..shape[2] {
                    let z = start_pos[2] + k as f32 * cell_extents[2] - grid_center[2];

                    for _ in 0..points_per_grid_cell {
                        let jx = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[0];
                        let jy = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[1];
                        let jz = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[2];

                        translations.push(Similarity::from_parts(
                            Translation3::new(x + jx, y + jy, z + jz),
                            UnitQuaternion::identity(),
                            1.0,
                        ));
                    }
                }
            }
        }

        Ok(MetaSDFNodeOutput::TransformGroup(translations))
    }
}

impl MetaSphereSurfaceTransforms {
    fn resolve<A>(
        &self,
        param_scratch: &mut ParamScratch<A>,
        arena: A,
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_param_rng(seed);

        let MetaSphereSurfaceParams {
            count,
            radius,
            jitter_fraction,
        } = self.sample_params(param_scratch, &mut rng)?;

        let count = count as usize;
        let radius = radius.max(0.0);
        let jitter_fraction = jitter_fraction.clamp(0.0, 1.0);

        if count == 0 {
            return Ok(MetaSDFNodeOutput::TransformGroup(AVec::new_in(arena)));
        }

        let max_jitter_angle = Self::compute_max_jitter_angle(count, jitter_fraction);

        let mut transforms = AVec::with_capacity_in(count, arena);

        for direction in compute_uniformly_distributed_radial_directions(count) {
            let jittered_direction =
                compute_jittered_direction(direction, max_jitter_angle, &mut rng);

            let translation = jittered_direction.scale(radius);

            let rotation = match self.rotation {
                SphereSurfaceRotation::Identity => UnitQuaternion::identity(),
                SphereSurfaceRotation::Radial => {
                    rotation_between_axes(&Vector3::y_axis(), &jittered_direction)
                }
            };

            transforms.push(Similarity3::from_parts(
                Translation3::from(translation),
                rotation,
                1.0,
            ));
        }

        Ok(MetaSDFNodeOutput::TransformGroup(transforms))
    }

    fn compute_max_jitter_angle(count: usize, jitter_fraction: f32) -> f32 {
        let solid_angle_per_transform = 4.0 * PI / (count as f32);

        let max_polar_angle_for_cap_covering_solid_angle = (1.0
            - solid_angle_per_transform / (2.0 * PI))
            .clamp(-1.0, 1.0)
            .acos();

        let max_jitter_angle = jitter_fraction * max_polar_angle_for_cap_covering_solid_angle;

        // Clamp to avoid very large angles for low counts
        max_jitter_angle.clamp(0.0, 0.5 * PI)
    }
}

impl MetaTransformTranslation {
    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        resolve_unary_transform_op(
            arena,
            "TransformTranslation",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_transform| {
                let MetaTransformTranslationParams {
                    translation_x,
                    translation_y,
                    translation_z,
                } = self.sample_params(param_scratch, rng)?;

                let translation = Translation3::from([translation_x, translation_y, translation_z]);

                Ok(match self.composition {
                    CompositionMode::Pre => input_transform * translation,
                    CompositionMode::Post => translation * input_transform,
                })
            },
        )
    }
}

impl MetaTransformRotation {
    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        resolve_unary_transform_op(
            arena,
            "TransformRotation",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_transform| {
                let MetaTransformRotationParams { roll, pitch, yaw } =
                    self.sample_params(param_scratch, rng)?;

                let rotation = UnitQuaternion::from_euler_angles(roll, pitch, yaw);

                Ok(match self.composition {
                    CompositionMode::Pre => input_transform * rotation,
                    CompositionMode::Post => rotation * input_transform,
                })
            },
        )
    }
}

impl MetaTransformScaling {
    fn resolve<A>(
        &self,
        arena: A,
        param_scratch: &mut ParamScratch<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        resolve_unary_transform_op(
            arena,
            "TransformScaling",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_transform| {
                let MetaTransformScalingParams { scaling } =
                    self.sample_params(param_scratch, rng)?;

                let scaling = scaling.max(f32::EPSILON);

                Ok(match self.composition {
                    CompositionMode::Pre => input_transform.prepend_scaling(scaling),
                    CompositionMode::Post => input_transform.append_scaling(scaling),
                })
            },
        )
    }
}

impl MetaClosestTranslationToSurface {
    fn resolve<A>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let subject_node_output = &outputs[self.subject_id as usize];

        let sdf_node_id = match &outputs[self.surface_sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => {
                return Ok(subject_node_output.clone());
            }
            MetaSDFNodeOutput::SingleSDF(Some(sdf_node_id)) => *sdf_node_id,
            child_output => {
                bail!(
                    "ClosestTranslationToSurface node expects SingleSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        if let MetaSDFNodeOutput::SingleSDF(None) | MetaSDFNodeOutput::SingleTransform(None) =
            subject_node_output
        {
            return Ok(subject_node_output.clone());
        };

        let generator = SDFGenerator::new_in(arena, arena, graph.nodes(), sdf_node_id)?;
        let mut buffers = generator.create_buffers_for_block(arena);

        let surface_sdf_node_to_parent_transform =
            graph.nodes()[sdf_node_id as usize].node_to_parent_transform();

        let mut compute_translation_to_surface =
            |subject_node_to_parent_transform: &Similarity3<f32>| {
                compute_translation_to_closest_point_on_surface(
                    &generator,
                    &mut buffers,
                    &surface_sdf_node_to_parent_transform,
                    subject_node_to_parent_transform,
                    5,
                    0.25,
                )
            };

        match subject_node_output {
            MetaSDFNodeOutput::SingleSDF(subject_node_id) => {
                let subject_node_id = subject_node_id.unwrap();
                let subject_node_to_parent_transform =
                    graph.nodes()[subject_node_id as usize].node_to_parent_transform();

                let Some(translation_to_surface_in_parent_space) =
                    compute_translation_to_surface(&subject_node_to_parent_transform)
                else {
                    return Ok(MetaSDFNodeOutput::SingleSDF(None));
                };

                let translated_subject_node_id = graph.add_node(SDFNode::new_translation(
                    subject_node_id,
                    translation_to_surface_in_parent_space,
                ));

                Ok(MetaSDFNodeOutput::SingleSDF(Some(
                    translated_subject_node_id,
                )))
            }
            MetaSDFNodeOutput::SDFGroup(subject_node_ids) => {
                let mut translated_subject_node_ids =
                    AVec::with_capacity_in(subject_node_ids.len(), arena);

                for &subject_node_id in subject_node_ids {
                    let subject_node_to_parent_transform =
                        graph.nodes()[subject_node_id as usize].node_to_parent_transform();

                    let Some(translation_to_surface_in_parent_space) =
                        compute_translation_to_surface(&subject_node_to_parent_transform)
                    else {
                        continue;
                    };

                    let translated_subject_node_id = graph.add_node(SDFNode::new_translation(
                        subject_node_id,
                        translation_to_surface_in_parent_space,
                    ));
                    translated_subject_node_ids.push(translated_subject_node_id);
                }

                Ok(MetaSDFNodeOutput::SDFGroup(translated_subject_node_ids))
            }
            MetaSDFNodeOutput::SingleTransform(subject_transform) => {
                let subject_node_to_parent_transform = subject_transform.as_ref().unwrap();

                let Some(translation_to_surface_in_parent_space) =
                    compute_translation_to_surface(subject_node_to_parent_transform)
                else {
                    return Ok(MetaSDFNodeOutput::SingleTransform(None));
                };

                let translated_subject_transform =
                    Translation3::from(translation_to_surface_in_parent_space)
                        * subject_node_to_parent_transform;

                Ok(MetaSDFNodeOutput::SingleTransform(Some(
                    translated_subject_transform,
                )))
            }
            MetaSDFNodeOutput::TransformGroup(subject_transforms) => {
                let mut translated_subject_transforms =
                    AVec::with_capacity_in(subject_transforms.len(), arena);

                for subject_node_to_parent_transform in subject_transforms {
                    let Some(translation_to_surface_in_parent_space) =
                        compute_translation_to_surface(subject_node_to_parent_transform)
                    else {
                        continue;
                    };

                    let translated_subject_transform =
                        Translation3::from(translation_to_surface_in_parent_space)
                            * subject_node_to_parent_transform;

                    translated_subject_transforms.push(translated_subject_transform);
                }

                Ok(MetaSDFNodeOutput::TransformGroup(
                    translated_subject_transforms,
                ))
            }
        }
    }
}

impl MetaRayTranslationToSurface {
    fn resolve<A>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let subject_node_output = &outputs[self.subject_id as usize];

        let sdf_node_id = match &outputs[self.surface_sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => {
                return Ok(subject_node_output.clone());
            }
            MetaSDFNodeOutput::SingleSDF(Some(sdf_node_id)) => *sdf_node_id,
            child_output => {
                bail!(
                    "RayTranslationToSurface node expects SingleSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        if let MetaSDFNodeOutput::SingleSDF(None) | MetaSDFNodeOutput::SingleTransform(None) =
            subject_node_output
        {
            return Ok(subject_node_output.clone());
        };

        let generator = SDFGenerator::new_in(arena, arena, graph.nodes(), sdf_node_id)?;
        let mut buffers = generator.create_buffers_for_block(arena);

        let surface_sdf_node_to_parent_transform =
            graph.nodes()[sdf_node_id as usize].node_to_parent_transform();

        let mut compute_translation_to_surface =
            |subject_node_to_parent_transform: &Similarity3<f32>| {
                compute_translation_to_ray_intersection_point_on_surface(
                    &generator,
                    &mut buffers,
                    &surface_sdf_node_to_parent_transform,
                    subject_node_to_parent_transform,
                    64,
                    0.25,
                    0.5,
                )
            };

        match subject_node_output {
            MetaSDFNodeOutput::SingleSDF(subject_node_id) => {
                let subject_node_id = subject_node_id.unwrap();
                let subject_node_to_parent_transform =
                    graph.nodes()[subject_node_id as usize].node_to_parent_transform();

                let Some(translation_to_surface_in_parent_space) =
                    compute_translation_to_surface(&subject_node_to_parent_transform)
                else {
                    return Ok(MetaSDFNodeOutput::SingleSDF(None));
                };

                let translated_subject_node_id = graph.add_node(SDFNode::new_translation(
                    subject_node_id,
                    translation_to_surface_in_parent_space,
                ));

                Ok(MetaSDFNodeOutput::SingleSDF(Some(
                    translated_subject_node_id,
                )))
            }
            MetaSDFNodeOutput::SDFGroup(subject_node_ids) => {
                let mut translated_subject_node_ids =
                    AVec::with_capacity_in(subject_node_ids.len(), arena);

                for &subject_node_id in subject_node_ids {
                    let subject_node_to_parent_transform =
                        graph.nodes()[subject_node_id as usize].node_to_parent_transform();

                    let Some(translation_to_surface_in_parent_space) =
                        compute_translation_to_surface(&subject_node_to_parent_transform)
                    else {
                        continue;
                    };

                    let translated_subject_node_id = graph.add_node(SDFNode::new_translation(
                        subject_node_id,
                        translation_to_surface_in_parent_space,
                    ));
                    translated_subject_node_ids.push(translated_subject_node_id);
                }

                Ok(MetaSDFNodeOutput::SDFGroup(translated_subject_node_ids))
            }
            MetaSDFNodeOutput::SingleTransform(subject_transform) => {
                let subject_node_to_parent_transform = subject_transform.as_ref().unwrap();

                let Some(translation_to_surface_in_parent_space) =
                    compute_translation_to_surface(subject_node_to_parent_transform)
                else {
                    return Ok(MetaSDFNodeOutput::SingleTransform(None));
                };

                let translated_subject_transform =
                    Translation3::from(translation_to_surface_in_parent_space)
                        * subject_node_to_parent_transform;

                Ok(MetaSDFNodeOutput::SingleTransform(Some(
                    translated_subject_transform,
                )))
            }
            MetaSDFNodeOutput::TransformGroup(subject_transforms) => {
                let mut translated_subject_transforms =
                    AVec::with_capacity_in(subject_transforms.len(), arena);

                for subject_node_to_parent_transform in subject_transforms {
                    let Some(translation_to_surface_in_parent_space) =
                        compute_translation_to_surface(subject_node_to_parent_transform)
                    else {
                        continue;
                    };

                    let translated_subject_transform =
                        Translation3::from(translation_to_surface_in_parent_space)
                            * subject_node_to_parent_transform;

                    translated_subject_transforms.push(translated_subject_transform);
                }

                Ok(MetaSDFNodeOutput::TransformGroup(
                    translated_subject_transforms,
                ))
            }
        }
    }
}

impl MetaRotationToGradient {
    fn resolve<A>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let subject_node_output = &outputs[self.subject_id as usize];

        let sdf_node_id = match &outputs[self.gradient_sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => {
                return Ok(subject_node_output.clone());
            }
            MetaSDFNodeOutput::SingleSDF(Some(sdf_node_id)) => *sdf_node_id,
            child_output => {
                bail!(
                    "RotationToGradient node expects SingleSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        if let MetaSDFNodeOutput::SingleSDF(None) | MetaSDFNodeOutput::SingleTransform(None) =
            subject_node_output
        {
            return Ok(subject_node_output.clone());
        };

        let generator = SDFGenerator::new_in(arena, arena, graph.nodes(), sdf_node_id)?;
        let mut buffers = generator.create_buffers_for_block(arena);

        let gradient_sdf_node_to_parent_transform =
            graph.nodes()[sdf_node_id as usize].node_to_parent_transform();

        let mut compute_rotation_to_gradient =
            |subject_node_to_parent_transform: &Similarity3<f32>| {
                compute_rotation_to_gradient(
                    &generator,
                    &mut buffers,
                    &gradient_sdf_node_to_parent_transform,
                    subject_node_to_parent_transform,
                )
            };

        match subject_node_output {
            MetaSDFNodeOutput::SingleSDF(subject_node_id) => {
                let subject_node_id = subject_node_id.unwrap();
                let subject_node_to_parent_transform =
                    graph.nodes()[subject_node_id as usize].node_to_parent_transform();

                let Some(rotation_to_gradient_in_parent_space) =
                    compute_rotation_to_gradient(&subject_node_to_parent_transform)
                else {
                    return Ok(MetaSDFNodeOutput::SingleSDF(None));
                };

                let rotated_subject_node_id = graph.add_node(SDFNode::new_rotation(
                    subject_node_id,
                    rotation_to_gradient_in_parent_space,
                ));

                Ok(MetaSDFNodeOutput::SingleSDF(Some(rotated_subject_node_id)))
            }
            MetaSDFNodeOutput::SDFGroup(subject_node_ids) => {
                let mut rotated_subject_node_ids =
                    AVec::with_capacity_in(subject_node_ids.len(), arena);

                for &subject_node_id in subject_node_ids {
                    let subject_node_to_parent_transform =
                        graph.nodes()[subject_node_id as usize].node_to_parent_transform();

                    let Some(rotation_to_gradient_in_parent_space) =
                        compute_rotation_to_gradient(&subject_node_to_parent_transform)
                    else {
                        continue;
                    };

                    let rotated_subject_node_id = graph.add_node(SDFNode::new_rotation(
                        subject_node_id,
                        rotation_to_gradient_in_parent_space,
                    ));
                    rotated_subject_node_ids.push(rotated_subject_node_id);
                }

                Ok(MetaSDFNodeOutput::SDFGroup(rotated_subject_node_ids))
            }
            MetaSDFNodeOutput::SingleTransform(subject_transform) => {
                let subject_node_to_parent_transform = subject_transform.as_ref().unwrap();

                let Some(rotation_to_gradient_in_parent_space) =
                    compute_rotation_to_gradient(subject_node_to_parent_transform)
                else {
                    return Ok(MetaSDFNodeOutput::SingleTransform(None));
                };

                let rotated_subject_transform =
                    rotation_to_gradient_in_parent_space * subject_node_to_parent_transform;

                Ok(MetaSDFNodeOutput::SingleTransform(Some(
                    rotated_subject_transform,
                )))
            }
            MetaSDFNodeOutput::TransformGroup(subject_transforms) => {
                let mut rotated_subject_transforms =
                    AVec::with_capacity_in(subject_transforms.len(), arena);

                for subject_node_to_parent_transform in subject_transforms {
                    let Some(rotation_to_gradient_in_parent_space) =
                        compute_rotation_to_gradient(subject_node_to_parent_transform)
                    else {
                        continue;
                    };

                    let rotated_subject_transform =
                        rotation_to_gradient_in_parent_space * subject_node_to_parent_transform;

                    rotated_subject_transforms.push(rotated_subject_transform);
                }

                Ok(MetaSDFNodeOutput::TransformGroup(
                    rotated_subject_transforms,
                ))
            }
        }
    }
}

impl MetaTransformApplication {
    fn resolve<A>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let sdf_node_ids = match &outputs[self.sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(sdf_node_id) => {
                let mut sdf_node_ids = AVec::new_in(arena);
                if let Some(sdf_node_id) = sdf_node_id {
                    sdf_node_ids.push(*sdf_node_id);
                }
                Cow::Owned(sdf_node_ids)
            }
            MetaSDFNodeOutput::SDFGroup(sdf_node_ids) => Cow::Borrowed(sdf_node_ids),
            child_output => {
                bail!(
                    "Scattering node expects SingleSDF or GroupSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        let transforms = match &outputs[self.transform_id as usize] {
            MetaSDFNodeOutput::SingleTransform(transform) => {
                let mut transforms = AVec::new_in(arena);
                if let Some(transform) = transform {
                    transforms.push(*transform);
                }
                Cow::Owned(transforms)
            }
            MetaSDFNodeOutput::TransformGroup(transforms) => Cow::Borrowed(transforms),
            child_output => {
                bail!(
                    "Scattering node expects SingleTransform or TransformGroup as input 2, got {}",
                    child_output.label(),
                );
            }
        };

        let mut apply_transform = |sdf_node_id: MetaSDFNodeID,
                                   transform: &Similarity3<f32>|
         -> MetaSDFNodeID {
            let scaling = transform.scaling();
            let rotation = transform.isometry.rotation;
            let translation = transform.isometry.translation.vector;

            let mut output_node_id = sdf_node_id;

            if abs_diff_ne!(scaling, 1.0) {
                output_node_id = graph.add_node(SDFNode::new_scaling(output_node_id, scaling));
            }
            if abs_diff_ne!(&rotation, &UnitQuaternion::identity()) {
                output_node_id = graph.add_node(SDFNode::new_rotation(output_node_id, rotation));
            }
            if abs_diff_ne!(&translation, &Vector3::zeros()) {
                output_node_id =
                    graph.add_node(SDFNode::new_translation(output_node_id, translation));
            }

            output_node_id
        };

        // There could be up to three SDF transform nodes (translation, rotation
        // and scaling) per (SDF, transform) pair, but most likely it is only
        // one SDF transform node
        let mut output_node_ids =
            AVec::with_capacity_in(sdf_node_ids.len() * transforms.len(), arena);

        for &sdf_node_id in sdf_node_ids.as_ref() {
            for transform in transforms.as_ref() {
                output_node_ids.push(apply_transform(sdf_node_id, transform));
            }
        }

        Ok(MetaSDFNodeOutput::SDFGroup(output_node_ids))
    }
}

impl MetaStochasticSelection {
    fn resolve<A>(
        &self,
        arena: A,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> MetaSDFNodeOutput<A>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_param_rng(seed);

        let pick_count = self.min_pick_count..=self.max_pick_count.max(self.min_pick_count);
        let pick_probability = self.pick_probability.clamp(0.0, 1.0);

        let mut single_is_selected =
            || *pick_count.start() > 0 && rng.random_range(0.0..1.0) < pick_probability;

        match &outputs[self.child_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => MetaSDFNodeOutput::SingleSDF(None),
            MetaSDFNodeOutput::SingleTransform(None) => MetaSDFNodeOutput::SingleTransform(None),
            MetaSDFNodeOutput::SingleSDF(Some(input_node_id)) => {
                let output_node_id = single_is_selected().then_some(*input_node_id);
                MetaSDFNodeOutput::SingleSDF(output_node_id)
            }
            MetaSDFNodeOutput::SingleTransform(Some(input_transform)) => {
                let output_transform = single_is_selected().then_some(*input_transform);
                MetaSDFNodeOutput::SingleTransform(output_transform)
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
            MetaSDFNodeOutput::TransformGroup(input_transforms) => {
                let mut output_transforms = AVec::with_capacity_in(input_transforms.len(), arena);
                let count = rng.random_range(pick_count.clone());
                for input_transform in input_transforms.choose_multiple(&mut rng, count as usize) {
                    if rng.random_range(0.0..1.0) < pick_probability {
                        output_transforms.push(*input_transform);
                    }
                }
                MetaSDFNodeOutput::TransformGroup(output_transforms)
            }
        }
    }
}

impl CompositionMode {
    pub fn try_from_str(variant: &str) -> Result<Self> {
        match variant {
            "Pre" => Ok(Self::Pre),
            "Post" => Ok(Self::Post),
            invalid => Err(anyhow!("Invalid CompositionMode variant: {invalid}")),
        }
    }
}

impl SphereSurfaceRotation {
    pub fn try_from_str(variant: &str) -> Result<Self> {
        match variant {
            "Identity" => Ok(Self::Identity),
            "Radial" => Ok(Self::Radial),
            invalid => Err(anyhow!("Invalid SphereSurfaceRotation variant: {invalid}")),
        }
    }
}

fn resolve_unary_sdf_op<A: Allocator>(
    arena: A,
    graph: &mut SDFGraph<A>,
    name: &str,
    seed: u64,
    child_output: &MetaSDFNodeOutput<A>,
    mut create_atomic_node: impl FnMut(&mut ParamRng, SDFNodeID) -> Result<SDFNode>,
) -> Result<MetaSDFNodeOutput<A>> {
    match child_output {
        MetaSDFNodeOutput::SingleSDF(None) => Ok(MetaSDFNodeOutput::SingleSDF(None)),
        MetaSDFNodeOutput::SingleSDF(Some(input_node_id)) => {
            let mut rng = create_param_rng(seed);
            let output_node_id = graph.add_node(create_atomic_node(&mut rng, *input_node_id)?);
            Ok(MetaSDFNodeOutput::SingleSDF(Some(output_node_id)))
        }
        MetaSDFNodeOutput::SDFGroup(input_node_ids) => {
            let output_node_ids =
                unary_sdf_group_op(arena, graph, seed, input_node_ids, |rng, input_node_id| {
                    create_atomic_node(rng, input_node_id)
                })?;
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

fn unary_sdf_group_op<A: Allocator>(
    arena: A,
    graph: &mut SDFGraph<A>,
    seed: u64,
    input_node_ids: &[SDFNodeID],
    mut create_output_node: impl FnMut(&mut ParamRng, SDFNodeID) -> Result<SDFNode>,
) -> Result<AVec<SDFNodeID, A>> {
    let mut rng = create_param_rng(seed);
    let mut output_node_ids = AVec::with_capacity_in(input_node_ids.len(), arena);
    for input_node_id in input_node_ids {
        output_node_ids.push(graph.add_node(create_output_node(&mut rng, *input_node_id)?));
    }
    Ok(output_node_ids)
}

fn resolve_unary_transform_op<A: Allocator>(
    arena: A,
    name: &str,
    seed: u64,
    child_output: &MetaSDFNodeOutput<A>,
    mut create_transform: impl FnMut(&mut ParamRng, &Similarity3<f32>) -> Result<Similarity3<f32>>,
) -> Result<MetaSDFNodeOutput<A>> {
    match child_output {
        MetaSDFNodeOutput::SingleTransform(None) => Ok(MetaSDFNodeOutput::SingleTransform(None)),
        MetaSDFNodeOutput::SingleTransform(Some(input_transform)) => {
            let mut rng = create_param_rng(seed);
            let output_transform = create_transform(&mut rng, input_transform)?;
            Ok(MetaSDFNodeOutput::SingleTransform(Some(output_transform)))
        }
        MetaSDFNodeOutput::TransformGroup(input_transforms) => {
            let output_transforms =
                unary_transform_group_op(arena, seed, input_transforms, |rng, input_transform| {
                    create_transform(rng, input_transform)
                })?;
            Ok(MetaSDFNodeOutput::TransformGroup(output_transforms))
        }
        child_output => {
            bail!(
                "{name} node expects SingleTransform or TransformGroup input, got {}",
                child_output.label()
            );
        }
    }
}

fn unary_transform_group_op<A: Allocator>(
    arena: A,
    seed: u64,
    input_transforms: &[Similarity3<f32>],
    mut create_transform: impl FnMut(&mut ParamRng, &Similarity3<f32>) -> Result<Similarity3<f32>>,
) -> Result<AVec<Similarity3<f32>, A>> {
    let mut rng = create_param_rng(seed);
    let mut output_transforms = AVec::with_capacity_in(input_transforms.len(), arena);
    for input_transform in input_transforms {
        output_transforms.push(create_transform(&mut rng, input_transform)?);
    }
    Ok(output_transforms)
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
    surface_sdf_node_to_parent_transform: &Similarity3<f32>,
    subject_node_to_parent_transform: &Similarity3<f32>,
    max_iterations: u32,
    max_distance_from_surface: f32,
) -> Option<Vector3<f32>> {
    const SAMPLE_BLOCK_SIZE: usize = 2;

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
        let block_origin =
            sampling_position - Vector3::repeat(0.5 * (SAMPLE_BLOCK_SIZE - 1) as f32);

        generator.compute_signed_distances_for_block_preserving_gradients::<SAMPLE_BLOCK_SIZE, _>(
            buffers,
            &block_origin,
        );

        let sampled_signed_distances = buffers.final_signed_distances();

        let signed_distance = compute_center_value_of_2x2x2_samples(sampled_signed_distances);
        let gradient = compute_gradient_from_2x2x2_samples(sampled_signed_distances);

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

fn compute_translation_to_ray_intersection_point_on_surface<A: Allocator>(
    generator: &SDFGenerator<A>,
    buffers: &mut SDFGeneratorBlockBuffers<1, A>,
    surface_sdf_node_to_parent_transform: &Similarity3<f32>,
    subject_node_to_parent_transform: &Similarity3<f32>,
    max_steps: u32,
    max_distance_from_surface: f32,
    safety_factor: f32,
) -> Option<Vector3<f32>> {
    assert!(safety_factor > 0.0 && safety_factor <= 1.0);

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

    // We also need the ray direction, which is the y-axis in the subject node's
    // space, in the parent space
    let subject_y_axis_in_parent_space =
        subject_node_to_parent_transform.transform_vector(&Vector3::y());

    // We can now transform the position and direction from the common parent
    // space to the space of the surface node
    let subject_center_in_surface_sdf_space = surface_sdf_node_to_parent_transform
        .inverse_transform_point(&subject_center_in_parent_space);

    let subject_y_axis_in_surface_sdf_space = surface_sdf_node_to_parent_transform
        .inverse_transform_vector(&subject_y_axis_in_parent_space);

    let ray_origin = subject_center_in_surface_sdf_space;
    let ray_direction = UnitVector3::try_new(subject_y_axis_in_surface_sdf_space, 1e-8)?;

    let domain_in_surface_sdf_space = generator.domain();

    let (start_distance_along_ray, max_distance_along_ray) =
        domain_in_surface_sdf_space.find_ray_intersection(ray_origin, ray_direction)?;

    let mut distance_along_ray = start_distance_along_ray;
    let mut sampling_position = ray_origin + ray_direction.scale(distance_along_ray);

    let mut signed_distance = generator.compute_signed_distance(buffers, &sampling_position);
    let mut distance = signed_distance.abs();

    let starting_sign = signed_distance.signum();

    let mut step_count = 0;
    let mut crossed_surface = false;

    while distance > max_distance_from_surface {
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

        // If the SDF was exact, the surface couldn't possibly be closer than
        // `distance`, so we could safely step that far without overshooting.
        // However, since the distances may be inaccurate due to things like
        // smoothing or perturbing with noise, we shorten the distance by a
        // safety factor. To handle overshoot, we also multiply with the sign of
        // the SDF at the starting position. This will cause us to step back
        // along the ray if we overshoot, regardless of whether we started
        // inside or outside of the surface.
        distance_along_ray += starting_sign * signed_distance * safety_factor;

        if !crossed_surface
            && signed_distance.is_sign_positive() != starting_sign.is_sign_positive()
        {
            crossed_surface = true;
        }

        // We have exited the SDF domain, so the ray didn't hit the surface
        if distance_along_ray > max_distance_along_ray
            || distance_along_ray < start_distance_along_ray
        {
            return None;
        }

        sampling_position = ray_origin + ray_direction.scale(distance_along_ray);

        signed_distance = generator.compute_signed_distance(buffers, &sampling_position);
        distance = signed_distance.abs();
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
    gradient_sdf_node_to_parent_transform: &Similarity3<f32>,
    subject_node_to_parent_transform: &Similarity3<f32>,
) -> Option<UnitQuaternion<f32>> {
    const SAMPLE_BLOCK_SIZE: usize = 2;

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

    // We will sample a block of 2x2x2 signed distance values centered at
    // the sampling position. The samples are one voxel apart, so the lower
    // samples will be offset by 0.5 voxels down from the sampling position.
    let block_origin = subject_center_in_gradient_sdf_space
        - Vector3::repeat(0.5 * (SAMPLE_BLOCK_SIZE - 1) as f32);

    generator.compute_signed_distances_for_block_preserving_gradients::<SAMPLE_BLOCK_SIZE, _>(
        buffers,
        &block_origin,
    );

    let gradient = compute_gradient_from_2x2x2_samples(buffers.final_signed_distances());

    // The rotation will be from the subject's y-axis to the gradient. When
    // applying the rotation to the subject node we need the rotation to be in
    // the parent space, so we transform both the y-axis and gradient to that
    // space.

    let subject_y_axis_in_parent_space =
        subject_node_to_parent_transform.transform_vector(&Vector3::y());

    let gradient_in_parent_space =
        gradient_sdf_node_to_parent_transform.transform_vector(&gradient);

    // If the source (y-axis) or destination (gradient) vector has length zero,
    // we can't determine the rotation, so we abort
    let y_axis = UnitVector3::try_new(subject_y_axis_in_parent_space, 1e-8)?;
    let gradient_direction = UnitVector3::try_new(gradient_in_parent_space, 1e-8)?;

    Some(rotation_between_axes(&y_axis, &gradient_direction))
}

/// Takes 2x2x2 signed distances (column-major order) sampled one voxel width
/// apart and estimates the value at the center of the sampled block by taking
/// their average.
fn compute_center_value_of_2x2x2_samples(signed_distances: &[f32; 8]) -> f32 {
    signed_distances.iter().sum::<f32>() * 0.125
}

/// Takes 2x2x2 signed distances (column-major order) sampled one voxel width
/// apart and estimates the gradient at the center of the sampled block by
/// calculating the analytic gradient of the trilinear interpolation of the
/// samples at the center.
fn compute_gradient_from_2x2x2_samples(signed_distances: &[f32; 8]) -> Vector3<f32> {
    let &[d000, d001, d010, d011, d100, d101, d110, d111] = signed_distances;
    vector![
        (d100 + d110 + d101 + d111) - (d000 + d010 + d001 + d011),
        (d010 + d110 + d011 + d111) - (d000 + d100 + d001 + d101),
        (d001 + d101 + d011 + d111) - (d000 + d100 + d010 + d110)
    ]
    .scale(0.25)
}

fn compute_jittered_direction(
    direction: UnitVector3<f32>,
    max_jitter_angle: f32,
    rng: &mut ParamRng,
) -> UnitVector3<f32> {
    assert!(max_jitter_angle >= 0.0);
    if abs_diff_eq!(max_jitter_angle, 0.0) {
        return direction;
    }

    let angle = rng.random_range(0.0..=max_jitter_angle);

    let mut axis = vector![
        rng.random_range(-1.0..=1.0),
        rng.random_range(-1.0..=1.0),
        rng.random_range(-1.0..=1.0)
    ];

    // Retain only the component perpendicular to `direction`
    axis -= direction.scale(axis.dot(&direction));

    let axis = UnitVector3::try_new(axis, 1e-8).unwrap_or_else(|| {
        // `axis` was either zero or parallel to `direction`, so we pick an
        // arbitrary non-parallel axis
        let axis = if direction.z.abs() < 0.9 {
            Vector3::z_axis()
        } else {
            Vector3::x_axis()
        };
        let axis = axis.as_ref() - direction.scale(axis.dot(&direction));
        UnitVector3::new_normalize(axis)
    });

    let rotation = UnitQuaternion::from_axis_angle(&axis, angle);

    rotation * direction
}
