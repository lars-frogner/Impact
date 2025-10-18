//! Generation of signed distance fields. This module implements the graph of
//! high-level "meta" SDF nodes that is compiled into the runtime graph of
//! simpler atomic nodes.

use crate::generation::sdf::{SDFGenerator, SDFGeneratorBuilder, SDFNode, SDFNodeID};
use allocator_api2::{
    alloc::{Allocator, Global},
    vec::Vec as AVec,
};
use anyhow::{Result, anyhow, bail};
use approx::abs_diff_ne;
use impact_containers::FixedQueue;
use nalgebra::{Similarity, Similarity3, Translation3, UnitQuaternion, Vector3};
use rand::{
    Rng, SeedableRng,
    distr::{Distribution, Uniform},
};
use rand_pcg::Pcg64Mcg;
use std::{array, borrow::Cow};

#[derive(Clone, Debug)]
pub struct MetaSDFGraph<A: Allocator = Global> {
    nodes: AVec<MetaSDFNode, A>,
}

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

    // Combination
    Union(MetaSDFUnion),
    Subtraction(MetaSDFSubtraction),
    Intersection(MetaSDFIntersection),
    GroupUnion(MetaSDFGroupUnion),

    // Placement
    StratifiedPlacement(MetaStratifiedPlacement),
    TranslationToSurface(MetaTranslationToSurface),
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

#[derive(Clone, Debug)]
enum MetaSDFNodeOutput<A: Allocator> {
    SingleSDF(Option<SDFNodeID>),
    SDFGroup(AVec<SDFNodeID, A>),
    SinglePlacement(Option<Similarity3<f32>>),
    PlacementGroup(AVec<Similarity3<f32>, A>),
}

#[derive(Clone, Copy, Debug)]
pub struct DiscreteParamRange {
    min: u32,
    max: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ContParamRange {
    min: f32,
    max: f32,
}

/// Output: `SingleSDF`
#[derive(Clone, Debug)]
pub struct MetaBoxSDF {
    extents: [ContParamRange; 3],
    seed: u32,
}

/// Output: `SingleSDF`
#[derive(Clone, Debug)]
pub struct MetaSphereSDF {
    radius: ContParamRange,
    seed: u32,
}

/// Output: `SingleSDF`
#[derive(Clone, Debug)]
pub struct MetaGradientNoiseSDF {
    extents: [ContParamRange; 3],
    noise_frequency: ContParamRange,
    noise_threshold: ContParamRange,
    seed: u32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
#[derive(Clone, Debug)]
pub struct MetaSDFTranslation {
    child_id: MetaSDFNodeID,
    translation: [ContParamRange; 3],
    seed: u32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
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
#[derive(Clone, Debug)]
pub struct MetaSDFScaling {
    child_id: MetaSDFNodeID,
    scaling: ContParamRange,
    seed: u32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: Same as input
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

/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[derive(Clone, Debug)]
pub struct MetaSDFUnion {
    child_1_id: MetaSDFNodeID,
    child_2_id: MetaSDFNodeID,
    smoothness: f32,
}

/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[derive(Clone, Debug)]
pub struct MetaSDFSubtraction {
    child_1_id: MetaSDFNodeID,
    child_2_id: MetaSDFNodeID,
    smoothness: f32,
}

/// Input 1: `SingleSDF`
/// Input 2: `SingleSDF`
/// Output: `SingleSDF`
#[derive(Clone, Debug)]
pub struct MetaSDFIntersection {
    child_1_id: MetaSDFNodeID,
    child_2_id: MetaSDFNodeID,
    smoothness: f32,
}

/// Input: `SDFGroup` or `SingleSDF`
/// Output: `SingleSDF`
#[derive(Clone, Debug)]
pub struct MetaSDFGroupUnion {
    child_id: MetaSDFNodeID,
    smoothness: f32,
}

/// Output: `PlacementGroup`
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
#[derive(Clone, Debug)]
pub struct MetaTranslationToSurface {
    surface_sdf_id: MetaSDFNodeID,
    to_translate_id: MetaSDFNodeID,
}

/// Input 1: `SDFGroup` or `SingleSDF`
/// Input 2: `PlacementGroup` or `SinglePlacement`
/// Output: `SDFGroup`
#[derive(Clone, Debug)]
pub struct MetaSDFScattering {
    sdf_id: MetaSDFNodeID,
    placement_id: MetaSDFNodeID,
}

/// Input: Any
/// Output: Same as input
#[derive(Clone, Debug)]
pub struct MetaStochasticSelection {
    child_id: MetaSDFNodeID,
    probability: f32,
    seed: u32,
}

type NodeRng = Pcg64Mcg;

impl<A: Allocator> MetaSDFGraph<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            nodes: AVec::new_in(alloc),
        }
    }

