//! Voronoi diagrams.

use crate::delaunay::{DelaunayTetrahedralization, NO_TETRAHEDRON_ID, VertexIdx};
use approx::relative_eq;
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_containers::BitVector;
use impact_geometry::{AxisAlignedBox, PlaneC};
use impact_math::{
    point::Point3C,
    transform::Isometry3,
    vector::{UnitVector3, UnitVector3C},
};

/// A polyhedron representing a region in a Voronoi diagram.
#[derive(Debug)]
pub struct VoronoiPolyhedron<A: Allocator> {
    /// The vertices of the polyhedron.
    pub vertices: AVec<Point3C, A>,
    /// The rays along which the polyhedron has infinite extent.
    pub rays: AVec<PolyhedronRay, A>,
    /// The planes representing the faces of the polyhedron.
    pub face_planes: AVec<PlaneC, A>,
}

/// A ray along which a polyhedron has infinite extent.
#[derive(Clone, Debug)]
pub struct PolyhedronRay {
    /// The position of the vertex where the ray originates.
    pub vertex: Point3C,
    /// The direction of the ray.
    pub direction: UnitVector3C,
}

/// A yet-to-be determined plane for a Voronoi polyhedron face dual to a
/// Delaunay tetrahedron edge.
#[derive(Clone, Debug)]
struct PartialPlane {
    /// The vertex at the end of the edge from the dual vertex. This edge
    /// uniquely identifies the plane.
    end_vertex_idx: VertexIdx,
    /// Up to three distinct points (Voronoi vertices) on the plane.
    points: [Point3C; 3],
    /// The number of valid points in the `points` array.
    point_count: u32,
}

impl VoronoiPolyhedron<impact_alloc::Global> {
    pub fn extract_from_delaunay_tetrahedra_g(
        &mut self,
        tetrahedralization: &DelaunayTetrahedralization<impact_alloc::Global>,
        dual_vertex_idx: VertexIdx,
    ) {
        self.extract_from_delaunay_tetrahedra(tetrahedralization, dual_vertex_idx);
    }
}

impl<A: Allocator> VoronoiPolyhedron<A> {
    /// Creates a new empty Voronoi polyhedron using the given allocator.
    pub fn empty_in(alloc: A) -> Self {
        let vertices = AVec::new_in(alloc);
        let rays = AVec::new_in(alloc);
        let face_planes = AVec::new_in(alloc);
        Self {
            vertices,
            rays,
            face_planes,
        }
    }

