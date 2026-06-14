//! Delaunay tetrahedralization.

use anyhow::{Result, bail};
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_geometry::{AxisAlignedBox, Sphere};
use impact_math::{
    consts::f32::{FRAC_1_SQRT_6, SQRT_3, SQRT_6},
    matrix::Matrix3,
    point::{Point3, Point3C},
    random::Rng,
    vector::Vector3,
};
use std::ops::Range;

/// The index of a vertex in a tetrahedralization.
pub type VertexIdx = u32;

/// The ID of a tetrahedron in a tetrahedralization.
pub type TetrahedronID = u32;

/// The ID representing an absent tetrahedron.
pub const NO_TETRAHEDRON_ID: TetrahedronID = u32::MAX;

/// How much to expand the bounding tetrahedron relative to the bounding sphere
/// of the point cloud.
const BOUNDING_TETRA_MARGIN_FACTOR: f32 = 1.1;

/// Points closer than this relative to the total size of the point cloud will
/// be merged.
const MIN_RELATIVE_POINT_SEPARATION: f32 = 1e-9;

/// A subdivision of space into tetrahedra that satisfy the Delaunay criterion,
/// meaning that the circumsphere of every tetrahedron contains no other
/// vertices.
#[derive(Clone, Debug)]
pub struct DelaunayTetrahedralization<A: Allocator> {
    inner: Tetrahedralization<A>,
}

/// A subdivision of space into tetrahedra.
#[derive(Clone, Debug)]
struct Tetrahedralization<A: Allocator> {
    vertices: AVec<Vertex, A>,
    tetrahedra: AVec<Tetrahedron, A>,
}

/// A tetrahedron defined by four vertices.
#[derive(Clone, Debug)]
pub struct Tetrahedron {
    /// The index of vertex A, B, C and D, respectively.
    pub vertices: [VertexIdx; 4],
    /// The ID of the tetrahedron adjoining face BCD, ACD, ADB and ABC,
    /// respectively (i.e. the face opposite the vertex at the same position in
    /// `vertices`). The ID has value [`NO_TETRAHEDRON_ID`] when there is no
    /// neighbor.
    pub neighbors: [TetrahedronID; 4],
}

/// A tetrahedron vertex.
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    /// The position of the vertex.
    pub point: Point3C,
    /// The ID of an arbitrary tetrahedron connected to the vertex.
    pub tetra_id: TetrahedronID,
}

#[derive(Debug)]
struct TetrahedronPointLocator {
    rng: Rng,
}

#[derive(Clone, Debug)]
struct TetrahedronIDChange {
    old: TetrahedronID,
    new: TetrahedronID,
}

impl<A: Allocator> DelaunayTetrahedralization<A> {
    /// Subdivides the convex hull of the given set of points into tetrahedra
    /// that satisfy the Delaunay criterion.
    ///
    /// Uses an incremental insertion algorithm described in Ledoux (2007),
    /// "Computing the 3D Voronoi Diagram Robustly: An Easy Explanation".
    ///
    /// # Errors
    /// Returns an error if the number of points or the resulting number of
    /// tetra exceeds a max limit.
    pub fn construct(alloc: A, points: &[Point3C]) -> Result<Self> {
        let n_points = points.len();

        if n_points + 4 > VertexIdx::MAX as usize {
            bail!("Number of points {n_points} is higher than supported");
        }

        let mut tetras = Tetrahedralization::with_vertex_capacity(alloc, n_points + 4);

        if n_points < 4 {
            return Ok(Self { inner: tetras });
        }

        let aabb = AxisAlignedBox::aabb_for_points(points);
        let bounding_sphere = Sphere::bounding_sphere_from_aabb(&aabb);

        tetras.add_bounding_tetrahedron(&bounding_sphere);

        let min_squared_point_separation =
            (MIN_RELATIVE_POINT_SEPARATION * bounding_sphere.radius()).powi(2);

        let arena = ArenaPool::get_arena();

        // When inserting a new vertex, we will push all new tetrahedra created
        // as result to the stack. For each tetrahedron popped off the stack, we
        // evaluate the Delaunay criterion locally, and if it is not satisfied,
        // we reconnect the local tetrahedra into a new configuration and push
        // them onto the stack. When the stack is empty, the new
        // tetrahedralization is Delaunay, and we can proceed to insert the next
        // vertex.
        let mut stack = AVec::with_capacity_in(64, &arena);

        let mut locator = TetrahedronPointLocator::new(0);

        'insertion: for new_vertex in points {
            let inside_tetra_id = locator.find_tetrahedron_containing_point(&tetras, new_vertex);
            assert_ne!(inside_tetra_id, NO_TETRAHEDRON_ID);

            let inside_tetra = tetras.tetrahedron(inside_tetra_id);

            // Skip the new vertex if it coincides with an existing vertex
            for vertex in inside_tetra.vertex_points(tetras.vertices()) {
                if Point3C::squared_distance_between(new_vertex, &vertex)
                    < min_squared_point_separation
                {
                    continue 'insertion;
                }
            }

            let new_vertex_idx = tetras.n_vertices() as VertexIdx;
            let new_tetra_ids = tetras.insert_and_connect_vertex(*new_vertex, inside_tetra_id)?;

            stack.extend_from_slice(&new_tetra_ids);

            while let Some(abcd_id) = stack.pop() {
                // The ID could have been cleared due to being invalidated by a
                // reconnection
                if abcd_id == NO_TETRAHEDRON_ID {
                    continue;
                }

                // Let the current tetrahedron be ABCD. The inserted vertex is A
                // (the reconnection operations never move A).
                let abcd = tetras.tetrahedron(abcd_id);

                let [a, b, c, d] = abcd.vertices;
                debug_assert_eq!(a, new_vertex_idx);

                // Find neighbor tetrahedron BCDE adjoining the BDC face
                let bcde_id = abcd.neighbors[0];
                if bcde_id == NO_TETRAHEDRON_ID {
                    continue;
                }
                let bcde = tetras.tetrahedron(bcde_id);

                // The neighbor shares the BDC face (in vertex order BCD).
                // Find the fourth vertex E not on that face.
                let e_corner = bcde.corner_not_on_face([b, c, d]);
                let e = bcde.vertices[e_corner];

                let vertex_a = abcd.vertex_point(tetras.vertices(), 0);
                let vertex_b = abcd.vertex_point(tetras.vertices(), 1);
                let vertex_c = abcd.vertex_point(tetras.vertices(), 2);
                let vertex_d = abcd.vertex_point(tetras.vertices(), 3);
                let vertex_e = bcde.vertex_point(tetras.vertices(), e_corner);

                // If E lies inside the circumsphere of ABCD, ABCD does not
                // satisfy the Delaunay criterion, so we divide the hull of ABCD
                // and ACBE into a new set of tetrahedra that may satisfy the
                // criterion. If E does not lie inside the circumsphere, ABCD is
                // locally Delaunay. When all tetrahedra connected to the
                // inserted vertex A are locally Delaunay, they also globally
                // Delaunay and we are done.
                if point_lies_strictly_inside_circumsphere(
                    vertex_a, vertex_b, vertex_c, vertex_d, vertex_e,
                ) {
                    if evaluate_side_of_triangle_plane_for_point(
                        vertex_b, vertex_c, vertex_d, vertex_e,
                    ) == PointTrianglePlaneSide::InPlane
                    {
                        // The neighbor tetrahedron is flat. No flips are performed.
                        continue;
                    };

                    match evaluate_infinite_line_triangle_intersection_one_sided(
                        vertex_a, vertex_e, vertex_b, vertex_d, vertex_c,
                    ) {
                        LineTriangleIntersection::Inside => {
                            // The hull of ABCD and BCDE is convex. They can be
                            // reconnected into three tetrahedra ABCE, ACDE and
                            // ADBE.
                            let new_tetra_ids = tetras.reconnect_two_to_three(abcd_id, bcde_id)?;

                            stack.extend_from_slice(&new_tetra_ids);
                        }
                        LineTriangleIntersection::Outside {
                            ab: beyond_bd,
                            bc: beyond_dc,
                            ca: beyond_cb,
                        } => {
                            // The hull of the two tetrahedra is concave, with a
                            // reflex edge on the shared BDC face. A
                            // reconnection is only possible if there exists a
                            // third tetrahedron AXYE adjoining both ABCD and
                            // BCDE across the faces sharing that reflex edge
                            // XY. The reflex edge is the single edge the line
                            // AE passes beyond; if it passes beyond two edges
                            // (a vertex region) there is no single reflex edge
                            // and we leave the face for a later reconnection.
                            let (axye_nb_of_abcd, axye_nb_of_bcde) =
                                if beyond_bd && !beyond_dc && !beyond_cb {
                                    (abcd.neighbors[2], bcde.id_of_neighbor_opposite_vertex(c))
                                } else if beyond_dc && !beyond_bd && !beyond_cb {
                                    (abcd.neighbors[1], bcde.id_of_neighbor_opposite_vertex(b))
                                } else if beyond_cb && !beyond_bd && !beyond_dc {
                                    (abcd.neighbors[3], bcde.id_of_neighbor_opposite_vertex(d))
                                } else {
                                    continue;
                                };

                            if axye_nb_of_abcd != axye_nb_of_bcde {
                                continue;
                            }
                            let axye_id = axye_nb_of_abcd;
                            if axye_id == NO_TETRAHEDRON_ID {
                                continue;
                            }
                            // Reconnect into two tetrahedra AXZE and AZYE
                            let (new_tetra_ids, id_change) =
                                tetras.reconnect_three_to_two(abcd_id, bcde_id, axye_id);

                            // The old tetrahedra ABCD, BCDE and AXYE are now
                            // removed and their IDs no longer point to them, so
                            // if they are present on the stack, they must be
                            // cleared. BCDE will never be on the stack as it
                            // doesn't contain A. ABCD will also not be on the
                            // stack so long as we clear AXYE (which is what
                            // would become ABCD in a later iteration) and
                            // perform corresponding clearing for the other
                            // reconnections.
                            for id in &mut stack {
                                if *id == axye_id {
                                    *id = NO_TETRAHEDRON_ID;
                                } else if *id == id_change.old {
                                    // Also apply the ID remapping caused by the reconnection
                                    *id = id_change.new;
                                }
                            }
                            stack.extend_from_slice(&new_tetra_ids);
                        }
                        LineTriangleIntersection::Edges {
                            ab: intersects_bd,
                            bc: intersects_dc,
                            ca: intersects_cb,
                        } => {
                            // The two tetrahedra have two co-planar faces
                            // connected by the intersected edge. Our options
                            // depend on whether or not ABCD is a flat
                            // tetrahedron.
                            if evaluate_side_of_triangle_plane_for_point(
                                vertex_b, vertex_d, vertex_c, vertex_a,
                            ) == PointTrianglePlaneSide::InPlane
                            {
                                // ABCD is flat, and the new vertex A lies
                                // directly on an edge of triangle BDC. If there
                                // exists a third tetrahedron AXYE sharing that
                                // edge, we can reconnect the three into two
                                // tetrahedra AXZE and AZYE. Otherwise, we can
                                // reconnect ABCD and BCDE into three tetrahedra
                                // ABCE, ACDE and ADBE. One of them (depending
                                // on the intersected edge) will still be flat,
                                // but it will be deleted by a later
                                // reconnection.
                                let (axye_nb_of_abcd, axye_nb_of_bcde) = if intersects_bd {
                                    (abcd.neighbors[2], bcde.id_of_neighbor_opposite_vertex(c))
                                } else if intersects_dc {
                                    (abcd.neighbors[1], bcde.id_of_neighbor_opposite_vertex(b))
                                } else {
                                    (abcd.neighbors[3], bcde.id_of_neighbor_opposite_vertex(d))
                                };

                                if axye_nb_of_abcd == axye_nb_of_bcde
                                    && axye_nb_of_abcd != NO_TETRAHEDRON_ID
                                {
                                    let axye_id = axye_nb_of_abcd;
                                    let (new_tetra_ids, id_change) =
                                        tetras.reconnect_three_to_two(abcd_id, bcde_id, axye_id);

                                    for id in &mut stack {
                                        if *id == axye_id {
                                            *id = NO_TETRAHEDRON_ID;
                                        } else if *id == id_change.old {
                                            *id = id_change.new;
                                        }
                                    }
                                    stack.extend_from_slice(&new_tetra_ids);
                                } else {
                                    let new_tetra_ids =
                                        tetras.reconnect_two_to_three(abcd_id, bcde_id)?;

                                    stack.extend_from_slice(&new_tetra_ids);
                                }
                            } else {
                                // ABCD is not flat. We can perform a
                                // four-to-four reconnection if we can find two
                                // other tetrahedra connected to the intersected
                                // edge with a commin fifth vertex F.
                                let (
                                    tetra_3_id,
                                    tetra_3_non_f_face,
                                    tetra_4_id,
                                    tetra_4_non_f_face,
                                ) = if intersects_bd {
                                    let abdf_id = abcd.neighbors[2];
                                    let abdf_non_f_face = [a, b, d];

                                    let bdfe_id = bcde.id_of_neighbor_opposite_vertex(c);
                                    let bdfe_non_f_face = [e, d, b];

                                    (abdf_id, abdf_non_f_face, bdfe_id, bdfe_non_f_face)
                                } else if intersects_dc {
                                    let adcf_id = abcd.neighbors[1];
                                    let adcf_non_f_face = [a, d, c];

                                    let dcfe_id = bcde.id_of_neighbor_opposite_vertex(b);
                                    let dcfe_non_f_face = [e, c, d];

                                    (adcf_id, adcf_non_f_face, dcfe_id, dcfe_non_f_face)
                                } else if intersects_cb {
                                    let acbf_id = abcd.neighbors[3];
                                    let acbf_non_f_face = [a, c, b];

                                    let cbfe_id = bcde.id_of_neighbor_opposite_vertex(d);
                                    let cbfe_non_f_face = [e, b, c];

                                    (acbf_id, acbf_non_f_face, cbfe_id, cbfe_non_f_face)
                                } else {
                                    continue;
                                };

                                if tetra_3_id == NO_TETRAHEDRON_ID
                                    || tetra_4_id == NO_TETRAHEDRON_ID
                                {
                                    continue;
                                }
                                let tetra_3 = tetras.tetrahedron(tetra_3_id);
                                let tetra_4 = tetras.tetrahedron(tetra_4_id);

                                let f_corner_t3 = tetra_3.corner_not_on_face(tetra_3_non_f_face);
                                let f_corner_t4 = tetra_4.corner_not_on_face(tetra_4_non_f_face);
                                let f_t3 = tetra_3.vertices[f_corner_t3];
                                let f_t4 = tetra_4.vertices[f_corner_t4];

                                // The neighbors must have a common fifth vertex F
                                if f_t3 != f_t4 {
                                    continue;
                                }

                                let new_tetra_ids = tetras.reconnect_four_to_four(
                                    abcd_id, bcde_id, tetra_3_id, tetra_4_id,
                                );

                                for id in &mut stack {
                                    if *id == tetra_3_id {
                                        *id = NO_TETRAHEDRON_ID;
                                    }
                                }
                                stack.extend_from_slice(&new_tetra_ids);
                            }
                        }
                    }
                }
            }
        }

