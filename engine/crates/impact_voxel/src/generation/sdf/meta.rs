//! Generation of signed distance fields. This module implements the graph of
//! high-level "meta" SDF nodes that is compiled into the runtime graph of
//! simpler atomic nodes.

use crate::generation::sdf::{
    SDFGenerator, SDFGeneratorBlockBuffers, SDFGraph, SDFNode, SDFNodeID,
};
use allocator_api2::{
    alloc::{Allocator, Global},
    vec::Vec as AVec,
};
use anyhow::{Result, anyhow, bail};
use approx::{abs_diff_eq, abs_diff_ne};
use impact_containers::FixedQueue;
use impact_geometry::rotation_between_axes;
use impact_math::splitmix;
use nalgebra::{
    Point3, Similarity, Similarity3, Translation3, UnitQuaternion, UnitVector3, Vector3, vector,
};
use rand::{
    Rng, SeedableRng,
    distr::{Distribution, Uniform},
    seq::IndexedRandom,
};
use rand_pcg::Pcg64Mcg;
use std::{array, borrow::Cow, ops::RangeInclusive};

#[derive(Clone, Debug)]
pub struct MetaSDFGraph<A: Allocator = Global> {
    seed: u64,
    nodes: AVec<MetaSDFNode, A>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum MetaSDFNode {
    // Primitives
    Box(MetaBoxSDF),
    Sphere(MetaSphereSDF),
    GradientNoise(MetaGradientNoiseSDF),

    // Transforms
    Translation(MetaSDFTranslation),
    Rotation(MetaSDFRotation),
    Scaling(MetaSDFScaling),

    // Modifiers
    MultifractalNoise(MetaMultifractalNoiseSDFModifier),
    MultiscaleSphere(MetaMultiscaleSphereSDFModifier),

    // Combination
    Union(MetaSDFUnion),
    Subtraction(MetaSDFSubtraction),
    Intersection(MetaSDFIntersection),
    GroupUnion(MetaSDFGroupUnion),

    // Placement
    StratifiedPlacement(MetaStratifiedPlacement),
    TranslationToSurface(MetaTranslationToSurface),
    RotationToGradient(MetaRotationToGradient),
    Scattering(MetaSDFScattering),

    // Masking
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
    /// Placements are transforms from the local space of the node being placed
    /// to the space of the node that applies the placement.
    SinglePlacement(Option<Similarity3<f32>>),
    PlacementGroup(AVec<Similarity3<f32>, A>),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug)]
pub struct DiscreteParamRange {
    pub min: u32,
    pub max: u32,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug)]
pub struct ContParamRange {
    pub min: f32,
    pub max: f32,
}

/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaBoxSDF {
    extents: [ContParamRange; 3],
    seed: u32,
}

/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSphereSDF {
    radius: ContParamRange,
    seed: u32,
}