    /// Extracts the Voronoi polyhedron dual to the given vertex in the given
    /// Delaunay tetrahedralization.
    pub fn extract_from_delaunay_tetrahedra<AT: Allocator>(
        &mut self,
        tetrahedralization: &DelaunayTetrahedralization<AT>,
        dual_vertex_idx: VertexIdx,
    ) {
        self.vertices.clear();
        self.rays.clear();
        self.face_planes.clear();

        let vertices = tetrahedralization.vertices();
        let tetrahedra = tetrahedralization.tetrahedra();

        let Some(dual_vertex) = vertices.get(dual_vertex_idx as usize) else {
            return;
        };
        if dual_vertex.tetra_id == NO_TETRAHEDRON_ID {
            return;
        }

        let arena = ArenaPool::get_arena();

        // We will go through every tetrahedron connected to the dual vertex.
        // The center of each such tetrahedron's circumsphere becomes a vertex
        // of the Voronoi polyhedron. Each face plane of the polyhedron is
        // associated with a tetrahedron edge containing the dual vertex. Each
        // tetrahedron we go through will have three such edges. For every edge,
        // we keep a `PartialPlane` holding the other vertex of the edge and up
        // to three of the Voronoi vertices from the surrounding tetrahedra.
        // These vertices are what define the face plane, and when we have three
        // (even though there may be more) we know enough to determine the
        // plane. So whenever we encounter a tetrahedron containing the edge, we
        // add its Voronoi vertex to the partial plane, and resolve the partial
        // plane into a completed plane when we have found three points.

        let mut ids_to_check = AVec::with_capacity_in(128, &arena);
        let mut partial_planes: AVec<PartialPlane, _> = AVec::with_capacity_in(64, &arena);
        let mut checked_ids = BitVector::zeroed_in(tetrahedra.len(), &arena);
        let mut completed_end_vertices = BitVector::zeroed_in(vertices.len(), &arena);

        // Start at the tetrahedron pointed to by the vertex (which is an
        // arbitrary tetrahedron connected to the vertex)
        ids_to_check.push(dual_vertex.tetra_id);
        checked_ids.set_bit(dual_vertex.tetra_id as usize);

        while let Some(id) = ids_to_check.pop() {
            let tetra = &tetrahedra[id as usize];

            let start_vertex_idx = dual_vertex_idx;
            let start_vertex_corner = tetra.corner_of_vertex(start_vertex_idx);
            let start_vertex_point = dual_vertex.point.aligned();

            // The tetrahedron's circumcenter becomes a Voronoi vertex
            let circumcenter = tetra.compute_circumcenter(vertices);

            self.vertices.push(circumcenter);

            // Find all neighbor tetrahedra that also contain the dual vertex
            // and add them to the stack for checking (unless they have already
            // been visited)
            for [nb_id, end_vertex_idx_1, end_vertex_idx_2] in
                tetra.neighbor_and_edges_for_faces_with_corner(start_vertex_corner)
            {
                // If the face has no adjoining neighbor, its edges are on the
                // boundary of the tetrahedralization. In this case, there will
                // typically be fewer than three tetrahedra sharing the edge, so
                // there will not be enough Voronoi vertices to determine the
                // plane. It also means the polyhedron will have infinite extent
                // in the outward direction normal to the face. We add this
                // direction to `rays`, and we compute the planes for the
                // boundary edges using the fact that the plane normal will be
                // aligned with the edge.
                if nb_id == NO_TETRAHEDRON_ID {
                    let end_vertex_point_1 = vertices[end_vertex_idx_1 as usize].point.aligned();
                    let end_vertex_point_2 = vertices[end_vertex_idx_2 as usize].point.aligned();

                    let edge_1 = end_vertex_point_1 - start_vertex_point;
                    let edge_2 = end_vertex_point_2 - start_vertex_point;

                    let ray_direction = UnitVector3::normalized_from(edge_2.cross(&edge_1));
                    self.rays.push(PolyhedronRay {
                        vertex: circumcenter,
                        direction: ray_direction.compact(),
                    });

                    for (edge, edge_end_vertex_idx) in
                        [(edge_1, end_vertex_idx_1), (edge_2, end_vertex_idx_2)]
                    {
                        if !completed_end_vertices.set_bit(edge_end_vertex_idx as usize) {
                            let normal = UnitVector3::normalized_from(edge);
                            // Place the plane at the Voroni vertex for the
                            // tetrahedron
                            let displacement = normal.dot(&circumcenter.aligned().into());
                            let plane = PlaneC::new(normal.compact(), displacement);
                            self.face_planes.push(plane);
                        }
                    }
                } else if !checked_ids.bit_is_set(nb_id as usize) {
                    // Since the neighbor exists and has not been checked, we
                    // add it for checking and preemptively mark it as checked
                    ids_to_check.push(nb_id);
                    checked_ids.set_bit(nb_id as usize);
                }
            }

            // Go through the tetrahedron's vertices to find the edges from the
            // start vertex to the three other end vertices
            for end_vertex_idx in tetra.vertices {
                // Skip the start vertex as well as end vertices for edges whose
                // Voronoi face planes have been competely determined
                if end_vertex_idx == start_vertex_idx
                    || completed_end_vertices.bit_is_set(end_vertex_idx as usize)
                {
                    continue;
                }

                // Find the partial plane for the current edge
                let plane = partial_planes
                    .iter_mut()
                    .enumerate()
                    .find(|(_, plane)| plane.end_vertex_idx == end_vertex_idx);

                if let Some((plane_idx, plane)) = plane {
                    // Multiple of the tetrahedra surrounding the edge may have
                    // the same circumcenter. We need distinct points to
                    // determine the plane, so we don't add the point if it's a
                    // duplicate.
                    if !plane.points[..plane.point_count as usize]
                        .iter()
                        .any(|point| {
                            relative_eq!(&circumcenter, point, epsilon = 1e-5, max_relative = 1e-5)
                        })
                    {
                        plane.points[plane.point_count as usize] = circumcenter;
                        plane.point_count += 1;

                        if plane.point_count == 3 {
                            // If the plane is completely determined (we have three
                            // distinct points), we compute the final plane and
                            // register it as completed
                            self.face_planes
                                .push(plane.compute_plane_containing_three_points());
                            completed_end_vertices.set_bit(end_vertex_idx as usize);

                            // Remove the partial plane to keep the list we have
                            // to search through small
                            partial_planes.swap_remove(plane_idx);
                        }
                    }
                } else {
                    // If no partial (or complete) plane currently exists for
                    // this edge, we initialize it
                    partial_planes.push(PartialPlane::with_single_point(
                        end_vertex_idx,
                        circumcenter,
                    ));
                }
            }
        }

        // The positive and negative sides of the face planes are so far
        // arbitrary, but using the fact that the dual vertex is inside the
        // polyhedron we can flip them to get the positive sides on the outside
        // of the tetrahedron
        orient_face_planes_outward(&mut self.face_planes, &dual_vertex.point);
    }

