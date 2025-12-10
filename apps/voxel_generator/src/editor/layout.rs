use impact::{
    egui::{Pos2, Rect, Vec2, pos2, vec2},
    impact_alloc::{AVec, arena::ArenaPool, avec},
    impact_containers::{BitVector, FixedQueue},
};

pub trait LayoutableGraph {
    fn n_nodes(&self) -> usize;

    fn child_indices(&self, node_idx: usize) -> impl Iterator<Item = usize>;

    fn node_rect_mut(&mut self, node_idx: usize) -> &mut Rect;
}

/// The top center of the root node will be placed at the provided origin.
pub fn layout_vertical(
    graph: &mut impl LayoutableGraph,
    origin: Pos2,
    horizontal_gap: f32,
    vertical_gap: f32,
) {
    let n_nodes = graph.n_nodes();
    if n_nodes == 0 {
        return;
    }

    let arena = ArenaPool::get_arena();
    let mut parent_counts = avec![in &arena; 0; n_nodes];
    let mut is_isolated = BitVector::zeroed_in(n_nodes, &arena);
    let mut node_layers = avec![in &arena; 0; n_nodes];
    let mut node_indices_in_visit_order = AVec::with_capacity_in(n_nodes, &arena);
    let mut next_node_idx_in_layer = avec![in &arena; usize::MAX; n_nodes];
    let mut queue = FixedQueue::with_capacity_in(n_nodes, &arena);

    // Count parents for each node
    let mut no_nodes_have_children = true;
    for node_idx in 0..n_nodes {
        let mut has_child = false;
        for child_idx in graph.child_indices(node_idx) {
            parent_counts[child_idx] += 1;
            has_child = true;
        }
        if !has_child {
            // We tentatively mark leaf nodes as isolated. We will correct any
            // leaf nodes that have parents to be non-isolated in a separate
            // pass.
            is_isolated.set_bit(node_idx);
        } else {
            no_nodes_have_children = false;
        }
    }

    // Correct any leaf nodes that have parents (are not roots) to be
    // non-isolated. Also make sure the primary (first) root is never marked as
    // isolated and that all other roots will be put on a separate layer.
    let mut primary_root_idx = None;
    for node_idx in 0..n_nodes {
        let is_root = parent_counts[node_idx] == 0;
        let is_first_root = is_root && primary_root_idx.is_none();

        if is_isolated.bit_is_set(node_idx) && (is_first_root || !is_root) {
            is_isolated.unset_bit(node_idx);
        }

        if is_root {
            primary_root_idx = Some(node_idx);
        }

        if is_root && !is_first_root {
            node_layers[node_idx] = 1;
        }
    }

    // Queue all unisolated root nodes
    let mut has_root = false;
    for node_idx in 0..n_nodes {
        if parent_counts[node_idx] == 0 {
            has_root = true;
            if !is_isolated.bit_is_set(node_idx) {
                queue.push_back(node_idx);
            }
        }
    }
    assert!(has_root, "Graph must have at least one root");

    // Traverse in topological order to determine visit order and the layer
    // (depth) for each node
    while let Some(node_idx) = queue.pop_front() {
        node_indices_in_visit_order.push(node_idx);
        let next_layer = node_layers[node_idx] + 1;

        for child_idx in graph.child_indices(node_idx) {
            // Keep the deepest layer seen from any parent
            node_layers[child_idx] = node_layers[child_idx].max(next_layer);

            // Decrement remaining parent count and enqueue when ready
            parent_counts[child_idx] -= 1;
            if parent_counts[child_idx] == 0 {
                queue.push_back(child_idx);
            }
        }
    }

    let mut max_layer = node_layers.iter().copied().max().unwrap();

    let isolated_layer = if no_nodes_have_children {
        1
    } else {
        max_layer + 1
    };

    // Put all isolated nodes in a separate layer at the bottom
    let mut has_isolated = false;
    for node_idx in 0..n_nodes {
        if is_isolated.bit_is_set(node_idx) {
            has_isolated = true;
            node_layers[node_idx] = isolated_layer;
            node_indices_in_visit_order.push(node_idx);
        }
    }

    if has_isolated {
        max_layer = isolated_layer;
    }

    assert_eq!(
        node_indices_in_visit_order.len(),
        n_nodes,
        "Graph contains a cycle"
    );

    // Create a linked list of nodes in visit order for each layer

    let n_layers = max_layer + 1;

    let mut layer_heads = avec![in &arena; usize::MAX; n_layers];
    let mut layer_tails = avec![in &arena; usize::MAX; n_layers];
    let mut layer_max_heights = avec![in &arena; 0.0; n_layers];
    let mut layer_top_y_coords = avec![in &arena; 0.0; n_layers];

    for &node_idx in &node_indices_in_visit_order {
        let layer = node_layers[node_idx];
        let tail_node_idx = layer_tails[layer];
        let layer_empty = tail_node_idx == usize::MAX;
        if layer_empty {
            // If this is the first node in the layer, initialize the layer's
            // linked list by assigning the node as the head and tail
            layer_heads[layer] = node_idx;
            layer_tails[layer] = node_idx;
        } else {
            // If there are already nodes in the layer, add a link from the old
            // tail to the current node and set the current node as the new tail
            next_node_idx_in_layer[tail_node_idx] = node_idx;
            layer_tails[layer] = node_idx;
        }
    }

    // Determine the maximum height of the nodes in each layer
    for layer in 0..n_layers {
        let mut max_height = 0.0_f32;
        for_node_in_layer(layer_heads[layer], &next_node_idx_in_layer, |node_idx| {
            max_height = max_height.max(graph.node_rect_mut(node_idx).height());
        });
        layer_max_heights[layer] = max_height;
    }

    // Determine the y-coordinate of the top of each layer, with the top of the
    // first layer begin at the origin
    let mut y = origin.y;
    for layer in 0..n_layers {
        layer_top_y_coords[layer] = y;
        y += layer_max_heights[layer] + vertical_gap;
    }

    // Lay out the nodes in each layer from left to right
    for layer in 0..n_layers {
        let mut sum_of_widths = 0.0;
        let mut node_count = 0_usize;
        for_node_in_layer(layer_heads[layer], &next_node_idx_in_layer, |node_idx| {
            sum_of_widths += graph.node_rect_mut(node_idx).width();
            node_count += 1;
        });

        let total_gap_width = horizontal_gap * node_count.saturating_sub(1) as f32;
        let total_width = sum_of_widths + total_gap_width;

        let top_y = layer_top_y_coords[layer];
        let max_height = layer_max_heights[layer];

        // Center the row of nodes on the origin
        let mut x = origin.x - 0.5 * total_width;

        for_node_in_layer(layer_heads[layer], &next_node_idx_in_layer, |node_idx| {
            let node_rect = graph.node_rect_mut(node_idx);
            let size = node_rect.size();
            // Center the node vertically on the row center
            let y = top_y + 0.5 * (max_height - size.y);
            *node_rect = Rect::from_min_size(pos2(x, y), size);
            x += size.x + horizontal_gap;
        });
    }
}

