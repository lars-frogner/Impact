//! Lines and line segments.

use impact_math::{point::Point3, vector::Vector3};

/// Returns the point on the given line segment that is closest to the given
/// point.
///
/// Follows "Real-Time Collision Detection" (Ericson 2005).
#[inline]
pub fn closest_point_on_line_segments_to_point(
    segment_start: &Point3,
    segment_vector: &Vector3,
    point: &Point3,
) -> Point3 {
    let segment_param =
        parameter_of_closest_point_on_line_segment_to_point(segment_start, segment_vector, point);

    segment_start + segment_vector * segment_param
}

/// Returns the point on the given line segment that is closest to the given
/// point.
///
/// Follows "Real-Time Collision Detection" (Ericson 2005).
#[inline]
pub fn parameter_of_closest_point_on_line_segment_to_point(
    segment_start: &Point3,
    segment_vector: &Vector3,
    point: &Point3,
) -> f32 {
    const EPSILON: f32 = 1e-8;

    let segment_length_squared = segment_vector.norm_squared();

    if segment_length_squared <= EPSILON {
        return 0.0;
    }

    let segment_start_to_point = point - segment_start;

    let segment_param =
        (segment_vector.dot(&segment_start_to_point) / segment_length_squared).clamp(0.0, 1.0);

    segment_param
}

/// Computes the point on each of the given line segments where the segments
/// are closest to each other.
///
/// Follows "Real-Time Collision Detection" (Ericson 2005).
#[inline]
pub fn closest_points_on_line_segments(
    segment_a_start: &Point3,
    segment_a_vector: &Vector3,
    segment_b_start: &Point3,
    segment_b_vector: &Vector3,
) -> (Point3, Point3) {
    let (segment_a_param, segment_b_param) = parameters_of_closest_points_on_line_segments(
        segment_a_start,
        segment_a_vector,
        segment_b_start,
        segment_b_vector,
    );
    (
        segment_a_start + segment_a_param * segment_a_vector,
        segment_b_start + segment_b_param * segment_b_vector,
    )
}