    /// Computes the AABB of the polyhedron, bounded by the given AABB. Returns
    /// [`None`] if the polyhedron is empty or outside the bounding AABB.
    pub fn compute_bounded_aabb(&self, bounding_aabb: &AxisAlignedBox) -> Option<AxisAlignedBox> {
        if self.vertices.is_empty() {
            assert!(self.rays.is_empty());
            return None;
        }

        let mut poly_aabb = AxisAlignedBox::aabb_for_points(&self.vertices);

        for PolyhedronRay { vertex, direction } in &self.rays {
            let vertex = vertex.aligned();
            let direction = direction.aligned();

            // Compute the intersection point with the closest boundary AABB far
            // plane (a plane whose inside faces the ray origin) and make the
            // polyhedron AABB encompass this point. Doing this for all rays
            // ensures that all parts of the inifinte regions of the polyhedron
            // inside the boundary AABB are encompassed by the polyhedron AABB.

            let lower_offset = vertex - bounding_aabb.lower_corner();
            let upper_offset = bounding_aabb.upper_corner() - vertex;

            let direction_recip = direction.component_recip();
            let lower_plane_dists = -lower_offset.component_mul(&direction_recip);
            let upper_plane_dists = upper_offset.component_mul(&direction_recip);

            let mut intersection_dist = f32::INFINITY;

            let mut update_dist = |new_dist| {
                if new_dist >= 0.0 {
                    intersection_dist = intersection_dist.min(new_dist);
                }
            };

            if direction.x() > 0.0 {
                update_dist(upper_plane_dists.x());
            } else if direction.x() < 0.0 {
                update_dist(lower_plane_dists.x());
            }
            if direction.y() > 0.0 {
                update_dist(upper_plane_dists.y());
            } else if direction.y() < 0.0 {
                update_dist(lower_plane_dists.y());
            }
            if direction.z() > 0.0 {
                update_dist(upper_plane_dists.z());
            } else if direction.z() < 0.0 {
                update_dist(lower_plane_dists.z());
            }

            if intersection_dist.is_finite() {
                let intersection_point = vertex + intersection_dist * direction;
                poly_aabb.expand_to_point(&intersection_point);
            }
        }

        poly_aabb.compute_overlap_with(bounding_aabb)
    }

    #[inline]
    pub fn deduplicate_vertices(&mut self) {
        deduplicate_vec_by(&mut self.vertices, |a, b| {
            relative_eq!(a, b, epsilon = 1e-5, max_relative = 1e-5)
        });
    }

    #[inline]
    pub fn deduplicate_face_planes(&mut self) {
        deduplicate_vec_by(&mut self.face_planes, |a, b| {
            relative_eq!(a, b, epsilon = 1e-5, max_relative = 1e-5)
        });
    }

