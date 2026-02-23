//! Bounding volume hierarchy construction using Approximate Agglomerative
//! Clustering.

use crate::bounding_volume::hierarchy::{Node, NodeID, NodePayload};
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_geometry::AxisAlignedBoxC;
use impact_math::{morton::MortonEncoder63Bit3D, sorting, vector::Vector3C};
use std::mem;

pub fn build(nodes: &mut Vec<Node>, primitive_aabbs: &[AxisAlignedBoxC]) -> Option<usize> {
    nodes.clear();

    if primitive_aabbs.is_empty() {
        return None;
    }
    let n_primitives = primitive_aabbs.len();

    let arena = ArenaPool::get_arena_for_capacity(
        n_primitives * (mem::size_of::<Primitive>() + 2 * mem::size_of::<u64>()),
    );

    let center_morton_codes = compute_center_morton_codes(&arena, primitive_aabbs);

    let max_morton_code = center_morton_codes.iter().copied().max().unwrap();
    let max_significant_morton_bits = 64 - max_morton_code.leading_zeros();

    let mut sorted_primitives = AVec::with_capacity_in(n_primitives, &arena);
    let mut sorted_morton_codes = AVec::with_capacity_in(n_primitives, &arena);

    sorting::radix_sort_by_u64_keys_with_max_significant_bits(
        &center_morton_codes,
        max_significant_morton_bits,
        |src_idx, _| {
            let aabb = primitive_aabbs[src_idx].clone();
            sorted_primitives.push(Primitive { idx: src_idx, aabb });
            sorted_morton_codes.push(center_morton_codes[src_idx]);
        },
    );

    let mut uncombined_nodes = AVec::new_in(&arena);

    let partition_bit = (max_significant_morton_bits > 0).then(|| max_significant_morton_bits - 1);

    build_tree(
        nodes,
        &mut uncombined_nodes,
        &sorted_primitives,
        &sorted_morton_codes,
        partition_bit,
        4,
    );

    combine_nodes(nodes, &mut uncombined_nodes, 1);

    assert_eq!(uncombined_nodes.len(), 1);
    Some(uncombined_nodes[0].id)
}

fn compute_center_morton_codes<A: Allocator>(
    alloc: A,
    primitive_aabbs: &[AxisAlignedBoxC],
) -> AVec<u64, A> {
    let mut center_morton_codes = AVec::with_capacity_in(primitive_aabbs.len(), alloc);

    let morton_encoder = create_morton_encoder(primitive_aabbs);

    for aabb in primitive_aabbs {
        let center = aabb.center();
        let code = morton_encoder.encode(center.as_vector());
        center_morton_codes.push(code);
    }

    center_morton_codes
}

fn create_morton_encoder(primitive_aabbs: &[AxisAlignedBoxC]) -> MortonEncoder63Bit3D {
    let (min_coords, max_coords) = determine_bounds_for_morton_codes(primitive_aabbs);
    MortonEncoder63Bit3D::new(&min_coords, &max_coords)
}

fn determine_bounds_for_morton_codes(primitive_aabbs: &[AxisAlignedBoxC]) -> (Vector3C, Vector3C) {
    let mut min = Vector3C::same(f32::INFINITY);
    let mut max = Vector3C::same(f32::NEG_INFINITY);

    for aabb in primitive_aabbs {
        let center = aabb.center();
        min = min.component_min(center.as_vector());
        max = max.component_max(center.as_vector());
    }

    (min, max)
}