/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaGradientNoiseSDF {
    extents: [ContParamRange; 3],
    noise_frequency: ContParamRange,
    noise_threshold: ContParamRange,
    seed: u32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFTranslation {
    child_id: MetaSDFNodeID,
    translation: [ContParamRange; 3],
    seed: u32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFRotation {
    child_id: MetaSDFNodeID,
    roll: ContParamRange,
    pitch: ContParamRange,
    yaw: ContParamRange,
    seed: u32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFScaling {
    child_id: MetaSDFNodeID,
    scaling: ContParamRange,
    seed: u32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaMultifractalNoiseSDFModifier {
    child_id: MetaSDFNodeID,
    octaves: DiscreteParamRange,
    frequency: ContParamRange,
    lacunarity: ContParamRange,
    persistence: ContParamRange,
    amplitude: ContParamRange,
    seed: u32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaMultiscaleSphereSDFModifier {
    child_id: MetaSDFNodeID,
    octaves: DiscreteParamRange,
    max_scale: ContParamRange,
    persistence: ContParamRange,
    inflation: ContParamRange,
    intersection_smoothness: ContParamRange,
    union_smoothness: ContParamRange,
    seed: u32,
}

/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFUnion {
    child_1_id: MetaSDFNodeID,
    child_2_id: MetaSDFNodeID,
    smoothness: f32,
}

/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFSubtraction {
    child_1_id: MetaSDFNodeID,
    child_2_id: MetaSDFNodeID,
    smoothness: f32,
}

/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFIntersection {
    child_1_id: MetaSDFNodeID,
    child_2_id: MetaSDFNodeID,
    smoothness: f32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: `SingleSDF`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFGroupUnion {
    child_id: MetaSDFNodeID,
    smoothness: f32,
}

/// Output: `PlacementGroup`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaStratifiedPlacement {
    shape: [DiscreteParamRange; 3],
    cell_extents: [ContParamRange; 3],
    points_per_grid_cell: DiscreteParamRange,
    jitter_fraction: ContParamRange,
    seed: u32,
}

/// Input 1: `SingleSDF`
/// Input 2: Any
/// Output: Same as input 2
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaTranslationToSurface {
    surface_sdf_id: MetaSDFNodeID,
    subject_id: MetaSDFNodeID,
}

/// Input 1: `SingleSDF`
/// Input 2: Any
/// Output: Same as input 2
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaRotationToGradient {
    gradient_sdf_id: MetaSDFNodeID,
    subject_id: MetaSDFNodeID,
}

/// Input 1: `SDFGroup` or `SingleSDF`
/// Input 2: `PlacementGroup` or `SinglePlacement`
/// Output: `SDFGroup`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaSDFScattering {
    sdf_id: MetaSDFNodeID,
    placement_id: MetaSDFNodeID,
}

/// Input: Any
/// Output: Same as input
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MetaStochasticSelection {
    child_id: MetaSDFNodeID,
    pick_count: RangeInclusive<u32>,
    pick_probability: f32,
    seed: u32,
}

type NodeRng = Pcg64Mcg;

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
                                MetaSDFNode::Box(_)
                                | MetaSDFNode::Sphere(_)
                                | MetaSDFNode::GradientNoise(_)
                                | MetaSDFNode::StratifiedPlacement(_) => {}
                                MetaSDFNode::Translation(MetaSDFTranslation {
                                    child_id, ..
                                })
                                | MetaSDFNode::Rotation(MetaSDFRotation { child_id, .. })
                                | MetaSDFNode::Scaling(MetaSDFScaling { child_id, .. })
                                | MetaSDFNode::MultifractalNoise(
                                    MetaMultifractalNoiseSDFModifier { child_id, .. },
                                )
                                | MetaSDFNode::MultiscaleSphere(
                                    MetaMultiscaleSphereSDFModifier { child_id, .. },
                                )
                                | MetaSDFNode::GroupUnion(MetaSDFGroupUnion { child_id, .. })
                                | MetaSDFNode::StochasticSelection(MetaStochasticSelection {
                                    child_id,
                                    ..
                                }) => {
                                    operation_stack.push(BuildOperation::VisitChildren(*child_id));
                                }
                                MetaSDFNode::Union(MetaSDFUnion {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | MetaSDFNode::Subtraction(MetaSDFSubtraction {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | MetaSDFNode::Intersection(MetaSDFIntersection {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | MetaSDFNode::Scattering(MetaSDFScattering {
                                    sdf_id: child_1_id,
                                    placement_id: child_2_id,
                                })
                                | MetaSDFNode::TranslationToSurface(MetaTranslationToSurface {
                                    surface_sdf_id: child_1_id,
                                    subject_id: child_2_id,
                                })
                                | MetaSDFNode::RotationToGradient(MetaRotationToGradient {
                                    gradient_sdf_id: child_1_id,
                                    subject_id: child_2_id,
                                }) => {
                                    operation_stack
                                        .push(BuildOperation::VisitChildren(*child_1_id));
                                    operation_stack
                                        .push(BuildOperation::VisitChildren(*child_2_id));
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

                    outputs[node_idx] = node.resolve(arena, &mut graph, &outputs, seed)?;
                }
            }
        }

        if let MetaSDFNodeOutput::SingleSDF(atomic_node_id) = &outputs[root_node_id as usize] {
            if let Some(id) = atomic_node_id {
                assert_eq!(*id, graph.root_node_id());
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
            Self::SinglePlacement(_) => "SinglePlacement",
            Self::PlacementGroup(_) => "PlacementGroup",
        }
    }
}

impl DiscreteParamRange {
    pub fn new(min: u32, max: u32) -> Self {
        assert!(min <= max);
        Self { min, max }
    }

    fn pick_value(&self, rng: &mut NodeRng) -> u32 {
        rng.random_range(self.min..=self.max)
    }
}

impl From<u32> for DiscreteParamRange {
    fn from(value: u32) -> Self {
        Self::new(value, value)
    }
}

impl ContParamRange {
    pub fn new(min: f32, max: f32) -> Self {
        assert!(min <= max);
        Self { min, max }
    }

    fn pick_value(&self, rng: &mut NodeRng) -> f32 {
        rng.random_range(self.min..=self.max)
    }
}

impl From<f32> for ContParamRange {
    fn from(value: f32) -> Self {
        Self::new(value, value)
    }
}

impl MetaSDFNode {
    pub fn new_box(extents: [ContParamRange; 3], seed: u32) -> Self {
        assert!(extents[0].min >= 0.0);
        assert!(extents[1].min >= 0.0);
        assert!(extents[2].min >= 0.0);
        Self::Box(MetaBoxSDF { extents, seed })
    }

    pub fn new_sphere(radius: ContParamRange, seed: u32) -> Self {
        assert!(radius.min >= 0.0);
        Self::Sphere(MetaSphereSDF { radius, seed })
    }

    pub fn new_gradient_noise(
        extents: [ContParamRange; 3],
        noise_frequency: ContParamRange,
        noise_threshold: ContParamRange,
        seed: u32,
    ) -> Self {
        assert!(extents[0].min >= 0.0);
        assert!(extents[1].min >= 0.0);
        assert!(extents[2].min >= 0.0);
        Self::GradientNoise(MetaGradientNoiseSDF {
            extents,
            noise_frequency,
            noise_threshold,
            seed,
        })
    }

    pub fn new_translation(
        child_id: MetaSDFNodeID,
        translation: [ContParamRange; 3],
        seed: u32,
    ) -> Self {
        Self::Translation(MetaSDFTranslation {
            child_id,
            translation,
            seed,
        })
    }

    pub fn new_rotation(
        child_id: MetaSDFNodeID,
        roll: ContParamRange,
        pitch: ContParamRange,
        yaw: ContParamRange,
        seed: u32,
    ) -> Self {
        Self::Rotation(MetaSDFRotation {
            child_id,
            roll,
            pitch,
            yaw,
            seed,
        })
    }

    pub fn new_scaling(child_id: MetaSDFNodeID, scaling: ContParamRange, seed: u32) -> Self {
        assert!(scaling.min > 0.0);
        Self::Scaling(MetaSDFScaling {
            child_id,
            scaling,
            seed,
        })
    }

    pub fn new_multifractal_noise(
        child_id: MetaSDFNodeID,
        octaves: DiscreteParamRange,
        frequency: ContParamRange,
        lacunarity: ContParamRange,
        persistence: ContParamRange,
        amplitude: ContParamRange,
        seed: u32,
    ) -> Self {
        Self::MultifractalNoise(MetaMultifractalNoiseSDFModifier {
            child_id,
            octaves,
            frequency,
            lacunarity,
            persistence,
            amplitude,
            seed,
        })
    }

    pub fn new_multiscale_sphere(
        child_id: MetaSDFNodeID,
        octaves: DiscreteParamRange,
        max_scale: ContParamRange,
        persistence: ContParamRange,
        inflation: ContParamRange,
        intersection_smoothness: ContParamRange,
        union_smoothness: ContParamRange,
        seed: u32,
    ) -> Self {
        Self::MultiscaleSphere(MetaMultiscaleSphereSDFModifier {
            child_id,
            octaves,
            max_scale,
            persistence,
            inflation,
            intersection_smoothness,
            union_smoothness,
            seed,
        })
    }

    pub fn new_union(
        child_1_id: MetaSDFNodeID,
        child_2_id: MetaSDFNodeID,
        smoothness: f32,
    ) -> Self {
        assert!(smoothness >= 0.0);
        Self::Union(MetaSDFUnion {
            child_1_id,
            child_2_id,
            smoothness,
        })
    }

    pub fn new_subtraction(
        child_1_id: MetaSDFNodeID,
        child_2_id: MetaSDFNodeID,
        smoothness: f32,
    ) -> Self {
        assert!(smoothness >= 0.0);
        Self::Subtraction(MetaSDFSubtraction {
            child_1_id,
            child_2_id,
            smoothness,
        })
    }

    pub fn new_intersection(
        child_1_id: MetaSDFNodeID,
        child_2_id: MetaSDFNodeID,
        smoothness: f32,
    ) -> Self {
        assert!(smoothness >= 0.0);
        Self::Intersection(MetaSDFIntersection {
            child_1_id,
            child_2_id,
            smoothness,
        })
    }

    pub fn new_group_union(child_id: MetaSDFNodeID, smoothness: f32) -> Self {
        assert!(smoothness >= 0.0);
        Self::GroupUnion(MetaSDFGroupUnion {
            child_id,
            smoothness,
        })
    }

    pub fn new_stratified_placement(
        shape: [DiscreteParamRange; 3],
        cell_extents: [ContParamRange; 3],
        points_per_grid_cell: DiscreteParamRange,
        jitter_fraction: ContParamRange,
        seed: u32,
    ) -> Self {
        assert!(cell_extents[0].min >= 0.0);
        assert!(cell_extents[1].min >= 0.0);
        assert!(cell_extents[2].min >= 0.0);
        assert!(jitter_fraction.min >= 0.0);
        assert!(jitter_fraction.max <= 1.0);
        Self::StratifiedPlacement(MetaStratifiedPlacement {
            shape,
            cell_extents,
            points_per_grid_cell,
            jitter_fraction,
            seed,
        })
    }

    pub fn new_translation_to_surface(
        surface_sdf_id: MetaSDFNodeID,
        subject_id: MetaSDFNodeID,
    ) -> Self {
        Self::TranslationToSurface(MetaTranslationToSurface {
            surface_sdf_id,
            subject_id,
        })
    }

    pub fn new_rotation_to_gradient(
        gradient_sdf_id: MetaSDFNodeID,
        subject_id: MetaSDFNodeID,
    ) -> Self {
        Self::RotationToGradient(MetaRotationToGradient {
            gradient_sdf_id,
            subject_id,
        })
    }

    pub fn new_scattering(sdf_id: MetaSDFNodeID, placement_id: MetaSDFNodeID) -> Self {
        Self::Scattering(MetaSDFScattering {
            sdf_id,
            placement_id,
        })
    }

    pub fn new_stochastic_selection(
        child_id: MetaSDFNodeID,
        pick_count: RangeInclusive<u32>,
        pick_probability: f32,
        seed: u32,
    ) -> Self {
        assert!(pick_probability >= 0.0);
        assert!(pick_probability <= 1.0);
        Self::StochasticSelection(MetaStochasticSelection {
            child_id,
            pick_count,
            pick_probability,
            seed,
        })
    }

    /// Combines a node type tag, node seed parameter (for applicable nodes) and
    /// the stable seeds of the child nodes to obtain a stable seed that will
    /// only change due to changes in the seeding, types or topology of the
    /// node's subtree.
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
            Self::Box(MetaBoxSDF { seed, .. }) => combine_seeded_leaf(0x01, seed),
            Self::Sphere(MetaSphereSDF { seed, .. }) => combine_seeded_leaf(0x02, seed),
            Self::GradientNoise(MetaGradientNoiseSDF { seed, .. }) => {
                combine_seeded_leaf(0x03, seed)
            }
            Self::Translation(MetaSDFTranslation { seed, child_id, .. }) => {
                combine_seeded_unary(0x10, seed, child_id)
            }
            Self::Rotation(MetaSDFRotation { seed, child_id, .. }) => {
                combine_seeded_unary(0x11, seed, child_id)
            }
            Self::Scaling(MetaSDFScaling { seed, child_id, .. }) => {
                combine_seeded_unary(0x12, seed, child_id)
            }
            Self::MultifractalNoise(MetaMultifractalNoiseSDFModifier {
                seed, child_id, ..
            }) => combine_seeded_unary(0x20, seed, child_id),
            Self::MultiscaleSphere(MetaMultiscaleSphereSDFModifier { seed, child_id, .. }) => {
                combine_seeded_unary(0x21, seed, child_id)
            }
            Self::Union(MetaSDFUnion {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary_commutative(0x30, child_1_id, child_2_id),
            Self::Subtraction(MetaSDFSubtraction {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary(0x31, child_1_id, child_2_id),
            Self::Intersection(MetaSDFIntersection {
                child_1_id,
                child_2_id,
                ..
            }) => combine_binary_commutative(0x32, child_1_id, child_2_id),
            Self::GroupUnion(MetaSDFGroupUnion { child_id, .. }) => combine_unary(0x33, child_id),
            Self::StratifiedPlacement(MetaStratifiedPlacement { seed, .. }) => {
                combine_seeded_leaf(0x40, seed)
            }
            Self::TranslationToSurface(MetaTranslationToSurface {
                surface_sdf_id,
                subject_id,
            }) => combine_binary(0x50, surface_sdf_id, subject_id),
            Self::RotationToGradient(MetaRotationToGradient {
                gradient_sdf_id,
                subject_id,
            }) => combine_binary(0x51, gradient_sdf_id, subject_id),
            Self::Scattering(MetaSDFScattering {
                sdf_id,
                placement_id,
            }) => combine_binary(0x52, sdf_id, placement_id),
            Self::StochasticSelection(MetaStochasticSelection { seed, child_id, .. }) => {
                combine_seeded_unary(0x60, seed, child_id)
            }
        }
    }

    fn resolve<A>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        match self {
            Self::Box(node) => Ok(node.resolve(graph, seed)),
            Self::Sphere(node) => Ok(node.resolve(graph, seed)),
            Self::GradientNoise(node) => Ok(node.resolve(graph, seed)),
            Self::Translation(node) => node.resolve(arena, graph, outputs, seed),
            Self::Rotation(node) => node.resolve(arena, graph, outputs, seed),
            Self::Scaling(node) => node.resolve(arena, graph, outputs, seed),
            Self::MultifractalNoise(node) => node.resolve(arena, graph, outputs, seed),
            Self::MultiscaleSphere(node) => node.resolve(arena, graph, outputs, seed),
            Self::Union(node) => node.resolve(graph, outputs),
            Self::Subtraction(node) => node.resolve(graph, outputs),
            Self::Intersection(node) => node.resolve(graph, outputs),
            Self::GroupUnion(node) => node.resolve(arena, graph, outputs),
            Self::StratifiedPlacement(node) => Ok(node.resolve(arena, seed)),
            Self::TranslationToSurface(node) => node.resolve(arena, graph, outputs),
            Self::RotationToGradient(node) => node.resolve(arena, graph, outputs),
            Self::Scattering(node) => node.resolve(arena, graph, outputs),
            Self::StochasticSelection(node) => Ok(node.resolve(arena, graph, outputs, seed)),
        }
    }
}

impl MetaBoxSDF {
    fn resolve<A: Allocator>(&self, graph: &mut SDFGraph<A>, seed: u64) -> MetaSDFNodeOutput<A> {
        let mut rng = create_rng(seed);
        let extents = self.extents.map(|range| range.pick_value(&mut rng));
        let node_id = graph.add_node(SDFNode::new_box(extents));
        MetaSDFNodeOutput::SingleSDF(Some(node_id))
    }
}

impl MetaSphereSDF {
    fn resolve<A: Allocator>(&self, graph: &mut SDFGraph<A>, seed: u64) -> MetaSDFNodeOutput<A> {
        let mut rng = create_rng(seed);
        let radius = self.radius.pick_value(&mut rng);
        let node_id = graph.add_node(SDFNode::new_sphere(radius));
        MetaSDFNodeOutput::SingleSDF(Some(node_id))
    }
}

impl MetaGradientNoiseSDF {
    fn resolve<A: Allocator>(&self, graph: &mut SDFGraph<A>, seed: u64) -> MetaSDFNodeOutput<A> {
        let mut rng = create_rng(seed);
        let extents = self.extents.map(|range| range.pick_value(&mut rng));
        let noise_frequency = self.noise_frequency.pick_value(&mut rng);
        let noise_threshold = self.noise_threshold.pick_value(&mut rng);
        let seed = rng.random();
        let node_id = graph.add_node(SDFNode::new_gradient_noise(
            extents,
            noise_frequency,
            noise_threshold,
            seed,
        ));
        MetaSDFNodeOutput::SingleSDF(Some(node_id))
    }
}

impl MetaSDFTranslation {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            graph,
            "Translation",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let translation = self.translation.map(|range| range.pick_value(rng));
                SDFNode::new_translation(input_node_id, translation.into())
            },
        )
    }
}

impl MetaSDFRotation {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            graph,
            "Rotation",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let roll = self.roll.pick_value(rng);
                let pitch = self.pitch.pick_value(rng);
                let yaw = self.yaw.pick_value(rng);
                SDFNode::new_rotation(
                    input_node_id,
                    UnitQuaternion::from_euler_angles(roll, pitch, yaw),
                )
            },
        )
    }
}

impl MetaSDFScaling {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            graph,
            "Scaling",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let scaling = self.scaling.pick_value(rng);
                SDFNode::new_scaling(input_node_id, scaling)
            },
        )
    }
}

