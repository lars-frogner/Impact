//! Delaunay tetrahedralization.

use anyhow::{Result, bail};
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_containers::NoHashMap;
use impact_geometry::{AxisAlignedBox, Sphere};
use impact_math::{
    consts::f32::{FRAC_1_SQRT_6, SQRT_3, SQRT_6},
    point::{Point3, Point3C},
    random::Rng,
};

type VertexIdx = u32;
type TetrahedronID = u32;

const NO_TETRAHEDRON_ID: TetrahedronID = 0;

// How much to expand the bounding tetrahedron relative to the bounding sphere
// of the point cloud.
const BOUNDING_TETRA_MARGIN_FACTOR: f32 = 1.1;

// Points closer than this relative to the total size of the point cloud will be
// merged.
const MIN_RELATIVE_POINT_SEPARATION: f32 = 1e-9;

#[derive(Clone, Debug)]
pub struct DelaunayTetrahedralization {
    tetrahedra: TetrahedronMap,
}

#[derive(Clone, Debug)]
struct TetrahedronMap {
    tetrahedra: NoHashMap<TetrahedronID, Tetrahedron>,
    id_counter: TetrahedronID,
}

#[derive(Clone, Debug)]
struct Tetrahedron {
    /// The index of vertex A, B, C and D, respectively.
    vertices: [VertexIdx; 4],
    /// The ID of the tetrahedron adjoining face ABC, ACD, ADB and BDC,
    /// respectively. The ID has value [`NO_TETRAHEDRON_ID`] when there is no
    /// neighbor.
    neighbors: [TetrahedronID; 4],
}

#[derive(Debug)]
struct TetrahedronPointLocator {
    rng: Rng,
}

#[derive(Debug)]
struct TetrahedronPointLocatorScratch<'a, A: Allocator> {
    available_tetrahedron_ids: &'a mut AVec<TetrahedronID, A>,
    candidate_tetrahedron_ids: &'a mut AVec<TetrahedronID, A>,
}

