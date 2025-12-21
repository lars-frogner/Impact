//! Generation of signed distance fields. This module implements the graph of
//! simple "atomic" SDF nodes that is traversed during generation.

use crate::{VoxelSignedDistance, chunks::ChunkedVoxelObject};
use anyhow::{Result, anyhow, bail};
use approx::abs_diff_ne;
use impact_alloc::{
    AVec, Allocator,
    arena::{ArenaPool, PoolArena},
    avec,
};
use impact_geometry::{AxisAlignedBox, OrientedBox};
use impact_math::{
    Float,
    matrix::Matrix4,
    point::Point3,
    quaternion::{Quaternion, UnitQuaternion},
    transform::Similarity3,
    vector::{UnitVector3, Vector3},
};
use ordered_float::OrderedFloat;
use simdnoise::{NoiseBuilder, Settings, SimplexSettings};
use std::{array, f32, mem};
use twox_hash::XxHash32;

/// A signed distance field generator.
///
/// # Note
/// We might not actually want a real signed distance field, because it is hard
/// to modify it efficiently without invalidating distances away from the
/// surface. Instead, it might be better to embrace it as a signed field that
/// has correct distances only close to the surface, as this is what we
/// typically care about.
#[derive(Clone, Debug)]
pub struct SDFGenerator<A: Allocator> {
    /// Nodes in reverse depth-first order, with multi-parent subgraphs
    /// duplicated for each parent in order to unroll the DAG into a tree. The
    /// last node is the root.
    nodes: AVec<ProcessedSDFNode, A>,
    required_forward_stack_size: usize,
    domain: AxisAlignedBox,
}

#[derive(Clone, Debug)]
pub struct SDFGeneratorBlockBuffers<const COUNT: usize, A: Allocator> {
    /// Contains `required_forward_stack_size + 1` arrays, where the last one is scratch
    /// space.
    pub signed_distance_stack: AVec<[f32; COUNT], A>,
}

const CHUNK_SIZE: usize = ChunkedVoxelObject::chunk_size();
const CHUNK_VOXEL_COUNT: usize = ChunkedVoxelObject::chunk_voxel_count();

pub type SDFGeneratorChunkBuffers<A> = SDFGeneratorBlockBuffers<CHUNK_VOXEL_COUNT, A>;

#[derive(Clone, Debug)]
pub struct SDFGraph<A: Allocator> {
    nodes: AVec<SDFNode, A>,
    root_node_id: SDFNodeID,
}

pub type SDFNodeID = u32;

#[derive(Clone, Debug)]
pub enum SDFNode {
    // Primitives
    Sphere(SphereSDF),
    Capsule(CapsuleSDF),
    Box(BoxSDF),

    // Transforms
    Translation(SDFTranslation),
    Rotation(SDFRotation),
    Scaling(SDFScaling),

    // Modifiers
    MultifractalNoise(MultifractalNoiseSDFModifier),
    MultiscaleSphere(MultiscaleSphereSDFModifier),

    // Combination
    Union(SDFUnion),
    Subtraction(SDFSubtraction),
    Intersection(SDFIntersection),
}

#[derive(Clone, Debug)]
struct ProcessedSDFNode {
    node: SDFNode,
    /// Transforms positions from the root SDF coordinate space to this node's
    /// local space.
    transform_to_node_space: Matrix4,
    /// The domain is defined in the node's local space.
    ///
    /// It is expanded by a small margin on each side relative to the original
    /// tight domain. This means that the signed distance outside that domain is
    /// never smaller than the margin. If we determine a margin such that the
    /// parent nodes will never care about the values outside it, we are free to
    /// fill in blocks outside the margin with the margin value rather than
    /// evaluating the SDF there. Note that this does leave an invalid SDF
    /// though, since the gradient becomes zero. But as long as we don't need
    /// the gradient, that is OK.
    domain_with_margin: AxisAlignedBox,
    domain_margin: f32,
    leaf_count: u32,
}

/// Generator for a signed distance field representing a sphere centered at the
/// origin.
#[derive(Clone, Debug)]
pub struct SphereSDF {
    radius: f32,
}

/// Generator for a signed distance field representing a vertical capsule
/// centered at the origin.
#[derive(Clone, Debug)]
pub struct CapsuleSDF {
    half_segment_length: f32,
    radius: f32,
}

/// Generator for a signed distance field representing an axis-aligned box
/// centered at the origin.
#[derive(Clone, Debug)]
pub struct BoxSDF {
    half_extents: Vector3,
}

#[derive(Clone, Debug)]
pub struct SDFTranslation {
    pub child_id: SDFNodeID,
    pub translation: Vector3,
}

#[derive(Clone, Debug)]
pub struct SDFRotation {
    pub child_id: SDFNodeID,
    pub rotation: UnitQuaternion,
}

#[derive(Clone, Debug)]
pub struct SDFScaling {
    pub child_id: SDFNodeID,
    pub scaling: f32,
}

#[derive(Clone, Debug)]
pub struct SDFUnion {
    pub child_1_id: SDFNodeID,
    pub child_2_id: SDFNodeID,
    pub smoothness: Smoothness,
}

#[derive(Clone, Debug)]
pub struct SDFSubtraction {
    pub child_1_id: SDFNodeID,
    pub child_2_id: SDFNodeID,
    pub smoothness: Smoothness,
}

#[derive(Clone, Debug)]
pub struct SDFIntersection {
    pub child_1_id: SDFNodeID,
    pub child_2_id: SDFNodeID,
    pub smoothness: Smoothness,
}

/// Modifier for a signed distance field that adds a multifractal noise term to
/// the signed distance.
///
/// Note that the resulting field will in general not contain correct distances,
/// so this is best used only for minor perturbations.
#[derive(Clone, Debug)]
pub struct MultifractalNoiseSDFModifier {
    pub child_id: SDFNodeID,
    octaves: u32,
    frequency: f32,
    lacunarity: f32,
    persistence: f32,
    amplitude: f32,
    noise_scale: f32,
    seed: u32,
}

/// Modifier for a signed distance field that performs a stochastic multiscale
/// modification of the signed distance around the surface. This is done by
/// superimposing a field representing a grid of spheres with randomized radii,
/// which is unioned with the original field aroud the surface. This is repeated
/// for each octave with successively smaller and more numerous spheres.
///
/// See <https://iquilezles.org/articles/fbmsdf/> for more information.
///
/// The output will be a valid signed distance field.
#[derive(Clone, Debug)]
pub struct MultiscaleSphereSDFModifier {
    pub child_id: SDFNodeID,
    octaves: u32,
    frequency: f32,
    persistence: f32,
    scaled_inflation: f32,
    scaled_intersection_smoothness: Smoothness,
    union_smoothness: Smoothness,
    seed: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Smoothness {
    smoothness: f32,
    quarter_inv_smoothness: f32,
}

#[derive(Clone, Debug)]
enum BuildOperation {
    VisitChildren(SDFNodeID),
    Process(SDFNodeID),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeBuildState {
    Unvisited,
    ChildrenBeingVisited,
    DomainDetermined,
}

impl<A: Allocator> SDFGenerator<A> {
    pub fn total_buffer_size_for_chunk(&self) -> usize {
        self.total_buffer_size_for_block::<CHUNK_VOXEL_COUNT>()
    }

    pub fn create_buffers_for_chunk_in<AB: Allocator>(
        &self,
        alloc: AB,
    ) -> SDFGeneratorChunkBuffers<AB> {
        self.create_buffers_for_block_in(alloc)
    }

    pub fn compute_signed_distances_for_chunk<AB: Allocator>(
        &self,
        buffers: &mut SDFGeneratorChunkBuffers<AB>,
        chunk_aabb_in_root_space: &AxisAlignedBox,
    ) {
        self.compute_signed_distances_for_block::<CHUNK_SIZE, CHUNK_VOXEL_COUNT, AB>(
            buffers,
            chunk_aabb_in_root_space,
        );
    }
}

impl<A: Allocator> SDFGenerator<A> {
    pub fn empty_in(alloc: A) -> Self {
        Self {
            nodes: AVec::new_in(alloc),
            required_forward_stack_size: 0,
            domain: AxisAlignedBox::new(Point3::origin(), Point3::origin()),
        }
    }

