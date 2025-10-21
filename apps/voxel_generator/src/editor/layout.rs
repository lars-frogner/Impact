use impact::egui::{Rect, Vec2, vec2};

const MIN_NODE_SEPARATION: f32 = 8.0;

pub fn compute_delta_to_resolve_overlaps<ID, I>(
    get_node_rects: impl Fn() -> I,
    moved_node_id: ID,
    moved_node_rect: Rect,
) -> Vec2
where
    ID: PartialEq,
    I: IntoIterator<Item = (ID, Rect)>,
{
    const EXPANSION: Vec2 = vec2(MIN_NODE_SEPARATION * 0.5, MIN_NODE_SEPARATION * 0.5);

    let mut moved_node_rect = moved_node_rect.expand2(EXPANSION);
    let mut total_delta = Vec2::ZERO;

    // Multiple iterations in case we push into someone else
    for _ in 0..64 {
        let mut moved = false;

        for (node_id, node_rect) in get_node_rects() {
            if node_id == moved_node_id {
                continue;
            }
            let node_rect = node_rect.expand2(EXPANSION);

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