        tetras.remove_boundary_tetrahedra(&arena);

        Ok(Self { inner: tetras })
    }

    /// Returns the vertices in the tetrahedralization.
    #[inline]
    pub fn vertices(&self) -> &[Vertex] {
        self.inner.vertices()
    }

    /// Returns the number of vertices in the tetrahedralization.
    #[inline]
    pub fn n_vertices(&self) -> usize {
        self.inner.n_vertices()
    }

    /// Returns the range of vertex indices excluding the four ad-hoc boundary
    /// vertices.
    #[inline]
    pub fn internal_vertex_indices(&self) -> Range<VertexIdx> {
        4..(self.inner.n_vertices() as VertexIdx)
    }

    /// Returns the tetrahedra in the tetrahedralization. The ID of each
    /// tetrahedron corresponds to its index in the slice.
    #[inline]
    pub fn tetrahedra(&self) -> &[Tetrahedron] {
        self.inner.tetrahedra()
    }

    /// Returns the number of tetrahedra in the tetrahedralization.
    #[inline]
    pub fn n_tetrahedra(&self) -> usize {
        self.inner.n_tetrahedra()
    }

    /// Returns the tetrahedron with the given ID.
    #[inline]
    pub fn tetrahedron(&self, id: TetrahedronID) -> &Tetrahedron {
        self.inner.tetrahedron(id)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn validate_brute_force(&self) {
        for (tetra_id, tetra) in self.inner.tetrahedra_with_ids() {
            for vertex_idx in tetra.vertices {
                assert!(
                    vertex_idx >= 4,
                    "Tetrahedron {tetra_id} contains boundary vertex {vertex_idx}"
                );
                let vertex = &self.inner.vertices()[vertex_idx as usize];
                assert_ne!(
                    vertex.tetra_id, NO_TETRAHEDRON_ID,
                    "Vertex {vertex_idx} points to no tetrahedron despite being used in tetrahedron {tetra_id}"
                );
            }

            let [a, b, c, d] = tetra.vertex_points(self.inner.vertices());

            for (nb_idx, &nb_id) in tetra.neighbors.iter().enumerate() {
                if nb_id == NO_TETRAHEDRON_ID {
                    continue;
                }
                assert!(
                    self.inner.has_tetrahedron(nb_id),
                    "Tetrahedron {tetra_id} is missing neighbor {nb_id} at slot {nb_idx}"
                );

                let face = tetra.face_opposite_neighbor(nb_idx);
                let neighbor = self.inner.tetrahedron(nb_id);

                let mut back_idx = None;
                for (i, &id) in neighbor.neighbors.iter().enumerate() {
                    if id == tetra_id {
                        assert!(
                            back_idx.is_none(),
                            "Neighbor {nb_id} points back to tetrahedron {tetra_id} \
                             across more than one face"
                        );
                        back_idx = Some(i);
                    }
                }
                let back_idx = back_idx.unwrap_or_else(|| {
                    panic!(
                        "Neighbor {nb_id} of tetrahedron {tetra_id} (slot {nb_idx}) \
                         does not point back to it"
                    )
                });

                let back_face = neighbor.face_opposite_neighbor(back_idx);
                let reversed_face = [face[0], face[2], face[1]];
                assert!(
                    back_face == reversed_face
                        || back_face == [reversed_face[1], reversed_face[2], reversed_face[0]]
                        || back_face == [reversed_face[2], reversed_face[0], reversed_face[1]],
                    "Shared face is inconsistently wound between tetrahedron {tetra_id} \
                     (slot {nb_idx}, face {face:?}) and neighbor {nb_id} \
                     (slot {back_idx}, face {back_face:?})"
                );
            }

            if evaluate_side_of_triangle_plane_for_point(&a, &b, &c, &d)
                == PointTrianglePlaneSide::Negative
            {
                panic!("Tetrahedron {tetra_id} has a negative signed volume");
            }

            for (v_idx, v) in self.inner.vertices().iter().enumerate() {
                if point_lies_strictly_inside_circumsphere(&a, &b, &c, &d, &v.point) {
                    let [a_idx, b_idx, c_idx, d_idx] = tetra.vertices;
                    panic!(
                        "Circumsphere of tetrahedron {tetra_id} with vertices \
                         [{a_idx}, {b_idx}, {c_idx}, {d_idx}] contains vertex {v_idx} \
                         (vertices = {{{a:?}, {b:?}, {c:?}, {d:?}}}, contains = {v:?})"
                    )
                }
            }
        }

        for (vertex_idx, vertex) in self.inner.vertices().iter().enumerate().skip(4) {
            let vertex_idx = vertex_idx as VertexIdx;
            if vertex.tetra_id == NO_TETRAHEDRON_ID {
                continue;
            }
            let Some(tetra) = self.inner.get_tetrahedron(vertex.tetra_id) else {
                panic!(
                    "Vertex {vertex_idx} points to missing tetrahedron {}",
                    vertex.tetra_id
                );
            };
            assert!(
                tetra.vertices.contains(&vertex_idx),
                "Vertex {vertex_idx} points to tetrahedron {} without that vertex",
                vertex.tetra_id
            );
        }
    }
}