impl MetaMultifractalNoiseSDFModifier {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            graph,
            "MultifractalNoise",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let octaves = self.octaves.pick_value(rng);
                let frequency = self.frequency.pick_value(rng);
                let lacunarity = self.lacunarity.pick_value(rng);
                let persistence = self.persistence.pick_value(rng);
                let amplitude = self.amplitude.pick_value(rng);
                let seed = rng.random();
                SDFNode::new_multifractal_noise(
                    input_node_id,
                    octaves,
                    frequency,
                    lacunarity,
                    persistence,
                    amplitude,
                    seed,
                )
            },
        )
    }
}

impl MetaMultiscaleSphereSDFModifier {
    fn resolve<A: Allocator>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            graph,
            "MultiscaleSphere",
            seed,
            &outputs[self.child_id as usize],
            |rng, input_node_id| {
                let octaves = self.octaves.pick_value(rng);
                let max_scale = self.max_scale.pick_value(rng);
                let persistence = self.persistence.pick_value(rng);
                let inflation = self.inflation.pick_value(rng);
                let intersection_smoothness = self.intersection_smoothness.pick_value(rng);
                let union_smoothness = self.union_smoothness.pick_value(rng);
                let seed = rng.random();
                SDFNode::new_multiscale_sphere(
                    input_node_id,
                    octaves,
                    max_scale,
                    persistence,
                    inflation,
                    intersection_smoothness,
                    union_smoothness,
                    seed,
                )
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
                    "Union node expects two SingleSDF inputs, got {} and {}",
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
                    self.smoothness,
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
                    "Subtraction node expects two SingleSDF inputs, got {} and {}",
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
                    self.smoothness,
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
                    "Intersection node expects two SingleSDF inputs, got {} and {}",
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
                    self.smoothness,
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
                            self.smoothness,
                        ))
                    },
                );
                Ok(MetaSDFNodeOutput::SingleSDF(output_node_id))
            }
            child_output => {
                bail!(
                    "GroupUnion node expects SDFGroup or SingleSDF input, got {}",
                    child_output.label()
                );
            }
        }
    }
}

