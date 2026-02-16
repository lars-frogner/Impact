//! Bounding volume hierarchy.

mod naive_bottom_up;

use crate::bounding_volume::BoundingVolumeID;
use anyhow::{Result, anyhow};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_containers::KeyIndexMapper;
use impact_geometry::{AxisAlignedBox, AxisAlignedBoxC, Frustum};
use std::mem;

#[derive(Debug)]
pub struct BoundingVolumeHierarchy {
    primitives: Primitives,
    nodes: Vec<Node>,
    root_node_idx: Option<usize>,
}

#[derive(Debug)]
struct Primitives {
    aabbs: Vec<AxisAlignedBoxC>,
    index_map: KeyIndexMapper<BoundingVolumeID>,
}

#[derive(Clone, Debug)]
struct Node {
    aabb: AxisAlignedBoxC,
    payload: PackedNodePayload,
}

#[derive(Clone, Debug)]
enum NodePayload {
    Children { left_idx: usize, right_idx: usize },
    Primitive,
}

#[derive(Clone, Debug)]
struct PackedNodePayload {
    /// These are the child node indices unless `left_idx ==
    /// PRIMITIVE_SENTINEL_IDX`, in which case the node represents a primitive.
    /// By reserving a sentinel value we save the extra 8 bytes an explicit enum
    /// would require.
    left_idx: usize,
    right_idx: usize,
}

const PRIMITIVE_SENTINEL_IDX: usize = usize::MAX;

impl BoundingVolumeHierarchy {
    pub fn new() -> Self {
        Self {
            primitives: Primitives::new(),
            nodes: Vec::new(),
            root_node_idx: None,
        }
    }

    pub fn add_primitive_volume(
        &mut self,
        id: BoundingVolumeID,
        aabb: AxisAlignedBoxC,
    ) -> Result<()> {
        self.primitives.insert(id, aabb)
    }

    pub fn primitive_count(&self) -> usize {
        self.primitives.aabbs.len()
    }

    pub fn build(&mut self) {
        self.root_node_idx = naive_bottom_up::build(&mut self.nodes, &self.primitives.aabbs);
    }

    pub fn root_bounding_volume(&self) -> AxisAlignedBoxC {
        let Some(root_node_idx) = self.root_node_idx else {
            return AxisAlignedBoxC::default();
        };
        let root_node = &self.nodes[root_node_idx];
        root_node.aabb.clone()
    }

    pub fn for_each_bounding_volume_in_axis_aligned_box(
        &self,
        axis_aligned_box: &AxisAlignedBox,
        mut f: impl FnMut(BoundingVolumeID),
    ) {
        self.for_each_intersecting_bounding_volume(
            |aabb| {
                let aabb = aabb.aligned();
                !aabb.box_lies_outside(axis_aligned_box)
            },
            |idx| {
                f(self.primitives.id_at_idx(idx));
            },
        );
    }

    pub fn for_each_bounding_volume_maybe_in_frustum(
        &self,
        frustum: &Frustum,
        mut f: impl FnMut(BoundingVolumeID),
    ) {
        self.for_each_intersecting_bounding_volume(
            |aabb| {
                let aabb = aabb.aligned();
                frustum.could_contain_part_of_axis_aligned_box(&aabb)
            },
            |idx| {
                f(self.primitives.id_at_idx(idx));
            },
        );
    }

    pub fn for_each_intersecting_bounding_volume_pair(
        &self,
        mut f: impl FnMut(BoundingVolumeID, BoundingVolumeID),
    ) {
        self.for_each_internal_intersection(|idx_i, idx_j| {
            f(
                self.primitives.id_at_idx(idx_i),
                self.primitives.id_at_idx(idx_j),
            );
        });
    }

    pub fn for_each_bounding_volume_in_axis_aligned_box_brute_force(
        &self,
        axis_aligned_box: &AxisAlignedBox,
        mut f: impl FnMut(BoundingVolumeID),
    ) {
        for (idx, aabb) in self.primitives.aabbs.iter().enumerate() {
            let aabb = aabb.aligned();
            if !axis_aligned_box.box_lies_outside(&aabb) {
                let id = self.primitives.index_map.key_at_idx(idx);
                f(id);
            }
        }
    }