/// Computes the parametric coordinate of the point on each of the given line
/// segments where the segments are closest to each other.
///
/// Follows "Real-Time Collision Detection" (Ericson 2005).
#[inline]
pub fn parameters_of_closest_points_on_line_segments(
    segment_a_start: &Point3,
    segment_a_vector: &Vector3,
    segment_b_start: &Point3,
    segment_b_vector: &Vector3,
) -> (f32, f32) {
    const EPSILON: f32 = 1e-8;

    let squared_segment_a_length = segment_a_vector.norm_squared();
    let squared_segment_b_length = segment_b_vector.norm_squared();

    if squared_segment_a_length <= EPSILON && squared_segment_b_length <= EPSILON {
        // Both segments are just points
        return (0.0, 0.0);
    }

    let start_displacement = segment_a_start - segment_b_start;

    let segment_b_vector_dot_start_diff = segment_b_vector.dot(&start_displacement);

    // Fractional distances (start = 0.0, end = 1.0) of the closest points
    // along the two line segments
    let mut segment_a_param = 0.0;
    let mut segment_b_param = 0.0;

    if squared_segment_a_length <= EPSILON {
        // Segment A is a point
        segment_b_param =
            (segment_b_vector_dot_start_diff / squared_segment_b_length).clamp(0.0, 1.0);
    } else {
        let segment_a_vector_dot_start_diff = segment_a_vector.dot(&start_displacement);

        if squared_segment_b_length <= EPSILON {
            // Segment B is a point
            segment_a_param =
                (segment_a_vector_dot_start_diff / (-squared_segment_a_length)).clamp(0.0, 1.0);
        } else {
            let segment_dot = segment_a_vector.dot(segment_b_vector);
            let denom =
                squared_segment_a_length * squared_segment_b_length - segment_dot * segment_dot;

            segment_a_param = if denom != 0.0 {
                ((segment_dot * segment_b_vector_dot_start_diff
                    - segment_a_vector_dot_start_diff * squared_segment_b_length)
                    / denom)
                    .clamp(0.0, 1.0)
            } else {
                // Segments are parallel, so we can pick any distance along
                // them
                0.0
            };

            segment_b_param = (segment_dot * segment_a_param + segment_b_vector_dot_start_diff)
                / squared_segment_b_length;

            // If `t` is outside the segment, clamp it to the segment and recompute `s`
            if segment_b_param.is_sign_negative() {
                segment_b_param = 0.0;
                segment_a_param =
                    (segment_a_vector_dot_start_diff / (-squared_segment_a_length)).clamp(0.0, 1.0);
            } else if segment_b_param > 1.0 {
                segment_b_param = 1.0;
                segment_a_param = ((segment_dot - segment_a_vector_dot_start_diff)
                    / squared_segment_a_length)
                    .clamp(0.0, 1.0);
            }
        }
    }

    (segment_a_param, segment_b_param)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn closest_point_on_degenerate_segment_to_point_is_segment_start() {
        let segment_start = Point3::new(1.0, 2.0, 3.0);
        let segment_vector = Vector3::zeros();
        let point = Point3::new(5.0, 6.0, 7.0);

        let closest =
            closest_point_on_line_segments_to_point(&segment_start, &segment_vector, &point);

        assert_abs_diff_eq!(closest, segment_start);
    }

    #[test]
    fn closest_point_on_segment_to_interior_projection_is_correct() {
        // Segment from (0,0,0) to (4,0,0); point at (3,2,0) projects to (3,0,0).
        let segment_start = Point3::origin();
        let segment_vector = Vector3::new(4.0, 0.0, 0.0);
        let point = Point3::new(3.0, 2.0, 0.0);

        let closest =
            closest_point_on_line_segments_to_point(&segment_start, &segment_vector, &point);

        assert_abs_diff_eq!(closest, Point3::new(3.0, 0.0, 0.0), epsilon = 1e-6);
    }

    #[test]
    fn closest_point_on_segment_clamps_to_start_when_projection_is_before_start() {
        // Segment from (2,0,0) to (4,0,0); point at (0,0,0) projects before start.
        let segment_start = Point3::new(2.0, 0.0, 0.0);
        let segment_vector = Vector3::new(2.0, 0.0, 0.0);
        let point = Point3::origin();

        let closest =
            closest_point_on_line_segments_to_point(&segment_start, &segment_vector, &point);

        assert_abs_diff_eq!(closest, segment_start, epsilon = 1e-6);
    }

    #[test]
    fn closest_point_on_segment_clamps_to_end_when_projection_is_past_end() {
        // Segment from (0,0,0) to (2,0,0); point at (5,0,0) projects past end.
        let segment_start = Point3::origin();
        let segment_vector = Vector3::new(2.0, 0.0, 0.0);
        let point = Point3::new(5.0, 0.0, 0.0);

        let closest =
            closest_point_on_line_segments_to_point(&segment_start, &segment_vector, &point);

        let expected = segment_start + segment_vector;
        assert_abs_diff_eq!(closest, expected, epsilon = 1e-6);
    }

    #[test]
    fn closest_point_on_segment_to_point_on_segment_is_point_itself() {
        let segment_start = Point3::origin();
        let segment_vector = Vector3::new(0.0, 6.0, 0.0);
        let point = Point3::new(0.0, 3.0, 0.0);

        let closest =
            closest_point_on_line_segments_to_point(&segment_start, &segment_vector, &point);

        assert_abs_diff_eq!(closest, point, epsilon = 1e-6);
    }

    #[test]
    fn closest_points_when_both_segments_are_points_are_those_points() {
        let a_start = Point3::new(1.0, 0.0, 0.0);
        let b_start = Point3::new(4.0, 5.0, 6.0);

        let (pa, pb) = closest_points_on_line_segments(
            &a_start,
            &Vector3::zeros(),
            &b_start,
            &Vector3::zeros(),
        );

        assert_abs_diff_eq!(pa, a_start);
        assert_abs_diff_eq!(pb, b_start);
    }

    #[test]
    fn closest_points_when_segment_a_is_a_point() {
        // A is point (1,0,0); B spans (0,-1,0)→(0,1,0).
        // Closest on B to (1,0,0) is (0,0,0).
        let a_start = Point3::new(1.0, 0.0, 0.0);
        let b_start = Point3::new(0.0, -1.0, 0.0);
        let b_vector = Vector3::new(0.0, 2.0, 0.0);

        let (pa, pb) =
            closest_points_on_line_segments(&a_start, &Vector3::zeros(), &b_start, &b_vector);

        assert_abs_diff_eq!(pa, a_start, epsilon = 1e-6);
        assert_abs_diff_eq!(pb, Point3::new(0.0, 0.0, 0.0), epsilon = 1e-6);
    }

    #[test]
    fn closest_points_when_segment_b_is_a_point() {
        // A spans (0,0,0)→(2,0,0); B is point (1,1,0).
        // Closest on A to (1,1,0) is (1,0,0).
        let a_start = Point3::origin();
        let a_vector = Vector3::new(2.0, 0.0, 0.0);
        let b_start = Point3::new(1.0, 1.0, 0.0);

        let (pa, pb) =
            closest_points_on_line_segments(&a_start, &a_vector, &b_start, &Vector3::zeros());

        assert_abs_diff_eq!(pa, Point3::new(1.0, 0.0, 0.0), epsilon = 1e-6);
        assert_abs_diff_eq!(pb, b_start, epsilon = 1e-6);
    }

    #[test]
    fn closest_points_on_intersecting_perpendicular_segments_is_intersection() {
        // A from (-1,0,0)→(1,0,0), B from (0,-1,0)→(0,1,0); they cross at origin.
        let a_start = Point3::new(-1.0, 0.0, 0.0);
        let a_vector = Vector3::new(2.0, 0.0, 0.0);
        let b_start = Point3::new(0.0, -1.0, 0.0);
        let b_vector = Vector3::new(0.0, 2.0, 0.0);

        let (pa, pb) = closest_points_on_line_segments(&a_start, &a_vector, &b_start, &b_vector);

        assert_abs_diff_eq!(pa, Point3::origin(), epsilon = 1e-6);
        assert_abs_diff_eq!(pb, Point3::origin(), epsilon = 1e-6);
    }

    #[test]
    fn closest_points_on_skew_segments_are_correct() {
        // A along x from (0,0,0)→(1,0,0); B along y from (0.5,0,1)→(0.5,1,1).
        // Lines get closest at (0.5,0,0) on A and (0.5,0,1) on B.
        let a_start = Point3::origin();
        let a_vector = Vector3::new(1.0, 0.0, 0.0);
        let b_start = Point3::new(0.5, 0.0, 1.0);
        let b_vector = Vector3::new(0.0, 1.0, 0.0);

        let (pa, pb) = closest_points_on_line_segments(&a_start, &a_vector, &b_start, &b_vector);

        assert_abs_diff_eq!(pa, Point3::new(0.5, 0.0, 0.0), epsilon = 1e-6);
        assert_abs_diff_eq!(pb, Point3::new(0.5, 0.0, 1.0), epsilon = 1e-6);
    }

    #[test]
    fn closest_points_on_parallel_segments_have_correct_perpendicular_distance() {
        // A along x at z=0; B along x at z=1 (parallel, distance = 1).
        let a_start = Point3::origin();
        let a_vector = Vector3::new(1.0, 0.0, 0.0);
        let b_start = Point3::new(0.0, 0.0, 1.0);
        let b_vector = Vector3::new(1.0, 0.0, 0.0);

        let (pa, pb) = closest_points_on_line_segments(&a_start, &a_vector, &b_start, &b_vector);

        // The perpendicular distance between the returned closest points must be 1.
        let dist = Point3::distance_between(&pa, &pb);
        assert_abs_diff_eq!(dist, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn closest_points_clamps_t_to_segment_end_and_recomputes_s() {
        // A along x (0,0,0)→(1,0,0); B from (0,2,0)→(0,1,0) — entirely above A.
        // Unclamped t would be 2; clamped to 1 gives B end (0,1,0).
        // Recomputed s gives closest on A to (0,1,0) which is (0,0,0).
        let a_start = Point3::origin();
        let a_vector = Vector3::new(1.0, 0.0, 0.0);
        let b_start = Point3::new(0.0, 2.0, 0.0);
        let b_vector = Vector3::new(0.0, -1.0, 0.0);

        let (pa, pb) = closest_points_on_line_segments(&a_start, &a_vector, &b_start, &b_vector);

        assert_abs_diff_eq!(pa, Point3::new(0.0, 0.0, 0.0), epsilon = 1e-6);
        assert_abs_diff_eq!(pb, Point3::new(0.0, 1.0, 0.0), epsilon = 1e-6);
    }
}