impl<A: Allocator> Tetrahedralization<A> {
    fn with_vertex_capacity(alloc: A, vertex_capacity: usize) -> Self {
        let vertices = AVec::with_capacity_in(vertex_capacity, alloc);
        let tetra_capacity = estimated_tetrahedron_count(vertex_capacity);
        let tetrahedra = AVec::with_capacity_in(tetra_capacity, alloc);
        Self {
            vertices,
            tetrahedra,
        }
    }

    /// Returns the vertices in the tetrahedralization.
    #[inline]
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Returns the number of vertices in the tetrahedralization.
    #[inline]
    pub fn n_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the tetrahedra in the tetrahedralization. The ID of each
    /// tetrahedron corresponds to its index in the slice.
    #[inline]
    pub fn tetrahedra(&self) -> &[Tetrahedron] {
        &self.tetrahedra
    }

    /// Returns the number of tetrahedra in the tetrahedralization.
    #[inline]
    pub fn n_tetrahedra(&self) -> usize {
        self.tetrahedra.len()
    }

    /// Returns the tetrahedron with the given ID, or [`None`] if it is not
    /// present.
    #[allow(dead_code)]
    #[inline]
    pub fn get_tetrahedron(&self, id: TetrahedronID) -> Option<&Tetrahedron> {
        self.tetrahedra.get(id as usize)
    }

    /// Returns the tetrahedron with the given ID.
    #[inline]
    pub fn tetrahedron(&self, id: TetrahedronID) -> &Tetrahedron {
        &self.tetrahedra[id as usize]
    }

    /// Whether the tetrahedron with the given ID is present.
    #[allow(dead_code)]
    #[inline]
    pub fn has_tetrahedron(&self, id: TetrahedronID) -> bool {
        (id as usize) < self.tetrahedra.len()
    }

    /// Returns an iterator over each tetrahedron with its ID.
    #[allow(dead_code)]
    #[inline]
    pub fn tetrahedra_with_ids(&self) -> impl Iterator<Item = (TetrahedronID, &Tetrahedron)> {
        self.tetrahedra
            .iter()
            .enumerate()
            .map(|(id, tetra)| (id as TetrahedronID, tetra))
    }

    /// Removes the given tetrahedron by replacing it with the last tetrahedron
    /// in the backing array, which then inherits the ID of the removed
    /// tetrahedron. Returns the corresponding [`TetrahedronIDChange`].
    #[inline]
    fn remove_tetrahedron(&mut self, tetra_id: TetrahedronID) -> TetrahedronIDChange {
        debug_assert_ne!(tetra_id, NO_TETRAHEDRON_ID);

        self.tetrahedra.swap_remove(tetra_id as usize);

        let old_swapped_tetra_id = self.tetrahedra.len() as TetrahedronID;
        let new_swapped_tetra_id = tetra_id;

        if new_swapped_tetra_id != old_swapped_tetra_id {
            let swapped_tetra = &self.tetrahedra[new_swapped_tetra_id as usize];

            for vertex_idx in swapped_tetra.vertices {
                let vertex = &mut self.vertices[vertex_idx as usize];
                if vertex.tetra_id == old_swapped_tetra_id {
                    vertex.tetra_id = new_swapped_tetra_id;
                }
            }
            for nb_id in swapped_tetra.neighbors {
                if nb_id != NO_TETRAHEDRON_ID {
                    let tetra_nb = &mut self.tetrahedra[nb_id as usize];
                    tetra_nb.replace_neighbor_id(old_swapped_tetra_id, new_swapped_tetra_id);
                }
            }
        }

        TetrahedronIDChange {
            old: old_swapped_tetra_id,
            new: new_swapped_tetra_id,
        }
    }

    /// Adds a tetrahedron bounding the given sphere as the first tetrahedron.
    fn add_bounding_tetrahedron(&mut self, bounding_sphere: &Sphere) {
        assert!(self.vertices.is_empty());
        assert!(self.tetrahedra.is_empty());

        // Initialize with a single big tetrahedron encompassing all points.
        // This ensures points will always be inserted into an existing
        // tetrahedron.
        let bounding_tetra_vertices = find_tetrahedron_encompassing_sphere(
            bounding_sphere.center(),
            bounding_sphere.radius() * BOUNDING_TETRA_MARGIN_FACTOR,
        );

        let bounding_tetra_id = 0;

        self.vertices.extend(
            bounding_tetra_vertices
                .into_iter()
                .map(|vertex| Vertex::new(vertex, bounding_tetra_id)),
        );

        self.tetrahedra.push(Tetrahedron {
            vertices: [0, 1, 2, 3],
            neighbors: [NO_TETRAHEDRON_ID; 4],
        });
    }

    /// Removes all tetrahedra connected to the ad-hoc bounding vertices.
    fn remove_boundary_tetrahedra<AR: Allocator>(&mut self, arena: AR) {
        let mut tetras_to_remove = AVec::new_in(arena);
        let mut visited_neighbors = AVec::new_in(arena);
        let mut neighbors_to_check = AVec::new_in(arena);

        for (tetra_idx, tetra) in self.tetrahedra.iter().enumerate() {
            let id = tetra_idx as TetrahedronID;

            let has_boundary_vertex = tetra.vertices.iter().any(|&vertex| vertex < 4);

            if !has_boundary_vertex {
                continue;
            }

            tetras_to_remove.push(id);

            // Each non-boundary vertex pointing to the boundary tetrahedron
            // that will be removed must be re-pointed to another non-boundary
            // tetrahedron with that vertex.
            for vertex_idx in tetra.vertices {
                if vertex_idx < 4 {
                    continue;
                }
                let internal_vertex = &mut self.vertices[vertex_idx as usize];

                if internal_vertex.tetra_id != id {
                    continue;
                }

                visited_neighbors.clear();
                neighbors_to_check.clear();

                visited_neighbors.push(id);
                neighbors_to_check.extend_from_slice(&tetra.neighbors_with_vertex(vertex_idx));

                while let Some(nb_id) = neighbors_to_check.pop() {
                    if nb_id == NO_TETRAHEDRON_ID {
                        continue;
                    }
                    let nb_tetra = &self.tetrahedra[nb_id as usize];
                    let nb_has_boundary_vertex = nb_tetra.vertices.iter().any(|&vertex| vertex < 4);

                    if nb_has_boundary_vertex {
                        visited_neighbors.push(nb_id);

                        for nb_nb_id in nb_tetra.neighbors_with_vertex(vertex_idx) {
                            if !visited_neighbors.contains(&nb_nb_id) {
                                neighbors_to_check.push(nb_nb_id);
                            }
                        }
                    } else {
                        internal_vertex.tetra_id = nb_id;
                        break;
                    }
                }

                if internal_vertex.tetra_id == id {
                    internal_vertex.tetra_id = NO_TETRAHEDRON_ID;
                }
            }
        }

        for id in tetras_to_remove.into_iter().rev() {
            let tetra = &self.tetrahedra[id as usize];

            for nb_id in tetra.neighbors {
                if nb_id != NO_TETRAHEDRON_ID {
                    let tetra_nb = &mut self.tetrahedra[nb_id as usize];
                    tetra_nb.replace_neighbor_id(id, NO_TETRAHEDRON_ID);
                }
            }

            // Since the ID change occurs for the last tetrahedron and we are
            // removing tetrahedra in descending order of IDs, the change does
            // not invalidate any of the remaining IDs in `tetras_to_remove`
            self.remove_tetrahedron(id);
        }

        for bounding_vertex in self.vertices[..4].iter_mut() {
            bounding_vertex.tetra_id = NO_TETRAHEDRON_ID;
        }
    }

