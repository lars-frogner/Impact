//! Bounding volume hierarchy construction using a naive bottom-up method.

use crate::bounding_volume::hierarchy::{Node, NodeID, NodePayload};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_geometry::AxisAlignedBoxC;
use std::mem;

pub fn build(nodes: &mut Vec<Node>, primitive_aabbs: &[AxisAlignedBoxC]) -> Option<NodeID> {
    nodes.clear();

    if primitive_aabbs.is_empty() {
        return None;
    }

    let arena = ArenaPool::get_arena_for_capacity(primitive_aabbs.len() * mem::size_of::<NodeID>());

    let mut uncombined_node_ids = AVec::with_capacity_in(primitive_aabbs.len(), &arena);

    for (idx, aabb) in primitive_aabbs.iter().enumerate() {
        let node = Node::new(aabb.clone(), NodePayload::Primitive { idx });
        nodes.push(node);
        uncombined_node_ids.push(idx);
    }

    let mut next_node_id = primitive_aabbs.len();

    while uncombined_node_ids.len() > 1 {
        let ((best_i, best_j), combined_aabb) =
            find_best_combined_pair(nodes, &uncombined_node_ids);

        let combined_node_id = next_node_id;
        let left_node_id = uncombined_node_ids[best_i];
        let right_node_id = uncombined_node_ids[best_j];

        let node = Node::new(
            combined_aabb,
            NodePayload::Children {
                left_id: left_node_id,
                right_id: right_node_id,
            },
        );
        nodes.push(node);
        next_node_id += 1;

        uncombined_node_ids[best_i] = combined_node_id;

        let last_node_id = uncombined_node_ids.pop().unwrap();
        if right_node_id != last_node_id {
            uncombined_node_ids[best_j] = last_node_id;
        }
    }

    let root_node_id = uncombined_node_ids.pop();

    root_node_id
}

fn find_best_combined_pair(
    nodes: &[Node],
    uncombined_node_ids: &[NodeID],
) -> ((usize, usize), AxisAlignedBoxC) {
    let n_remaining = uncombined_node_ids.len();

    let mut smallest_dist = f32::INFINITY;
    let mut best_i = 0;
    let mut best_j = 0;
    let mut best_combined_aabb = AxisAlignedBoxC::default();

    for (i, node_i_id) in (0..n_remaining - 1).zip(&uncombined_node_ids[0..n_remaining - 1]) {
        let aabb_i = &nodes[*node_i_id].aabb;

        for (j, node_j_id) in (i + 1..n_remaining).zip(&uncombined_node_ids[i + 1..n_remaining]) {
            let aabb_j = &nodes[*node_j_id].aabb;

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
