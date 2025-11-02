use impact::{
    egui::{Pos2, Rect, Vec2, pos2, vec2},
    impact_containers::{BitVector, FixedQueue},
};

pub trait LayoutableGraph {
    fn n_nodes(&self) -> usize;

    fn child_indices(&self, node_idx: usize) -> impl Iterator<Item = usize>;

    fn node_rect_mut(&mut self, node_idx: usize) -> &mut Rect;
}

#[derive(Clone, Debug)]
pub struct LayoutScratch {
    parent_counts: Vec<usize>,
    is_isolated: BitVector,
    node_layers: Vec<usize>,
    node_indices_in_visit_order: Vec<usize>,
    next_node_idx_in_layer: Vec<usize>,
    layer_heads: Vec<usize>,
    layer_tails: Vec<usize>,
    layer_max_heights: Vec<f32>,
    layer_top_y_coords: Vec<f32>,
    queue: FixedQueue<usize>,
}

impl LayoutScratch {
    pub fn new() -> Self {
        Self {
            parent_counts: Vec::new(),
            is_isolated: BitVector::new(),
            node_layers: Vec::new(),
            node_indices_in_visit_order: Vec::new(),
            next_node_idx_in_layer: Vec::new(),
            layer_heads: Vec::new(),
            layer_tails: Vec::new(),
            layer_max_heights: Vec::new(),
            layer_top_y_coords: Vec::new(),
            queue: FixedQueue::with_capacity(0),
        }
    }

    fn ensure_node_capacity(&mut self, n_nodes: usize) {
        self.parent_counts.clear();
        self.parent_counts.resize(n_nodes, 0);

        self.is_isolated.resize_and_unset_all(n_nodes);

        self.node_layers.clear();
        self.node_layers.resize(n_nodes, 0);

        self.node_indices_in_visit_order.clear();
        self.node_indices_in_visit_order.reserve(n_nodes);

        self.next_node_idx_in_layer.clear();
        self.next_node_idx_in_layer.resize(n_nodes, usize::MAX);

        self.queue.clear_and_set_capacity(n_nodes);
    }

    fn ensure_layer_capacity(&mut self, n_layers: usize) {
        self.layer_heads.clear();
        self.layer_heads.resize(n_layers, usize::MAX);

        self.layer_tails.clear();
        self.layer_tails.resize(n_layers, usize::MAX);

        self.layer_max_heights.clear();
        self.layer_max_heights.resize(n_layers, 0.0);

        self.layer_top_y_coords.clear();
        self.layer_top_y_coords.resize(n_layers, 0.0);
    }
}