    pub fn add_node(&mut self, node: MetaSDFNode) -> MetaSDFNodeID {
        let id = self.nodes.len().try_into().unwrap();
        self.nodes.push(node);
        id
    }

    pub fn build<AR>(&self, arena: AR) -> Result<SDFGenerator>
    where
        AR: Allocator + Copy,
    {
        if self.nodes.is_empty() {
            return Ok(SDFGenerator::empty());
        }

        let mut builder = SDFGeneratorBuilder::new_in(arena);

        let mut outputs = AVec::new_in(arena);
        outputs.resize(self.nodes.len(), MetaSDFNodeOutput::<AR>::SingleSDF(None));

        let mut states = AVec::new_in(arena);
        states.resize(self.nodes.len(), MetaNodeBuildState::Unvisited);

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

                    operation_stack.push(BuildOperation::Process(node_id));

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
                                    to_translate_id: child_2_id,
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
                    let node = &self.nodes[node_idx];
                    let state = &mut states[node_idx];

                    if *state != MetaNodeBuildState::Resolved {
                        *state = MetaNodeBuildState::Resolved;

                        outputs[node_idx] = node.resolve(arena, &mut builder, &outputs)?;
                    }
                }
            }
        }

        if let MetaSDFNodeOutput::SingleSDF(atomic_node_id) = &outputs[root_node_id as usize] {
            if let Some(id) = atomic_node_id {
                assert_eq!(*id, builder.root_node_id());
            }
        } else {
            bail!("Root meta node must have single SDF output");
        }

        builder.build_with_arena(arena)
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

    pub fn new_stratified_grid_placement(
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
        to_translate_id: MetaSDFNodeID,
    ) -> Self {
        Self::TranslationToSurface(MetaTranslationToSurface {
            surface_sdf_id,
            to_translate_id,
        })
    }

    pub fn new_scattering(sdf_id: MetaSDFNodeID, placement_id: MetaSDFNodeID) -> Self {
        Self::Scattering(MetaSDFScattering {
            sdf_id,
            placement_id,
        })
    }

    pub fn new_stochastic_selection(child_id: MetaSDFNodeID, probability: f32, seed: u32) -> Self {
        assert!(probability >= 0.0);
        assert!(probability <= 1.0);
        Self::StochasticSelection(MetaStochasticSelection {
            child_id,
            probability,
            seed,
        })
    }

    fn resolve<A>(
        &self,
        arena: A,
        builder: &mut SDFGeneratorBuilder<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        match self {
            Self::Box(node) => Ok(node.resolve(builder)),
            Self::Sphere(node) => Ok(node.resolve(builder)),
            Self::GradientNoise(node) => Ok(node.resolve(builder)),
            Self::Translation(node) => node.resolve(arena, builder, outputs),
            Self::Rotation(node) => node.resolve(arena, builder, outputs),
            Self::Scaling(node) => node.resolve(arena, builder, outputs),
            Self::MultifractalNoise(node) => node.resolve(arena, builder, outputs),
            Self::Union(node) => node.resolve(builder, outputs),
            Self::Subtraction(node) => node.resolve(builder, outputs),
            Self::Intersection(node) => node.resolve(builder, outputs),
            Self::GroupUnion(node) => node.resolve(arena, builder, outputs),
            Self::StratifiedPlacement(node) => Ok(node.resolve(arena)),
            Self::TranslationToSurface(node) => node.resolve(arena, builder, outputs),
            Self::Scattering(node) => node.resolve(arena, builder, outputs),
            Self::StochasticSelection(node) => Ok(node.resolve(arena, outputs)),
        }
    }
}