    #[inline]
    pub fn shift_face_planes(&mut self, displacement: f32) {
        for plane in &mut self.face_planes {
            plane.displace_along_normal(displacement);
        }
    }

    #[inline]
    pub fn iso_transform(&mut self, transform: &Isometry3) {
        for vertex in &mut self.vertices {
            *vertex = transform.transform_point(&vertex.aligned()).compact();
        }
        for ray in &mut self.rays {
            ray.vertex = transform.transform_point(&ray.vertex.aligned()).compact();
            ray.direction = transform
                .transform_unit_vector(&ray.direction.aligned())
                .compact();
        }
        for plane in &mut self.face_planes {
            *plane = plane.aligned().iso_transformed(transform).compact();
        }
    }
}

impl PartialPlane {
    #[inline]
    fn with_single_point(end_vertex_idx: VertexIdx, point: Point3C) -> Self {
        Self {
            end_vertex_idx,
            points: [point, Point3C::origin(), Point3C::origin()],
            point_count: 1,
        }
    }

    #[inline]
    fn compute_plane_containing_three_points(&self) -> PlaneC {
        debug_assert_eq!(self.point_count, 3);

        let a = self.points[0].aligned();
        let b = self.points[1].aligned();
        let c = self.points[2].aligned();

        let ab = b - a;
        let ac = c - a;

        let normal = UnitVector3::normalized_from(ab.cross(&ac));
        let displacement = normal.dot(&a.into());

        PlaneC::new(normal.compact(), displacement)
    }
}

#[inline]
fn orient_face_planes_outward(planes: &mut [PlaneC], inside_point: &Point3C) {
    for plane in planes {
        if plane
            .compute_signed_distance(inside_point)
            .is_sign_positive()
        {
            plane.flip_normal();
        }
    }
}