impl MetaStratifiedPlacement {
    fn resolve<A: Allocator>(&self, arena: A, seed: u64) -> MetaSDFNodeOutput<A> {
        let mut rng = create_rng(seed);
        let shape = self.shape.map(|range| range.pick_value(&mut rng));
        let cell_extents = self.cell_extents.map(|range| range.pick_value(&mut rng));
        let points_per_grid_cell = self.points_per_grid_cell.pick_value(&mut rng);
        let jitter_fraction = self.jitter_fraction.pick_value(&mut rng);

        let grid_cell_count = (shape[0] as usize) * (shape[1] as usize) * (shape[2] as usize);
        let point_count = grid_cell_count * points_per_grid_cell as usize;

        if point_count == 0 {
            return MetaSDFNodeOutput::PlacementGroup(AVec::new_in(arena));
        }

        // Center of the lower corner cell
        let start_pos: [_; 3] = array::from_fn(|i| {
            let cell_extent = cell_extents[i];
            let grid_extent = shape[i] as f32 * cell_extent;
            -0.5 * grid_extent + 0.5 * cell_extent
        });

        let mut placements = AVec::with_capacity_in(point_count, arena);

        let uniform_distr = Uniform::new(-0.5, 0.5).unwrap();

        for i in 0..shape[0] {
            let x = start_pos[0] + i as f32 * cell_extents[0];
            for j in 0..shape[1] {
                let y = start_pos[1] + j as f32 * cell_extents[1];
                for k in 0..shape[2] {
                    let z = start_pos[2] + k as f32 * cell_extents[2];

                    for _ in 0..points_per_grid_cell {
                        let jx = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[0];
                        let jy = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[1];
                        let jz = uniform_distr.sample(&mut rng) * jitter_fraction * cell_extents[2];

                        placements.push(Similarity::from_parts(
                            Translation3::new(x + jx, y + jy, z + jz),
                            UnitQuaternion::identity(),
                            1.0,
                        ));
                    }
                }
            }
        }

        MetaSDFNodeOutput::PlacementGroup(placements)
    }
}