    pub fn new_in(alloc: A, nodes: &[SDFNode], root_node_id: SDFNodeID) -> Result<Self> {
        let mut processed_nodes = AVec::with_capacity_in(nodes.len(), alloc);

        // Estimate capacity based on node count for domain calculations and processing
        let capacity = nodes.len() * (mem::size_of::<AxisAlignedBox>() + 64); // Domain + overhead per node
        let arena = ArenaPool::get_arena_for_capacity(capacity);

        // The domains of each node computed from child domains, not accounting
        // for required padding due to soft combination operations
        let mut domains = avec![in &arena; zero_domain(); nodes.len()];

        // The number of leaves below each node, for computing padding for
        // soft combination nodes
        let mut leaf_counts = avec![in &arena; 0_u32; nodes.len()];

        // The padding we must add to each node's domain to account for soft
        // combination operations
        let mut required_padding = avec![in &arena; 0.0_f32; nodes.len()];

        let mut states = avec![in &arena; NodeBuildState::Unvisited; nodes.len()];

        let mut operation_stack = AVec::with_capacity_in(3 * nodes.len(), &arena);

        operation_stack.push(BuildOperation::VisitChildren(root_node_id));

        let mut stack_top = 0;
        let mut max_stack_top = 0;

        while let Some(operation) = operation_stack.pop() {
            match operation {
                BuildOperation::VisitChildren(node_id) => {
                    let node_idx = node_id as usize;

                    let state = states
                        .get_mut(node_idx)
                        .ok_or_else(|| anyhow!("Missing SDF node {node_id}"))?;

                    match *state {
                        NodeBuildState::ChildrenBeingVisited => {
                            // We got back to the same node while visiting its children
                            bail!("Detected cycle in SDF generator node graph")
                        }
                        NodeBuildState::Unvisited | NodeBuildState::DomainDetermined => {
                            // Only enter the`ChildrenBeingVisited` state the
                            // first time we visit the children of this node. If
                            // we re-enter this subgraph through a different
                            // parent once the domain has been determined, we
                            // have already checked for cycles.
                            if *state == NodeBuildState::Unvisited {
                                *state = NodeBuildState::ChildrenBeingVisited;
                            }

                            operation_stack.push(BuildOperation::Process(node_id));

                            match &nodes[node_idx] {
                                SDFNode::Sphere(_) | SDFNode::Capsule(_) | SDFNode::Box(_) => {}
                                SDFNode::Translation(SDFTranslation { child_id, .. })
                                | SDFNode::Rotation(SDFRotation { child_id, .. })
                                | SDFNode::Scaling(SDFScaling { child_id, .. })
                                | SDFNode::MultifractalNoise(MultifractalNoiseSDFModifier {
                                    child_id,
                                    ..
                                })
                                | SDFNode::MultiscaleSphere(MultiscaleSphereSDFModifier {
                                    child_id,
                                    ..
                                }) => {
                                    operation_stack.push(BuildOperation::VisitChildren(*child_id));
                                }
                                SDFNode::Union(SDFUnion {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | SDFNode::Subtraction(SDFSubtraction {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                })
                                | SDFNode::Intersection(SDFIntersection {
                                    child_1_id,
                                    child_2_id,
                                    ..
                                }) => {
                                    // Push visits in reverse order so that
                                    // child 1 is processed and added to the
                                    // node list before child 2. This must be
                                    // consistent with how we look up the first
                                    // and second operand (child) when doing
                                    // non-commutative binary ops (subtraction).
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
                    let node = &nodes[node_idx];
                    let state = &mut states[node_idx];

                    if *state != NodeBuildState::DomainDetermined {
                        *state = NodeBuildState::DomainDetermined;

                        match node {
                            SDFNode::Sphere(sphere_generator) => {
                                domains[node_idx] = sphere_generator.domain_bounds();
                                leaf_counts[node_idx] = 1;
                            }
                            SDFNode::Capsule(capsule_generator) => {
                                domains[node_idx] = capsule_generator.domain_bounds();
                                leaf_counts[node_idx] = 1;
                            }
                            SDFNode::Box(box_generator) => {
                                domains[node_idx] = box_generator.domain_bounds();
                                leaf_counts[node_idx] = 1;
                            }
                            &SDFNode::Translation(SDFTranslation {
                                child_id,
                                translation,
                            }) => {
                                let child_domain = &domains[child_id as usize];
                                domains[node_idx] = child_domain.translated(&translation);

                                leaf_counts[node_idx] = leaf_counts[child_id as usize];

                                required_padding[node_idx] = required_padding[child_id as usize];
                            }
                            &SDFNode::Rotation(SDFRotation { child_id, rotation }) => {
                                let child_domain = &domains[child_id as usize];
                                let domain_ob = OrientedBox::from_axis_aligned_box(child_domain)
                                    .rotated(&rotation);
                                domains[node_idx] = AxisAlignedBox::aabb_for_point_array(
                                    &domain_ob.compute_corners(),
                                );

                                leaf_counts[node_idx] = leaf_counts[child_id as usize];
                                required_padding[node_idx] = required_padding[child_id as usize];
                            }
                            &SDFNode::Scaling(SDFScaling { child_id, scaling }) => {
                                let child_domain = &domains[child_id as usize];
                                domains[node_idx] = child_domain.scaled(scaling);

                                leaf_counts[node_idx] = leaf_counts[child_id as usize];
                                required_padding[node_idx] = required_padding[child_id as usize];
                            }
                            SDFNode::MultifractalNoise(MultifractalNoiseSDFModifier {
                                child_id,
                                amplitude,
                                ..
                            }) => {
                                let child_domain = &domains[*child_id as usize];
                                domains[node_idx] = child_domain.expanded_about_center(*amplitude);

                                leaf_counts[node_idx] = leaf_counts[*child_id as usize];
                                required_padding[node_idx] = required_padding[*child_id as usize];
                            }
                            SDFNode::MultiscaleSphere(
                                modifier @ MultiscaleSphereSDFModifier { child_id, .. },
                            ) => {
                                let child_domain = &domains[*child_id as usize];
                                domains[node_idx] =
                                    child_domain.expanded_about_center(modifier.domain_expansion());

                                leaf_counts[node_idx] = leaf_counts[*child_id as usize];
                                required_padding[node_idx] = required_padding[*child_id as usize];
                            }
                            &SDFNode::Union(SDFUnion {
                                child_1_id,
                                child_2_id,
                                smoothness,
                            }) => {
                                let child_1_domain = &domains[child_1_id as usize];
                                let child_2_domain = &domains[child_2_id as usize];
                                domains[node_idx] =
                                    AxisAlignedBox::aabb_from_pair(child_1_domain, child_2_domain);

                                let leaf_count = leaf_counts[child_1_id as usize]
                                    + leaf_counts[child_2_id as usize];

                                leaf_counts[node_idx] = leaf_count;

                                required_padding[node_idx] =
                                    soft_combine_domain_padding(smoothness.get(), leaf_count);
                            }
                            &SDFNode::Subtraction(SDFSubtraction {
                                child_1_id,
                                child_2_id,
                                smoothness,
                            }) => {
                                let selected_child_domain = &domains[child_1_id as usize];
                                domains[node_idx] = selected_child_domain.clone();

                                let leaf_count = leaf_counts[child_1_id as usize]
                                    + leaf_counts[child_2_id as usize];

                                leaf_counts[node_idx] = leaf_count;

                                required_padding[node_idx] =
                                    soft_combine_domain_padding(smoothness.get(), leaf_count);
                            }
                            &SDFNode::Intersection(SDFIntersection {
                                child_1_id,
                                child_2_id,
                                smoothness,
                            }) => {
                                let child_1_domain = &domains[child_1_id as usize];
                                let child_2_domain = &domains[child_2_id as usize];
                                domains[node_idx] = child_1_domain
                                    .compute_overlap_with(child_2_domain)
                                    .unwrap_or_else(zero_domain);

                                let leaf_count = leaf_counts[child_1_id as usize]
                                    + leaf_counts[child_2_id as usize];

                                leaf_counts[node_idx] = leaf_count;

                                required_padding[node_idx] =
                                    soft_combine_domain_padding(smoothness.get(), leaf_count);
                            }
                        }
                    }

                    let padded_domain =
                        domains[node_idx].expanded_about_center(required_padding[node_idx]);

                    // We push a node even when its domain has already been
                    // determined (meaning we duplicate the node) so that we can
                    // traverse the graph by iterating linearly through the
                    // ordered node list rather than jumping around.
                    processed_nodes.push(ProcessedSDFNode {
                        node: node.clone(),
                        // We will determine the correct transform in
                        // `determine_transforms_and_margins`
                        transform_to_node_space: Matrix4::identity(),
                        // The domain stays without margin (but with padding)
                        // until we have determined the appropriate margin
                        domain_with_margin: padded_domain,
                        domain_margin: 0.0,
                        leaf_count: leaf_counts[node_idx],
                    });

                    // Keep track of where the top of an operation stack would
                    // be during an unrolled traversal from children to parents
                    match node {
                        SDFNode::Sphere(_) | SDFNode::Capsule(_) | SDFNode::Box(_) => {
                            stack_top += 1;
                            if stack_top > max_stack_top {
                                max_stack_top = stack_top;
                            }
                        }
                        SDFNode::Union(_) | SDFNode::Subtraction(_) | SDFNode::Intersection(_) => {
                            debug_assert!(stack_top >= 2);
                            stack_top -= 1;
                        }
                        SDFNode::Translation(_)
                        | SDFNode::Rotation(_)
                        | SDFNode::Scaling(_)
                        | SDFNode::MultifractalNoise(_)
                        | SDFNode::MultiscaleSphere(_) => {}
                    }
                }
            }
        }

        debug_assert_eq!(stack_top, 1);

        Self::determine_transforms_and_margins(&arena, &mut processed_nodes);

        let root_domain = domains[root_node_id as usize]
            .expanded_about_center(required_padding[root_node_id as usize]);

        Ok(Self {
            nodes: processed_nodes,
            required_forward_stack_size: max_stack_top,
            domain: root_domain,
        })
    }

    fn determine_transforms_and_margins(arena: &PoolArena, nodes: &mut [ProcessedSDFNode]) {
        // We determine the transforms to node space by walking the graph from
        // parent to children, taking the parent transform and either
        // propagating it unchanged to the child or, if the child is a transform
        // node, applying the inverse of that transform to move from parent space
        // into the child’s local space.

        // Similarly, we determine the margins by walking the graph from parent
        // to children, taking the parent margin and adjusting it based on the
        // margin the parent will need for its child in order to evaluate the
        // SDF correctly.

        let mut transform_stack = avec![in arena; Matrix4::zeros(); nodes.len()];

        let mut margin_stack = avec![in arena; 0.0; nodes.len()];

        let mut stack_top = 0;
        transform_stack[stack_top] = Matrix4::identity();
        margin_stack[stack_top] = VoxelSignedDistance::MAX_F32;

        for node in nodes.iter_mut().rev() {
            let transform = transform_stack[stack_top];
            let margin = margin_stack[stack_top];

            node.transform_to_node_space = transform;
            node.domain_margin = margin;
            node.domain_with_margin = node.domain_with_margin.expanded_about_center(margin);

            match &node.node {
                SDFNode::Sphere(_) | SDFNode::Capsule(_) | SDFNode::Box(_) => {
                    stack_top = stack_top.saturating_sub(1);
                }
                SDFNode::Translation(SDFTranslation { translation, .. }) => {
                    // Transform: Shift the coordinate system in the opposite
                    // direction so positions are expressed relative to the
                    // child’s origin
                    transform_stack[stack_top].translate_transform(&(-translation));

                    // Margin: A translation node should have the same margin as
                    // its child
                }
                SDFNode::Rotation(SDFRotation { rotation, .. }) => {
                    // Transform: Rotate the coordinate system in the opposite
                    // direction so coordinates align with the child’s local
                    // axes
                    transform_stack[stack_top] =
                        rotation.inverse().to_homogeneous_matrix() * transform;

                    // Margin: A rotation node should have the same margin as
                    // its child
                }
                &SDFNode::Scaling(SDFScaling { scaling, .. }) => {
                    // Transform: Rescale coordinates so they match the child’s
                    // scale
                    transform_stack[stack_top].scale_transform(scaling.recip());

                    // Margin: A scaling node should preserve the same effective
                    // margin in parent space. Since SDF values scale by
                    // `scaling`, the child must use a margin of `margin /
                    // scaling` so that after scaling its distances still reach
                    // the required margin in the parent.
                    let margin_for_child = margin / scaling;
                    margin_stack[stack_top] = margin_for_child;
                }
                SDFNode::MultifractalNoise(modifier) => {
                    // Transform: The child of this node should have the same
                    // transform as its parent

                    // Margin: Any point that could fall within this node's
                    // margin might come from a child point as far as `margin +
                    // amplitude` from the child surface
                    let margin_for_child = margin + modifier.amplitude;
                    margin_stack[stack_top] = margin_for_child;
                }
                SDFNode::MultiscaleSphere(modifier) => {
                    // Transform: The child of this node should have the same
                    // transform as its parent

                    // Margin: Same logic as for `MultifractalNoise`
                    let margin_for_child = margin + modifier.domain_expansion();
                    margin_stack[stack_top] = margin_for_child;
                }
                &SDFNode::Union(SDFUnion { smoothness, .. })
                | &SDFNode::Subtraction(SDFSubtraction { smoothness, .. })
                | &SDFNode::Intersection(SDFIntersection { smoothness, .. }) => {
                    // Transform: Duplicate current transform for the second
                    // child branch
                    transform_stack[stack_top + 1] = transform;

                    // Margin: The smoothing operation can distort the distance
                    // field of its entire subtree by up to roughly `2 *
                    // soft_combine_domain_padding`.
                    // `soft_combine_domain_padding` estimates how far the
                    // *surface* of this subtree can move due to smoothing, but
                    // interior distances can deviate by up to about twice that
                    // amount. Any point that could fall within this node's
                    // margin might thus come from a child point as far as
                    // roughly `margin + 2 * soft_combine_domain_padding` from
                    // the child surface. For safety we use 2.5.
                    let margin_for_child = margin
                        + 2.5 * soft_combine_domain_padding(smoothness.get(), node.leaf_count);
                    margin_stack[stack_top] = margin_for_child;
                    margin_stack[stack_top + 1] = margin_for_child;

                    stack_top += 1;
                }
            }
        }
        assert_eq!(stack_top, 0);
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the domain where the signed distance field can be negative, in
    /// voxel grid coordinates relative to the origin of the root SDF coordinate
    /// space. If the domain is not translated, the origin coincides with the
    /// center of the domain.
    pub fn domain(&self) -> &AxisAlignedBox {
        &self.domain
    }

    pub fn total_buffer_size_for_block<const COUNT: usize>(&self) -> usize {
        mem::size_of::<f32>() * COUNT * (self.required_forward_stack_size + 1)
    }

    pub fn create_buffers_for_block_in<const COUNT: usize, AB: Allocator>(
        &self,
        alloc: AB,
    ) -> SDFGeneratorBlockBuffers<COUNT, AB> {
        // We only strictly need `self.required_forward_stack_size` signed
        // distance arrays, but we include one additional array for scratch
        // space at the end of the allocation
        let signed_distance_stack =
            avec![in alloc; [0.0; COUNT]; self.required_forward_stack_size + 1];

        SDFGeneratorBlockBuffers {
            signed_distance_stack,
        }
    }

    /// For performance, this method may clamp signed distances sufficiently far
    /// from node domain boundaries rather than evaluating them. If you need
    /// correct gradients, use
    /// [`Self::compute_signed_distances_for_block_preserving_gradients`].
    pub fn compute_signed_distances_for_block<
        const SIZE: usize,
        const COUNT: usize,
        AB: Allocator,
    >(
        &self,
        buffers: &mut SDFGeneratorBlockBuffers<COUNT, AB>,
        block_aabb_in_root_space: &AxisAlignedBox,
    ) {
        if self.nodes.is_empty() {
            buffers.signed_distance_stack[0].fill(VoxelSignedDistance::MAX_F32);
            return;
        }

        let block_origin_in_root_space = block_aabb_in_root_space.lower_corner();

        let mut stack_top: usize = 0;

        for node in &self.nodes {
            match &node.node {
                SDFNode::Sphere(sphere_generator) => {
                    let block_aabb_in_node_space =
                        block_aabb_in_root_space.aabb_of_transformed(&node.transform_to_node_space);

                    if node
                        .domain_with_margin
                        .box_lies_outside(&block_aabb_in_node_space)
                    {
                        // Fully outside: distances are assumed >= margin
                        buffers.signed_distance_stack[stack_top].fill(node.domain_margin);
                    } else if sphere_generator
                        .expanded_interior_domain_bounds(-node.domain_margin)
                        .contains_box(&block_aabb_in_node_space)
                    {
                        // Fully inside: distances are assumed <= -margin
                        buffers.signed_distance_stack[stack_top].fill(-node.domain_margin);
                    } else {
                        update_signed_distances_for_block::<SIZE, COUNT>(
                            &mut buffers.signed_distance_stack[stack_top],
                            &node.transform_to_node_space,
                            block_origin_in_root_space,
                            &|signed_distance, position_in_node_space| {
                                *signed_distance = sphere_generator
                                    .compute_signed_distance(position_in_node_space);
                            },
                        );
                    }

                    stack_top += 1;
                }
                SDFNode::Capsule(capsule_generator) => {
                    let block_aabb_in_node_space =
                        block_aabb_in_root_space.aabb_of_transformed(&node.transform_to_node_space);

                    if node
                        .domain_with_margin
                        .box_lies_outside(&block_aabb_in_node_space)
                    {
                        buffers.signed_distance_stack[stack_top].fill(node.domain_margin);
                    } else if capsule_generator
                        .expanded_interior_domain_bounds(-node.domain_margin)
                        .contains_box(&block_aabb_in_node_space)
                    {
                        buffers.signed_distance_stack[stack_top].fill(-node.domain_margin);
                    } else {
                        update_signed_distances_for_block::<SIZE, COUNT>(
                            &mut buffers.signed_distance_stack[stack_top],
                            &node.transform_to_node_space,
                            block_origin_in_root_space,
                            &|signed_distance, position_in_node_space| {
                                *signed_distance = capsule_generator
                                    .compute_signed_distance(position_in_node_space);
                            },
                        );
                    }

                    stack_top += 1;
                }
                SDFNode::Box(box_generator) => {
                    let block_aabb_in_node_space =
                        block_aabb_in_root_space.aabb_of_transformed(&node.transform_to_node_space);

                    if node
                        .domain_with_margin
                        .box_lies_outside(&block_aabb_in_node_space)
                    {
                        buffers.signed_distance_stack[stack_top].fill(node.domain_margin);
                    } else if box_generator
                        .expanded_domain_bounds(-node.domain_margin)
                        .contains_box(&block_aabb_in_node_space)
                    {
                        buffers.signed_distance_stack[stack_top].fill(-node.domain_margin);
                    } else {
                        update_signed_distances_for_block::<SIZE, COUNT>(
                            &mut buffers.signed_distance_stack[stack_top],
                            &node.transform_to_node_space,
                            block_origin_in_root_space,
                            &|signed_distance, position_in_node_space| {
                                *signed_distance =
                                    box_generator.compute_signed_distance(position_in_node_space);
                            },
                        );
                    }

                    stack_top += 1;
                }
                SDFNode::Translation(_) | SDFNode::Rotation(_) => {}
                SDFNode::Scaling(SDFScaling { scaling, .. }) => {
                    debug_assert!(stack_top >= 1);

                    let distances = &mut buffers.signed_distance_stack[stack_top - 1];

                    for signed_distance in distances.iter_mut() {
                        *signed_distance *= scaling;
                    }
                }
                SDFNode::MultifractalNoise(modifier) => {
                    let block_aabb_in_node_space =
                        block_aabb_in_root_space.aabb_of_transformed(&node.transform_to_node_space);

                    let scratch_idx = buffers.signed_distance_stack.len() - 1;
                    let [distances, scratch] = buffers
                        .signed_distance_stack
                        .get_disjoint_mut([stack_top - 1, scratch_idx])
                        .unwrap();

                    if !node
                        .domain_with_margin
                        .box_lies_outside(&block_aabb_in_node_space)
                        || !modifier
                            .all_modified_signed_distances_at_block_test_positions_pass_predicate::<SIZE, COUNT>(
                                distances,
                                &node.transform_to_node_space,
                                block_origin_in_root_space,
                                |signed_distance| signed_distance >= node.domain_margin,
                            )
                    {
                    modifier.modify_signed_distances_for_block::<SIZE, COUNT>(
                        distances,
                        scratch,
                        &node.transform_to_node_space,
                        block_origin_in_root_space,
                    );
                    }
                }
                SDFNode::MultiscaleSphere(modifier) => {
                    let block_aabb_in_node_space =
                        block_aabb_in_root_space.aabb_of_transformed(&node.transform_to_node_space);

                    let distances = &mut buffers.signed_distance_stack[stack_top - 1];

                    if !node
                        .domain_with_margin
                        .box_lies_outside(&block_aabb_in_node_space)
                        || !all_block_test_positions_pass_predicate::<SIZE, COUNT>(
                            &node.transform_to_node_space,
                            block_origin_in_root_space,
                            &|idx, position_in_node_space| {
                                modifier
                                    .modify_signed_distance(&position_in_node_space, distances[idx])
                                    >= node.domain_margin
                            },
                        )
                    {
                        update_signed_distances_for_block::<SIZE, COUNT>(
                            distances,
                            &node.transform_to_node_space,
                            block_origin_in_root_space,
                            &|signed_distance, position_in_node_space| {
                                *signed_distance = modifier.modify_signed_distance(
                                    position_in_node_space,
                                    *signed_distance,
                                );
                            },
                        );
                    }
                }
                &SDFNode::Union(SDFUnion { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    stack_top -= 1;

                    let block_aabb_in_node_space =
                        block_aabb_in_root_space.aabb_of_transformed(&node.transform_to_node_space);

                    let [distances_1, distances_2] = buffers
                        .signed_distance_stack
                        .get_disjoint_mut([stack_top - 1, stack_top])
                        .unwrap();

                    if !node
                        .domain_with_margin
                        .box_lies_outside(&block_aabb_in_node_space)
                        || !all_block_test_positions_pass_predicate::<SIZE, COUNT>(
                            &node.transform_to_node_space,
                            block_origin_in_root_space,
                            &|idx, _| {
                                sdf_union(distances_1[idx], distances_2[idx], smoothness)
                                    >= node.domain_margin
                            },
                        )
                    {
                        apply_sdf_unions(distances_1, distances_2, smoothness);
                    }
                }
                &SDFNode::Subtraction(SDFSubtraction { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    stack_top -= 1;

                    let block_aabb_in_node_space =
                        block_aabb_in_root_space.aabb_of_transformed(&node.transform_to_node_space);

                    let [distances_1, distances_2] = buffers
                        .signed_distance_stack
                        .get_disjoint_mut([stack_top - 1, stack_top])
                        .unwrap();

                    if !node
                        .domain_with_margin
                        .box_lies_outside(&block_aabb_in_node_space)
                        || !all_block_test_positions_pass_predicate::<SIZE, COUNT>(
                            &node.transform_to_node_space,
                            block_origin_in_root_space,
                            &|idx, _| {
                                sdf_subtraction(distances_1[idx], distances_2[idx], smoothness)
                                    >= node.domain_margin
                            },
                        )
                    {
                        apply_sdf_subtractions(distances_1, distances_2, smoothness);
                    }
                }
                &SDFNode::Intersection(SDFIntersection { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    stack_top -= 1;

                    let block_aabb_in_node_space =
                        block_aabb_in_root_space.aabb_of_transformed(&node.transform_to_node_space);

                    let [distances_1, distances_2] = buffers
                        .signed_distance_stack
                        .get_disjoint_mut([stack_top - 1, stack_top])
                        .unwrap();

                    if !node
                        .domain_with_margin
                        .box_lies_outside(&block_aabb_in_node_space)
                        || !all_block_test_positions_pass_predicate::<SIZE, COUNT>(
                            &node.transform_to_node_space,
                            block_origin_in_root_space,
                            &|idx, _| {
                                sdf_intersection(distances_1[idx], distances_2[idx], smoothness)
                                    >= node.domain_margin
                            },
                        )
                    {
                        apply_sdf_intersections(distances_1, distances_2, smoothness);
                    }
                }
            }
        }

        assert_eq!(stack_top, 1);
    }

    pub fn compute_signed_distances_for_block_preserving_gradients<
        const SIZE: usize,
        const COUNT: usize,
    >(
        &self,
        buffers: &mut SDFGeneratorBlockBuffers<COUNT, A>,
        block_origin_in_root_space: &Point3,
    ) {
        if self.nodes.is_empty() {
            buffers.signed_distance_stack[0].fill(VoxelSignedDistance::MAX_F32);
            return;
        }

        let mut stack_top: usize = 0;

        for node in &self.nodes {
            match &node.node {
                SDFNode::Sphere(sphere_generator) => {
                    update_signed_distances_for_block::<SIZE, COUNT>(
                        &mut buffers.signed_distance_stack[stack_top],
                        &node.transform_to_node_space,
                        block_origin_in_root_space,
                        &|signed_distance, position_in_node_space| {
                            *signed_distance =
                                sphere_generator.compute_signed_distance(position_in_node_space);
                        },
                    );

                    stack_top += 1;
                }
                SDFNode::Capsule(capsule_generator) => {
                    update_signed_distances_for_block::<SIZE, COUNT>(
                        &mut buffers.signed_distance_stack[stack_top],
                        &node.transform_to_node_space,
                        block_origin_in_root_space,
                        &|signed_distance, position_in_node_space| {
                            *signed_distance =
                                capsule_generator.compute_signed_distance(position_in_node_space);
                        },
                    );

                    stack_top += 1;
                }
                SDFNode::Box(box_generator) => {
                    update_signed_distances_for_block::<SIZE, COUNT>(
                        &mut buffers.signed_distance_stack[stack_top],
                        &node.transform_to_node_space,
                        block_origin_in_root_space,
                        &|signed_distance, position_in_node_space| {
                            *signed_distance =
                                box_generator.compute_signed_distance(position_in_node_space);
                        },
                    );

                    stack_top += 1;
                }
                SDFNode::Translation(_) | SDFNode::Rotation(_) => {}
                SDFNode::Scaling(SDFScaling { scaling, .. }) => {
                    debug_assert!(stack_top >= 1);

                    for signed_distance in &mut buffers.signed_distance_stack[stack_top - 1] {
                        *signed_distance *= scaling;
                    }
                }
                SDFNode::MultifractalNoise(modifier) => {
                    let scratch_idx = buffers.signed_distance_stack.len() - 1;
                    let [distances, scratch] = buffers
                        .signed_distance_stack
                        .get_disjoint_mut([stack_top - 1, scratch_idx])
                        .unwrap();

                    modifier.modify_signed_distances_for_block::<SIZE, COUNT>(
                        distances,
                        scratch,
                        &node.transform_to_node_space,
                        block_origin_in_root_space,
                    );
                }
                SDFNode::MultiscaleSphere(modifier) => {
                    update_signed_distances_for_block::<SIZE, COUNT>(
                        &mut buffers.signed_distance_stack[stack_top - 1],
                        &node.transform_to_node_space,
                        block_origin_in_root_space,
                        &|signed_distance, position_in_node_space| {
                            *signed_distance = modifier
                                .modify_signed_distance(position_in_node_space, *signed_distance);
                        },
                    );
                }
                &SDFNode::Union(SDFUnion { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    stack_top -= 1;

                    let [distances_1, distances_2] = buffers
                        .signed_distance_stack
                        .get_disjoint_mut([stack_top - 1, stack_top])
                        .unwrap();

                    apply_sdf_unions(distances_1, distances_2, smoothness);
                }
                &SDFNode::Subtraction(SDFSubtraction { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    stack_top -= 1;

                    let [distances_1, distances_2] = buffers
                        .signed_distance_stack
                        .get_disjoint_mut([stack_top - 1, stack_top])
                        .unwrap();

                    apply_sdf_subtractions(distances_1, distances_2, smoothness);
                }
                &SDFNode::Intersection(SDFIntersection { smoothness, .. }) => {
                    debug_assert!(stack_top >= 2);
                    stack_top -= 1;

                    let [distances_1, distances_2] = buffers
                        .signed_distance_stack
                        .get_disjoint_mut([stack_top - 1, stack_top])
                        .unwrap();

                    apply_sdf_intersections(distances_1, distances_2, smoothness);
                }
            }
        }

        assert_eq!(stack_top, 1);
    }

    pub fn compute_signed_distance(
        &self,
        buffers: &mut SDFGeneratorBlockBuffers<1, A>,
        position_in_root_space: &Point3,
    ) -> f32 {
        self.compute_signed_distances_for_block_preserving_gradients::<1, 1>(
            buffers,
            position_in_root_space,
        );
        buffers.final_signed_distances()[0]
    }
}

impl<const COUNT: usize, A: Allocator> SDFGeneratorBlockBuffers<COUNT, A> {
    pub fn final_signed_distances(&self) -> &[f32; COUNT] {
        &self.signed_distance_stack[0]
    }
}

impl<A: Allocator> SDFGraph<A> {
    pub fn new_in(alloc: A) -> Self {
        Self::with_capacity_in(0, alloc)
    }

    pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
        Self {
            nodes: AVec::<_, A>::with_capacity_in(capacity, alloc),
            root_node_id: 0,
        }
    }

    pub fn build_in<AL: Allocator>(&self, alloc: AL) -> Result<SDFGenerator<AL>> {
        if self.nodes.is_empty() {
            Ok(SDFGenerator::empty_in(alloc))
        } else {
            SDFGenerator::new_in(alloc, &self.nodes, self.root_node_id)
        }
    }

    pub fn root_node_id(&self) -> SDFNodeID {
        self.root_node_id
    }

    pub fn nodes(&self) -> &[SDFNode] {
        &self.nodes
    }

    pub fn add_node(&mut self, node: SDFNode) -> SDFNodeID {
        let node_id = self.nodes.len().try_into().unwrap();
        self.nodes.push(node);
        self.root_node_id = node_id;
        node_id
    }

    pub fn set_root_node(&mut self, node_id: SDFNodeID) {
        assert!((node_id as usize) < self.nodes.len());
        self.root_node_id = node_id;
    }
}

impl SDFNode {
    #[inline]
    pub fn new_sphere(radius: f32) -> Self {
        Self::Sphere(SphereSDF::new(radius))
    }

    #[inline]
    pub fn new_capsule(segment_length: f32, radius: f32) -> Self {
        Self::Capsule(CapsuleSDF::new(segment_length, radius))
    }

    #[inline]
    pub fn new_box(extents: [f32; 3]) -> Self {
        Self::Box(BoxSDF::new(extents))
    }

    #[inline]
    pub fn new_translation(child_id: SDFNodeID, translation: Vector3) -> Self {
        Self::Translation(SDFTranslation {
            child_id,
            translation,
        })
    }

    #[inline]
    pub fn new_rotation(child_id: SDFNodeID, rotation: UnitQuaternion) -> Self {
        Self::Rotation(SDFRotation { child_id, rotation })
    }

    #[inline]
    pub fn new_scaling(child_id: SDFNodeID, scaling: f32) -> Self {
        Self::Scaling(SDFScaling::new(child_id, scaling))
    }

    #[inline]
    pub fn new_multifractal_noise(
        child_id: SDFNodeID,
        octaves: u32,
        frequency: f32,
        lacunarity: f32,
        persistence: f32,
        amplitude: f32,
        seed: u32,
    ) -> Self {
        Self::MultifractalNoise(MultifractalNoiseSDFModifier::new(
            child_id,
            octaves,
            frequency,
            lacunarity,
            persistence,
            amplitude,
            seed,
        ))
    }

    #[inline]
    pub fn new_multiscale_sphere(
        child_id: SDFNodeID,
        octaves: u32,
        max_scale: f32,
        persistence: f32,
        inflation: f32,
        intersection_smoothness: f32,
        union_smoothness: f32,
        seed: u32,
    ) -> Self {
        Self::MultiscaleSphere(MultiscaleSphereSDFModifier::new(
            child_id,
            octaves,
            max_scale,
            persistence,
            inflation,
            intersection_smoothness,
            union_smoothness,
            seed,
        ))
    }

    #[inline]
    pub fn new_union(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        Self::Union(SDFUnion::new(child_1_id, child_2_id, smoothness))
    }

    #[inline]
    pub fn new_subtraction(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        Self::Subtraction(SDFSubtraction::new(child_1_id, child_2_id, smoothness))
    }

    #[inline]
    pub fn new_intersection(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        Self::Intersection(SDFIntersection::new(child_1_id, child_2_id, smoothness))
    }

    #[inline]
    pub fn node_to_parent_translation(&self) -> Vector3 {
        match self {
            Self::Translation(SDFTranslation { translation, .. }) => *translation,
            _ => Vector3::zeros(),
        }
    }

    #[inline]
    pub fn node_to_parent_transform(&self) -> Similarity3 {
        match self {
            Self::Translation(SDFTranslation { translation, .. }) => {
                Similarity3::from_translation(*translation)
            }
            Self::Rotation(SDFRotation { rotation, .. }) => Similarity3::from_rotation(*rotation),
            Self::Scaling(SDFScaling { scaling, .. }) => Similarity3::from_scaling(*scaling),
            _ => Similarity3::identity(),
        }
    }
}

impl SphereSDF {
    /// Creates a new generator for a sphere with the given radius (in voxels).
    #[inline]
    pub fn new(radius: f32) -> Self {
        assert!(radius >= 0.0);
        Self { radius }
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    #[inline]
    pub fn domain_bounds(&self) -> AxisAlignedBox {
        let half_extents = Vector3::same(self.radius);
        AxisAlignedBox::new((-half_extents).into(), half_extents.into())
    }

    #[inline]
    fn expanded_interior_domain_bounds(&self, margin: f32) -> AxisAlignedBox {
        let extent_of_internal_box_in_sphere = self.radius * f32::FRAC_1_SQRT_3;

        let expanded_half_extents = Vector3::same(extent_of_internal_box_in_sphere + margin);

        AxisAlignedBox::new(
            (-expanded_half_extents).into(),
            expanded_half_extents.into(),
        )
    }

    #[inline]
    pub fn compute_signed_distance(&self, position_in_node_space: &Point3) -> f32 {
        position_in_node_space.as_vector().norm() - self.radius
    }
}

impl CapsuleSDF {
    /// Creates a new generator for a capsule with the given segment length and
    /// radius (in voxels).
    #[inline]
    pub fn new(segment_length: f32, radius: f32) -> Self {
        assert!(segment_length >= 0.0);
        assert!(radius >= 0.0);
        Self {
            half_segment_length: 0.5 * segment_length,
            radius,
        }
    }

    #[inline]
    pub fn segment_length(&self) -> f32 {
        2.0 * self.half_segment_length
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    #[inline]
    pub fn domain_bounds(&self) -> AxisAlignedBox {
        let mut half_extents = Vector3::same(self.radius);
        *half_extents.y_mut() += self.half_segment_length;
        AxisAlignedBox::new((-half_extents).into(), half_extents.into())
    }

    #[inline]
    fn expanded_interior_domain_bounds(&self, margin: f32) -> AxisAlignedBox {
        let extent_of_internal_box_in_sphere = self.radius * f32::FRAC_1_SQRT_3;

        let mut expanded_half_extents = Vector3::same(extent_of_internal_box_in_sphere + margin);
        *expanded_half_extents.y_mut() += self.half_segment_length;

        AxisAlignedBox::new(
            (-expanded_half_extents).into(),
            expanded_half_extents.into(),
        )
    }

    #[inline]
    pub fn compute_signed_distance(&self, position_in_node_space: &Point3) -> f32 {
        let mut position = *position_in_node_space;
        *position.y_mut() -= position
            .y()
            .clamp(-self.half_segment_length, self.half_segment_length);
        position.as_vector().norm() - self.radius
    }
}

impl BoxSDF {
    /// Creates a new generator for a box with the given extents (in voxels).
    #[inline]
    pub fn new(extents: [f32; 3]) -> Self {
        assert!(extents.iter().copied().all(f32::is_sign_positive));
        let half_extents = 0.5 * Vector3::from(extents);
        Self { half_extents }
    }

    #[inline]
    pub fn extents(&self) -> [f32; 3] {
        [
            2.0 * self.half_extents.x(),
            2.0 * self.half_extents.y(),
            2.0 * self.half_extents.z(),
        ]
    }

    #[inline]
    pub fn domain_bounds(&self) -> AxisAlignedBox {
        AxisAlignedBox::new((-self.half_extents).into(), self.half_extents.into())
    }

    #[inline]
    fn expanded_domain_bounds(&self, margin: f32) -> AxisAlignedBox {
        let expanded_half_extents = self.half_extents + Vector3::same(margin);
        AxisAlignedBox::new(
            (-expanded_half_extents).into(),
            expanded_half_extents.into(),
        )
    }

    #[inline]
    pub fn compute_signed_distance(&self, position_in_node_space: &Point3) -> f32 {
        let q = position_in_node_space.as_vector().component_abs() - self.half_extents;
        q.component_max(&Vector3::zeros()).norm() + f32::min(q.max_component(), 0.0)
    }
}

impl SDFRotation {
    #[inline]
    pub fn from_axis_angle(child_id: SDFNodeID, axis: Vector3, angle: f32) -> Self {
        let rotation = UnitQuaternion::from_axis_angle(&UnitVector3::normalized_from(axis), angle);
        Self { child_id, rotation }
    }

    #[inline]
    pub fn from_euler_angles(child_id: SDFNodeID, roll: f32, pitch: f32, yaw: f32) -> Self {
        let rotation = UnitQuaternion::from_euler_angles(roll, pitch, yaw);
        Self { child_id, rotation }
    }

    /// Returns the Euler angles as `(roll, pitch, yaw)`.
    #[inline]
    pub fn euler_angles(&self) -> (f32, f32, f32) {
        self.rotation.euler_angles()
    }
}

impl SDFScaling {
    #[inline]
    pub fn new(child_id: SDFNodeID, scaling: f32) -> Self {
        assert!(scaling > 0.0);
        Self { child_id, scaling }
    }
}

impl SDFUnion {
    #[inline]
    pub fn new(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        assert!(smoothness >= 0.0);
        Self {
            child_1_id,
            child_2_id,
            smoothness: smoothness.into(),
        }
    }
}

impl SDFSubtraction {
    #[inline]
    pub fn new(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        assert!(smoothness >= 0.0);
        Self {
            child_1_id,
            child_2_id,
            smoothness: smoothness.into(),
        }
    }
}

impl SDFIntersection {
    #[inline]
    pub fn new(child_1_id: SDFNodeID, child_2_id: SDFNodeID, smoothness: f32) -> Self {
        assert!(smoothness >= 0.0);
        Self {
            child_1_id,
            child_2_id,
            smoothness: smoothness.into(),
        }
    }
}

impl MultifractalNoiseSDFModifier {
    #[inline]
    pub fn new(
        child_id: SDFNodeID,
        octaves: u32,
        frequency: f32,
        lacunarity: f32,
        persistence: f32,
        amplitude: f32,
        seed: u32,
    ) -> Self {
        let inherent_amplitude = theoretical_max_amplitude_of_fbm_noise(octaves, persistence);
        let noise_scale = if abs_diff_ne!(inherent_amplitude, 0.0) {
            amplitude / inherent_amplitude
        } else {
            0.0
        };
        Self {
            child_id,
            octaves,
            frequency,
            lacunarity,
            persistence,
            amplitude,
            noise_scale,
            seed,
        }
    }

    #[inline]
    pub fn octaves(&self) -> u32 {
        self.octaves
    }

    #[inline]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    #[inline]
    pub fn lacunarity(&self) -> f32 {
        self.lacunarity
    }

    #[inline]
    pub fn persistence(&self) -> f32 {
        self.persistence
    }

    #[inline]
    pub fn amplitude(&self) -> f32 {
        self.amplitude
    }

    #[inline]
    pub fn seed(&self) -> u32 {
        self.seed
    }

    #[inline]
    fn modify_signed_distances_for_block<const SIZE: usize, const COUNT: usize>(
        &self,
        signed_distances: &mut [f32; COUNT],
        scratch: &mut [f32],
        transform_to_node_space: &Matrix4,
        block_origin_in_root_space: &Point3,
    ) {
        let origin_in_node_space =
            transform_to_node_space.transform_point(block_origin_in_root_space);

        let dx = transform_to_node_space.column1().xyz();
        let dy = transform_to_node_space.column2().xyz();
        let dz = transform_to_node_space.column3().xyz();

        let inverse_scale = dx.norm();
        let scale = inverse_scale.recip();

        // We incorporate the scaling into the noise by dividing the original
        // frequency with the scale and adjusting the origin to compensate
        let unscaled_frequency = self.frequency * inverse_scale;
        let origin_for_noise = scale * origin_in_node_space;

        // Fall back to per-voxel evaluation if there is any rotation
        if abs_diff_ne!(dx.x() * inverse_scale, 1.0, epsilon = 1e-6)
            || abs_diff_ne!(dy.y() * inverse_scale, 1.0, epsilon = 1e-6)
        {
            let dx_for_noise = scale * dx;
            let dy_for_noise = scale * dy;
            let dz_for_noise = scale * dz;
            let mut noise = [0.0];

            let mut idx = 0;
            for i in 0..SIZE {
                let origin_plus_x = origin_for_noise + (i as f32) * dx_for_noise;
                for j in 0..SIZE {
                    let mut pos = origin_plus_x + (j as f32) * dy_for_noise;
                    for _ in 0..SIZE {
                        NoiseBuilder::fbm_3d_offset(
                            // Warning: We reverse the order of dimensions here to match the
                            // block-wise evaluation below
                            pos.z(),
                            1,
                            pos.y(),
                            1,
                            pos.x(),
                            1,
                        )
                        .with_octaves(self.octaves as u8)
                        .with_freq(unscaled_frequency)
                        .with_lacunarity(self.lacunarity)
                        .with_gain(self.persistence)
                        .with_seed(self.seed as i32)
                        .generate(&mut noise);

                        signed_distances[idx] += noise[0] * self.noise_scale;

                        pos += dz_for_noise;
                        idx += 1;
                    }
                }
            }
            return;
        }

        NoiseBuilder::fbm_3d_offset(
            // Warning: We reverse the order of dimensions here because the
            // generated noise is laid out in row-major order
            origin_for_noise.z(),
            SIZE,
            origin_for_noise.y(),
            SIZE,
            origin_for_noise.x(),
            SIZE,
        )
        .with_octaves(self.octaves as u8)
        .with_freq(unscaled_frequency)
        .with_lacunarity(self.lacunarity)
        .with_gain(self.persistence)
        .with_seed(self.seed as i32)
        .generate(scratch);

        for (signed_distance, noise) in signed_distances.iter_mut().zip(scratch.iter()) {
            *signed_distance += *noise * self.noise_scale;
        }
    }

    #[inline]
    fn all_modified_signed_distances_at_block_test_positions_pass_predicate<
        const SIZE: usize,
        const COUNT: usize,
    >(
        &self,
        signed_distances: &[f32; COUNT],
        transform_to_node_space: &Matrix4,
        block_origin_in_root_space: &Point3,
        predicate: impl Fn(f32) -> bool,
    ) -> bool {
        let origin_in_node_space =
            transform_to_node_space.transform_point(block_origin_in_root_space);

        let dx = transform_to_node_space.column1().xyz();
        let dy = transform_to_node_space.column2().xyz();
        let dz = transform_to_node_space.column3().xyz();

        let inverse_scale = dx.norm();
        let scale = inverse_scale.recip();

        // We incorporate the scaling into the noise by dividing the original
        // frequency with the scale and adjusting the origin to compensate
        let unscaled_frequency = self.frequency * inverse_scale;
        let origin_for_noise = scale * origin_in_node_space;

        let dx_for_noise = scale * dx;
        let dy_for_noise = scale * dy;
        let dz_for_noise = scale * dz;

        let mut noise = [0.0];

        for (idx, point) in all_block_test_positions_with_indices::<SIZE, COUNT>(
            &origin_for_noise,
            &dx_for_noise,
            &dy_for_noise,
            &dz_for_noise,
        ) {
            NoiseBuilder::fbm_3d_offset(
                // Warning: We reverse the order of dimensions here to match
                // `Self::modify_signed_distances_for_block`
                point.z(),
                1,
                point.y(),
                1,
                point.x(),
                1,
            )
            .with_octaves(self.octaves as u8)
            .with_freq(unscaled_frequency)
            .with_lacunarity(self.lacunarity)
            .with_gain(self.persistence)
            .with_seed(self.seed as i32)
            .generate(&mut noise);

            let modified_signed_distance = signed_distances[idx] + noise[0] * self.noise_scale;

            if !predicate(modified_signed_distance) {
                return false;
            }
        }

        true
    }
}

impl MultiscaleSphereSDFModifier {
    /// Inflation should probably always be 1.0. Intersection smoothness should
    /// probably exceed inflation.
    #[inline]
    pub fn new(
        child_id: SDFNodeID,
        octaves: u32,
        max_scale: f32,
        persistence: f32,
        inflation: f32,
        intersection_smoothness: f32,
        union_smoothness: f32,
        seed: u32,
    ) -> Self {
        let frequency = 0.5 / max_scale;

        // Scale inflation and intersection smoothness according to the scale of
        // perturbations
        let scaled_inflation = max_scale * inflation;
        let scaled_intersection_smoothness = max_scale * intersection_smoothness;

        Self {
            child_id,
            octaves,
            frequency,
            persistence,
            scaled_inflation,
            scaled_intersection_smoothness: scaled_intersection_smoothness.into(),
            union_smoothness: union_smoothness.into(),
            seed,
        }
    }

    #[inline]
    pub fn octaves(&self) -> u32 {
        self.octaves
    }

    #[inline]
    pub fn max_scale(&self) -> f32 {
        0.5 / self.frequency
    }

    #[inline]
    pub fn persistence(&self) -> f32 {
        self.persistence
    }

    #[inline]
    pub fn inflation(&self) -> f32 {
        self.scaled_inflation / self.max_scale()
    }

    #[inline]
    pub fn intersection_smoothness(&self) -> f32 {
        self.scaled_intersection_smoothness.get() / self.max_scale()
    }

    #[inline]
    pub fn union_smoothness(&self) -> f32 {
        self.union_smoothness.get()
    }

    #[inline]
    pub fn seed(&self) -> u32 {
        self.seed
    }

    #[inline]
    fn domain_expansion(&self) -> f32 {
        self.scaled_inflation + displacement_due_to_smoothness(self.union_smoothness.get())
    }

    #[inline]
    fn modify_signed_distance(&self, position_in_node_space: &Point3, signed_distance: f32) -> f32 {
        /// Rotates with an angle of `2 * pi / golden_ratio` around the axis
        /// `[1, 1, 1]` (to break up the regular grid pattern).
        const ROTATION: UnitQuaternion = UnitQuaternion::unchecked_from(Quaternion::from_parts(
            -0.3623749,
            Vector3::new(0.5381091, 0.5381091, 0.5381091),
        ));

        let mut parent_distance = signed_distance;
        let mut position = self.frequency * position_in_node_space;
        let mut scale = 1.0;

        for _ in 0..self.octaves {
            let sphere_grid_distance = scale * self.evaluate_sphere_grid_sdf(&position);

            let intersected_sphere_grid_distance = smooth_sdf_intersection(
                sphere_grid_distance,
                parent_distance - self.scaled_inflation * scale,
                self.scaled_intersection_smoothness.scaled(scale),
            );

            parent_distance = smooth_sdf_union(
                intersected_sphere_grid_distance,
                parent_distance,
                self.union_smoothness.scaled(scale),
            );

            position = ROTATION.transform_point(&(position / self.persistence));

            scale *= self.persistence;
        }
        parent_distance
    }

    #[inline]
    fn evaluate_sphere_grid_sdf(&self, position: &Point3) -> f32 {
        const CORNER_OFFSETS: [[i32; 3]; 8] = [
            [0, 0, 0],
            [0, 0, 1],
            [0, 1, 0],
            [0, 1, 1],
            [1, 0, 0],
            [1, 0, 1],
            [1, 1, 0],
            [1, 1, 1],
        ];
        let grid_cell_indices = [
            position.x().floor() as i32,
            position.y().floor() as i32,
            position.z().floor() as i32,
        ];
        let offset_in_grid_cell =
            position.as_vector() - Vector3::from(grid_cell_indices.map(|idx| idx as f32));

        CORNER_OFFSETS
            .iter()
            .map(|corner_offsets| {
                OrderedFloat(self.evaluate_corner_sphere_sdf(
                    &grid_cell_indices,
                    &offset_in_grid_cell,
                    corner_offsets,
                ))
            })
            .min()
            .unwrap()
            .0
    }

    #[inline]
    fn evaluate_corner_sphere_sdf(
        &self,
        grid_cell_indices: &[i32; 3],
        offset_in_grid_cell: &Vector3,
        corner_offsets: &[i32; 3],
    ) -> f32 {
        let sphere_radius = self.corner_sphere_radius(grid_cell_indices, corner_offsets);
        let distance_to_sphere_center =
            (offset_in_grid_cell - Vector3::from(corner_offsets.map(|idx| idx as f32))).norm();
        distance_to_sphere_center - sphere_radius
    }

    /// Every sphere gets a random radius based on its location in the grid.
    #[inline]
    fn corner_sphere_radius(&self, grid_cell_indices: &[i32; 3], corner_offsets: &[i32; 3]) -> f32 {
        // The maximum radius is half the extent of a grid cell, i.e. 0.5
        const HASH_TO_RADIUS: f32 = 0.5 / u32::MAX as f32;
        let hash = XxHash32::oneshot(
            self.seed,
            bytemuck::bytes_of(&array::from_fn::<i32, 3, _>(|idx| {
                grid_cell_indices[idx] + corner_offsets[idx]
            })),
        );
        HASH_TO_RADIUS * hash as f32
    }
}

impl Smoothness {
    #[inline]
    pub fn new(smoothness: f32) -> Self {
        Self {
            smoothness,
            quarter_inv_smoothness: 0.25 / smoothness,
        }
    }

    #[inline]
    pub fn get(&self) -> f32 {
        self.smoothness
    }

    #[inline]
    pub fn scaled(&self, scale: f32) -> Self {
        Self::new(self.smoothness * scale)
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.smoothness == 0.0
    }
}

impl From<f32> for Smoothness {
    #[inline]
    fn from(smoothness: f32) -> Self {
        Self::new(smoothness)
    }
}

#[inline]
fn zero_domain() -> AxisAlignedBox {
    AxisAlignedBox::new(Point3::origin(), Point3::origin())
}

/// When several SDF fields are blended with a soft operator, the smoothing
/// can push the resulting surface slightly outside the true union of the
/// input domains. This outward displacement does not only depend on the
/// smoothing factor itself, but also on how many leaf SDFs contribute to the
/// blend: combining more fields compounds the effect.
///
/// To capture this, the combined domain is expanded by an amount that grows
/// with both the smoothing factor and the number of leaf nodes beneath the
/// combination node.
#[inline]
fn soft_combine_domain_padding(smoothness: f32, leaf_count: u32) -> f32 {
    let local_padding = displacement_due_to_smoothness(smoothness);
    local_padding * (leaf_count as f32).log2()
}

#[inline]
fn displacement_due_to_smoothness(smoothness: f32) -> f32 {
    0.25 * smoothness
}

#[inline]
pub fn update_signed_distances_for_block<const SIZE: usize, const COUNT: usize>(
    signed_distances: &mut [f32; COUNT],
    transform_to_node_space: &Matrix4,
    block_origin_in_root_space: &Point3,
    update_signed_distance: &impl Fn(&mut f32, &Point3),
) {
    assert_eq!(COUNT, SIZE.pow(3));

    let origin = transform_to_node_space.transform_point(block_origin_in_root_space);
    let dx = transform_to_node_space.column1().xyz();
    let dy = transform_to_node_space.column2().xyz();
    let dz = transform_to_node_space.column3().xyz();

    let mut idx = 0;
    for i in 0..SIZE {
        let origin_plus_x = origin + (i as f32) * dx;
        for j in 0..SIZE {
            let mut position = origin_plus_x + (j as f32) * dy;
            for _ in 0..SIZE {
                let signed_distance = unsafe { signed_distances.get_unchecked_mut(idx) };
                update_signed_distance(signed_distance, &position);
                position += dz;
                idx += 1;
            }
        }
    }
}

#[inline]
fn all_block_test_positions_pass_predicate<const SIZE: usize, const COUNT: usize>(
    transform_to_node_space: &Matrix4,
    block_origin_in_root_space: &Point3,
    predicate: &impl Fn(usize, Point3) -> bool,
) -> bool {
    assert_eq!(COUNT, SIZE.pow(3));

    let lower = transform_to_node_space.transform_point(block_origin_in_root_space);
    let dx = transform_to_node_space.column1().xyz();
    let dy = transform_to_node_space.column2().xyz();
    let dz = transform_to_node_space.column3().xyz();

    for (idx, point) in all_block_test_positions_with_indices::<SIZE, COUNT>(&lower, &dx, &dy, &dz)
    {
        if !predicate(idx, point) {
            return false;
        }
    }
    true
}

#[inline]
fn all_block_test_positions_with_indices<const SIZE: usize, const COUNT: usize>(
    origin: &Point3,
    dx: &Vector3,
    dy: &Vector3,
    dz: &Vector3,
) -> impl Iterator<Item = (usize, Point3)> {
    block_corner_positions_with_indices::<SIZE, COUNT>(origin, dx, dy, dz)
        .into_iter()
        .chain(block_edge_midpoint_positions_with_indices::<SIZE, COUNT>(
            origin, dx, dy, dz,
        ))
        .chain(block_face_center_positions_with_indices::<SIZE, COUNT>(
            origin, dx, dy, dz,
        ))
}

#[inline]
fn block_corner_positions_with_indices<const SIZE: usize, const COUNT: usize>(
    origin: &Point3,
    dx: &Vector3,
    dy: &Vector3,
    dz: &Vector3,
) -> [(usize, Point3); 8] {
    let flat_idx = |i: usize, j: usize, k: usize| i * SIZE * SIZE + j * SIZE + k;

    let s = (SIZE - 1) as f32;

    [
        (flat_idx(0, 0, 0), *origin),
        (flat_idx(SIZE - 1, 0, 0), *origin + s * dx),
        (flat_idx(0, SIZE - 1, 0), *origin + s * dy),
        (flat_idx(0, 0, SIZE - 1), *origin + s * dz),
        (flat_idx(SIZE - 1, SIZE - 1, 0), *origin + s * (dx + dy)),
        (flat_idx(SIZE - 1, 0, SIZE - 1), *origin + s * (dx + dz)),
        (flat_idx(0, SIZE - 1, SIZE - 1), *origin + s * (dy + dz)),
        (
            flat_idx(SIZE - 1, SIZE - 1, SIZE - 1),
            *origin + s * (dx + dy + dz),
        ),
    ]
}

#[inline]
fn block_edge_midpoint_positions_with_indices<const SIZE: usize, const COUNT: usize>(
    origin: &Point3,
    dx: &Vector3,
    dy: &Vector3,
    dz: &Vector3,
) -> [(usize, Point3); 12] {
    let flat_idx = |i: usize, j: usize, k: usize| i * SIZE * SIZE + j * SIZE + k;

    let s = (SIZE - 1) as f32;
    let h = s * 0.5;

    [
        // X edges
        (flat_idx(0, 0, 0), *origin + h * dx),
        (flat_idx(0, SIZE - 1, 0), *origin + h * dx + s * dy),
        (flat_idx(0, 0, SIZE - 1), *origin + h * dx + s * dz),
        (
            flat_idx(0, SIZE - 1, SIZE - 1),
            *origin + h * dx + s * (dy + dz),
        ),
        // Y edges
        (flat_idx(0, 0, 0), *origin + h * dy),
        (flat_idx(SIZE - 1, 0, 0), *origin + h * dy + s * dx),
        (flat_idx(0, 0, SIZE - 1), *origin + h * dy + s * dz),
        (
            flat_idx(SIZE - 1, 0, SIZE - 1),
            *origin + h * dy + s * (dx + dz),
        ),
        // Z edges
        (flat_idx(0, 0, 0), *origin + h * dz),
        (flat_idx(SIZE - 1, 0, 0), *origin + h * dz + s * dx),
        (flat_idx(0, SIZE - 1, 0), *origin + h * dz + s * dy),
        (
            flat_idx(SIZE - 1, SIZE - 1, 0),
            *origin + h * dz + s * (dx + dy),
        ),
    ]
}

#[inline]
fn block_face_center_positions_with_indices<const SIZE: usize, const COUNT: usize>(
    origin: &Point3,
    dx: &Vector3,
    dy: &Vector3,
    dz: &Vector3,
) -> [(usize, Point3); 6] {
    let flat_idx = |i: usize, j: usize, k: usize| i * SIZE * SIZE + j * SIZE + k;

    let s = (SIZE - 1) as f32;
    let h = s * 0.5;

    [
        // X- faces
        (flat_idx(0, SIZE / 2, SIZE / 2), *origin + h * dy + h * dz),
        (
            flat_idx(SIZE - 1, SIZE / 2, SIZE / 2),
            *origin + s * dx + h * dy + h * dz,
        ),
        // Y- faces
        (flat_idx(SIZE / 2, 0, SIZE / 2), *origin + h * dx + h * dz),
        (
            flat_idx(SIZE / 2, SIZE - 1, SIZE / 2),
            *origin + s * dy + h * dx + h * dz,
        ),
        // Z- faces
        (flat_idx(SIZE / 2, SIZE / 2, 0), *origin + h * dx + h * dy),
        (
            flat_idx(SIZE / 2, SIZE / 2, SIZE - 1),
            *origin + s * dz + h * dx + h * dy,
        ),
    ]
}

#[inline]
fn apply_sdf_unions<const COUNT: usize>(
    distances_1: &mut [f32; COUNT],
    distances_2: &[f32; COUNT],
    smoothness: Smoothness,
) {
    if smoothness.is_zero() {
        for (distance_1, &distance_2) in distances_1.iter_mut().zip(distances_2.iter()) {
            *distance_1 = f32::min(*distance_1, distance_2);
        }
    } else {
        for (distance_1, &distance_2) in distances_1.iter_mut().zip(distances_2.iter()) {
            *distance_1 = smooth_sdf_union(*distance_1, distance_2, smoothness);
        }
    }
}

#[inline]
fn apply_sdf_subtractions<const COUNT: usize>(
    distances_1: &mut [f32; COUNT],
    distances_2: &[f32; COUNT],
    smoothness: Smoothness,
) {
    if smoothness.is_zero() {
        for (distance_1, &distance_2) in distances_1.iter_mut().zip(distances_2.iter()) {
            *distance_1 = f32::max(*distance_1, -distance_2);
        }
    } else {
        for (distance_1, &distance_2) in distances_1.iter_mut().zip(distances_2.iter()) {
            *distance_1 = smooth_sdf_subtraction(*distance_1, distance_2, smoothness);
        }
    }
}

#[inline]
fn apply_sdf_intersections<const COUNT: usize>(
    distances_1: &mut [f32; COUNT],
    distances_2: &[f32; COUNT],
    smoothness: Smoothness,
) {
    if smoothness.is_zero() {
        for (distance_1, &distance_2) in distances_1.iter_mut().zip(distances_2.iter()) {
            *distance_1 = f32::max(*distance_1, distance_2);
        }
    } else {
        for (distance_1, &distance_2) in distances_1.iter_mut().zip(distances_2.iter()) {
            *distance_1 = smooth_sdf_intersection(*distance_1, distance_2, smoothness);
        }
    }
}

#[inline]
fn sdf_union(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    if smoothness.is_zero() {
        f32::min(distance_1, distance_2)
    } else {
        smooth_sdf_union(distance_1, distance_2, smoothness)
    }
}

#[inline]
fn sdf_subtraction(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    if smoothness.is_zero() {
        f32::max(distance_1, -distance_2)
    } else {
        smooth_sdf_subtraction(distance_1, distance_2, smoothness)
    }
}

#[inline]
fn sdf_intersection(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    if smoothness.is_zero() {
        f32::max(distance_1, distance_2)
    } else {
        smooth_sdf_intersection(distance_1, distance_2, smoothness)
    }
}

#[inline]
fn smooth_sdf_union(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    let h = (smoothness.get() - (distance_1 - distance_2).abs()).max(0.0);
    distance_1.min(distance_2) - (h * h) * smoothness.quarter_inv_smoothness
}

#[inline]
fn smooth_sdf_subtraction(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    -smooth_sdf_union(-distance_1, distance_2, smoothness)
}

#[inline]
fn smooth_sdf_intersection(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    -smooth_sdf_union(-distance_1, -distance_2, smoothness)
}

/// Assumes underlying gradient noise in range [-1.0, 1.0].
#[inline]
fn theoretical_max_amplitude_of_fbm_noise(octaves: u32, persistence: f32) -> f32 {
    if abs_diff_ne!(persistence, 1.0, epsilon = 1e-6) {
        (1.0 - persistence.powi(octaves as i32)) / (1.0 - persistence)
    } else {
        octaves as f32
    }
}
