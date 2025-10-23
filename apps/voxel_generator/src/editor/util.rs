use impact::egui::{
    Color32, Pos2,
    epaint::{CubicBezierShape, PathStroke},
};

const CURVATURE_FACTOR: f32 = 0.7;
const MIN_CURVATURE: f32 = 12.0;

pub fn create_bezier_edge(
    child_pos: Pos2,
    parent_pos: Pos2,
    stroke: PathStroke,
) -> CubicBezierShape {
    // Vertical distance guides curvature
    let dy = (parent_pos.y - child_pos.y).abs();

    let ctrl = (dy * CURVATURE_FACTOR).max(MIN_CURVATURE);

    let p0 = child_pos;
    let p3 = parent_pos;

    // Vertical tangents: out of parent downward, into child upward
    let p1 = Pos2::new(p0.x, p0.y + ctrl);
    let p2 = Pos2::new(p3.x, p3.y - ctrl);

    CubicBezierShape::from_points_stroke([p0, p1, p2, p3], false, Color32::TRANSPARENT, stroke)
}