impl MetaTranslationToSurface {
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
                    "TranslationToSurface node expects SingleSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        if let MetaSDFNodeOutput::SingleSDF(None) | MetaSDFNodeOutput::SinglePlacement(None) =
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
                compute_translation_to_surface(
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
            MetaSDFNodeOutput::SinglePlacement(subject_placement) => {
                let subject_node_to_parent_transform = subject_placement.as_ref().unwrap();

                let Some(translation_to_surface_in_parent_space) =
                    compute_translation_to_surface(subject_node_to_parent_transform)
                else {
                    return Ok(MetaSDFNodeOutput::SinglePlacement(None));
                };

                let translated_subject_placement =
                    Translation3::from(translation_to_surface_in_parent_space)
                        * subject_node_to_parent_transform;

                Ok(MetaSDFNodeOutput::SinglePlacement(Some(
                    translated_subject_placement,
                )))
            }
            MetaSDFNodeOutput::PlacementGroup(subject_placements) => {
                let mut translated_subject_placements =
                    AVec::with_capacity_in(subject_placements.len(), arena);

                for subject_node_to_parent_transform in subject_placements {
                    let Some(translation_to_surface_in_parent_space) =
                        compute_translation_to_surface(subject_node_to_parent_transform)
                    else {
                        continue;
                    };

                    let translated_subject_placement =
                        Translation3::from(translation_to_surface_in_parent_space)
                            * subject_node_to_parent_transform;

                    translated_subject_placements.push(translated_subject_placement);
                }

                Ok(MetaSDFNodeOutput::PlacementGroup(
                    translated_subject_placements,
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

        if let MetaSDFNodeOutput::SingleSDF(None) | MetaSDFNodeOutput::SinglePlacement(None) =
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
            MetaSDFNodeOutput::SinglePlacement(subject_placement) => {
                let subject_node_to_parent_transform = subject_placement.as_ref().unwrap();

                let Some(rotation_to_gradient_in_parent_space) =
                    compute_rotation_to_gradient(subject_node_to_parent_transform)
                else {
                    return Ok(MetaSDFNodeOutput::SinglePlacement(None));
                };

                let rotated_subject_placement =
                    rotation_to_gradient_in_parent_space * subject_node_to_parent_transform;

                Ok(MetaSDFNodeOutput::SinglePlacement(Some(
                    rotated_subject_placement,
                )))
            }
            MetaSDFNodeOutput::PlacementGroup(subject_placements) => {
                let mut rotated_subject_placements =
                    AVec::with_capacity_in(subject_placements.len(), arena);

                for subject_node_to_parent_transform in subject_placements {
                    let Some(rotation_to_gradient_in_parent_space) =
                        compute_rotation_to_gradient(subject_node_to_parent_transform)
                    else {
                        continue;
                    };

                    let rotated_subject_placement =
                        rotation_to_gradient_in_parent_space * subject_node_to_parent_transform;

                    rotated_subject_placements.push(rotated_subject_placement);
                }

                Ok(MetaSDFNodeOutput::PlacementGroup(
                    rotated_subject_placements,
                ))
            }
        }
    }
}

impl MetaSDFScattering {
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

        let placements = match &outputs[self.placement_id as usize] {
            MetaSDFNodeOutput::SinglePlacement(placement) => {
                let mut placements = AVec::new_in(arena);
                if let Some(placement) = placement {
                    placements.push(*placement);
                }
                Cow::Owned(placements)
            }
            MetaSDFNodeOutput::PlacementGroup(placements) => Cow::Borrowed(placements),
            child_output => {
                bail!(
                    "Scattering node expects SinglePlacement or GroupPlacement as input 2, got {}",
                    child_output.label(),
                );
            }
        };

        let mut apply_placement = |sdf_node_id: MetaSDFNodeID,
                                   placement: &Similarity3<f32>|
         -> MetaSDFNodeID {
            let scaling = placement.scaling();
            let rotation = placement.isometry.rotation;
            let translation = placement.isometry.translation.vector;

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

        // There could be up to three transform nodes per (SDF, placement) pair,
        // but most likely it is only one transform
        let mut output_node_ids =
            AVec::with_capacity_in(sdf_node_ids.len() * placements.len(), arena);

        for &sdf_node_id in sdf_node_ids.as_ref() {
            for placement in placements.as_ref() {
                output_node_ids.push(apply_placement(sdf_node_id, placement));
            }
        }

        Ok(MetaSDFNodeOutput::SDFGroup(output_node_ids))
    }
}

impl MetaStochasticSelection {
    fn resolve<A>(
        &self,
        arena: A,
        graph: &mut SDFGraph<A>,
        outputs: &[MetaSDFNodeOutput<A>],
        seed: u64,
    ) -> MetaSDFNodeOutput<A>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_rng(seed);

        let mut single_is_selected =
            || *self.pick_count.start() > 0 && rng.random_range(0.0..1.0) < self.pick_probability;

        match &outputs[self.child_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => MetaSDFNodeOutput::SingleSDF(None),
            MetaSDFNodeOutput::SinglePlacement(None) => MetaSDFNodeOutput::SinglePlacement(None),
            MetaSDFNodeOutput::SingleSDF(Some(input_node_id)) => {
                let output_node_id = single_is_selected().then_some(*input_node_id);
                MetaSDFNodeOutput::SingleSDF(output_node_id)
            }
            MetaSDFNodeOutput::SinglePlacement(Some(input_placement)) => {
                let output_placement = single_is_selected().then_some(*input_placement);
                MetaSDFNodeOutput::SinglePlacement(output_placement)
            }
            MetaSDFNodeOutput::SDFGroup(input_node_ids) => {
                let mut output_node_ids = AVec::with_capacity_in(input_node_ids.len(), arena);
                let count = rng.random_range(self.pick_count.clone());
                for &input_node_id in input_node_ids.choose_multiple(&mut rng, count as usize) {
                    if rng.random_range(0.0..1.0) < self.pick_probability {
                        output_node_ids.push(input_node_id);
                        // The current root node will be the last of the input
                        // node IDs. That might not be included in the
                        // selection, so we set the last of the actually
                        // selected nodes as root instead.
                        graph.set_root_node(input_node_id);
                    }
                }
                MetaSDFNodeOutput::SDFGroup(output_node_ids)
            }
            MetaSDFNodeOutput::PlacementGroup(input_placements) => {
                let mut output_placements = AVec::with_capacity_in(input_placements.len(), arena);
                let count = rng.random_range(self.pick_count.clone());
                for input_placement in input_placements.choose_multiple(&mut rng, count as usize) {
                    if rng.random_range(0.0..1.0) < self.pick_probability {
                        output_placements.push(*input_placement);
                    }
                }
                MetaSDFNodeOutput::PlacementGroup(output_placements)
            }
        }
    }
}

fn create_rng(seed: u64) -> NodeRng {
    NodeRng::seed_from_u64(seed)
}

fn resolve_unary_sdf_op<A: Allocator>(
    arena: A,
    graph: &mut SDFGraph<A>,
    name: &str,
    seed: u64,
    child_output: &MetaSDFNodeOutput<A>,
    create_atomic_node: impl Fn(&mut NodeRng, SDFNodeID) -> SDFNode,
) -> Result<MetaSDFNodeOutput<A>> {
    match child_output {
        MetaSDFNodeOutput::SingleSDF(None) => Ok(MetaSDFNodeOutput::SingleSDF(None)),
        MetaSDFNodeOutput::SingleSDF(Some(input_node_id)) => {
            let mut rng = create_rng(seed);
            let output_node_id = graph.add_node(create_atomic_node(&mut rng, *input_node_id));
            Ok(MetaSDFNodeOutput::SingleSDF(Some(output_node_id)))
        }
        MetaSDFNodeOutput::SDFGroup(input_node_ids) => {
            let output_node_ids =
                unary_sdf_group_op(arena, graph, seed, input_node_ids, |rng, input_node_id| {
                    create_atomic_node(rng, input_node_id)
                });
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
    create_output_node: impl Fn(&mut NodeRng, SDFNodeID) -> SDFNode,
) -> AVec<SDFNodeID, A> {
    let mut rng = create_rng(seed);
    let mut output_node_ids = AVec::with_capacity_in(input_node_ids.len(), arena);
    for input_node_id in input_node_ids {
        output_node_ids.push(graph.add_node(create_output_node(&mut rng, *input_node_id)));
    }
    output_node_ids
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

fn compute_translation_to_surface<A: Allocator>(
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
    // the space of the surface node, since this is where we will sample the
    // SDF. The center of the subject node's domain in its own space is the
    // origin, and we start by transforming that to the (common) parent space.
    let subject_center_in_parent_space =
        subject_node_to_parent_transform.transform_point(&Point3::origin());

    // We can now transform it from the common parent space to the space of the
    // surface node
    let subject_center_in_surface_sdf_space = surface_sdf_node_to_parent_transform
        .inverse_transform_point(&subject_center_in_parent_space);

    // To find the surface (where the signed distance is zero), we use the
    // Newton-Raphson method

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

        if signed_distance.abs() < max_distance_from_surface {
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
