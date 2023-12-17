//! Representation and manipulation of voxels.

mod generation;

pub use generation::UniformBoxVoxelGenerator;

use crate::{
    geometry::{ClusterInstanceTransform, Sphere},
    num::Float,
};
use nalgebra::{vector, Vector3};
use num_derive::{FromPrimitive as DeriveFromPrimitive, ToPrimitive as DeriveToPrimitive};
use num_traits::FromPrimitive;
use std::iter;

/// A type identifier that determines all the properties of a voxel.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DeriveToPrimitive, DeriveFromPrimitive)]
pub enum VoxelType {
    Empty = 0,
    Default = 1,
}

/// Represents a voxel generator that provides a voxel type given the voxel
/// indices.
pub trait VoxelGenerator<F: Float> {
    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> F;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// Returns the voxel type at the given indices in a voxel grid. If the
    /// indices are outside the bounds of the grid, this method should return
    /// [`VoxelType::Empty`].
    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType;
}

/// An octree representation of a voxel grid.
#[derive(Debug)]
pub struct VoxelTree<F: Float> {
    voxel_extent: F,
    tree_height: u32,
    root_node: VoxelTreeNode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum VoxelTreeNode {
    Internal(VoxelTreeInternalNode),
    External(VoxelTreeExternalNode),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VoxelTreeInternalNode {
    children: Box<[VoxelTreeNode; 8]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VoxelTreeExternalNode {
    pub voxel_type: VoxelType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VoxelIndices {
    i: usize,
    j: usize,
    k: usize,
}

impl VoxelType {
    /// Returns an iterator over each voxel type in the order of their index.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..=1).map(|idx| Self::from_usize(idx).unwrap())
    }

    /// Whether the voxel is empty.
    pub fn is_empty(&self) -> bool {
        *self == Self::Empty
    }
}

impl<F: Float> VoxelTree<F> {
    /// Builds a new [`VoxelTree`] using the given [`VoxelGenerator`]. Groups of
    /// eight adjacent voxels of the same type will be recursively merged into
    /// larger voxels.
    pub fn build<G>(generator: &G) -> Self
    where
        G: VoxelGenerator<F>,
    {
        let voxel_extent = generator.voxel_extent();

        let tree_height = Self::tree_height_from_shape(generator.grid_shape());

        let root_node = VoxelTreeNode::build(generator, tree_height, 0, VoxelIndices::zeros());

        Self {
            voxel_extent,
            tree_height,
            root_node,
        }
    }

    /// Returns the extent of single unmerged voxel in the tree.
    pub fn voxel_extent(&self) -> F {
        self.voxel_extent
    }

    /// Returns the full height of the tree. The unmerged voxels reside at
    /// height zero. The grid size decreases by half for each successive height,
    /// down to one at the full height of the tree.
    pub fn tree_height(&self) -> u32 {
        self.tree_height
    }

    /// Returns the number of unmerged voxels along one axis of the grid. The
    /// grid size is always a power of two.
    pub fn grid_size(&self) -> usize {
        self.grid_size_at_height(0)
    }

    /// Computes a sphere bounding the entire voxel tree. Returns [`None`] if
    /// the tree is empty.
    pub fn compute_bounding_sphere(&self) -> Option<Sphere<F>> {
        self.root_node
            .compute_bounding_sphere(self, 0, 0, VoxelIndices::zeros())
    }

    /// Computes the transform of each (potentially merged) voxel in the tree.
    pub fn compute_voxel_transforms(&self) -> Vec<ClusterInstanceTransform<F>> {
        let mut transforms = Vec::new();
        self.root_node
            .add_voxel_transforms(self, &mut transforms, 0, VoxelIndices::zeros());
        transforms
    }

    /// Returns a reference to the root node of the tree.
    fn root_node(&self) -> &VoxelTreeNode {
        &self.root_node
    }

    fn compute_bounding_sphere_of_voxel(&self, depth: u32, indices: VoxelIndices) -> Sphere<F> {
        let voxel_extent = self.voxel_extent_at_depth(depth);
        let center = indices.voxel_center_offset(voxel_extent).into();
        let radius = F::ONE_HALF * F::sqrt(F::THREE) * voxel_extent;
        Sphere::new(center, radius)
    }

    fn voxel_scale_at_depth(&self, depth: u32) -> F {
        Self::voxel_scale_at_height(self.depth_to_height(depth))
    }

    fn voxel_extent_at_depth(&self, depth: u32) -> F {
        self.voxel_scale_at_depth(depth) * self.voxel_extent()
    }

    fn grid_size_at_height(&self, height: u32) -> usize {
        Self::grid_size_at_depth(self.height_to_depth(height))
    }

    fn height_to_depth(&self, height: u32) -> u32 {
        self.tree_height.checked_sub(height).unwrap()
    }

    fn depth_to_height(&self, depth: u32) -> u32 {
        self.height_to_depth(depth)
    }

    fn voxel_scale_at_height(height: u32) -> F {
        F::from_usize(Self::grid_size_at_depth(height)).unwrap()
    }

    fn grid_size_at_depth(depth: u32) -> usize {
        1_usize.checked_shl(depth).unwrap()
    }

    fn tree_height_from_shape([shape_x, shape_y, shape_z]: [usize; 3]) -> u32 {
        shape_x
            .max(shape_y)
            .max(shape_z)
            .checked_next_power_of_two()
            .unwrap()
            .trailing_zeros()
    }
}

impl VoxelTreeNode {
    fn build<F, G>(
        generator: &G,
        tree_height: u32,
        current_depth: u32,
        current_indices: VoxelIndices,
    ) -> Self
    where
        F: Float,
        G: VoxelGenerator<F>,
    {
        if current_depth == tree_height {
            Self::External(VoxelTreeExternalNode::create(generator, current_indices))
        } else {
            assert!(current_depth < tree_height);

            let next_depth = current_depth + 1;

            let mut has_common_child_voxel_type = true;
            let mut common_child_voxel_type = None;

            let children = current_indices.for_children().map(|next_indices| {
                let child = Self::build(generator, tree_height, next_depth, next_indices);

                match &child {
                    Self::External(child) if has_common_child_voxel_type => {
                        if let Some(common_child_voxel_type) = common_child_voxel_type {
                            has_common_child_voxel_type =
                                child.voxel_type == common_child_voxel_type;
                        } else {
                            common_child_voxel_type = Some(child.voxel_type);
                        }
                    }
                    _ => {
                        has_common_child_voxel_type = false;
                    }
                }

                child
            });

            match common_child_voxel_type {
                Some(common_child_voxel_type) if has_common_child_voxel_type => {
                    Self::External(VoxelTreeExternalNode {
                        voxel_type: common_child_voxel_type,
                    })
                }
                _ => Self::Internal(VoxelTreeInternalNode {
                    children: Box::new(children),
                }),
            }
        }
    }

    fn is_external(&self) -> bool {
        self.get_external().is_some()
    }

    fn is_internal(&self) -> bool {
        self.get_internal().is_some()
    }

    fn get_external(&self) -> Option<&VoxelTreeExternalNode> {
        if let Self::External(external) = self {
            Some(external)
        } else {
            None
        }
    }

    fn get_internal(&self) -> Option<&VoxelTreeInternalNode> {
        if let Self::Internal(internal) = self {
            Some(internal)
        } else {
            None
        }
    }

    fn compute_bounding_sphere<F: Float>(
        &self,
        tree: &VoxelTree<F>,
        max_depth: u32,
        current_depth: u32,
        current_indices: VoxelIndices,
    ) -> Option<Sphere<F>> {
        match self {
            Self::External(external) => {
                if !external.voxel_type.is_empty() {
                    Some(tree.compute_bounding_sphere_of_voxel(current_depth, current_indices))
                } else {
                    None
                }
            }
            Self::Internal(internal) => {
                let next_depth = current_depth + 1;

                if next_depth <= max_depth {
                    let mut aggregate_bounding_sphere: Option<Sphere<F>> = None;

                    for (child, next_indices) in
                        internal.children.iter().zip(current_indices.for_children())
                    {
                        match (
                            &mut aggregate_bounding_sphere,
                            child.compute_bounding_sphere(
                                tree,
                                max_depth,
                                next_depth,
                                next_indices,
                            ),
                        ) {
                            (Some(aggregate_bounding_sphere), Some(child_bounding_sphere)) => {
                                *aggregate_bounding_sphere = child_bounding_sphere
                                    .bounding_sphere_with(iter::once(&*aggregate_bounding_sphere));
                            }
                            (None, Some(child_bounding_sphere)) => {
                                aggregate_bounding_sphere = Some(child_bounding_sphere);
                            }
                            _ => {}
                        };
                    }

                    aggregate_bounding_sphere
                } else {
                    Some(tree.compute_bounding_sphere_of_voxel(current_depth, current_indices))
                }
            }
        }
    }

    fn add_voxel_transforms<F: Float>(
        &self,
        tree: &VoxelTree<F>,
        transforms: &mut Vec<ClusterInstanceTransform<F>>,
        current_depth: u32,
        current_indices: VoxelIndices,
    ) {
        match self {
            Self::External(external) => {
                if !external.voxel_type.is_empty() {
                    let voxel_scale = tree.voxel_scale_at_depth(current_depth);
                    let voxel_center_offset =
                        current_indices.voxel_center_offset(voxel_scale * tree.voxel_extent());

                    transforms.push(ClusterInstanceTransform::new(
                        voxel_center_offset,
                        voxel_scale,
                    ));
                }
            }
            Self::Internal(internal) => {
                let next_depth = current_depth + 1;
                for (child, next_indices) in
                    internal.children.iter().zip(current_indices.for_children())
                {
                    child.add_voxel_transforms(tree, transforms, next_depth, next_indices);
                }
            }
        }
    }
}

impl VoxelTreeInternalNode {
    fn children(&self) -> impl Iterator<Item = &'_ VoxelTreeNode> {
        self.children.iter()
    }

    fn internal_children(&self) -> impl Iterator<Item = &'_ VoxelTreeInternalNode> {
        self.children().filter_map(|child| child.get_internal())
    }

    fn external_children(&self) -> impl Iterator<Item = &'_ VoxelTreeExternalNode> {
        self.children().filter_map(|child| child.get_external())
    }