impl DelaunayTetrahedralization {
    /// Subdivides the convex hull of the given set of points into tetrahedra
    /// that satisfy the Delaunay criterion.
    ///
    /// Uses an incremental insertion algorithm described in Ledoux (2007),
    /// "Computing the 3D Voronoi Diagram Robustly: An Easy Explanation".
    pub fn construct(points: &[Point3C]) -> Result<Self> {
        let n_points = points.len();

        if n_points < 4 {
            return Ok(Self {
                tetrahedra: TetrahedronMap::new(),
            });
        }

        if n_points > VertexIdx::MAX as usize - 4 {
            bail!("Number of points {n_points} is higher than supported");
        }

        let mut tetrahedra = TetrahedronMap::with_capacity(n_points);

        let arena = ArenaPool::get_arena();

        // Array for looking up vertex coordinates with vertex indices. The
        // first part will contain all the points we are tetrahedralizing, the
        // last part will contain the four vertices of the bounding tetrahedron.
        let mut vertices = AVec::with_capacity_in(n_points + 4, &arena);
        vertices.extend_from_slice(points);

        let aabb = AxisAlignedBox::aabb_for_points(points);
        let bounding_sphere = Sphere::bounding_sphere_from_aabb(&aabb);

        // Initialize with a single big tetrahedron encompassing all points.
        // This ensures points will always be inserted into an existing
        // tetrahedron.
        let bounding_tetra_vertices = find_tetrahedron_encompassing_sphere(
            bounding_sphere.center(),
            bounding_sphere.radius() * BOUNDING_TETRA_MARGIN_FACTOR,
        );

        vertices.extend_from_slice(&bounding_tetra_vertices);

        let first_bounding_vertex_idx = n_points as VertexIdx;

        tetrahedra.add_tetrahedron(Tetrahedron {
            vertices: [
                first_bounding_vertex_idx,
                first_bounding_vertex_idx + 1,
                first_bounding_vertex_idx + 2,
                first_bounding_vertex_idx + 3,
            ],
            neighbors: [NO_TETRAHEDRON_ID; 4],
        });

        let min_squared_point_separation =
            (MIN_RELATIVE_POINT_SEPARATION * bounding_sphere.radius()).powi(2);

        // When inserting a new vertex, we will push all new tetrahedra created
        // as result to the stack. For each tetrahedron popped off the stack, we
        // evaluate the Delaunay criterion locally, and if it is not satisfied,
        // we reconnect the local tetrahedra into a new configuration and push
        // them onto the stack. When the stack is empty, the new
        // tetrahedralization is Delaunay, and we can proceed to insert the next
        // vertex.
        let mut stack = AVec::with_capacity_in(64, &arena);

        let mut tetrahedron_ids_scratch = AVec::with_capacity_in(vertices.len(), &arena);
        let mut candidate_tetrahedron_ids = AVec::with_capacity_in(
            TetrahedronPointLocator::n_candidates_for_vertices(vertices.len()),
            &arena,
        );

        let mut locator = TetrahedronPointLocator::new(0);
        let mut current_vertex_count = 4;

        'insertion: for (new_vertex_idx, new_vertex) in vertices[..n_points].iter().enumerate() {
            let new_vertex_idx = new_vertex_idx as VertexIdx;

            let inside_tetra_id = locator.find_tetrahedron_containing_point(
                TetrahedronPointLocatorScratch {
                    available_tetrahedron_ids: &mut tetrahedron_ids_scratch,
                    candidate_tetrahedron_ids: &mut candidate_tetrahedron_ids,
                },
                &vertices,
                &tetrahedra,
                current_vertex_count,
                new_vertex,
            );
            assert_ne!(inside_tetra_id, NO_TETRAHEDRON_ID);

            // Skip the new vertex if it coincides with an existing vertex
            for vertex in tetrahedra.tetrahedron(inside_tetra_id).vertices(&vertices) {
                if Point3C::squared_distance_between(new_vertex, vertex)
                    < min_squared_point_separation
                {
                    continue 'insertion;
                }
            }

            let new_tetra_ids =
                tetrahedra.insert_and_connect_vertex(new_vertex_idx, inside_tetra_id);

            stack.extend_from_slice(&new_tetra_ids);

            while let Some(abcd_id) = stack.pop() {
                // Let the current tetrahedron be ABCD. The inserted vertex is A
                // (the reconnection operations never move A).
                let Some(abcd) = tetrahedra.get_tetrahedron(abcd_id) else {
                    // The tetrahedron may have been deleted by a reconnection
                    // after being pushed on the stack
                    continue;
                };

                let [a, b, c, d] = abcd.vertices;
                assert_eq!(a, new_vertex_idx);

                // Find neighbor tetrahedron BCDE adjoining the BDC face
                let bcde_id = abcd.neighbors[3];
                if bcde_id == NO_TETRAHEDRON_ID {
                    continue;
                }
                let bcde = tetrahedra.tetrahedron(bcde_id);

                // The neighbor shares the BDC face (in vertex order BCD).
                // Find the fourth vertex E not on that face.
                let e_corner = bcde.corner_not_on_face([b, c, d]);
                let e = bcde.vertices[e_corner];

                let vertex_a = abcd.vertex(&vertices, 0);
                let vertex_b = abcd.vertex(&vertices, 1);
                let vertex_c = abcd.vertex(&vertices, 2);
                let vertex_d = abcd.vertex(&vertices, 3);
                let vertex_e = bcde.vertex(&vertices, e_corner);

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
                            let new_tetra_ids = tetrahedra.reconnect_two_to_three(abcd_id, bcde_id);
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
                                    (
                                        abcd.neighbors[2],
                                        bcde.id_of_neighbor_adjoining_face([e, b, d]),
                                    )
                                } else if beyond_dc && !beyond_bd && !beyond_cb {
                                    (
                                        abcd.neighbors[1],
                                        bcde.id_of_neighbor_adjoining_face([e, d, c]),
                                    )
                                } else if beyond_cb && !beyond_bd && !beyond_dc {
                                    (
                                        abcd.neighbors[0],
                                        bcde.id_of_neighbor_adjoining_face([e, c, b]),
                                    )
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
                            let new_tetra_ids =
                                tetrahedra.reconnect_three_to_two(abcd_id, bcde_id, axye_id);
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
                                // directly on an edge of triangle BDC. We can
                                // reconnect ABCD and BCDE into three tetrahedra
                                // ABCE, ACDE and ADBE. One of the (depending on
                                // the intersected edge) will still be flat, but
                                // it will be deleted by a later reconnection.
                                let new_tetra_ids =
                                    tetrahedra.reconnect_two_to_three(abcd_id, bcde_id);
                                stack.extend_from_slice(&new_tetra_ids);
                            } else {
                                // ABCD is not flat. We can perform a
                                // four-to-four reconnection if we can find two
                                // other tetrahedra connected to the intersected
                                // edge with a fifth vertex F that is co-planar
                                // with BDC.
                                let (
                                    tetra_3_id,
                                    tetra_3_non_f_face,
                                    tetra_4_id,
                                    tetra_4_non_f_face,
                                ) = if intersects_bd {
                                    let abdf_id = abcd.neighbors[2];
                                    let abdf_non_f_face = [a, b, d];

                                    let bdfe_id = bcde.id_of_neighbor_adjoining_face([e, b, d]);
                                    let bdfe_non_f_face = [e, d, b];

                                    (abdf_id, abdf_non_f_face, bdfe_id, bdfe_non_f_face)
                                } else if intersects_dc {
                                    let adcf_id = abcd.neighbors[1];
                                    let adcf_non_f_face = [a, d, c];

                                    let dcfe_id = bcde.id_of_neighbor_adjoining_face([e, d, c]);
                                    let dcfe_non_f_face = [e, c, d];

                                    (adcf_id, adcf_non_f_face, dcfe_id, dcfe_non_f_face)
                                } else if intersects_cb {
                                    let acbf_id = abcd.neighbors[0];
                                    let acbf_non_f_face = [a, c, b];

                                    let cbfe_id = bcde.id_of_neighbor_adjoining_face([e, c, b]);
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
                                let tetra_3 = tetrahedra.tetrahedron(tetra_3_id);
                                let tetra_4 = tetrahedra.tetrahedron(tetra_4_id);

                                let f_corner_t3 = tetra_3.corner_not_on_face(tetra_3_non_f_face);
                                let f_corner_t4 = tetra_4.corner_not_on_face(tetra_4_non_f_face);
                                let f_t3 = tetra_3.vertices[f_corner_t3];
                                let f_t4 = tetra_4.vertices[f_corner_t4];

                                // The neighbors must have a common fifth vertex F
                                if f_t3 != f_t4 {
                                    continue;
                                }

                                let vertex_f = tetra_3.vertex(&vertices, f_corner_t3);

                                // F must be co-planar with BDC
                                if evaluate_side_of_triangle_plane_for_point(
                                    vertex_b, vertex_d, vertex_c, vertex_f,
                                ) != PointTrianglePlaneSide::InPlane
                                {
                                    continue;
                                }

                                let new_tetra_ids = tetrahedra.reconnect_four_to_four(
                                    abcd_id, bcde_id, tetra_3_id, tetra_4_id,
                                );
                                stack.extend_from_slice(&new_tetra_ids);
                            }
                        }
                    }
                }
            }

            current_vertex_count += 1;
        }

        // Remove all tetrahedra connected to the ad-hoc bounding vertices
        tetrahedra.retain(&mut tetrahedron_ids_scratch, |tetra| {
            tetra
                .vertices
                .iter()
                .all(|&vertex| vertex < first_bounding_vertex_idx)
        });

        Ok(Self { tetrahedra })
    }

    /// Returns the number of tetrahedra in the tetrahedralization.
    pub fn n_tetrahedra(&self) -> usize {
        self.tetrahedra.n_tetrahedra()
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn validate_brute_force(&self, vertices: &[Point3C]) {
        for (&tetra_id, tetra) in self.tetrahedra.tetrahedra() {
            let [a, b, c, d] = tetra.vertices(vertices);

            for (nb_idx, &nb_id) in tetra.neighbors.iter().enumerate() {
                if nb_id == NO_TETRAHEDRON_ID {
                    continue;
                }
                assert!(
                    self.tetrahedra.has_tetrahedron(nb_id),
                    "Tetrahedron {tetra_id} is missing neighbor {nb_id} at slot {nb_idx}"
                );

                let face = tetra.face(nb_idx);
                let neighbor = self.tetrahedra.tetrahedron(nb_id);

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

                let back_face = neighbor.face(back_idx);
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

            if evaluate_side_of_triangle_plane_for_point(a, b, c, d)
                == PointTrianglePlaneSide::Negative
            {
                panic!("Tetrahedron {tetra_id} has a negative signed volume");
            }

            for (v_idx, v) in vertices.iter().enumerate() {
                if point_lies_strictly_inside_circumsphere(a, b, c, d, v) {
                    let [a_idx, b_idx, c_idx, d_idx] = tetra.vertices;
                    panic!(
                        "Circumsphere of tetrahedron {tetra_id} with vertices \
                         [{a_idx}, {b_idx}, {c_idx}, {d_idx}] contains vertex {v_idx} \
                         (vertices = {{{a:?}, {b:?}, {c:?}, {d:?}}}, contains = {v:?})"
                    )
                }
            }
        }
    }
}

impl TetrahedronMap {
    fn new() -> Self {
        Self::with_capacity(0)
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            tetrahedra: NoHashMap::with_capacity_and_hasher(capacity, Default::default()),
            id_counter: NO_TETRAHEDRON_ID + 1,
        }
    }

    fn n_tetrahedra(&self) -> usize {
        self.tetrahedra.len()
    }

    fn ids(&self) -> impl Iterator<Item = TetrahedronID> {
        self.tetrahedra.keys().copied()
    }

    fn get_tetrahedron(&self, id: TetrahedronID) -> Option<&Tetrahedron> {
        self.tetrahedra.get(&id)
    }

    fn tetrahedron(&self, id: TetrahedronID) -> &Tetrahedron {
        self.get_tetrahedron(id).unwrap()
    }

    #[allow(dead_code)]
    fn has_tetrahedron(&self, id: TetrahedronID) -> bool {
        self.tetrahedra.contains_key(&id)
    }

    #[allow(dead_code)]
    fn tetrahedra(&self) -> impl Iterator<Item = (&TetrahedronID, &Tetrahedron)> {
        self.tetrahedra.iter()
    }

    fn add_tetrahedron(&mut self, tetra: Tetrahedron) -> TetrahedronID {
        let id = self.create_new_id();
        self.tetrahedra.insert(id, tetra);
        id
    }

    fn retain<A: Allocator>(
        &mut self,
        id_scratch: &mut AVec<TetrahedronID, A>,
        should_keep: impl Fn(&Tetrahedron) -> bool,
    ) {
        id_scratch.clear();
        id_scratch.extend(self.ids());

        for &id in &*id_scratch {
            let Some(tetra) = self.tetrahedra.get(&id) else {
                continue;
            };
            if should_keep(tetra) {
                continue;
            }
            // Clear the ID from the neighbor ID lists of the neighbors
            for nb_id in tetra.neighbors {
                if nb_id != NO_TETRAHEDRON_ID
                    && let Some(tetra_nb) = self.tetrahedra.get_mut(&nb_id)
                {
                    tetra_nb.replace_neighbor_id(id, NO_TETRAHEDRON_ID);
                }
            }
            self.tetrahedra.remove(&id);
        }
    }

    /// Takes a vertex A and a tetrahedron BCDE into which to insert it by
    /// converting BCDE into four new tetrahedra ABCE, ACDE, ADBE and ACBD
    /// connected to A. Returns the respective IDs of the four new tetrahedra.
    /// The vertices in the new tetrahedra will be in the order implied by their
    /// names.
    fn insert_and_connect_vertex(
        &mut self,
        vertex: VertexIdx,
        inside_tetra_id: TetrahedronID,
    ) -> [TetrahedronID; 4] {
        assert_ne!(inside_tetra_id, NO_TETRAHEDRON_ID);

        let abce_id = inside_tetra_id;
        let acde_id = self.create_new_id();
        let adbe_id = self.create_new_id();
        let acbd_id = self.create_new_id();

        let tetra = self.tetrahedra.get_mut(&inside_tetra_id).unwrap();
        let a = vertex;
        let [b, c, d, e] = tetra.vertices;
        let [bcd_nb_id, bde_nb_id, bec_nb_id, ced_nb_id] = tetra.neighbors;

        let abce = tetra;
        *abce = Tetrahedron {
            vertices: [a, b, c, e],
            neighbors: [acbd_id, acde_id, adbe_id, bec_nb_id],
        };

        let acde = Tetrahedron {
            vertices: [a, c, d, e],
            neighbors: [acbd_id, adbe_id, abce_id, ced_nb_id],
        };
        self.tetrahedra.insert(acde_id, acde);

        let adbe = Tetrahedron {
            vertices: [a, d, b, e],
            neighbors: [acbd_id, abce_id, acde_id, bde_nb_id],
        };
        self.tetrahedra.insert(adbe_id, adbe);

        let acbd = Tetrahedron {
            vertices: [a, c, b, d],
            neighbors: [abce_id, adbe_id, acde_id, bcd_nb_id],
        };
        self.tetrahedra.insert(acbd_id, acbd);

        // Update invalidated neighbor IDs for affected neighbors
        if ced_nb_id != NO_TETRAHEDRON_ID {
            let ced_nb = self.tetrahedra.get_mut(&ced_nb_id).unwrap();
            ced_nb.replace_neighbor_id(inside_tetra_id, acde_id);
        }
        if bde_nb_id != NO_TETRAHEDRON_ID {
            let bde_nb = self.tetrahedra.get_mut(&bde_nb_id).unwrap();
            bde_nb.replace_neighbor_id(inside_tetra_id, adbe_id);
        }
        if bcd_nb_id != NO_TETRAHEDRON_ID {
            let bcd_nb = self.tetrahedra.get_mut(&bcd_nb_id).unwrap();
            bcd_nb.replace_neighbor_id(inside_tetra_id, acbd_id);
        }

        [abce_id, acde_id, adbe_id, acbd_id]
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
    fn reconnect_two_to_three(
        &mut self,
        abcd_id: TetrahedronID,
        bcde_id: TetrahedronID,
    ) -> [TetrahedronID; 3] {
        assert_ne!(abcd_id, NO_TETRAHEDRON_ID);
        assert_ne!(bcde_id, NO_TETRAHEDRON_ID);

        let abcd = self.tetrahedra.get(&abcd_id).unwrap();
        let bcde = self.tetrahedra.get(&bcde_id).unwrap();

        let [a, b, c, d] = abcd.vertices;

        let e_corner = bcde.corner_not_on_face([b, c, d]);
        let e = bcde.vertices[e_corner];

        let [abc_nb_id, acd_nb_id, adb_nb_id, bdc_nb_id] = abcd.neighbors;
        assert_eq!(bdc_nb_id, bcde_id);

        let ecb_nb_id = bcde.id_of_neighbor_adjoining_face([e, c, b]);
        let edc_nb_id = bcde.id_of_neighbor_adjoining_face([e, d, c]);
        let ebd_nb_id = bcde.id_of_neighbor_adjoining_face([e, b, d]);

        let abce_id = abcd_id;
        let acde_id = bcde_id;
        let adbe_id = self.create_new_id();

        let abce = Tetrahedron {
            vertices: [a, b, c, e],
            neighbors: [abc_nb_id, acde_id, adbe_id, ecb_nb_id],
        };
        self.tetrahedra.insert(abce_id, abce);

        let acde = Tetrahedron {
            vertices: [a, c, d, e],
            neighbors: [acd_nb_id, adbe_id, abce_id, edc_nb_id],
        };
        self.tetrahedra.insert(acde_id, acde);

        let adbe = Tetrahedron {
            vertices: [a, d, b, e],
            neighbors: [adb_nb_id, abce_id, acde_id, ebd_nb_id],
        };
        self.tetrahedra.insert(adbe_id, adbe);

        // Update invalidated neighbor IDs for affected neighbors
        if acd_nb_id != NO_TETRAHEDRON_ID {
            let acd_nb = self.tetrahedra.get_mut(&acd_nb_id).unwrap();
            acd_nb.replace_neighbor_id(abcd_id, acde_id);
        }
        if adb_nb_id != NO_TETRAHEDRON_ID {
            let adb_nb = self.tetrahedra.get_mut(&adb_nb_id).unwrap();
            adb_nb.replace_neighbor_id(abcd_id, adbe_id);
        }
        if ecb_nb_id != NO_TETRAHEDRON_ID {
            let ecb_nb = self.tetrahedra.get_mut(&ecb_nb_id).unwrap();
            ecb_nb.replace_neighbor_id(bcde_id, abce_id);
        }
        if ebd_nb_id != NO_TETRAHEDRON_ID {
            let ebd_nb = self.tetrahedra.get_mut(&ebd_nb_id).unwrap();
            ebd_nb.replace_neighbor_id(bcde_id, adbe_id);
        }

        [abce_id, acde_id, adbe_id]
    }

    /// Takes the IDs of three adjacent tetrahedra ABCD, BCDE and AXYE sharing an
    /// edge XY = [CB, DC, BD] and reconfigures them into two adjacent tetrahedra
    /// AXZE and AZYE sharing the face AZE (where Z is the third vertex of the
    /// face BDC containing the shared edge XY). Returns the respective IDs of
    /// the two reconfigured tetrahedra. The vertices in the new tetrahedra will
    /// be in the order implied by their names.
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
    ) -> [TetrahedronID; 2] {
        assert_ne!(abcd_id, NO_TETRAHEDRON_ID);
        assert_ne!(bcde_id, NO_TETRAHEDRON_ID);
        assert_ne!(axye_id, NO_TETRAHEDRON_ID);

        let abcd = self.tetrahedra.get(&abcd_id).unwrap();
        let bcde = self.tetrahedra.get(&bcde_id).unwrap();
        let axye = self.tetrahedra.get(&axye_id).unwrap();

        let [a, b, c, d] = abcd.vertices;

        let [x, y, z] = if axye_id == abcd.neighbors[0] {
            [c, b, d]
        } else if axye_id == abcd.neighbors[1] {
            [d, c, b]
        } else if axye_id == abcd.neighbors[2] {
            [b, d, c]
        } else {
            panic!("AXYE {axye_id} does not adjoin a non-shared face of ABCD {abcd_id}");
        };

        let e_corner = bcde.corner_not_on_face([b, c, d]);
        let e = bcde.vertices[e_corner];

        let e_corner_axye = axye.corner_not_on_face([a, x, y]);
        assert_eq!(axye.vertices[e_corner_axye], e);

        let axz_nb_id = abcd.id_of_neighbor_adjoining_face([a, x, z]);
        let azy_nb_id = abcd.id_of_neighbor_adjoining_face([a, z, y]);
        assert_eq!(abcd.id_of_neighbor_adjoining_face([a, y, x]), axye_id);
        assert_eq!(abcd.id_of_neighbor_adjoining_face([b, d, c]), bcde_id);

        let ezx_nb_id = bcde.id_of_neighbor_adjoining_face([e, z, x]);
        let eyz_nb_id = bcde.id_of_neighbor_adjoining_face([e, y, z]);
        let xae_nb_id = axye.id_of_neighbor_adjoining_face([x, a, e]);
        let yea_nb_id = axye.id_of_neighbor_adjoining_face([y, e, a]);

        let axze_id = abcd_id;
        let azye_id = bcde_id;

        let axze = Tetrahedron {
            vertices: [a, x, z, e],
            neighbors: [axz_nb_id, azye_id, xae_nb_id, ezx_nb_id],
        };
        self.tetrahedra.insert(axze_id, axze);

        let azye = Tetrahedron {
            vertices: [a, z, y, e],
            neighbors: [azy_nb_id, yea_nb_id, axze_id, eyz_nb_id],
        };
        self.tetrahedra.insert(azye_id, azye);

        self.tetrahedra.remove(&axye_id);

        // Update invalidated neighbor IDs for affected neighbors
        if azy_nb_id != NO_TETRAHEDRON_ID {
            let azy_nb = self.tetrahedra.get_mut(&azy_nb_id).unwrap();
            azy_nb.replace_neighbor_id(abcd_id, azye_id);
        }
        if ezx_nb_id != NO_TETRAHEDRON_ID {
            let ezx_nb = self.tetrahedra.get_mut(&ezx_nb_id).unwrap();
            ezx_nb.replace_neighbor_id(bcde_id, axze_id);
        }
        if xae_nb_id != NO_TETRAHEDRON_ID {
            let xae_nb = self.tetrahedra.get_mut(&xae_nb_id).unwrap();
            xae_nb.replace_neighbor_id(axye_id, axze_id);
        }
        if yea_nb_id != NO_TETRAHEDRON_ID {
            let yea_nb = self.tetrahedra.get_mut(&yea_nb_id).unwrap();
            yea_nb.replace_neighbor_id(axye_id, azye_id);
        }

        [axze_id, azye_id]
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
        assert_ne!(abcd_id, NO_TETRAHEDRON_ID);
        assert_ne!(bcde_id, NO_TETRAHEDRON_ID);
        assert_ne!(axyf_id, NO_TETRAHEDRON_ID);
        assert_ne!(xyfe_id, NO_TETRAHEDRON_ID);

        let abcd = self.tetrahedra.get(&abcd_id).unwrap();
        let bcde = self.tetrahedra.get(&bcde_id).unwrap();
        let axyf = self.tetrahedra.get(&axyf_id).unwrap();
        let xyfe = self.tetrahedra.get(&xyfe_id).unwrap();

        let [a, b, c, d] = abcd.vertices;

        let [x, y, z] = if axyf_id == abcd.neighbors[0] {
            [c, b, d]
        } else if axyf_id == abcd.neighbors[1] {
            [d, c, b]
        } else if axyf_id == abcd.neighbors[2] {
            [b, d, c]
        } else {
            panic!("AXYF {axyf_id} does not adjoin a non-shared face of ABCD {abcd_id}");
        };

        let e_corner = bcde.corner_not_on_face([b, c, d]);
        let e = bcde.vertices[e_corner];

        let f_corner = axyf.corner_not_on_face([a, x, y]);
        let f = axyf.vertices[f_corner];

        let e_corner_xyfe = xyfe.corner_not_on_face([y, f, x]);
        assert_eq!(xyfe.vertices[e_corner_xyfe], e);

        let f_corner_xyfe = xyfe.corner_not_on_face([e, y, x]);
        assert_eq!(xyfe.vertices[f_corner_xyfe], f);

        let axz_nb_id = abcd.id_of_neighbor_adjoining_face([a, x, z]);
        let azy_nb_id = abcd.id_of_neighbor_adjoining_face([a, z, y]);
        assert_eq!(abcd.id_of_neighbor_adjoining_face([a, y, x]), axyf_id);
        assert_eq!(abcd.id_of_neighbor_adjoining_face([b, d, c]), bcde_id);

        let ezx_nb_id = bcde.id_of_neighbor_adjoining_face([e, z, x]);
        let eyz_nb_id = bcde.id_of_neighbor_adjoining_face([e, y, z]);
        let afx_nb_id = axyf.id_of_neighbor_adjoining_face([a, f, x]);
        let ayf_nb_id = axyf.id_of_neighbor_adjoining_face([a, y, f]);
        let exf_nb_id = xyfe.id_of_neighbor_adjoining_face([e, x, f]);
        let efy_nb_id = xyfe.id_of_neighbor_adjoining_face([e, f, y]);

        let axze_id = abcd_id;
        let azye_id = bcde_id;
        let afxe_id = axyf_id;
        let ayfe_id = xyfe_id;

        let axze = Tetrahedron {
            vertices: [a, x, z, e],
            neighbors: [axz_nb_id, azye_id, afxe_id, ezx_nb_id],
        };
        self.tetrahedra.insert(axze_id, axze);

        let azye = Tetrahedron {
            vertices: [a, z, y, e],
            neighbors: [azy_nb_id, ayfe_id, axze_id, eyz_nb_id],
        };
        self.tetrahedra.insert(azye_id, azye);

        let afxe = Tetrahedron {
            vertices: [a, f, x, e],
            neighbors: [afx_nb_id, axze_id, ayfe_id, exf_nb_id],
        };
        self.tetrahedra.insert(afxe_id, afxe);

        let ayfe = Tetrahedron {
            vertices: [a, y, f, e],
            neighbors: [ayf_nb_id, afxe_id, azye_id, efy_nb_id],
        };
        self.tetrahedra.insert(ayfe_id, ayfe);

        // Update invalidated neighbor IDs for affected neighbors
        if azy_nb_id != NO_TETRAHEDRON_ID {
            let azy_nb = self.tetrahedra.get_mut(&azy_nb_id).unwrap();
            azy_nb.replace_neighbor_id(abcd_id, azye_id);
        }
        if ezx_nb_id != NO_TETRAHEDRON_ID {
            let ezx_nb = self.tetrahedra.get_mut(&ezx_nb_id).unwrap();
            ezx_nb.replace_neighbor_id(bcde_id, axze_id);
        }
        if ayf_nb_id != NO_TETRAHEDRON_ID {
            let ayf_nb = self.tetrahedra.get_mut(&ayf_nb_id).unwrap();
            ayf_nb.replace_neighbor_id(axyf_id, ayfe_id);
        }
        if exf_nb_id != NO_TETRAHEDRON_ID {
            let exf_nb = self.tetrahedra.get_mut(&exf_nb_id).unwrap();
            exf_nb.replace_neighbor_id(xyfe_id, afxe_id);
        }

        [axze_id, azye_id, afxe_id, ayfe_id]
    }

    fn create_new_id(&mut self) -> TetrahedronID {
        let id = self.id_counter;
        self.id_counter = self
            .id_counter
            .checked_add(1)
            .expect("Exceeded max tetrahedron count");
        id
    }
}