    /// Takes a vertex A and a tetrahedron BCDE into which to insert it by
    /// converting BCDE into four new tetrahedra ABCE, ACDE, ADBE and ACBD
    /// connected to A. Returns the respective IDs of the four new tetrahedra.
    /// The vertices in the new tetrahedra will be in the order implied by their
    /// names.
    ///
    /// # Errors
    /// Returns an error if the resulting total number of tetrahedra would
    /// exceed the max limit.
    fn insert_and_connect_vertex(
        &mut self,
        vertex: Point3C,
        inside_tetra_id: TetrahedronID,
    ) -> Result<[TetrahedronID; 4]> {
        debug_assert_ne!(inside_tetra_id, NO_TETRAHEDRON_ID);

        if self.tetrahedra.len() + 2 >= TetrahedronID::MAX as usize {
            bail!("Number of tetrahedra exceeded max limit");
        }

        let abce_id = inside_tetra_id;
        let acde_id = self.tetrahedra.len() as TetrahedronID;
        let adbe_id = (self.tetrahedra.len() + 1) as TetrahedronID;
        let acbd_id = (self.tetrahedra.len() + 2) as TetrahedronID;

        let a = self.vertices.len() as VertexIdx;

        let tetra = &mut self.tetrahedra[inside_tetra_id as usize];
        let [b, c, d, e] = tetra.vertices;
        let [ced_nb_id, bde_nb_id, bec_nb_id, bcd_nb_id] = tetra.neighbors;

        let abce = tetra;
        *abce = Tetrahedron {
            vertices: [a, b, c, e],
            neighbors: [bec_nb_id, acde_id, adbe_id, acbd_id],
        };

        let acde = Tetrahedron {
            vertices: [a, c, d, e],
            neighbors: [ced_nb_id, adbe_id, abce_id, acbd_id],
        };
        let adbe = Tetrahedron {
            vertices: [a, d, b, e],
            neighbors: [bde_nb_id, abce_id, acde_id, acbd_id],
        };
        let acbd = Tetrahedron {
            vertices: [a, c, b, d],
            neighbors: [bcd_nb_id, adbe_id, acde_id, abce_id],
        };

        self.tetrahedra.push(acde);
        self.tetrahedra.push(adbe);
        self.tetrahedra.push(acbd);

        // Update invalidated neighbor IDs for affected neighbors
        if ced_nb_id != NO_TETRAHEDRON_ID {
            let ced_nb = &mut self.tetrahedra[ced_nb_id as usize];
            ced_nb.replace_neighbor_id(inside_tetra_id, acde_id);
        }
        if bde_nb_id != NO_TETRAHEDRON_ID {
            let bde_nb = &mut self.tetrahedra[bde_nb_id as usize];
            bde_nb.replace_neighbor_id(inside_tetra_id, adbe_id);
        }
        if bcd_nb_id != NO_TETRAHEDRON_ID {
            let bcd_nb = &mut self.tetrahedra[bcd_nb_id as usize];
            bcd_nb.replace_neighbor_id(inside_tetra_id, acbd_id);
        }

        // Add new vertex A
        self.vertices.push(Vertex::new(vertex, abce_id));

        // Update potentially invalidated tetrahedron ID for vertex D
        self.vertices[d as usize].tetra_id = acde_id;

        Ok([abce_id, acde_id, adbe_id, acbd_id])
    }

    /// Takes the IDs of two adjacent tetrahedra ABCD and BCDE sharing the face
    /// BCD and reconfigures them into three adjacent tetrahedra ABCE, ACDE and
    /// ADBE sharing the edge AE. Returns the respective IDs of the three
    /// reconfigured tetrahedra. The vertices in the new tetrahedra will be in
    /// the order implied by their names.
    ///
    /// Which vertex is assigned which label is defined by the first input
    /// tetrahedron, in the sense that its vertex list will be assumed to be
    /// `[A, B, C, D]`. The vertices of the other tetrahedra do not have to be
    /// in the same order as their name implies.
    ///
    /// This is the inverse of
    /// [`reconnect_three_to_two`](Self::reconnect_three_to_two).
    ///
    /// # Errors
    /// Returns an error if the resulting total number of tetrahedra would
    /// exceed the max limit.
    fn reconnect_two_to_three(
        &mut self,
        abcd_id: TetrahedronID,
        bcde_id: TetrahedronID,
    ) -> Result<[TetrahedronID; 3]> {
        debug_assert_ne!(abcd_id, NO_TETRAHEDRON_ID);
        debug_assert_ne!(bcde_id, NO_TETRAHEDRON_ID);

        if self.tetrahedra.len() >= TetrahedronID::MAX as usize {
            bail!("Number of tetrahedra exceeded max limit");
        }

        let abce_id = abcd_id;
        let acde_id = bcde_id;
        let adbe_id = self.tetrahedra.len() as TetrahedronID;

        let [abcd, bcde] = self
            .tetrahedra
            .get_disjoint_mut([abcd_id as usize, bcde_id as usize])
            .unwrap();

        let [a, b, c, d] = abcd.vertices;

        let e_corner = bcde.corner_not_on_face([b, c, d]);
        let e = bcde.vertices[e_corner];

        let [bdc_nb_id, acd_nb_id, adb_nb_id, abc_nb_id] = abcd.neighbors;
        debug_assert_eq!(bdc_nb_id, bcde_id);

        let edc_nb_id = bcde.id_of_neighbor_opposite_vertex(b);
        let ebd_nb_id = bcde.id_of_neighbor_opposite_vertex(c);
        let ecb_nb_id = bcde.id_of_neighbor_opposite_vertex(d);

        let abce = abcd;
        *abce = Tetrahedron {
            vertices: [a, b, c, e],
            neighbors: [ecb_nb_id, acde_id, adbe_id, abc_nb_id],
        };

        let acde = bcde;
        *acde = Tetrahedron {
            vertices: [a, c, d, e],
            neighbors: [edc_nb_id, adbe_id, abce_id, acd_nb_id],
        };

        let adbe = Tetrahedron {
            vertices: [a, d, b, e],
            neighbors: [ebd_nb_id, abce_id, acde_id, adb_nb_id],
        };

        self.tetrahedra.push(adbe);

        // Update invalidated neighbor IDs for affected neighbors
        if acd_nb_id != NO_TETRAHEDRON_ID {
            let acd_nb = &mut self.tetrahedra[acd_nb_id as usize];
            acd_nb.replace_neighbor_id(abcd_id, acde_id);
        }
        if adb_nb_id != NO_TETRAHEDRON_ID {
            let adb_nb = &mut self.tetrahedra[adb_nb_id as usize];
            adb_nb.replace_neighbor_id(abcd_id, adbe_id);
        }
        if ecb_nb_id != NO_TETRAHEDRON_ID {
            let ecb_nb = &mut self.tetrahedra[ecb_nb_id as usize];
            ecb_nb.replace_neighbor_id(bcde_id, abce_id);
        }
        if ebd_nb_id != NO_TETRAHEDRON_ID {
            let ebd_nb = &mut self.tetrahedra[ebd_nb_id as usize];
            ebd_nb.replace_neighbor_id(bcde_id, adbe_id);
        }

        // Update potentially invalidated tetrahedron ID for applicable vertices
        self.vertices[b as usize].tetra_id = abce_id;
        self.vertices[d as usize].tetra_id = acde_id;

        Ok([abce_id, acde_id, adbe_id])
    }