    fn external_nonempty_children(&self) -> impl Iterator<Item = &'_ VoxelTreeExternalNode> {
        self.external_children()
            .filter(|child| !child.voxel_type.is_empty())
    }

    fn n_internal_children(&self) -> usize {
        self.internal_children().count()
    }

    fn n_external_children(&self) -> usize {
        self.external_children().count()
    }

    fn n_external_nonempty_children(&self) -> usize {
        self.external_nonempty_children().count()
    }
}

impl VoxelTreeExternalNode {
    fn create<F, G>(generator: &G, indices: VoxelIndices) -> Self
    where
        F: Float,
        G: VoxelGenerator<F>,
    {
        let voxel_type = generator.voxel_at_indices(indices.i, indices.j, indices.k);
        Self { voxel_type }
    }
}

impl VoxelIndices {
    fn new(i: usize, j: usize, k: usize) -> Self {
        Self { i, j, k }
    }

    fn zeros() -> Self {
        Self::new(0, 0, 0)
    }

    fn for_children(&self) -> [Self; 8] {
        let i0 = 2 * self.i;
        let i1 = i0 + 1;
        let j0 = 2 * self.j;
        let j1 = j0 + 1;
        let k0 = 2 * self.k;
        let k1 = k0 + 1;

        [
            Self::new(i0, j0, k0),
            Self::new(i0, j0, k1),
            Self::new(i0, j1, k0),
            Self::new(i0, j1, k1),
            Self::new(i1, j0, k0),
            Self::new(i1, j0, k1),
            Self::new(i1, j1, k0),
            Self::new(i1, j1, k1),
        ]
    }