#[inline]
fn deduplicate_vec_by<A: Allocator, T>(
    vector: &mut AVec<T, A>,
    same_bucket: impl Fn(&T, &T) -> bool,
) {
    let mut completed_count = 0;

    while completed_count < vector.len() {
        let next_item = &vector[completed_count];

        let mut found = false;
        for item in &vector[..completed_count] {
            if same_bucket(next_item, item) {
                found = true;
                break;
            }
        }
        if found {
            vector.swap_remove(completed_count);
        } else {
            completed_count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::{abs_diff_eq, assert_abs_diff_eq};
    use impact_alloc::Global;
    use impact_math::{point::Point3, vector::Vector3C};

    fn make_polyhedron(
        vertices: &[Point3C],
        rays: &[(Point3C, UnitVector3C)],
    ) -> VoronoiPolyhedron<Global> {
        let mut poly = VoronoiPolyhedron::empty_in(Global);
        for &v in vertices {
            poly.vertices.push(v);
        }
        for &(v, d) in rays {
            poly.rays.push(PolyhedronRay {
                vertex: v,
                direction: d,
            });
        }
        poly
    }

    #[test]
    fn voronoi_polyhedra_for_four_points_have_appropriate_structure() {
        let points = [
            [-1.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [0.0, 2.0, 0.0],
        ]
        .map(Point3C::from);

        let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

        let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);

            let n_vertices = polyhedron.vertices.len();
            let n_rays = polyhedron.rays.len();
            let n_faces = polyhedron.face_planes.len();

            assert_eq!(n_vertices, 1);
            assert_eq!(n_rays, 3);
            assert_eq!(n_faces, 3);
        }
    }

    #[test]
    fn voronoi_polyhedra_for_five_points_have_appropriate_structure() {
        let points = [
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ]
        .map(Point3C::from);

        let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

        let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

        let mut single_vertex_count = 0;
        let mut double_vertex_count = 0;

        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);

            let n_vertices = polyhedron.vertices.len();
            let n_rays = polyhedron.rays.len();
            let n_faces = polyhedron.face_planes.len();

            if n_vertices == 1 {
                assert_eq!(n_rays, 3);
                assert_eq!(n_faces, 3);
                single_vertex_count += 1;
            } else if n_vertices == 2 {
                assert_eq!(n_faces, 4);
                double_vertex_count += 1;
            } else {
                panic!();
            }
        }

        assert_eq!(single_vertex_count, 2);
        assert_eq!(double_vertex_count, 3);
    }

    #[test]
    fn voronoi_polyhedra_for_regular_grid_have_appropriate_structure() {
        enum PointLocation {
            Corner,
            Edge,
            Face,
            Interior,
        }

        let points_per_dim = 3;

        let mut points = Vec::new();
        let mut point_locations = Vec::new();

        for i in 0..points_per_dim {
            for j in 0..points_per_dim {
                for k in 0..points_per_dim {
                    let mut on_boundary_count = 0;
                    for x in [i, j, k] {
                        if x == 0 || x == points_per_dim - 1 {
                            on_boundary_count += 1;
                        }
                    }
                    let location = match on_boundary_count {
                        0 => PointLocation::Interior,
                        1 => PointLocation::Face,
                        2 => PointLocation::Edge,
                        3 => PointLocation::Corner,
                        _ => unreachable!(),
                    };

                    points.push(Point3C::new(i as f32, j as f32, k as f32));
                    point_locations.push(location);
                }
            }
        }

        let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

        let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

        for (dual_vertex_idx, location) in tetrahedralization
            .internal_vertex_indices()
            .zip(point_locations)
        {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);
            polyhedron.deduplicate_vertices();
            polyhedron.deduplicate_face_planes();

            let n_vertices = polyhedron.vertices.len();
            let n_faces = polyhedron.face_planes.len();

            let mut ray_directions: AVec<_, Global> =
                polyhedron.rays.iter().map(|ray| ray.direction).collect();

            deduplicate_vec_by(&mut ray_directions, |a, b| {
                relative_eq!(a, b, epsilon = 1e-5, max_relative = 1e-5)
            });
            let n_ray_directions = ray_directions.len();

            match location {
                PointLocation::Corner => {
                    assert_eq!(n_vertices, 1);
                    assert_eq!(n_ray_directions, 3);
                    assert!(n_faces == 3 || n_faces == 5);
                }
                PointLocation::Edge => {
                    assert_eq!(n_vertices, 2);
                    assert_eq!(n_ray_directions, 2);
                    assert!(n_faces == 4 || n_faces == 6);
                }
                PointLocation::Face => {
                    assert_eq!(n_vertices, 4);
                    assert_eq!(n_ray_directions, 1);
                    assert!(n_faces == 5 || n_faces == 7);
                }
                PointLocation::Interior => {
                    assert_eq!(n_vertices, 8);
                    assert_eq!(n_ray_directions, 0);
                    assert_eq!(n_faces, 6);
                }
            }

            let mut axis_aligned_count = 0;

            let dual_vertex = &tetrahedralization.vertices()[dual_vertex_idx as usize];

            for face in &polyhedron.face_planes {
                let normal = face.unit_normal();

                let is_axis_aligned = abs_diff_eq!(normal.x().abs(), 1.0)
                    || abs_diff_eq!(normal.y().abs(), 1.0)
                    || abs_diff_eq!(normal.z().abs(), 1.0);

                if is_axis_aligned {
                    axis_aligned_count += 1;
                    assert_abs_diff_eq!(face.compute_signed_distance(&dual_vertex.point), -0.5);
                } else {
                    assert_abs_diff_eq!(
                        face.compute_signed_distance(&dual_vertex.point),
                        -0.5 * f32::sqrt(2.0)
                    );
                }
            }

            assert_eq!(
                axis_aligned_count,
                match location {
                    PointLocation::Corner => 3,
                    PointLocation::Edge => 4,
                    PointLocation::Face => 5,
                    PointLocation::Interior => 6,
                }
            );
        }
    }

    #[test]
    fn computing_bounded_aabb_with_interior_vertex_gives_point_aabb() {
        let polyhedron = make_polyhedron(&[Point3C::new(5.0, 5.0, 5.0)], &[]);
        let bounding_aabb =
            AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 10.0, 10.0));
        assert_abs_diff_eq!(
            polyhedron.compute_bounded_aabb(&bounding_aabb).unwrap(),
            AxisAlignedBox::new(Point3::new(5.0, 5.0, 5.0), Point3::new(5.0, 5.0, 5.0)),
            epsilon = 1e-5
        );
    }

    #[test]
    fn computing_bounded_aabb_with_exterior_vertex_gives_none() {
        let polyhedron = make_polyhedron(&[Point3C::new(15.0, 5.0, 5.0)], &[]);
        let bounding_aabb =
            AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 10.0, 10.0));
        assert!(polyhedron.compute_bounded_aabb(&bounding_aabb).is_none());
    }

    #[test]
    fn computing_bounded_aabb_with_straddling_vertices_gives_truncated_aabb() {
        let polyhedron = make_polyhedron(
            &[
                Point3C::new(-1.0, 5.0, 5.0),
                Point3C::new(5.0, 12.0, 5.0),
                Point3C::new(5.0, 2.0, 5.0),
                Point3C::new(5.0, 5.0, 11.0),
            ],
            &[],
        );
        let bounding_aabb =
            AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 10.0, 10.0));
        assert_abs_diff_eq!(
            polyhedron.compute_bounded_aabb(&bounding_aabb).unwrap(),
            AxisAlignedBox::new(Point3::new(0.0, 2.0, 5.0), Point3::new(5.0, 10.0, 10.0)),
            epsilon = 1e-5
        );
    }

    #[test]
    fn computing_bounded_aabb_with_interior_ray_origin_gives_aabb_extended_to_exit_face() {
        // Ray points along y, exits at y = 10.
        let vertex = Point3C::new(5.0, 3.0, 5.0);
        let polyhedron = make_polyhedron(&[vertex], &[(vertex, UnitVector3C::unit_y())]);
        let bounding_aabb =
            AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 10.0, 10.0));
        assert_abs_diff_eq!(
            polyhedron.compute_bounded_aabb(&bounding_aabb).unwrap(),
            AxisAlignedBox::new(Point3::new(5.0, 3.0, 5.0), Point3::new(5.0, 10.0, 5.0)),
            epsilon = 1e-5
        );
    }

    #[test]
    fn computing_bounded_aabb_with_exterior_ray_origin_aabb_extended_to_exit_face() {
        // Ray vertex is outside the bounding AABB (below y=0). The ray points
        // along y, entering the AABB at y=0 and exiting at y=10.
        let vertex = Point3C::new(5.0, -5.0, 5.0);
        let polyhedron = make_polyhedron(&[vertex], &[(vertex, UnitVector3C::unit_y())]);
        let bounding_aabb =
            AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 10.0, 10.0));
        assert_abs_diff_eq!(
            polyhedron.compute_bounded_aabb(&bounding_aabb).unwrap(),
            AxisAlignedBox::new(Point3::new(5.0, 0.0, 5.0), Point3::new(5.0, 10.0, 5.0)),
            epsilon = 1e-5
        );
    }

    #[test]
    fn computing_bounded_aabb_with_exterior_rays_covering_bounding_aabb_gives_full_aabb() {
        // Both rays miss on opposite sides of the bounding AABB
        let vertex_1 = Point3C::new(8.0, 15.0, 2.0);
        let vertex_2 = Point3C::new(15.0, 3.0, 8.0);
        let direction = UnitVector3C::normalized_from(Vector3C::new(-1.0, -0.5, 0.0));
        let polyhedron = make_polyhedron(
            &[vertex_1, vertex_2],
            &[(vertex_1, direction), (vertex_2, direction)],
        );
        let bounding_aabb =
            AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 10.0, 10.0));
        assert_abs_diff_eq!(
            polyhedron.compute_bounded_aabb(&bounding_aabb).unwrap(),
            AxisAlignedBox::new(Point3::new(0.0, 0.0, 2.0), Point3::new(10.0, 10.0, 8.0)),
            epsilon = 1e-5
        );
    }
}