    /// Takes the IDs of three adjacent tetrahedra ABCD, BCDE and AXYE sharing
    /// an edge XY = [CB, DC, BD] and reconfigures them into two adjacent
    /// tetrahedra AXZE and AZYE sharing the face AZE (where Z is the third
    /// vertex of the face BDC containing the shared edge XY). Returns the
    /// respective IDs of the two reconfigured tetrahedra, along with the ID
    /// change caused by the removal of the last of the input tetrahedra. The
    /// vertices in the new tetrahedra will be in the order implied by their
    /// names.
    ///
    /// Which vertex is assigned which label is determined by taking the first
    /// vertex of the first input tetrahedron as A and using the relationships
    /// between the faces of the other tetrahedra to determine the rest.
    ///
    /// This is the inverse of
    /// [`reconnect_two_to_three`](Self::reconnect_two_to_three).
    fn reconnect_three_to_two(
        &mut self,
        abcd_id: TetrahedronID,
        bcde_id: TetrahedronID,
        axye_id: TetrahedronID,
    ) -> ([TetrahedronID; 2], TetrahedronIDChange) {
        debug_assert_ne!(abcd_id, NO_TETRAHEDRON_ID);
        debug_assert_ne!(bcde_id, NO_TETRAHEDRON_ID);
        debug_assert_ne!(axye_id, NO_TETRAHEDRON_ID);

        let mut axze_id = abcd_id;
        let mut azye_id = bcde_id;

        let [abcd, bcde, axye] = self
            .tetrahedra
            .get_disjoint_mut([abcd_id as usize, bcde_id as usize, axye_id as usize])
            .unwrap();

        let [a, b, c, d] = abcd.vertices;

        let [x, y, z] = if axye_id == abcd.neighbors[1] {
            [d, c, b]
        } else if axye_id == abcd.neighbors[2] {
            [b, d, c]
        } else if axye_id == abcd.neighbors[3] {
            [c, b, d]
        } else {
            panic!("AXYE {axye_id} does not adjoin a non-shared face of ABCD {abcd_id}");
        };

        let e_corner = bcde.corner_not_on_face([b, c, d]);
        let e = bcde.vertices[e_corner];

        debug_assert_eq!(axye.vertices[axye.corner_not_on_face([a, x, y])], e);

        let azy_nb_id = abcd.id_of_neighbor_opposite_vertex(x);
        let axz_nb_id = abcd.id_of_neighbor_opposite_vertex(y);
        debug_assert_eq!(abcd.id_of_neighbor_opposite_vertex(z), axye_id);
        debug_assert_eq!(abcd.id_of_neighbor_opposite_vertex(a), bcde_id);

        let eyz_nb_id = bcde.id_of_neighbor_opposite_vertex(x);
        let ezx_nb_id = bcde.id_of_neighbor_opposite_vertex(y);
        let yea_nb_id = axye.id_of_neighbor_opposite_vertex(x);
        let xae_nb_id = axye.id_of_neighbor_opposite_vertex(y);

        let axze = abcd;
        *axze = Tetrahedron {
            vertices: [a, x, z, e],
            neighbors: [ezx_nb_id, azye_id, xae_nb_id, axz_nb_id],
        };

        let azye = bcde;
        *azye = Tetrahedron {
            vertices: [a, z, y, e],
            neighbors: [eyz_nb_id, yea_nb_id, axze_id, azy_nb_id],
        };

        // Update invalidated neighbor IDs for affected neighbors
        if azy_nb_id != NO_TETRAHEDRON_ID {
            let azy_nb = &mut self.tetrahedra[azy_nb_id as usize];
            azy_nb.replace_neighbor_id(abcd_id, azye_id);
        }
        if ezx_nb_id != NO_TETRAHEDRON_ID {
            let ezx_nb = &mut self.tetrahedra[ezx_nb_id as usize];
            ezx_nb.replace_neighbor_id(bcde_id, axze_id);
        }
        if xae_nb_id != NO_TETRAHEDRON_ID {
            let xae_nb = &mut self.tetrahedra[xae_nb_id as usize];
            xae_nb.replace_neighbor_id(axye_id, axze_id);
        }
        if yea_nb_id != NO_TETRAHEDRON_ID {
            let yea_nb = &mut self.tetrahedra[yea_nb_id as usize];
            yea_nb.replace_neighbor_id(axye_id, azye_id);
        }

        // Update potentially invalidated tetrahedron ID for applicable vertices
        self.vertices[a as usize].tetra_id = axze_id;
        self.vertices[x as usize].tetra_id = axze_id;
        self.vertices[y as usize].tetra_id = azye_id;
        self.vertices[e as usize].tetra_id = axze_id;

        // Remove AXYE
        let id_change = self.remove_tetrahedron(axye_id);

        if id_change.old == axze_id {
            axze_id = id_change.new;
        } else if id_change.old == azye_id {
            azye_id = id_change.new;
        }

        ([axze_id, azye_id], id_change)
    }

    /// Takes the IDs of four adjacent tetrahedra ABCD, BCDE, AXYF and XYFE
    /// sharing an edge XY = [CB, DC, BD] and reconfigures them into four
    /// adjacent tetrahedra AXZE, AZYE, AFXE and AYFE sharing the edge AE (where
    /// Z is the third vertex of the face BDC containing the shared edge XY).
    /// Returns the respective IDs of the four reconfigured tetrahedra. The
    /// vertices in the new tetrahedra will be in the order implied by their
    /// names.
    ///
    /// Which vertex is assigned which label is determined by taking the first
    /// vertex of the first input tetrahedron as A and using the relationships
    /// between the faces of the other tetrahedra to determine the rest.
    fn reconnect_four_to_four(
        &mut self,
        abcd_id: TetrahedronID,
        bcde_id: TetrahedronID,
        axyf_id: TetrahedronID,
        xyfe_id: TetrahedronID,
    ) -> [TetrahedronID; 4] {
        debug_assert_ne!(abcd_id, NO_TETRAHEDRON_ID);
        debug_assert_ne!(bcde_id, NO_TETRAHEDRON_ID);
        debug_assert_ne!(axyf_id, NO_TETRAHEDRON_ID);
        debug_assert_ne!(xyfe_id, NO_TETRAHEDRON_ID);

        let axze_id = abcd_id;
        let azye_id = bcde_id;
        let afxe_id = axyf_id;
        let ayfe_id = xyfe_id;

        let [abcd, bcde, axyf, xyfe] = self
            .tetrahedra
            .get_disjoint_mut([
                abcd_id as usize,
                bcde_id as usize,
                axyf_id as usize,
                xyfe_id as usize,
            ])
            .unwrap();

        let [a, b, c, d] = abcd.vertices;

        let [x, y, z] = if axyf_id == abcd.neighbors[1] {
            [d, c, b]
        } else if axyf_id == abcd.neighbors[2] {
            [b, d, c]
        } else if axyf_id == abcd.neighbors[3] {
            [c, b, d]
        } else {
            panic!("AXYF {axyf_id} does not adjoin a non-shared face of ABCD {abcd_id}");
        };

        let e_corner = bcde.corner_not_on_face([b, c, d]);
        let e = bcde.vertices[e_corner];

        let f_corner = axyf.corner_not_on_face([a, x, y]);
        let f = axyf.vertices[f_corner];

        debug_assert_eq!(xyfe.vertices[xyfe.corner_not_on_face([y, f, x])], e);
        debug_assert_eq!(xyfe.vertices[xyfe.corner_not_on_face([e, y, x])], f);

        let azy_nb_id = abcd.id_of_neighbor_opposite_vertex(x);
        let axz_nb_id = abcd.id_of_neighbor_opposite_vertex(y);
        debug_assert_eq!(abcd.id_of_neighbor_opposite_vertex(z), axyf_id);
        debug_assert_eq!(abcd.id_of_neighbor_opposite_vertex(a), bcde_id);

        let eyz_nb_id = bcde.id_of_neighbor_opposite_vertex(x);
        let ezx_nb_id = bcde.id_of_neighbor_opposite_vertex(y);
        let ayf_nb_id = axyf.id_of_neighbor_opposite_vertex(x);
        let afx_nb_id = axyf.id_of_neighbor_opposite_vertex(y);
        let efy_nb_id = xyfe.id_of_neighbor_opposite_vertex(x);
        let exf_nb_id = xyfe.id_of_neighbor_opposite_vertex(y);

        let axze = abcd;
        *axze = Tetrahedron {
            vertices: [a, x, z, e],
            neighbors: [ezx_nb_id, azye_id, afxe_id, axz_nb_id],
        };

        let azye = bcde;
        *azye = Tetrahedron {
            vertices: [a, z, y, e],
            neighbors: [eyz_nb_id, ayfe_id, axze_id, azy_nb_id],
        };

        let afxe = axyf;
        *afxe = Tetrahedron {
            vertices: [a, f, x, e],
            neighbors: [exf_nb_id, axze_id, ayfe_id, afx_nb_id],
        };

        let ayfe = xyfe;
        *ayfe = Tetrahedron {
            vertices: [a, y, f, e],
            neighbors: [efy_nb_id, afxe_id, azye_id, ayf_nb_id],
        };

        // Update invalidated neighbor IDs for affected neighbors
        if azy_nb_id != NO_TETRAHEDRON_ID {
            let azy_nb = &mut self.tetrahedra[azy_nb_id as usize];
            azy_nb.replace_neighbor_id(abcd_id, azye_id);
        }
        if ezx_nb_id != NO_TETRAHEDRON_ID {
            let ezx_nb = &mut self.tetrahedra[ezx_nb_id as usize];
            ezx_nb.replace_neighbor_id(bcde_id, axze_id);
        }
        if ayf_nb_id != NO_TETRAHEDRON_ID {
            let ayf_nb = &mut self.tetrahedra[ayf_nb_id as usize];
            ayf_nb.replace_neighbor_id(axyf_id, ayfe_id);
        }
        if exf_nb_id != NO_TETRAHEDRON_ID {
            let exf_nb = &mut self.tetrahedra[exf_nb_id as usize];
            exf_nb.replace_neighbor_id(xyfe_id, afxe_id);
        }

        // Update potentially invalidated tetrahedron ID for applicable vertices
        self.vertices[x as usize].tetra_id = axze_id;
        self.vertices[y as usize].tetra_id = azye_id;

        [axze_id, azye_id, afxe_id, ayfe_id]
    }
}