impl Tetrahedron {
    #[inline]
    fn vertex<'a>(&self, vertices: &'a [Point3C], corner: usize) -> &'a Point3C {
        &vertices[self.vertices[corner] as usize]
    }

    #[inline]
    fn vertices<'a>(&self, vertices: &'a [Point3C]) -> [&'a Point3C; 4] {
        self.vertices.map(|idx| &vertices[idx as usize])
    }

    #[inline]
    fn replace_neighbor_id(&mut self, old_id: TetrahedronID, new_id: TetrahedronID) {
        for id in &mut self.neighbors {
            if *id == old_id {
                *id = new_id;
            }
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn face(&self, neighbor_idx: usize) -> [VertexIdx; 3] {
        let [a, b, c, d] = self.vertices;
        match neighbor_idx {
            0 => [a, b, c],
            1 => [a, c, d],
            2 => [a, d, b],
            3 => [b, d, c],
            _ => panic!("Invalid neighbor index {neighbor_idx}"),
        }
    }

    #[inline]
    fn corner_not_on_face(&self, face: [VertexIdx; 3]) -> usize {
        let [a, b, c, d] = self.vertices;

        if face == [a, b, c] || face == [c, a, b] || face == [b, c, a] {
            return 3;
        }
        if face == [a, c, d] || face == [d, a, c] || face == [c, d, a] {
            return 1;
        }
        if face == [a, d, b] || face == [b, a, d] || face == [d, b, a] {
            return 2;
        }
        if face == [b, d, c] || face == [c, b, d] || face == [d, c, b] {
            return 0;
        }

        panic!("Tried to find corner not on missing face");
    }

    #[inline]
    fn id_of_neighbor_adjoining_face(&self, face: [VertexIdx; 3]) -> TetrahedronID {
        let [a, b, c, d] = self.vertices;

        if face == [a, b, c] || face == [c, a, b] || face == [b, c, a] {
            return self.neighbors[0];
        }
        if face == [a, c, d] || face == [d, a, c] || face == [c, d, a] {
            return self.neighbors[1];
        }
        if face == [a, d, b] || face == [b, a, d] || face == [d, b, a] {
            return self.neighbors[2];
        }
        if face == [b, d, c] || face == [c, b, d] || face == [d, c, b] {
            return self.neighbors[3];
        }

        panic!("Tried to find ID of neighbor adjoining missing face");
    }

    #[inline]
    fn next_neighbor_towards_point(
        &self,
        vertices: &[Point3C],
        point: &Point3C,
        rng: &mut Rng,
    ) -> Option<TetrahedronID> {
        let va = self.vertex(vertices, 0);
        let vb = self.vertex(vertices, 1);
        let vc = self.vertex(vertices, 2);
        let vd = self.vertex(vertices, 3);

        let triangles = [
            [&va, &vb, &vc],
            [&va, &vc, &vd],
            [&va, &vd, &vb],
            [&vb, &vd, &vc],
        ];

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

impl TetrahedronPointLocator {
    fn new(seed: u64) -> Self {
        let rng = Rng::with_seed(seed);
        Self { rng }
    }

    fn n_candidates_for_vertices(n_vertices: usize) -> usize {
        // From Mücke et al. (1999)
        let n_candidates = 7.0 * (n_vertices as f32).sqrt().sqrt();
        n_candidates.ceil().max(1.0) as usize
    }

    fn find_tetrahedron_containing_point<A: Allocator>(
        &mut self,
        scratch: TetrahedronPointLocatorScratch<'_, A>,
        vertices: &[Point3C],
        tetrahedra: &TetrahedronMap,
        current_vertex_count: usize,
        point: &Point3C,
    ) -> TetrahedronID {
        if tetrahedra.n_tetrahedra() == 0 {
            return NO_TETRAHEDRON_ID;
        }

        let n_candidates =
            Self::n_candidates_for_vertices(current_vertex_count).min(tetrahedra.n_tetrahedra());

        scratch.available_tetrahedron_ids.clear();
        scratch.available_tetrahedron_ids.extend(tetrahedra.ids());

        scratch
            .candidate_tetrahedron_ids
            .resize(n_candidates, NO_TETRAHEDRON_ID);

        self.rng.clone_random_subset_from_slice(
            scratch.candidate_tetrahedron_ids,
            scratch.available_tetrahedron_ids,
        );

        let mut current_tetra_id = NO_TETRAHEDRON_ID;
        let mut closest_dist_sq = f32::INFINITY;

        for &id in &*scratch.candidate_tetrahedron_ids {
            let tetra = tetrahedra.tetrahedron(id);
            let vertex = tetra.vertex(vertices, 0);
            let dist_sq = Point3C::squared_distance_between(vertex, point);
            if dist_sq < closest_dist_sq {
                current_tetra_id = id;
                closest_dist_sq = dist_sq;
            }
        }

        loop {
            let tetra = tetrahedra.tetrahedron(current_tetra_id);
            if let Some(neighbor_id) =
                tetra.next_neighbor_towards_point(vertices, point, &mut self.rng)
            {
                assert_ne!(neighbor_id, NO_TETRAHEDRON_ID);
                current_tetra_id = neighbor_id;
            } else {
                break;
            }
        }

        current_tetra_id
    }
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
    let a: [f64; 3] = (*vertex_a).into();
    let b: [f64; 3] = (*vertex_b).into();
    let c: [f64; 3] = (*vertex_c).into();
    let p: [f64; 3] = (*point).into();

    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);

    let factor = dot(ap, cross(ab, ac));

    if factor > 0.0 {
        PointTrianglePlaneSide::Positive
    } else if factor < 0.0 {
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
    let a: [f64; 3] = (*vertex_a).into();
    let b: [f64; 3] = (*vertex_b).into();
    let c: [f64; 3] = (*vertex_c).into();
    let d: [f64; 3] = (*vertex_d).into();
    let p: [f64; 3] = (*point).into();

    let pa = sub(a, p);
    let pb = sub(b, p);
    let pc = sub(c, p);
    let pd = sub(d, p);

    let pa2 = dot(pa, pa);
    let pb2 = dot(pb, pb);
    let pc2 = dot(pc, pc);
    let pd2 = dot(pd, pd);

    let det = determinant4x4(
        extend(pa, pa2),
        extend(pb, pb2),
        extend(pc, pc2),
        extend(pd, pd2),
    );

    det < 0.0
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum LineTriangleIntersection {
    Inside,
    Outside { ab: bool, bc: bool, ca: bool },
    Edges { ab: bool, bc: bool, ca: bool },
}

#[inline]
fn sub([ax, ay, az]: [f64; 3], [bx, by, bz]: [f64; 3]) -> [f64; 3] {
    [ax - bx, ay - by, az - bz]
}

#[inline]
fn dot([ax, ay, az]: [f64; 3], [bx, by, bz]: [f64; 3]) -> f64 {
    ax * bx + ay * by + az * bz
}

#[inline]
fn cross([ax, ay, az]: [f64; 3], [bx, by, bz]: [f64; 3]) -> [f64; 3] {
    [ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx]
}

#[inline]
fn extend([x, y, z]: [f64; 3], w: f64) -> [f64; 4] {
    [x, y, z, w]
}

#[inline]
fn determinant4x4(
    [c11, c21, c31, c41]: [f64; 4],
    [c12, c22, c32, c42]: [f64; 4],
    [c13, c23, c33, c43]: [f64; 4],
    [c14, c24, c34, c44]: [f64; 4],
) -> f64 {
    c11 * determinant3x3([c22, c32, c42], [c23, c33, c43], [c24, c34, c44])
        - c12 * determinant3x3([c21, c31, c41], [c23, c33, c43], [c24, c34, c44])
        + c13 * determinant3x3([c21, c31, c41], [c22, c32, c42], [c24, c34, c44])
        - c14 * determinant3x3([c21, c31, c41], [c22, c32, c42], [c23, c33, c43])
}

#[inline]
fn determinant3x3(
    [c11, c21, c31]: [f64; 3],
    [c12, c22, c32]: [f64; 3],
    [c13, c23, c33]: [f64; 3],
) -> f64 {
    c11 * determinant2x2([c22, c32], [c23, c33]) - c12 * determinant2x2([c21, c31], [c23, c33])
        + c13 * determinant2x2([c21, c31], [c22, c32])
}

#[inline]
fn determinant2x2([c11, c21]: [f64; 2], [c12, c22]: [f64; 2]) -> f64 {
    c11 * c22 - c21 * c12
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;

    pub fn fuzz_test_delaunay_tetrahedralization(points: Vec<Point3C>) {
        let tetrahedra = DelaunayTetrahedralization::construct(&points).unwrap();
        tetrahedra.validate_brute_force(&points);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_geometry::Plane;
    use impact_math::vector::UnitVector3;

    #[test]
    fn delaunay_tetrahedralization_of_less_than_four_points_is_empty() {
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].map(Point3C::from);
        let tetrahedra = DelaunayTetrahedralization::construct(&points).unwrap();
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

        let tetrahedra = DelaunayTetrahedralization::construct(&points).unwrap();
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

        let tetrahedra = DelaunayTetrahedralization::construct(&points).unwrap();
        assert_eq!(tetrahedra.n_tetrahedra(), 1);
        tetrahedra.validate_brute_force(&points);
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

        let tetrahedra = DelaunayTetrahedralization::construct(&points).unwrap();
        assert_eq!(tetrahedra.n_tetrahedra(), 2);
        tetrahedra.validate_brute_force(&points);
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

        let tetrahedra = DelaunayTetrahedralization::construct(&points).unwrap();
        assert_eq!(tetrahedra.n_tetrahedra(), 1);
        tetrahedra.validate_brute_force(&points);
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

        let tetrahedra = DelaunayTetrahedralization::construct(&points).unwrap();
        assert!(tetrahedra.n_tetrahedra() > 0);
        tetrahedra.validate_brute_force(&points);
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

        let tetrahedra = DelaunayTetrahedralization::construct(&points).unwrap();
        assert!(tetrahedra.n_tetrahedra() > 0);
        tetrahedra.validate_brute_force(&points);
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
}