/// The top center of the root node will be placed at the provided origin.
pub fn layout_vertical(
    scratch: &mut LayoutScratch,
    graph: &mut impl LayoutableGraph,
    origin: Pos2,
    horizontal_gap: f32,
    vertical_gap: f32,
) {
    let n_nodes = graph.n_nodes();
    if n_nodes == 0 {
        return;
    }

    scratch.ensure_node_capacity(n_nodes);

    // Count parents for each node
    for node_idx in 0..n_nodes {
        let mut has_child = false;
        for child_idx in graph.child_indices(node_idx) {
            scratch.parent_counts[child_idx] += 1;
            has_child = true;
        }
        if !has_child {
            // We tentatively mark leaf nodes as isolated. We will correct any
            // leaf nodes that have parents to be non-isolated in a separate
            // pass.
            scratch.is_isolated.set_bit(node_idx);
        }
    }

    // Correct any leaf nodes that have parents to be non-isolated
    for node_idx in 0..n_nodes {
        if scratch.is_isolated.bit_is_set(node_idx) && scratch.parent_counts[node_idx] > 0 {
            scratch.is_isolated.unset_bit(node_idx);
        }
    }

    // Queue all root nodes that have children
    let mut has_root = false;
    for node_idx in 0..n_nodes {
        if scratch.parent_counts[node_idx] == 0 {
            has_root = true;
            if !scratch.is_isolated.bit_is_set(node_idx) {
                scratch.queue.push_back(node_idx);
            }
        }
    }
    assert!(has_root, "Graph must have at least one root");

    // Traverse in topological order to determine visit order and the layer
    // (depth) for each node
    while let Some(node_idx) = scratch.queue.pop_front() {
        scratch.node_indices_in_visit_order.push(node_idx);
        let next_layer = scratch.node_layers[node_idx] + 1;

        for child_idx in graph.child_indices(node_idx) {
            // Keep the deepest layer seen from any parent
            scratch.node_layers[child_idx] = scratch.node_layers[child_idx].max(next_layer);

            // Decrement remaining parent count and enqueue when ready
            scratch.parent_counts[child_idx] -= 1;
            if scratch.parent_counts[child_idx] == 0 {
                scratch.queue.push_back(child_idx);
            }
        }
    }

    let mut max_layer = scratch.node_layers.iter().copied().max().unwrap();

    // Put all isolated nodes in a separate layer at the bottom
    let mut has_isolated = false;
    for node_idx in 0..n_nodes {
        if scratch.is_isolated.bit_is_set(node_idx) {
            has_isolated = true;
            scratch.node_layers[node_idx] = max_layer + 1;
            scratch.node_indices_in_visit_order.push(node_idx);
        }
    }

    if has_isolated {
        max_layer += 1;
    }

    assert_eq!(
        scratch.node_indices_in_visit_order.len(),
        n_nodes,
        "Graph contains a cycle"
    );

    // Create a linked list of nodes in visit order for each layer

    let n_layers = max_layer + 1;
    scratch.ensure_layer_capacity(n_layers);

    for &node_idx in &scratch.node_indices_in_visit_order {
        let layer = scratch.node_layers[node_idx];
        let tail_node_idx = scratch.layer_tails[layer];
        let layer_empty = tail_node_idx == usize::MAX;
        if layer_empty {
            // If this is the first node in the layer, initialize the layer's
            // linked list by assigning the node as the head and tail
            scratch.layer_heads[layer] = node_idx;
            scratch.layer_tails[layer] = node_idx;
        } else {
            // If there are already nodes in the layer, add a link from the old
            // tail to the current node and set the current node as the new tail
            scratch.next_node_idx_in_layer[tail_node_idx] = node_idx;
            scratch.layer_tails[layer] = node_idx;
        }
    }

    // Determine the maximum height of the nodes in each layer
    for layer in 0..n_layers {
        let mut max_height = 0.0_f32;
        for_node_in_layer(
            scratch.layer_heads[layer],
            &scratch.next_node_idx_in_layer,
            |node_idx| {
                max_height = max_height.max(graph.node_rect_mut(node_idx).height());
            },
        );
        scratch.layer_max_heights[layer] = max_height;
    }

    // Determine the y-coordinate of the top of each layer, with the top of the
    // first layer begin at the origin
    let mut y = origin.y;
    for layer in 0..n_layers {
        scratch.layer_top_y_coords[layer] = y;
        y += scratch.layer_max_heights[layer] + vertical_gap;
    }

    // Lay out the nodes in each layer from left to right
    for layer in 0..n_layers {
        let mut sum_of_widths = 0.0;
        let mut node_count = 0_usize;
        for_node_in_layer(
            scratch.layer_heads[layer],
            &scratch.next_node_idx_in_layer,
            |node_idx| {
                sum_of_widths += graph.node_rect_mut(node_idx).width();
                node_count += 1;
            },
        );

        let total_gap_width = horizontal_gap * node_count.saturating_sub(1) as f32;
        let total_width = sum_of_widths + total_gap_width;

        let top_y = scratch.layer_top_y_coords[layer];
        let max_height = scratch.layer_max_heights[layer];

        // Center the row of nodes on the origin
        let mut x = origin.x - 0.5 * total_width;

        for_node_in_layer(
            scratch.layer_heads[layer],
            &scratch.next_node_idx_in_layer,
            |node_idx| {
                let node_rect = graph.node_rect_mut(node_idx);
                let size = node_rect.size();
                // Center the node vertically on the row center
                let y = top_y + 0.5 * (max_height - size.y);
                *node_rect = Rect::from_min_size(pos2(x, y), size);
                x += size.x + horizontal_gap;
            },
        );
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