/// Takes primitive AABBs sorted by the Morton codes of their centers. Uses the
/// sorted Morton codes to recursively partition primitives into groups along
/// alternating cartesian axes. When a group becomes smaller than
/// `traversal_stopping_threshold`, leaf nodes are created for the primitives,
/// and these are greedily combined into parent nodes until the number of
/// uncombined nodes for the group is no longer larger than a limit determined
/// by [`eval_cluster_count_reduction_function`].
fn build_tree<A: Allocator>(
    nodes: &mut Vec<Node>,
    uncombined_nodes: &mut AVec<UncombinedNode, A>,
    primitives: &[Primitive],
    morton_codes: &[u64],
    partition_bit: Option<u32>,
    traversal_stopping_threshold: usize,
) {
    assert!(!primitives.is_empty());
    assert_eq!(primitives.len(), morton_codes.len());

    let n_primitives = primitives.len();

    if n_primitives >= traversal_stopping_threshold {
        let n_primitives_in_left_group = partition_morton_codes(morton_codes, partition_bit);

        let next_partition_bit =
            partition_bit.and_then(|bit| if bit > 0 { Some(bit - 1) } else { None });

        if n_primitives_in_left_group > 0 {
            build_tree(
                nodes,
                uncombined_nodes,
                &primitives[..n_primitives_in_left_group],
                &morton_codes[..n_primitives_in_left_group],
                next_partition_bit,
                traversal_stopping_threshold,
            );
        }
        if n_primitives_in_left_group < n_primitives {
            build_tree(
                nodes,
                uncombined_nodes,
                &primitives[n_primitives_in_left_group..],
                &morton_codes[n_primitives_in_left_group..],
                next_partition_bit,
                traversal_stopping_threshold,
            );
        }
    } else {
        // The current group of primitives is small enough that we can begin
        // combining them
        create_primitive_nodes(nodes, uncombined_nodes, primitives);
    }

    let max_remaining_uncombined = eval_cluster_count_reduction_function(n_primitives);
    combine_nodes(nodes, uncombined_nodes, max_remaining_uncombined);
}

/// Finds the index in a sorted Morton code list where the bit at the specified
/// position switches from 0 to 1. If no partition bit is supplied, partitions
/// the list in half.
fn partition_morton_codes(morton_codes: &[u64], partition_bit: Option<u32>) -> usize {
    if let Some(partition_bit) = partition_bit {
        let partition_bit_mask = 1 << partition_bit;
        morton_codes.partition_point(|code| *code & partition_bit_mask == 0)
    } else {
        // We can't partition further by Morton code, so just partition the list
        // in half
        morton_codes.len() / 2
    }
}

/// Creates uncombined leaf nodes for the given primitives.
fn create_primitive_nodes<A: Allocator>(
    nodes: &mut Vec<Node>,
    uncombined_nodes: &mut AVec<UncombinedNode, A>,
    primitives: &[Primitive],
) {
    let mut next_node_id = nodes.len();

    for primitive in primitives {
        let node = Node::new(
            primitive.aabb.clone(),
            NodePayload::Primitive { idx: primitive.idx },
        );
        nodes.push(node);
        uncombined_nodes.push(UncombinedNode::new(next_node_id));
        next_node_id += 1;
    }
}