impl Tetrahedron {
    /// Returns the position of the vertex at the given corner (index in
    /// `self.vertices`) for this tetrahedron.
    #[inline]
    pub fn vertex_point<'a>(&self, vertices: &'a [Vertex], corner: usize) -> &'a Point3C {
        &vertices[self.vertices[corner] as usize].point
    }

    /// Returns the positions of the vertices of this tetrahedron.
    #[inline]
    pub fn vertex_points(&self, vertices: &[Vertex]) -> [Point3C; 4] {
        self.vertices.map(|idx| vertices[idx as usize].point)
    }

    /// Computes the center of the sphere passing through all four vertices of
    /// the tetrahedron.
    #[inline]
    pub fn compute_circumcenter(&self, vertices: &[Vertex]) -> Point3C {
        let [a, b, c, d] = self.vertices;
        compute_circumcenter(
            &vertices[a as usize].point,
            &vertices[b as usize].point,
            &vertices[c as usize].point,
            &vertices[d as usize].point,
        )
    }

    /// Returns the position of the given vertex in the tetrahedron's `vertices`
    /// array.
    ///
    /// This method assumes, without verifying, that the given vertex is indeed
    /// part of the tetrahedron.
    #[inline]
    pub fn corner_of_vertex(&self, vertex: VertexIdx) -> usize {
        debug_assert!(self.vertices.contains(&vertex));

        let [_, b, c, d] = self.vertices;
        let mut corner = 0;

        if vertex == b {
            corner = 1;
        }
        if vertex == c {
            corner = 2;
        }
        if vertex == d {
            corner = 3;
        }

        corner
    }

    /// Returns the position of the vertex in the tetrahedron `vertices` array
    /// that is not on the face defined by the given vertices.
    ///
    /// This method assumes, without verifying, that all vertices in `face` are
    /// indeed part of the tetrahedron.
    #[inline]
    fn corner_not_on_face(&self, face: [VertexIdx; 3]) -> usize {
        let [v1, v2, v3] = face;
        debug_assert!(
            self.vertices.contains(&v1)
                && self.vertices.contains(&v2)
                && self.vertices.contains(&v3)
        );

        let [_, b, c, d] = self.vertices;
        let mut corner = 0;

        // Use bitwise AND to avoid introducing branches
        if (v1 != b) & (v2 != b) & (v3 != b) {
            corner = 1;
        }
        if (v1 != c) & (v2 != c) & (v3 != c) {
            corner = 2;
        }
        if (v1 != d) & (v2 != d) & (v3 != d) {
            corner = 3;
        }

        corner
    }

    /// Returns the ID of the neighbor tetrahedron adjoining the face opposite
    /// the given vertex.
    ///
    /// This method assumes, without verifying, that the given vertex is indeed
    /// part of the tetrahedron.
    #[inline]
    pub fn id_of_neighbor_opposite_vertex(&self, vertex: VertexIdx) -> TetrahedronID {
        let corner = self.corner_of_vertex(vertex);
        self.neighbors[corner]
    }

    /// Returns the IDs of the neighbor tetrahedra sharing the given vertex with
    /// this tetrahedron.
    ///
    /// This method assumes, without verifying, that the given vertex is indeed
    /// part of the tetrahedron.
    #[inline]
    pub fn neighbors_with_vertex(&self, vertex: VertexIdx) -> [TetrahedronID; 3] {
        let corner = self.corner_of_vertex(vertex);
        self.neighbors_with_corner(corner)
    }

    /// Returns the IDs of the neighbor tetrahedra sharing the vertex at the
    /// given corner with this tetrahedron.
    #[inline]
    pub fn neighbors_with_corner(&self, corner: usize) -> [TetrahedronID; 3] {
        if corner == 0 {
            [self.neighbors[1], self.neighbors[2], self.neighbors[3]]
        } else if corner == 1 {
            [self.neighbors[0], self.neighbors[2], self.neighbors[3]]
        } else if corner == 2 {
            [self.neighbors[0], self.neighbors[1], self.neighbors[3]]
        } else {
            [self.neighbors[0], self.neighbors[1], self.neighbors[2]]
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn face_opposite_neighbor(&self, neighbor_idx: usize) -> [VertexIdx; 3] {
        let [a, b, c, d] = self.vertices;
        match neighbor_idx {
            0 => [b, d, c],
            1 => [a, c, d],
            2 => [a, d, b],
            3 => [a, b, c],
            _ => panic!("Invalid neighbor index {neighbor_idx}"),
        }
    }

    /// Assumes that `old_id` is present.
    #[inline]
    fn replace_neighbor_id(&mut self, old_id: TetrahedronID, new_id: TetrahedronID) {
        debug_assert!(self.neighbors.contains(&old_id));

        let [_, acd, adb, abc] = self.neighbors;
        let mut neighbor = 0;

        if old_id == acd {
            neighbor = 1;
        }
        if old_id == adb {
            neighbor = 2;
        }
        if old_id == abc {
            neighbor = 3;
        }

        self.neighbors[neighbor] = new_id;
    }

    #[inline]
    fn next_neighbor_towards_point(
        &self,
        vertices: &[Vertex],
        point: &Point3C,
        rng: &mut Rng,
    ) -> Option<TetrahedronID> {
        let va = self.vertex_point(vertices, 0);
        let vb = self.vertex_point(vertices, 1);
        let vc = self.vertex_point(vertices, 2);
        let vd = self.vertex_point(vertices, 3);

        let triangles = [[vb, vd, vc], [va, vc, vd], [va, vd, vb], [va, vb, vc]];

        let mut neighbor_indices = [0, 1, 2, 3];
        rng.shuffle(&mut neighbor_indices);

        for neighbor_idx in neighbor_indices {
            let [v1, v2, v3] = triangles[neighbor_idx];
            if evaluate_side_of_triangle_plane_for_point(v1, v2, v3, point)
                == PointTrianglePlaneSide::Negative
            {
                return Some(self.neighbors[neighbor_idx]);
            }
        }
        // Point lies inside the tetrahedron
        None
    }
}

impl Vertex {
    fn new(point: Point3C, tetra_id: TetrahedronID) -> Self {
        Self { point, tetra_id }
    }
}

impl From<Point3C> for Vertex {
    fn from(point: Point3C) -> Self {
        Self::new(point, NO_TETRAHEDRON_ID)
    }
}

impl TetrahedronPointLocator {
    fn new(seed: u64) -> Self {
        let rng = Rng::with_seed(seed);
        Self { rng }
    }

    fn n_candidates_for_vertices(n_vertices: usize) -> usize {
        // From Mücke et al. (1999)
        let n_candidates = 7.0 * (n_vertices as f32).sqrt().sqrt();
        (n_candidates.ceil().max(1.0) as usize).min(n_vertices)
    }

    fn find_tetrahedron_containing_point<A: Allocator>(
        &mut self,
        tetras: &Tetrahedralization<A>,
        point: &Point3C,
    ) -> TetrahedronID {
        if tetras.n_tetrahedra() == 0 {
            return NO_TETRAHEDRON_ID;
        }

        let vertices = tetras.vertices();

        let n_candidates = Self::n_candidates_for_vertices(vertices.len());

        let mut closest_dist_sq = f32::INFINITY;
        let mut current_tetra_id = NO_TETRAHEDRON_ID;

        for _ in 0..n_candidates {
            let idx = self.rng.random_u32_in_range(0..vertices.len() as u32);
            let vertex = vertices[idx as usize];
            let dist_sq = Point3C::squared_distance_between(&vertex.point, point);
            if dist_sq < closest_dist_sq {
                closest_dist_sq = dist_sq;
                current_tetra_id = vertex.tetra_id;
            }
        }

        assert_ne!(current_tetra_id, NO_TETRAHEDRON_ID);

        loop {
            let tetra = tetras.tetrahedron(current_tetra_id);
            if let Some(neighbor_id) =
                tetra.next_neighbor_towards_point(vertices, point, &mut self.rng)
            {
                debug_assert_ne!(neighbor_id, NO_TETRAHEDRON_ID);
                current_tetra_id = neighbor_id;
            } else {
                break;
            }
        }

        current_tetra_id
    }
}

#[inline]
fn estimated_tetrahedron_count(n_vertices: usize) -> usize {
    // For many uniformly random points, we expect ~6n tetrahedra for n
    // vertices. For more clustered points, we expect O(n²). We conservatively
    // use 6n.
    6 * n_vertices
}

#[inline]
fn find_tetrahedron_encompassing_sphere(center: &Point3, radius: f32) -> [Point3C; 4] {
    let [x, y, z] = (*center).into();

    let edge_length = 12.0 * FRAC_1_SQRT_6 * radius;
    let triangle_height = 0.5 * SQRT_3 * edge_length;
    let tetrahedron_height = (SQRT_6 / 3.0) * edge_length;

    let min_x = x - 0.5 * edge_length;
    let max_x = x + 0.5 * edge_length;
    let min_y = y - radius;
    let max_y = min_y + tetrahedron_height;
    let min_z = z - (1.0 / 3.0) * triangle_height;
    let max_z = z + (2.0 / 3.0) * triangle_height;

    [
        Point3C::new(min_x, min_y, min_z),
        Point3C::new(x, min_y, max_z),
        Point3C::new(max_x, min_y, min_z),
        Point3C::new(x, max_y, z),
    ]
}

#[inline]
fn evaluate_side_of_triangle_plane_for_point(
    vertex_a: &Point3C,
    vertex_b: &Point3C,
    vertex_c: &Point3C,
    point: &Point3C,
) -> PointTrianglePlaneSide {
    let factor = robust::orient3d(
        point_to_robust_coord(vertex_a),
        point_to_robust_coord(vertex_b),
        point_to_robust_coord(vertex_c),
        point_to_robust_coord(point),
    );

    if factor < 0.0 {
        PointTrianglePlaneSide::Positive
    } else if factor > 0.0 {
        PointTrianglePlaneSide::Negative
    } else {
        PointTrianglePlaneSide::InPlane
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointTrianglePlaneSide {
    Positive,
    Negative,
    InPlane,
}

impl PointTrianglePlaneSide {
    #[inline]
    fn is_positive(&self) -> bool {
        *self == Self::Positive
    }

    #[inline]
    fn is_negative(&self) -> bool {
        *self == Self::Negative
    }

    #[inline]
    fn is_in_plane(&self) -> bool {
        *self == Self::InPlane
    }
}

#[inline]
fn point_lies_strictly_inside_circumsphere(
    vertex_a: &Point3C,
    vertex_b: &Point3C,
    vertex_c: &Point3C,
    vertex_d: &Point3C,
    point: &Point3C,
) -> bool {
    let factor = robust::insphere(
        // Swap order to satisfy `robust`'s definition of positive orientation
        point_to_robust_coord(vertex_b),
        point_to_robust_coord(vertex_a),
        point_to_robust_coord(vertex_c),
        point_to_robust_coord(vertex_d),
        point_to_robust_coord(point),
    );

    factor > 0.0
}

#[inline]
fn point_to_robust_coord(point: &Point3C) -> robust::Coord3D<f32> {
    robust::Coord3D {
        x: point.x(),
        y: point.y(),
        z: point.z(),
    }
}

/// Given two points P and Q on an infinite line, returns whether the line
/// passes inside, outside, or through specific edges of the triangle with
/// vertices A, B and C.
///
/// The test assumes that the vertices are ordered so that applying the
/// right-hand rule to them points the thumb in the opposite direction as the
/// vector from P to Q.
///
/// If P and Q both lie in the triangle plane, the function will report that all
/// edges are intersected, even if the line misses the triangle.
///
/// Based on "Real-Time Collision Detection" (Ericson 2005).
#[inline]
fn evaluate_infinite_line_triangle_intersection_one_sided(
    line_point_p: &Point3C,
    line_point_q: &Point3C,
    vertex_a: &Point3C,
    vertex_b: &Point3C,
    vertex_c: &Point3C,
) -> LineTriangleIntersection {
    let ab_side =
        evaluate_side_of_triangle_plane_for_point(vertex_a, vertex_b, line_point_p, line_point_q);
    let bc_side =
        evaluate_side_of_triangle_plane_for_point(vertex_b, vertex_c, line_point_p, line_point_q);
    let ca_side =
        evaluate_side_of_triangle_plane_for_point(vertex_c, vertex_a, line_point_p, line_point_q);

    if ab_side.is_positive() || bc_side.is_positive() || ca_side.is_positive() {
        return LineTriangleIntersection::Outside {
            ab: ab_side.is_positive(),
            bc: bc_side.is_positive(),
            ca: ca_side.is_positive(),
        };
    }

    if ab_side.is_negative() && bc_side.is_negative() && ca_side.is_negative() {
        return LineTriangleIntersection::Inside;
    }

    LineTriangleIntersection::Edges {
        ab: ab_side.is_in_plane(),
        bc: bc_side.is_in_plane(),
        ca: ca_side.is_in_plane(),
    }
}

#[inline]
fn compute_circumcenter(
    vertex_a: &Point3C,
    vertex_b: &Point3C,
    vertex_c: &Point3C,
    vertex_d: &Point3C,
) -> Point3C {
    let a = vertex_a.aligned();
    let b = vertex_b.aligned();
    let c = vertex_c.aligned();
    let d = vertex_d.aligned();

    let da = a - d;
    let db = b - d;
    let dc = c - d;

    let da2 = da.norm_squared();
    let db2 = db.norm_squared();
    let dc2 = dc.norm_squared();

    let det_r = Matrix3::from_columns(da, db, dc).determinant();

    let det_x = Matrix3::from_columns(
        Vector3::new(da2, da.y(), da.z()),
        Vector3::new(db2, db.y(), db.z()),
        Vector3::new(dc2, dc.y(), dc.z()),
    )
    .determinant();

    let det_y = Matrix3::from_columns(
        Vector3::new(da2, da.x(), da.z()),
        Vector3::new(db2, db.x(), db.z()),
        Vector3::new(dc2, dc.x(), dc.z()),
    )
    .determinant();

    let det_z = Matrix3::from_columns(
        Vector3::new(da2, da.x(), da.y()),
        Vector3::new(db2, db.x(), db.y()),
        Vector3::new(dc2, dc.x(), dc.y()),
    )
    .determinant();

    let scale = (2.0 * det_r).recip();

    let center = d + Vector3::new(scale * det_x, -scale * det_y, scale * det_z);

    center.compact()
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LineTriangleIntersection {
    Inside,
    Outside { ab: bool, bc: bool, ca: bool },
    Edges { ab: bool, bc: bool, ca: bool },
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use arbitrary::{Arbitrary, Result, Unstructured};
    use bytemuck::{Pod, Zeroable};
    use impact_alloc::Global;
    use std::mem;

    const FLOAT_RESOLUTION: u32 = 10000;
    const DOMAIN_EXTENT: f32 = 100.0;

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Zeroable, Pod)]
    pub struct DelaunayPoint(Point3C);

    impl Arbitrary<'_> for DelaunayPoint {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let x = DOMAIN_EXTENT * arbitrary_norm_f32(u)? - (0.5 * DOMAIN_EXTENT);
            let y = DOMAIN_EXTENT * arbitrary_norm_f32(u)? - (0.5 * DOMAIN_EXTENT);
            let z = DOMAIN_EXTENT * arbitrary_norm_f32(u)? - (0.5 * DOMAIN_EXTENT);
            Ok(Self(Point3C::new(x, y, z)))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 3 * mem::size_of::<u32>();
            (size, Some(size))
        }
    }

    pub fn fuzz_test_delaunay_tetrahedralization(input: Vec<DelaunayPoint>) {
        let points = bytemuck::cast_slice(&input);
        let tetrahedra = DelaunayTetrahedralization::construct(Global, points).unwrap();
        tetrahedra.validate_brute_force();
    }

    fn arbitrary_norm_f32(u: &mut Unstructured<'_>) -> Result<f32> {
        Ok((f64::from(u.int_in_range(0..=FLOAT_RESOLUTION)?) / f64::from(FLOAT_RESOLUTION)) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_alloc::Global;
    use impact_geometry::Plane;
    use impact_math::vector::{UnitVector3, Vector3C};

    #[test]
    fn delaunay_tetrahedralization_of_less_than_four_points_is_empty() {
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].map(Point3C::from);
        let tetrahedra = DelaunayTetrahedralization::construct(Global, &points).unwrap();
        assert_eq!(tetrahedra.n_tetrahedra(), 0);
    }

    #[test]
    fn delaunay_tetrahedralization_of_coplanar_points_is_empty() {
        let points = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]
        .map(Point3C::from);

        let tetrahedra = DelaunayTetrahedralization::construct(Global, &points).unwrap();
        assert_eq!(tetrahedra.n_tetrahedra(), 0);
    }

    #[test]
    fn delaunay_tetrahedralization_of_four_points_is_valid() {
        let points = [
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ]
        .map(Point3C::from);

        let tetrahedra = DelaunayTetrahedralization::construct(Global, &points).unwrap();
        assert_eq!(tetrahedra.n_tetrahedra(), 1);
        tetrahedra.validate_brute_force();
    }

    #[test]
    fn delaunay_tetrahedralization_of_five_points_is_valid() {
        let points = [
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ]
        .map(Point3C::from);

        let tetrahedra = DelaunayTetrahedralization::construct(Global, &points).unwrap();
        assert_eq!(tetrahedra.n_tetrahedra(), 2);
        tetrahedra.validate_brute_force();
    }

    #[test]
    fn delaunay_tetrahedralization_ignores_coincident_points() {
        let points = [
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]
        .map(Point3C::from);

        let tetrahedra = DelaunayTetrahedralization::construct(Global, &points).unwrap();
        assert_eq!(tetrahedra.n_tetrahedra(), 1);
        tetrahedra.validate_brute_force();
    }

    #[test]
    fn delaunay_tetrahedralization_of_randomized_grid_is_valid() {
        let mut rng = Rng::with_seed(0);
        let mut points = Vec::new();
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    points.push(Point3C::new(
                        i as f32 + (rng.random_f32_fraction() - 0.5),
                        j as f32 + (rng.random_f32_fraction() - 0.5),
                        k as f32 + (rng.random_f32_fraction() - 0.5),
                    ));
                }
            }
        }

        let tetrahedra = DelaunayTetrahedralization::construct(Global, &points).unwrap();
        assert!(tetrahedra.n_tetrahedra() > 0);
        tetrahedra.validate_brute_force();
    }

    #[test]
    fn delaunay_tetrahedralization_of_regular_grid_is_valid() {
        let mut points = Vec::new();
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    points.push(Point3C::new(i as f32, j as f32, k as f32));
                }
            }
        }

        let tetrahedra = DelaunayTetrahedralization::construct(Global, &points).unwrap();
        assert!(tetrahedra.n_tetrahedra() > 0);
        tetrahedra.validate_brute_force();
    }

    #[test]
    fn tetrahedron_encompasses_sphere() {
        let center = Point3::new(1.0, 2.0, 3.0);
        let radius = 4.0;

        let [a, b, c, d] = find_tetrahedron_encompassing_sphere(&center, radius);

        let a = a.aligned();
        let b = b.aligned();
        let c = c.aligned();
        let d = d.aligned();

        let abc_normal = UnitVector3::normalized_from((b - a).cross(&(c - a)));
        let acd_normal = UnitVector3::normalized_from((c - a).cross(&(d - a)));
        let adb_normal = UnitVector3::normalized_from((d - a).cross(&(b - a)));
        let bdc_normal = UnitVector3::normalized_from((d - b).cross(&(c - b)));

        let abc = Plane::from_normal_and_point(abc_normal, &a);
        let acd = Plane::from_normal_and_point(acd_normal, &a);
        let adb = Plane::from_normal_and_point(adb_normal, &a);
        let bdc = Plane::from_normal_and_point(bdc_normal, &b);

        assert_abs_diff_eq!(abc.compute_signed_distance(&center), radius, epsilon = 1e-5);
        assert_abs_diff_eq!(acd.compute_signed_distance(&center), radius, epsilon = 1e-5);
        assert_abs_diff_eq!(adb.compute_signed_distance(&center), radius, epsilon = 1e-5);
        assert_abs_diff_eq!(bdc.compute_signed_distance(&center), radius, epsilon = 1e-5);
    }

    #[test]
    fn barely_above_point_lies_on_positive_side_of_horizontal_triangle_plane() {
        assert_eq!(
            evaluate_side_of_triangle_plane_for_point(
                &Point3C::new(-1.0, 0.0, 0.0),
                &Point3C::new(0.0, 0.0, 1.0),
                &Point3C::new(1.0, 0.0, 0.0),
                &Point3C::new(0.0, 0.001, 0.0),
            ),
            PointTrianglePlaneSide::Positive
        );
    }

    #[test]
    fn plane_point_lies_in_plane_for_horizontal_triangle_plane() {
        assert_eq!(
            evaluate_side_of_triangle_plane_for_point(
                &Point3C::new(-1.0, 0.0, 0.0),
                &Point3C::new(0.0, 0.0, 1.0),
                &Point3C::new(1.0, 0.0, 0.0),
                &Point3C::new(0.0, 0.0, 0.0),
            ),
            PointTrianglePlaneSide::InPlane
        );
    }

    #[test]
    fn barely_below_point_lies_on_negative_side_of_horizontal_triangle_plane() {
        assert_eq!(
            evaluate_side_of_triangle_plane_for_point(
                &Point3C::new(-1.0, 0.0, 0.0),
                &Point3C::new(0.0, 0.0, 1.0),
                &Point3C::new(1.0, 0.0, 0.0),
                &Point3C::new(0.0, -0.001, 0.0),
            ),
            PointTrianglePlaneSide::Negative
        );
    }

    #[test]
    fn barely_inside_point_lies_strictly_inside_sphere() {
        assert!(point_lies_strictly_inside_circumsphere(
            &Point3C::new(-1.0, 0.0, 0.0),
            &Point3C::new(0.0, 0.0, 1.0),
            &Point3C::new(1.0, 0.0, 0.0),
            &Point3C::new(0.0, 1.0, 0.0),
            &Point3C::new(0.0, 0.0, -0.999),
        ));
    }

    #[test]
    fn boundary_point_does_not_lie_strictly_inside_sphere() {
        assert!(!point_lies_strictly_inside_circumsphere(
            &Point3C::new(-1.0, 0.0, 0.0),
            &Point3C::new(0.0, 0.0, 1.0),
            &Point3C::new(1.0, 0.0, 0.0),
            &Point3C::new(0.0, 1.0, 0.0),
            &Point3C::new(0.0, 0.0, -1.0),
        ));
    }

    #[test]
    fn barely_outside_point_does_not_lie_strictly_inside_sphere() {
        assert!(!point_lies_strictly_inside_circumsphere(
            &Point3C::new(-1.0, 0.0, 0.0),
            &Point3C::new(0.0, 0.0, 1.0),
            &Point3C::new(1.0, 0.0, 0.0),
            &Point3C::new(0.0, 1.0, 0.0),
            &Point3C::new(0.0, 0.0, -1.001),
        ));
    }

    #[test]
    fn line_through_interior_gives_inside() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        let p = Point3C::new(1.0 / 3.0, 1.0 / 3.0, 1.0);
        let q = Point3C::new(1.0 / 3.0, 1.0 / 3.0, -1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(result, LineTriangleIntersection::Inside));
    }

    #[test]
    fn reversed_line_direction_gives_outside() {
        // Same geometry as the interior test but P and Q are swapped;
        // the wrong orientation gives Outside.
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        let p = Point3C::new(1.0 / 3.0, 1.0 / 3.0, -1.0);
        let q = Point3C::new(1.0 / 3.0, 1.0 / 3.0, 1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        // The reversed orientation places the line beyond all three edges.
        assert!(matches!(
            result,
            LineTriangleIntersection::Outside {
                ab: true,
                bc: true,
                ca: true
            }
        ));
    }

    #[test]
    fn line_past_edge_bc_gives_outside() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        // Line at (1, 1) is past edge BC where x+y=1.
        let p = Point3C::new(1.0, 1.0, 1.0);
        let q = Point3C::new(1.0, 1.0, -1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(
            result,
            LineTriangleIntersection::Outside {
                ab: false,
                bc: true,
                ca: false
            }
        ));
    }

    #[test]
    fn line_through_vertex_a_gives_edges_ab_and_ca() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        let p = Point3C::new(0.0, 0.0, 1.0);
        let q = Point3C::new(0.0, 0.0, -1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(
            result,
            LineTriangleIntersection::Edges {
                ab: true,
                bc: false,
                ca: true
            }
        ));
    }

    #[test]
    fn line_through_vertex_b_gives_edges_ab_and_bc() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        let p = Point3C::new(1.0, 0.0, 1.0);
        let q = Point3C::new(1.0, 0.0, -1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(
            result,
            LineTriangleIntersection::Edges {
                ab: true,
                bc: true,
                ca: false
            }
        ));
    }

    #[test]
    fn line_through_vertex_c_gives_edges_bc_and_ca() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        let p = Point3C::new(0.0, 1.0, 1.0);
        let q = Point3C::new(0.0, 1.0, -1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(
            result,
            LineTriangleIntersection::Edges {
                ab: false,
                bc: true,
                ca: true
            }
        ));
    }

    #[test]
    fn line_through_midpoint_of_edge_ab_gives_ab_edge_only() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        // Midpoint of AB is (0.5, 0, 0).
        let p = Point3C::new(0.5, 0.0, 1.0);
        let q = Point3C::new(0.5, 0.0, -1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(
            result,
            LineTriangleIntersection::Edges {
                ab: true,
                bc: false,
                ca: false
            }
        ));
    }

    #[test]
    fn line_through_midpoint_of_edge_bc_gives_bc_edge_only() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        // Midpoint of BC is (0.5, 0.5, 0).
        let p = Point3C::new(0.5, 0.5, 1.0);
        let q = Point3C::new(0.5, 0.5, -1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(
            result,
            LineTriangleIntersection::Edges {
                ab: false,
                bc: true,
                ca: false
            }
        ));
    }

    #[test]
    fn line_through_midpoint_of_edge_ca_gives_ca_edge_only() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        // Midpoint of CA is (0, 0.5, 0).
        let p = Point3C::new(0.0, 0.5, 1.0);
        let q = Point3C::new(0.0, 0.5, -1.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(
            result,
            LineTriangleIntersection::Edges {
                ab: false,
                bc: false,
                ca: true
            }
        ));
    }

    #[test]
    fn coplanar_line_gives_all_edges() {
        let a = Point3C::origin();
        let b = Point3C::new(1.0, 0.0, 0.0);
        let c = Point3C::new(0.0, 1.0, 0.0);
        // Line in the z=0 plane; all scalar triple products vanish, so all
        // edges are reported even though the line misses the triangle.
        let p = Point3C::new(0.0, -1.0, 0.0);
        let q = Point3C::new(1.0, -1.0, 0.0);

        let result = evaluate_infinite_line_triangle_intersection_one_sided(&p, &q, &a, &b, &c);

        assert!(matches!(
            result,
            LineTriangleIntersection::Edges {
                ab: true,
                bc: true,
                ca: true
            }
        ));
    }

    #[test]
    fn circumcenter_is_correct_for_simple_points() {
        let computed_center = compute_circumcenter(
            &Point3C::new(-1.0, 0.0, 0.0),
            &Point3C::new(0.0, 0.0, 1.0),
            &Point3C::new(1.0, 0.0, 0.0),
            &Point3C::new(0.0, 1.0, 0.0),
        );
        assert_abs_diff_eq!(computed_center, Point3C::origin());
    }

    #[test]
    fn circumcenter_is_correct_for_close_points() {
        let center = Point3C::new(1.0, 2.0, 3.0);
        let radius = 4.0;

        let offset = 0.2;

        let computed_center = compute_circumcenter(
            &(center + radius * Vector3C::new(1.0 - offset, 1.0, 1.0).normalized()),
            &(center + radius * Vector3C::new(1.0, 1.0, 1.0 + offset).normalized()),
            &(center + radius * Vector3C::new(1.0 + offset, 1.0, 1.0).normalized()),
            &(center + radius * Vector3C::new(1.0, 1.0 + offset, 1.0).normalized()),
        );
        assert_abs_diff_eq!(computed_center, center, epsilon = 1e-3);
    }
}
