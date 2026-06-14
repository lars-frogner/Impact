//! Voronoi diagrams.

use crate::delaunay::{
    DelaunayTetrahedralization, NO_TETRAHEDRON_ID, Tetrahedron, Vertex, VertexIdx,
};
use approx::{abs_diff_eq, relative_eq};
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_containers::BitVector;
use impact_geometry::PlaneC;
use impact_math::{point::Point3C, vector::UnitVector3};

/// A polyhedron representing a region in a Voronoi diagram.
#[derive(Debug)]
pub struct VoronoiPolyhedron<A: Allocator> {
    pub vertices: AVec<Point3C, A>,
    pub face_planes: AVec<PlaneC, A>,
}

/// A yet-to-be determined plane for a Voronoi polyhedron face dual to a
/// Delaunay tetrahedron edge.
#[derive(Clone, Debug)]
struct PartialPlane {
    /// The vertex at the end of the edge from the dual vertex. This edge
    /// uniquely identifies the plane.
    end_vertex_idx: VertexIdx,
    /// Whether the edge lies on the boundary of the Delaunay
    /// tetrahedralization.
    edge_on_boundary: bool,
    /// Up to three distinct points (Voronoi vertices) on the plane.
    points: [Point3C; 3],
    /// The number of valid points in the `points` array.
    point_count: u32,
}

impl<A: Allocator> VoronoiPolyhedron<A> {
    /// Creates a new empty Voronoi polyhedron using the given allocator.
    pub fn empty_in(alloc: A) -> Self {
        let vertices = AVec::new_in(alloc);
        let face_planes = AVec::new_in(alloc);
        Self {
            vertices,
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

            // The tetrahedron's circumcenter becomes a Voronoi vertex
            let circumcenter = tetra.compute_circumcenter(vertices);

            self.vertices.push(circumcenter);

            // Go through the tetrahedron's vertices to find the edges from the
            // start vertex to the three other end vertices
            for (end_vertex_corner, end_vertex_idx) in tetra.vertices.into_iter().enumerate() {
                // Skip the start vertex as well as end vertices for edges whose
                // Voronoi face planes have been competely determined
                if end_vertex_idx == start_vertex_idx
                    || completed_end_vertices.bit_is_set(end_vertex_idx as usize)
                {
                    continue;
                }

                // The edge lies on the boundary of the tetrahedralization if
                // any of the surrounding tetrahedra are missing a neighbor
                // across one of the two faces sharing the edge. Those neighbors
                // are the ones opposite the two vertices that do not lie on the
                // edge. Here we do this check for the current tetrahedron.
                let edge_on_boundary_for_this_tetra =
                    edge_on_boundary_for_tetra(tetra, start_vertex_corner, end_vertex_corner);

                let mut completed = None;
                let mut found = false;

                for (plane_idx, plane) in partial_planes.iter_mut().enumerate() {
                    // Find the partial plane for the current edge
                    if plane.end_vertex_idx != end_vertex_idx {
                        continue;
                    }

                    found = true;

                    plane.edge_on_boundary |= edge_on_boundary_for_this_tetra;

                    // Multiple of the tetrahedra surrounding the edge may have
                    // the same circumcenter. We need distinct points to
                    // determine the plane, so we don't add the point if it's a
                    // duplicate.
                    if plane.points[..plane.point_count as usize]
                        .iter()
                        .any(|point| {
                            relative_eq!(&circumcenter, point, epsilon = 1e-5, max_relative = 1e-5)
                        })
                    {
                        break;
                    }

                    plane.points[plane.point_count as usize] = circumcenter;
                    plane.point_count += 1;

                    if plane.point_count == 3 {
                        completed = Some(plane_idx);
                    }
                    break;
                }

                if let Some(completed_idx) = completed {
                    // If the plane is completely determined (we have three
                    // distinct points), we register it as completed and convert
                    // it from partial to complete
                    completed_end_vertices.set_bit(end_vertex_idx as usize);
                    let partial_plane = partial_planes.swap_remove(completed_idx);

                    let plane = partial_plane.compute_plane_containing_three_points();
                    self.face_planes.push(plane);
                } else if !found {
                    // If no partial (or complete) plane currently exists for
                    // this edge, we initialize it
                    partial_planes.push(PartialPlane::with_single_point(
                        end_vertex_idx,
                        circumcenter,
                        edge_on_boundary_for_this_tetra,
                    ));
                }
            }

            // Find all neighbor tetrahedra that also contain the dual vertex
            // and add them to the stack for checking (unless they have already
            // been visited)
            for nb_id in tetra.neighbors_with_corner(start_vertex_corner) {
                if nb_id != NO_TETRAHEDRON_ID && !checked_ids.bit_is_set(nb_id as usize) {
                    ids_to_check.push(nb_id);
                    checked_ids.set_bit(nb_id as usize);
                }
            }
        }

        // We may be left with partial planes with fewer than three distinct
        // points. This could be because the corresponding edge is on the
        // boundary of the tetrahedralization so that there are fewer than three
        // tetrahedra surrounding the edge, or because there are three or more
        // but some of them have the same circumcenter. In the former case, the
        // appropriate face plane can be computed using the direction of the
        // boundary edge as the normal vector and the location of a point for
        // the displacement. In the latter case, the face plane is degenerate
        // and should be ignored.
        for partial_plane in partial_planes {
            if !partial_plane.edge_on_boundary || partial_plane.end_vertex_idx == VertexIdx::MAX {
                continue;
            }
            let plane = partial_plane.compute_plane_normal_to_edge(vertices, &dual_vertex.point);
            self.face_planes.push(plane);
        }

        // The positive and negative sides of the face planes are so far
        // arbitrary, but using the fact that the dual vertex is inside the
        // polyhedron we can flip them to get the positive sides on the outside
        // of the tetrahedron
        orient_face_planes_outward(&mut self.face_planes, &dual_vertex.point);
    }