    pub fn for_each_bounding_volume_maybe_in_frustum_brute_force(
        &self,
        frustum: &Frustum,
        mut f: impl FnMut(BoundingVolumeID),
    ) {
        for (idx, aabb) in self.primitives.aabbs.iter().enumerate() {
            let aabb = aabb.aligned();
            if frustum.could_contain_part_of_axis_aligned_box(&aabb) {
                let id = self.primitives.index_map.key_at_idx(idx);
                f(id);
            }
        }
    }

    pub fn for_each_intersecting_bounding_volume_pair_brute_force(
        &self,
        mut f: impl FnMut(BoundingVolumeID, BoundingVolumeID),
    ) {
        let aabbs = &self.primitives.aabbs;
        let n_primitives = aabbs.len();

        if n_primitives < 2 {
            return;
        }

        for (i, aabb_i) in (0..n_primitives - 1).zip(&aabbs[0..n_primitives - 1]) {
            let id_i = self.primitives.index_map.key_at_idx(i);
            let aabb_i = aabb_i.aligned();

            for (j, aabb_j) in (i + 1..n_primitives).zip(&aabbs[i + 1..n_primitives]) {
                if !aabb_i.box_lies_outside(&aabb_j.aligned()) {
                    let id_j = self.primitives.index_map.key_at_idx(j);
                    f(id_i, id_j);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
    }

    fn for_each_intersecting_bounding_volume(
        &self,
        mut is_intersection: impl FnMut(&AxisAlignedBoxC) -> bool,
        mut process_intersection: impl FnMut(usize),
    ) {
        let Some(root_node_idx) = self.root_node_idx else {
            return;
        };

        let arena = ArenaPool::get_arena_for_capacity(self.nodes.len() * mem::size_of::<usize>());
        let mut node_idx_stack = AVec::with_capacity_in(self.nodes.len(), &arena);

        node_idx_stack.push(root_node_idx);

        while let Some(node_idx) = node_idx_stack.pop() {
            let node = &self.nodes[node_idx];

            if is_intersection(&node.aabb) {
                match node.payload() {
                    NodePayload::Children {
                        left_idx,
                        right_idx,
                    } => {
                        node_idx_stack.push(right_idx);
                        node_idx_stack.push(left_idx);
                    }
                    NodePayload::Primitive => {
                        process_intersection(node_idx);
                    }
                }
            }
        }
    }

    fn for_each_internal_intersection(&self, mut f: impl FnMut(usize, usize)) {
        enum Operation {
            CheckSelf { check_idx: usize },
            CheckPair { left_idx: usize, right_idx: usize },
        }

        let Some(root_node_idx) = self.root_node_idx else {
            return;
        };

        let arena = ArenaPool::get_arena_for_capacity(self.nodes.len() * mem::size_of::<usize>());
        let mut operation_stack = AVec::with_capacity_in(self.nodes.len(), &arena);

        operation_stack.push(Operation::CheckSelf {
            check_idx: root_node_idx,
        });

        while let Some(op) = operation_stack.pop() {
            match op {
                Operation::CheckSelf { check_idx: idx } => {
                    let node = &self.nodes[idx];
                    match node.payload() {
                        NodePayload::Primitive => {
                            // Nothing to do (the primitive can't collide with itself)
                        }
                        NodePayload::Children {
                            left_idx,
                            right_idx,
                        } => {
                            // Check for intersections between left and right branch
                            operation_stack.push(Operation::CheckPair {
                                left_idx,
                                right_idx,
                            });
                            // Check for internal intersections in right branch
                            operation_stack.push(Operation::CheckSelf {
                                check_idx: right_idx,
                            });
                            // Check for internal intersections in left branch
                            operation_stack.push(Operation::CheckSelf {
                                check_idx: left_idx,
                            });
                        }
                    }
                }
                Operation::CheckPair {
                    left_idx,
                    right_idx,
                } => {
                    let left_node = &self.nodes[left_idx];
                    let right_node = &self.nodes[right_idx];

                    if left_node.aabb.box_lies_outside(&right_node.aabb) {
                        continue;
                    }

                    match (left_node.payload(), right_node.payload()) {
                        (NodePayload::Primitive, NodePayload::Primitive) => {
                            // The primitives are intersecting
                            f(left_idx, right_idx);
                        }
                        (
                            NodePayload::Children {
                                left_idx: left_idx_for_left,
                                right_idx: right_idx_for_left,
                            },
                            NodePayload::Primitive,
                        ) => {
                            // Check the left node's children against the right
                            // primitive node
                            operation_stack.push(Operation::CheckPair {
                                left_idx: right_idx_for_left,
                                right_idx,
                            });
                            operation_stack.push(Operation::CheckPair {
                                left_idx: left_idx_for_left,
                                right_idx,
                            });
                        }
                        (
                            NodePayload::Primitive,
                            NodePayload::Children {
                                left_idx: left_idx_for_right,
                                right_idx: right_idx_for_right,
                            },
                        ) => {
                            // Check the right node's children against the left
                            // primitive node
                            operation_stack.push(Operation::CheckPair {
                                left_idx,
                                right_idx: right_idx_for_right,
                            });
                            operation_stack.push(Operation::CheckPair {
                                left_idx,
                                right_idx: left_idx_for_right,
                            });
                        }
                        (
                            NodePayload::Children {
                                left_idx: left_idx_for_left,
                                right_idx: right_idx_for_left,
                            },
                            NodePayload::Children {
                                left_idx: left_idx_for_right,
                                right_idx: right_idx_for_right,
                            },
                        ) => {
                            if right_node.aabb.volume() > left_node.aabb.volume() {
                                // Since the right node is larger, we split it
                                // and test its children against the left node
                                operation_stack.push(Operation::CheckPair {
                                    left_idx,
                                    right_idx: right_idx_for_right,
                                });
                                operation_stack.push(Operation::CheckPair {
                                    left_idx,
                                    right_idx: left_idx_for_right,
                                });
                            } else {
                                // The left node is larger, so we split that
                                // instead
                                operation_stack.push(Operation::CheckPair {
                                    left_idx: right_idx_for_left,
                                    right_idx,
                                });
                                operation_stack.push(Operation::CheckPair {
                                    left_idx: left_idx_for_left,
                                    right_idx,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Primitives {
    fn new() -> Self {
        Self {
            aabbs: Vec::new(),
            index_map: KeyIndexMapper::new(),
        }
    }

    #[inline]
    fn insert(&mut self, id: BoundingVolumeID, aabb: AxisAlignedBoxC) -> Result<()> {
        self.index_map
            .try_push_key(id)
            .map_err(|_idx| anyhow!("A bounding volume with ID {id} is already present"))?;
        self.aabbs.push(aabb);
        Ok(())
    }

    #[inline]
    fn id_at_idx(&self, idx: usize) -> BoundingVolumeID {
        self.index_map.key_at_idx(idx)
    }

    fn clear(&mut self) {
        self.aabbs.clear();
        self.index_map.clear();
    }
}

impl Node {
    #[inline]
    fn new(aabb: AxisAlignedBoxC, payload: NodePayload) -> Self {
        Self {
            aabb,
            payload: payload.pack(),
        }
    }

    #[inline]
    fn payload(&self) -> NodePayload {
        self.payload.unpack()
    }
}

impl NodePayload {
    #[inline]
    fn pack(&self) -> PackedNodePayload {
        match self {
            &Self::Children {
                left_idx,
                right_idx,
            } => PackedNodePayload {
                left_idx,
                right_idx,
            },
            Self::Primitive => PackedNodePayload {
                left_idx: PRIMITIVE_SENTINEL_IDX,
                right_idx: PRIMITIVE_SENTINEL_IDX,
            },
        }
    }
}

impl PackedNodePayload {
    #[inline]
    fn unpack(&self) -> NodePayload {
        if self.left_idx != PRIMITIVE_SENTINEL_IDX {
            NodePayload::Children {
                left_idx: self.left_idx,
                right_idx: self.right_idx,
            }
        } else {
            NodePayload::Primitive
        }
    }
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use arbitrary::{Arbitrary, Result, Unstructured};
    use impact_containers::HashSet;
    use impact_math::{point::Point3, vector::Vector3};
    use std::mem;

    #[derive(Clone, Debug)]
    pub struct ArbitraryAABB(AxisAlignedBox);

    impl Arbitrary<'_> for ArbitraryAABB {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let x = 2.0 * arbitrary_norm_f32(u)? - 1.0;
            let y = 2.0 * arbitrary_norm_f32(u)? - 1.0;
            let z = 2.0 * arbitrary_norm_f32(u)? - 1.0;
            let center = Point3::new(x, y, z);

            let ex = arbitrary_norm_f32(u)?;
            let ey = arbitrary_norm_f32(u)?;
            let ez = arbitrary_norm_f32(u)?;
            let half_extents = Vector3::new(ex, ey, ez);

            Ok(Self(AxisAlignedBox::new(
                center - half_extents,
                center + half_extents,
            )))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 6 * mem::size_of::<i32>();
            (size, Some(size))
        }
    }

    pub fn fuzz_test_single_aabb_intersection_query(
        (aabbs_for_hierarchy, test_aabb): (Vec<ArbitraryAABB>, ArbitraryAABB),
    ) {
        let mut bvh = BoundingVolumeHierarchy::new();
        for (idx, aabb) in aabbs_for_hierarchy.iter().enumerate() {
            bvh.add_primitive_volume(BoundingVolumeID::from_u64(idx as u64), aabb.0.compact())
                .unwrap();
        }
        bvh.build();

        let mut intersected_ids = Vec::new();

        bvh.for_each_bounding_volume_in_axis_aligned_box(&test_aabb.0, |id| {
            intersected_ids.push(id);
        });

        let mut intersected_ids_brute_force = Vec::new();

        bvh.for_each_bounding_volume_in_axis_aligned_box_brute_force(&test_aabb.0, |id| {
            intersected_ids_brute_force.push(id);
        });

        assert_eq!(intersected_ids.len(), intersected_ids_brute_force.len());

        let intersected_ids: HashSet<BoundingVolumeID> = HashSet::from_iter(intersected_ids);
        let intersected_ids_brute_force = HashSet::from_iter(intersected_ids_brute_force);

        assert_eq!(intersected_ids, intersected_ids_brute_force);
    }

    pub fn fuzz_test_all_internal_intersections_query(aabbs_for_hierarchy: Vec<ArbitraryAABB>) {
        let mut bvh = BoundingVolumeHierarchy::new();
        for (idx, aabb) in aabbs_for_hierarchy.iter().enumerate() {
            bvh.add_primitive_volume(BoundingVolumeID::from_u64(idx as u64), aabb.0.compact())
                .unwrap();
        }
        bvh.build();

        let mut intersecting_pairs = Vec::new();

        bvh.for_each_intersecting_bounding_volume_pair(|id_i, id_j| {
            intersecting_pairs.push(if id_j.as_u64() >= id_i.as_u64() {
                (id_i, id_j)
            } else {
                (id_j, id_i)
            });
        });

        let mut intersecting_pairs_brute_force = Vec::new();

        bvh.for_each_intersecting_bounding_volume_pair_brute_force(|id_i, id_j| {
            intersecting_pairs_brute_force.push(if id_j.as_u64() >= id_i.as_u64() {
                (id_i, id_j)
            } else {
                (id_j, id_i)
            });
        });

        assert_eq!(
            intersecting_pairs.len(),
            intersecting_pairs_brute_force.len()
        );

        let intersecting_pairs: HashSet<(BoundingVolumeID, BoundingVolumeID)> =
            HashSet::from_iter(intersecting_pairs);
        let intersecting_pairs_brute_force = HashSet::from_iter(intersecting_pairs_brute_force);

        assert_eq!(intersecting_pairs, intersecting_pairs_brute_force);
    }

    fn arbitrary_norm_f32(u: &mut Unstructured<'_>) -> Result<f32> {
        Ok((f64::from(u.int_in_range(0..=1000000)?) / 1000000.0) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use impact_math::point::{Point3, Point3C};

    #[test]
    fn test_name() {
        let mut bvh = BoundingVolumeHierarchy::new();

        bvh.add_primitive_volume(
            BoundingVolumeID::from_u64(0),
            AxisAlignedBoxC::new(
                Point3C::new(-1.728949, -1.777199, -1.777149),
                Point3C::new(-0.17455101, -0.22280103, -0.22275102),
            ),
        )
        .unwrap();

        bvh.add_primitive_volume(
            BoundingVolumeID::from_u64(1),
            AxisAlignedBoxC::new(
                Point3C::new(0.55439794, -0.99949, -1.0),
                Point3C::new(0.55439794, -0.99949, -1.0),
            ),
        )
        .unwrap();

        bvh.build();

        let test_aab =
            AxisAlignedBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(-1.0, -1.0, -1.0));

        let mut intersections = Vec::new();
        bvh.for_each_bounding_volume_in_axis_aligned_box(&test_aab, |id| {
            intersections.push(id);
        });

        assert_eq!(&intersections, &[BoundingVolumeID::from_u64(0)]);
    }
}