fn combine_nodes<A: Allocator>(
    nodes: &mut Vec<Node>,
    uncombined_nodes: &mut AVec<UncombinedNode, A>,
    max_remaining_uncombined: usize,
) {
    assert!(max_remaining_uncombined > 0);

    let mut remaining_uncombined = uncombined_nodes.len();

    if remaining_uncombined <= max_remaining_uncombined {
        return;
    }

    let mut next_node_id = nodes.len();

    // Find closest partner for each node
    // TODO: don't do double work
    for uncombined_node_idx in 0..remaining_uncombined {
        let (closest_other_uncombined_node_idx, smallest_dist) =
            find_closest_other_uncombined_node(nodes, uncombined_nodes, uncombined_node_idx);

        let uncombined_node = &mut uncombined_nodes[uncombined_node_idx];
        uncombined_node.closest_idx = closest_other_uncombined_node_idx;
        uncombined_node.dist_to_closest = smallest_dist;
    }

    while remaining_uncombined > max_remaining_uncombined {
        // Find the pair with the smallest distance

        let mut smallest_dist = f32::INFINITY;
        let mut best_i = 0;
        let mut best_j = 0;

        for (uncombined_node_idx, uncombined_node) in uncombined_nodes.iter().enumerate() {
            if uncombined_node.dist_to_closest < smallest_dist {
                smallest_dist = uncombined_node.dist_to_closest;
                best_i = uncombined_node_idx;
                best_j = uncombined_node.closest_idx;
            }
        }

        // Create a new node by combining that pair

        let combination_node_id = next_node_id;
        let left_node_id = uncombined_nodes[best_i].id;
        let right_node_id = uncombined_nodes[best_j].id;

        let combined_aabb =
            AxisAlignedBoxC::aabb_from_pair(&nodes[left_node_id].aabb, &nodes[right_node_id].aabb);

        let node = Node::new(
            combined_aabb,
            NodePayload::Children {
                left_id: left_node_id,
                right_id: right_node_id,
            },
        );
        nodes.push(node);
        next_node_id += 1;

        let combination_node_idx = best_i;
        uncombined_nodes[combination_node_idx] = UncombinedNode::new(combination_node_id);

        let last_uncombined_node = uncombined_nodes.pop().unwrap();
        if right_node_id != last_uncombined_node.id {
            uncombined_nodes[best_j] = last_uncombined_node;
        }

        remaining_uncombined = uncombined_nodes.len();

        if remaining_uncombined < 2 {
            continue;
        }

        // Find the closest partner for the new node

        let (closest_other_uncombined_node_idx, smallest_dist) =
            find_closest_other_uncombined_node(nodes, uncombined_nodes, best_i);

        let uncombined_node = &mut uncombined_nodes[best_i];
        uncombined_node.closest_idx = closest_other_uncombined_node_idx;
        uncombined_node.dist_to_closest = smallest_dist;

        // For each node that had one of the newly combined nodes as the closest
        // partner, find the new closest partner

        for uncombined_node_idx in 0..remaining_uncombined {
            let uncombined_node = &mut uncombined_nodes[uncombined_node_idx];
            if uncombined_node.closest_idx == best_i || uncombined_node.closest_idx == best_j {
                let (closest_other_uncombined_node_idx, smallest_dist) =
                    find_closest_other_uncombined_node(
                        nodes,
                        uncombined_nodes,
                        uncombined_node_idx,
                    );

                let uncombined_node = &mut uncombined_nodes[uncombined_node_idx];
                uncombined_node.closest_idx = closest_other_uncombined_node_idx;
                uncombined_node.dist_to_closest = smallest_dist;
            } else if uncombined_node.closest_idx == remaining_uncombined {
                // We moved the last uncombined node to `best_j`, so we must fix
                // up any references to that node
                uncombined_node.closest_idx = best_j;
            }
        }
    }
}

fn eval_cluster_count_reduction_function(n_primitives: usize) -> usize {
    n_primitives.div_ceil(2)
}

fn find_closest_other_uncombined_node(
    nodes: &[Node],
    uncombined_nodes: &[UncombinedNode],
    uncombined_node_idx: usize,
) -> (usize, f32) {
    let n_uncombined = uncombined_nodes.len();

    assert!(n_uncombined > 1);

    let uncombined_node = &uncombined_nodes[uncombined_node_idx];
    let aabb = &nodes[uncombined_node.id].aabb;

    let mut smallest_dist = f32::INFINITY;
    let mut closest_other_uncombined_node_idx = 0;

    for (other_uncombined_node_idx, &other_uncombined_node) in (0..uncombined_node_idx)
        .zip(&uncombined_nodes[..uncombined_node_idx])
        .chain((uncombined_node_idx + 1..).zip(&uncombined_nodes[uncombined_node_idx + 1..]))
    {
        let other_aabb = &nodes[other_uncombined_node.id].aabb;

        let combined_aabb = AxisAlignedBoxC::aabb_from_pair(aabb, other_aabb);
        let dist = evaluate_distance_metric(&combined_aabb);

        if dist < smallest_dist {
            smallest_dist = dist;
            closest_other_uncombined_node_idx = other_uncombined_node_idx;
        }
    }

    (closest_other_uncombined_node_idx, smallest_dist)
}

#[inline]
fn evaluate_distance_metric(combined_aabb: &AxisAlignedBoxC) -> f32 {
    combined_aabb.volume()
}

#[derive(Clone, Debug)]
struct Primitive {
    /// Index in the original primitive list.
    idx: usize,
    aabb: AxisAlignedBoxC,
}

#[derive(Clone, Copy, Debug)]
struct UncombinedNode {
    id: NodeID,
    closest_idx: NodeID,
    dist_to_closest: f32,
}

impl UncombinedNode {
    #[inline]
    fn new(id: NodeID) -> Self {
        Self {
            id,
            closest_idx: 0,
            dist_to_closest: 0.0,
        }
    }
}