    pub fn deduplicate_vertices(&mut self) {
        let mut completed_count = 0;
        while completed_count < self.vertices.len() {
            let next_vertex = &self.vertices[completed_count];

            let mut found = false;
            for vertex in &self.vertices[..completed_count] {
                if abs_diff_eq!(next_vertex, vertex, epsilon = 1e-4) {
                    found = true;
                    break;
                }
            }

            if found {
                self.vertices.swap_remove(completed_count);
            } else {
                completed_count += 1;
            }
        }
    }

    pub fn deduplicate_face_planes(&mut self) {
        let mut completed_count = 0;
        while completed_count < self.face_planes.len() {
            let next_vertex = &self.face_planes[completed_count];

            let mut found = false;
            for vertex in &self.face_planes[..completed_count] {
                if abs_diff_eq!(next_vertex, vertex, epsilon = 1e-4) {
                    found = true;
                    break;
                }
            }

            if found {
                self.face_planes.swap_remove(completed_count);
            } else {
                completed_count += 1;
            }
        }
    }
}

impl PartialPlane {
    #[inline]
    fn with_single_point(
        end_vertex_idx: VertexIdx,
        point: Point3C,
        edge_on_boundary: bool,
    ) -> Self {
        Self {
            end_vertex_idx,
            points: [point, Point3C::origin(), Point3C::origin()],
            point_count: 1,
            edge_on_boundary,
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

    #[inline]
    fn compute_plane_normal_to_edge(
        &self,
        vertices: &[Vertex],
        src_vertex_point: &Point3C,
    ) -> PlaneC {
        debug_assert!(self.point_count == 1 || self.point_count == 2);

        let src_vertex_point = src_vertex_point.aligned();
        let dest_vertex_point = vertices[self.end_vertex_idx as usize].point.aligned();

        // If there are two points, both will have the same displacement along
        // the edge
        let a = self.points[0].aligned();

        let edge = dest_vertex_point - src_vertex_point;

        let normal = UnitVector3::normalized_from(edge);
        let displacement = normal.dot(&a.into());

        PlaneC::new(normal.compact(), displacement)
    }
}

#[inline]
fn edge_on_boundary_for_tetra(
    tetra: &Tetrahedron,
    start_vertex_corner: usize,
    end_vertex_corner: usize,
) -> bool {
    let mut on_boundary = false;
    for i in 0..4 {
        // Use non-short-circuiting comparisons to avoid branches
        let opposite_to_edge = (start_vertex_corner != i) & (end_vertex_corner != i);
        on_boundary |= opposite_to_edge & (tetra.neighbors[i] == NO_TETRAHEDRON_ID);
    }
    on_boundary
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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_alloc::Global;

    #[test]
    fn voronoi_polyhedra_for_four_points_have_appropriate_structure() {
        let points = [
            [-1.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [0.0, 2.0, 0.0],
        ]
        .map(Point3C::from);

        let tetrahedralization = DelaunayTetrahedralization::construct(Global, &points).unwrap();

        let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);

            let n_vertices = polyhedron.vertices.len();
            let n_faces = polyhedron.face_planes.len();

            assert_eq!(n_vertices, 1);
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

        let tetrahedralization = DelaunayTetrahedralization::construct(Global, &points).unwrap();

        let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

        let mut single_vertex_count = 0;
        let mut double_vertex_count = 0;

        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);

            let n_vertices = polyhedron.vertices.len();
            let n_faces = polyhedron.face_planes.len();

            if n_vertices == 1 {
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

        let tetrahedralization = DelaunayTetrahedralization::construct(Global, &points).unwrap();

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

            match location {
                PointLocation::Corner => {
                    assert_eq!(n_vertices, 1);
                    assert!(n_faces == 3 || n_faces == 5);
                }
                PointLocation::Edge => {
                    assert_eq!(n_vertices, 2);
                    assert!(n_faces == 4 || n_faces == 6);
                }
                PointLocation::Face => {
                    assert_eq!(n_vertices, 4);
                    assert!(n_faces == 5 || n_faces == 7);
                }
                PointLocation::Interior => {
                    assert_eq!(n_vertices, 8);
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
}
