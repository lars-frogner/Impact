//! Bounding volume hierarchy construction using a naive bottom-up method.

use crate::bounding_volume::hierarchy::{Node, NodePayload};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_geometry::AxisAlignedBoxC;
use std::mem;

pub fn build(nodes: &mut Vec<Node>, primitive_aabbs: &[AxisAlignedBoxC]) -> Option<usize> {
    nodes.clear();

    let arena = ArenaPool::get_arena_for_capacity(primitive_aabbs.len() * mem::size_of::<usize>());

    let mut remaining_node_indices = AVec::with_capacity_in(primitive_aabbs.len(), &arena);

    for (idx, aabb) in primitive_aabbs.iter().enumerate() {
        let node = Node::new(aabb.clone(), NodePayload::Primitive);
        nodes.push(node);
        remaining_node_indices.push(idx);
    }

    let mut next_node_idx = primitive_aabbs.len();

    while remaining_node_indices.len() > 1 {
        let ((best_i, best_j), combined_aabb) =
            find_best_combined_pair(nodes, &remaining_node_indices);

        let combined_node_idx = next_node_idx;
        let left_node_idx = remaining_node_indices[best_i];
        let right_node_idx = remaining_node_indices[best_j];

        let node = Node::new(
            combined_aabb,
            NodePayload::Children {
                left_idx: left_node_idx,
                right_idx: right_node_idx,
            },
        );
        nodes.push(node);
        next_node_idx += 1;

        remaining_node_indices[best_i] = combined_node_idx;

        let last_node_idx = remaining_node_indices.pop().unwrap();
        if right_node_idx != last_node_idx {
            remaining_node_indices[best_j] = last_node_idx;
        }
    }

    let root_node_idx = remaining_node_indices.pop();

    root_node_idx
}

fn find_best_combined_pair(
    nodes: &[Node],
    remaining_node_indices: &[usize],
) -> ((usize, usize), AxisAlignedBoxC) {
    let n_remaining = remaining_node_indices.len();

    let mut smallest_dist = f32::INFINITY;
    let mut best_i = 0;
    let mut best_j = 0;
    let mut best_combined_aabb = AxisAlignedBoxC::default();

    for (i, node_i_idx) in (0..n_remaining - 1).zip(&remaining_node_indices[0..n_remaining - 1]) {
        let aabb_i = &nodes[*node_i_idx].aabb;

        for (j, node_j_idx) in (i + 1..n_remaining).zip(&remaining_node_indices[i + 1..n_remaining])
        {
            let aabb_j = &nodes[*node_j_idx].aabb;

            let combined_aabb = AxisAlignedBoxC::aabb_from_pair(aabb_i, aabb_j);
            let dist = evaluate_distance_metric(&combined_aabb);

            if dist < smallest_dist {
                smallest_dist = dist;
                best_i = i;
                best_j = j;
                best_combined_aabb = combined_aabb;
            }
        }
    }

    ((best_i, best_j), best_combined_aabb)
}

fn evaluate_distance_metric(combined_aabb: &AxisAlignedBoxC) -> f32 {
    combined_aabb.volume()
}