#[inline]
fn for_node_in_layer(
    head_node_idx: usize,
    next_node_idx_in_layer: &[usize],
    mut f: impl FnMut(usize),
) {
    let mut node_idx = head_node_idx;
    while node_idx != usize::MAX {
        f(node_idx);
        node_idx = next_node_idx_in_layer[node_idx];
    }
}

pub fn compute_delta_to_resolve_overlaps<ID, I>(
    get_node_rects: impl Fn() -> I,
    moved_node_id: ID,
    moved_node_rect: Rect,
    min_separation: f32,
) -> Vec2
where
    ID: PartialEq,
    I: IntoIterator<Item = (ID, Rect)>,
{
    let expansion = vec2(0.5 * min_separation, 0.5 * min_separation);

    let mut moved_node_rect = moved_node_rect.expand2(expansion);
    let mut total_delta = Vec2::ZERO;

    // Multiple iterations in case we push into someone else
    for _ in 0..64 {
        let mut moved = false;

        for (node_id, node_rect) in get_node_rects() {
            if node_id == moved_node_id {
                continue;
            }
            let node_rect = node_rect.expand2(expansion);

            if moved_node_rect.intersects(node_rect) {
                // Compute minimal push on x or y to separate
                let dx_left = node_rect.left() - moved_node_rect.right(); // negative
                let dx_right = node_rect.right() - moved_node_rect.left(); // positive
                let dy_up = node_rect.top() - moved_node_rect.bottom(); // negative
                let dy_down = node_rect.bottom() - moved_node_rect.top(); // positive

                // Amount to move moved_node_rect so it *just* stops intersecting
                let push_x = if dx_right.abs() < dx_left.abs() {
                    dx_right
                } else {
                    dx_left
                };
                let push_y = if dy_down.abs() < dy_up.abs() {
                    dy_down
                } else {
                    dy_up
                };

                // Choose smallest magnitude axis (separating axis)
                let (mx, my) = (push_x.abs(), push_y.abs());
                let delta = if mx < my {
                    vec2(push_x, 0.0)
                } else {
                    vec2(0.0, push_y)
                };

                // Apply to the node’s world position
                total_delta += delta;
                moved_node_rect = moved_node_rect.translate(delta);
                moved = true;
            }
        }

        if !moved {
            break;
        }
    }

    total_delta
}