    fn voxel_center_offset<F: Float>(&self, voxel_extent: F) -> Vector3<F> {
        let half_voxel_extent = F::ONE_HALF * voxel_extent;
        vector![
            F::from_usize(self.i).unwrap() * voxel_extent + half_voxel_extent,
            F::from_usize(self.j).unwrap() * voxel_extent + half_voxel_extent,
            F::from_usize(self.k).unwrap() * voxel_extent + half_voxel_extent
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::{abs_diff_eq, assert_abs_diff_eq};
    use std::{collections::HashMap, sync::Mutex};

    struct EmptyVoxelGenerator {
        shape: [usize; 3],
    }

    struct DefaultVoxelGenerator {
        shape: [usize; 3],
    }

    struct RecordingVoxelGenerator {
        shape: [usize; 3],
        call_counts: Mutex<HashMap<(usize, usize, usize), usize>>,
    }

    impl VoxelGenerator<f64> for EmptyVoxelGenerator {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            self.shape
        }

        fn voxel_at_indices(&self, _i: usize, _j: usize, _k: usize) -> VoxelType {
            VoxelType::Empty
        }
    }

    impl VoxelGenerator<f64> for DefaultVoxelGenerator {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            self.shape
        }

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType {
            if i < self.shape[0] && j < self.shape[1] && k < self.shape[2] {
                VoxelType::Default
            } else {
                VoxelType::Empty
            }
        }
    }

    impl RecordingVoxelGenerator {
        fn new(shape: [usize; 3]) -> Self {
            Self {
                shape,
                call_counts: Mutex::new(HashMap::new()),
            }
        }