impl MetaBoxSDF {
    fn resolve<A: Allocator>(&self, builder: &mut SDFGeneratorBuilder<A>) -> MetaSDFNodeOutput<A> {
        let mut rng = create_rng(self.seed);
        let extents = self.extents.map(|range| range.pick_value(&mut rng));
        let node_id = builder.add_node(SDFNode::new_box(extents));
        MetaSDFNodeOutput::SingleSDF(Some(node_id))
    }
}

impl MetaSphereSDF {
    fn resolve<A: Allocator>(&self, builder: &mut SDFGeneratorBuilder<A>) -> MetaSDFNodeOutput<A> {
        let mut rng = create_rng(self.seed);
        let radius = self.radius.pick_value(&mut rng);
        let node_id = builder.add_node(SDFNode::new_sphere(radius));
        MetaSDFNodeOutput::SingleSDF(Some(node_id))
    }
}

impl MetaGradientNoiseSDF {
    fn resolve<A: Allocator>(&self, builder: &mut SDFGeneratorBuilder<A>) -> MetaSDFNodeOutput<A> {
        let mut rng = create_rng(self.seed);
        let extents = self.extents.map(|range| range.pick_value(&mut rng));
        let noise_frequency = self.noise_frequency.pick_value(&mut rng);
        let noise_threshold = self.noise_threshold.pick_value(&mut rng);
        let seed = rng.random();
        let node_id = builder.add_node(SDFNode::new_gradient_noise(
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
        builder: &mut SDFGeneratorBuilder<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            builder,
            "Translation",
            self.seed,
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
        builder: &mut SDFGeneratorBuilder<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            builder,
            "Rotation",
            self.seed,
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
        builder: &mut SDFGeneratorBuilder<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            builder,
            "Scaling",
            self.seed,
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
        builder: &mut SDFGeneratorBuilder<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>> {
        resolve_unary_sdf_op(
            arena,
            builder,
            "MultifractalNoise",
            self.seed,
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

impl MetaSDFUnion {
    fn resolve<A>(
        &self,
        builder: &mut SDFGeneratorBuilder<A>,
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
                let output_node_id = builder.add_node(SDFNode::new_union(
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
        builder: &mut SDFGeneratorBuilder<A>,
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
                let output_node_id = builder.add_node(SDFNode::new_subtraction(
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
        builder: &mut SDFGeneratorBuilder<A>,
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
                let output_node_id = builder.add_node(SDFNode::new_intersection(
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
        builder: &mut SDFGeneratorBuilder<A>,
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
                        builder.add_node(SDFNode::new_union(
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
    fn resolve<A: Allocator>(&self, arena: A) -> MetaSDFNodeOutput<A> {
        let mut rng = create_rng(self.seed);
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
        builder: &mut SDFGeneratorBuilder<A>,
        outputs: &[MetaSDFNodeOutput<A>],
    ) -> Result<MetaSDFNodeOutput<A>>
    where
        A: Allocator + Copy,
    {
        let sdf_node_id = match &outputs[self.surface_sdf_id as usize] {
            MetaSDFNodeOutput::SingleSDF(sdf_node_id) => sdf_node_id,
            child_output => {
                bail!(
                    "TranslationToSurface node expects SingleSDF as input 1, got {}",
                    child_output.label(),
                );
            }
        };

        todo!()
    }
}

impl MetaSDFScattering {
    fn resolve<A>(
        &self,
        arena: A,
        builder: &mut SDFGeneratorBuilder<A>,
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
                output_node_id = builder.add_node(SDFNode::new_scaling(output_node_id, scaling));
            }
            if abs_diff_ne!(&rotation, &UnitQuaternion::identity()) {
                output_node_id = builder.add_node(SDFNode::new_rotation(output_node_id, rotation));
            }
            if abs_diff_ne!(&translation, &Vector3::zeros()) {
                output_node_id =
                    builder.add_node(SDFNode::new_translation(output_node_id, translation));
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
    fn resolve<A>(&self, arena: A, outputs: &[MetaSDFNodeOutput<A>]) -> MetaSDFNodeOutput<A>
    where
        A: Allocator + Copy,
    {
        let mut rng = create_rng(self.seed);
        let mut is_selected = || rng.random_range(0.0..1.0) < self.probability;

        match &outputs[self.child_id as usize] {
            MetaSDFNodeOutput::SingleSDF(None) => MetaSDFNodeOutput::SingleSDF(None),
            MetaSDFNodeOutput::SinglePlacement(None) => MetaSDFNodeOutput::SinglePlacement(None),
            MetaSDFNodeOutput::SingleSDF(Some(input_node_id)) => {
                let output_node_id = is_selected().then_some(*input_node_id);
                MetaSDFNodeOutput::SingleSDF(output_node_id)
            }
            MetaSDFNodeOutput::SinglePlacement(Some(input_placement)) => {
                let output_placement = is_selected().then_some(*input_placement);
                MetaSDFNodeOutput::SinglePlacement(output_placement)
            }
            MetaSDFNodeOutput::SDFGroup(input_node_ids) => {
                let mut output_node_ids = AVec::with_capacity_in(input_node_ids.len(), arena);
                for input_node_id in input_node_ids {
                    if is_selected() {
                        output_node_ids.push(*input_node_id);
                    }
                }
                MetaSDFNodeOutput::SDFGroup(output_node_ids)
            }
            MetaSDFNodeOutput::PlacementGroup(input_placements) => {
                let mut output_placements = AVec::with_capacity_in(input_placements.len(), arena);
                for input_placement in input_placements {
                    if is_selected() {
                        output_placements.push(*input_placement);
                    }
                }
                MetaSDFNodeOutput::PlacementGroup(output_placements)
            }
        }
    }
}

fn create_rng(seed: u32) -> NodeRng {
    NodeRng::seed_from_u64(u64::from(seed))
}

fn resolve_unary_sdf_op<A: Allocator>(
    arena: A,
    builder: &mut SDFGeneratorBuilder<A>,
    name: &str,
    seed: u32,
    child_output: &MetaSDFNodeOutput<A>,
    create_atomic_node: impl Fn(&mut NodeRng, SDFNodeID) -> SDFNode,
) -> Result<MetaSDFNodeOutput<A>> {
    match child_output {
        MetaSDFNodeOutput::SingleSDF(None) => Ok(MetaSDFNodeOutput::SingleSDF(None)),
        MetaSDFNodeOutput::SingleSDF(Some(input_node_id)) => {
            let mut rng = create_rng(seed);
            let output_node_id = builder.add_node(create_atomic_node(&mut rng, *input_node_id));
            Ok(MetaSDFNodeOutput::SingleSDF(Some(output_node_id)))
        }
        MetaSDFNodeOutput::SDFGroup(input_node_ids) => {
            let output_node_ids = unary_sdf_group_op(
                arena,
                builder,
                seed,
                input_node_ids,
                |rng, input_node_id| create_atomic_node(rng, input_node_id),
            );
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
    builder: &mut SDFGeneratorBuilder<A>,
    seed: u32,
    input_node_ids: &[SDFNodeID],
    create_output_node: impl Fn(&mut NodeRng, SDFNodeID) -> SDFNode,
) -> AVec<SDFNodeID, A> {
    let mut rng = create_rng(seed);
    let mut output_node_ids = AVec::with_capacity_in(input_node_ids.len(), arena);
    for input_node_id in input_node_ids {
        output_node_ids.push(builder.add_node(create_output_node(&mut rng, *input_node_id)));
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