        fn n_unique_queries(&self) -> usize {
            self.call_counts.lock().unwrap().len()
        }

        fn count_is_one_for_all_queries(&self) -> bool {
            self.call_counts
                .lock()
                .unwrap()
                .values()
                .all(|count| *count == 1)
        }
    }

    impl VoxelGenerator<f64> for RecordingVoxelGenerator {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            self.shape
        }

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType {
            self.call_counts
                .lock()
                .unwrap()
                .entry((i, j, k))
                .and_modify(|count| *count += 1)
                .or_insert(1);

            if i < self.shape[0] && j < self.shape[1] && k < self.shape[2] {
                VoxelType::Default
            } else {
                VoxelType::Empty
            }
        }
    }

    #[test]
    fn should_get_voxel_extent_of_generator() {
        let generator = EmptyVoxelGenerator { shape: [0; 3] };
        let tree = VoxelTree::build(&generator);
        assert_eq!(tree.voxel_extent(), generator.voxel_extent());
    }

    #[test]
    fn should_build_tree_with_grid_size_one_for_zero_voxel_generator() {
        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [0; 3] });
        assert_eq!(tree.tree_height(), 0);
        assert_eq!(tree.grid_size(), 1);
    }

    #[test]
    fn should_build_tree_with_grid_size_one_for_single_voxel_generator() {
        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [1; 3] });
        assert_eq!(tree.tree_height(), 0);
        assert_eq!(tree.grid_size(), 1);
    }

    #[test]
    fn should_build_tree_with_grid_size_two_for_two_voxel_generator() {
        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [2, 1, 1] });
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);

        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [1, 2, 1] });
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);

        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [1, 1, 2] });
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);
    }

    #[test]
    fn should_build_tree_with_grid_size_four_for_three_voxel_generator() {
        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [3, 1, 1] });
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);

        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [1, 3, 1] });
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);

        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [1, 1, 3] });
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);
    }

    #[test]
    fn should_query_zero_voxel_generator_once() {
        let generator = RecordingVoxelGenerator::new([0; 3]);
        VoxelTree::build(&generator);
        assert_eq!(generator.n_unique_queries(), 1);
    }

    #[test]
    fn should_query_one_voxel_generator_once() {
        let generator = RecordingVoxelGenerator::new([1; 3]);
        VoxelTree::build(&generator);
        assert_eq!(generator.n_unique_queries(), 1);
    }

    #[test]
    fn should_perform_8_unique_queries_on_two_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([2, 1, 1]);
        VoxelTree::build(&generator);
        assert_eq!(generator.n_unique_queries(), 8);
    }

    #[test]
    fn should_perform_64_unique_queries_on_three_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([3, 1, 1]);
        VoxelTree::build(&generator);
        assert_eq!(generator.n_unique_queries(), 64);
    }

    #[test]
    fn should_not_query_same_indices_twice_for_zero_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([0; 3]);
        VoxelTree::build(&generator);
        assert!(generator.count_is_one_for_all_queries());
    }

    #[test]
    fn should_not_query_same_indices_twice_for_one_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([1; 3]);
        VoxelTree::build(&generator);
        assert!(generator.count_is_one_for_all_queries());
    }

    #[test]
    fn should_not_query_same_indices_twice_for_two_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([2, 1, 1]);
        VoxelTree::build(&generator);
        assert!(generator.count_is_one_for_all_queries());
    }

    #[test]
    fn should_not_query_same_indices_twice_for_three_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([3, 1, 1]);
        VoxelTree::build(&generator);
        assert!(generator.count_is_one_for_all_queries());
    }

    #[test]
    fn should_have_external_root_node_for_single_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1; 3] });
        assert!(tree.root_node().is_external());
    }

    #[test]
    fn should_have_default_external_root_node_for_single_default_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1; 3] });
        let root_node = tree.root_node().get_external().unwrap();
        assert_eq!(root_node.voxel_type, VoxelType::Default);
    }

    #[test]
    fn should_have_empty_external_root_node_for_single_empty_voxel_generator() {
        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [1; 3] });
        let root_node = tree.root_node().get_external().unwrap();
        assert!(root_node.voxel_type.is_empty());
    }

    #[test]
    fn should_have_internal_root_node_with_two_external_nonempty_children_for_two_default_voxel_generator(
    ) {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [2, 1, 1] });
        let root_node = tree.root_node().get_internal().unwrap();
        assert_eq!(root_node.n_external_children(), 8);
        assert_eq!(root_node.n_external_nonempty_children(), 2);
    }

    #[test]
    fn should_have_empty_external_root_node_for_two_empty_voxel_generator() {
        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [2, 1, 1] });
        let root_node = tree.root_node().get_external().unwrap();
        assert!(root_node.voxel_type.is_empty());
    }

    #[test]
    fn should_have_empty_external_root_node_for_8_empty_voxel_generator() {
        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [2; 3] });
        let root_node = tree.root_node().get_external().unwrap();
        assert!(root_node.voxel_type.is_empty());
    }

    #[test]
    fn should_have_default_external_root_node_for_8_default_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [2; 3] });
        let root_node = tree.root_node().get_external().unwrap();
        assert_eq!(root_node.voxel_type, VoxelType::Default);
    }

    #[test]
    fn should_have_internal_root_node_with_correct_internal_and_external_children_for_12_default_voxel_generator(
    ) {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [2, 2, 3] });
        let root_node = tree.root_node().get_internal().unwrap();
        assert_eq!(root_node.n_external_children(), 7);
        assert_eq!(root_node.n_external_nonempty_children(), 1);
        assert_eq!(root_node.n_internal_children(), 1);
        let internal_child = root_node.internal_children().next().unwrap();
        assert_eq!(internal_child.n_external_children(), 8);
        assert_eq!(internal_child.n_external_nonempty_children(), 4);
    }

    #[test]
    fn should_compute_no_transform_for_empty_voxel_generator() {
        let tree = VoxelTree::build(&EmptyVoxelGenerator { shape: [1; 3] });
        let transforms = tree.compute_voxel_transforms();
        assert!(transforms.is_empty());
    }

    #[test]
    fn should_compute_correct_transform_for_single_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [1; 3] };
        let tree = VoxelTree::build(&generator);
        let transforms = tree.compute_voxel_transforms();

        assert_eq!(transforms.len(), 1);
        let transform = &transforms[0];

        let half_voxel_extent = 0.5 * generator.voxel_extent();
        let correct_translation = vector![half_voxel_extent, half_voxel_extent, half_voxel_extent];
        assert_abs_diff_eq!(transform.translation(), &correct_translation);
        assert_abs_diff_eq!(transform.scaling(), 1.0);
    }

    #[test]
    fn should_compute_correct_transform_for_8_default_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [2; 3] };
        let tree = VoxelTree::build(&generator);
        let transforms = tree.compute_voxel_transforms();

        assert_eq!(transforms.len(), 1);
        let transform = &transforms[0];

        let half_merged_voxel_extent = generator.voxel_extent();
        let correct_translation = vector![
            half_merged_voxel_extent,
            half_merged_voxel_extent,
            half_merged_voxel_extent
        ];
        assert_abs_diff_eq!(transform.translation(), &correct_translation);
        assert_abs_diff_eq!(transform.scaling(), 2.0);
    }

    #[test]
    fn should_compute_correct_transform_for_64_default_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [4; 3] };
        let tree = VoxelTree::build(&generator);
        let transforms = tree.compute_voxel_transforms();

        assert_eq!(transforms.len(), 1);
        let transform = &transforms[0];

        let half_merged_voxel_extent = 2.0 * generator.voxel_extent();
        let correct_translation = vector![
            half_merged_voxel_extent,
            half_merged_voxel_extent,
            half_merged_voxel_extent
        ];
        assert_abs_diff_eq!(transform.translation(), &correct_translation);
        assert_abs_diff_eq!(transform.scaling(), 4.0);
    }

    #[test]
    fn should_compute_correct_transforms_for_12_default_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [2, 2, 3] };
        let tree = VoxelTree::build(&generator);
        let transforms = tree.compute_voxel_transforms();

        assert_eq!(transforms.len(), 5);

        let check_transform = |x, y, z, scaling| {
            assert!(transforms.iter().any(|transform| {
                let correct_translation = vector![
                    x * generator.voxel_extent(),
                    y * generator.voxel_extent(),
                    z * generator.voxel_extent()
                ];
                abs_diff_eq!(transform.translation(), &correct_translation)
                    && abs_diff_eq!(transform.scaling(), scaling)
            }));
        };

        // Merged voxel
        check_transform(1.0, 1.0, 1.0, 2.0);
        // Voxel at (0, 0, 2)
        check_transform(0.5, 0.5, 2.5, 1.0);
        // Voxel at (0, 1, 2)
        check_transform(0.5, 1.5, 2.5, 1.0);
        // Voxel at (1, 0, 2)
        check_transform(1.5, 0.5, 2.5, 1.0);
        // Voxel at (1, 1, 2)
        check_transform(1.5, 1.5, 2.5, 1.0);
    }
}
