//! Representation and manipulation of voxels.

#![allow(dead_code)]

mod generation;

pub use generation::{UniformBoxVoxelGenerator, UniformSphereVoxelGenerator};

use crate::{
    geometry::{AxisAlignedBox, Frustum, InstanceModelViewTransform, OrientedBox, Sphere},
    num::Float,
    rendering::fre,
};
use approx::AbsDiffEq;
use impact_utils::KeyIndexMapper;
use nalgebra::{point, vector, Point3, Similarity3, Vector3};
use nohash_hasher::BuildNoHashHasher;
use num_derive::{FromPrimitive as DeriveFromPrimitive, ToPrimitive as DeriveToPrimitive};
use num_traits::FromPrimitive;
use simba::scalar::{SubsetOf, SupersetOf};
use std::iter;

/// A type identifier that determines all the properties of a voxel.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DeriveToPrimitive, DeriveFromPrimitive)]
pub enum VoxelType {
    Default = 0,
}

/// Represents a voxel generator that provides a voxel type given the voxel
/// indices.
pub trait VoxelGenerator<F: Float> {
    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> F;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// Returns the voxel type at the given indices in a voxel grid, or [`None`]
    /// if the voxel is absent or the indices are outside the bounds of the
    /// grid.
    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType>;
}

/// An octree representation of a voxel grid.
#[derive(Debug)]
pub struct VoxelTree<F: Float> {
    voxel_extent: F,
    tree_height: VoxelTreeHeight,
    root_node_id: VoxelTreeNodeID,
    internal_nodes: VoxelTreeInternalNodeStorage<F>,
    external_nodes: VoxelTreeExternalNodeStorage,
}

/// A transform from the space of an voxel in a voxel tree to the space of the
/// whole tree.
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelTransform<F: Float> {
    translation: Vector3<F>,
    scaling: F,
}

/// Represents a type of node in a voxel tree.
trait VoxelTreeNode {
    /// Type of the node's ID.
    type ID: VoxelTreeNodeStorageID;
}

/// Represents a type of voxel tree node identifier.
trait VoxelTreeNodeStorageID: Copy + std::hash::Hash + Eq + std::fmt::Debug {
    /// Creates the node ID corresponding to the given numerical value.
    fn from_number(number: usize) -> Self;

    /// Returns the numerical value corresponding to the node ID.
    fn number(&self) -> usize;
}

/// The total number of levels in a voxel tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct VoxelTreeHeight {
    tree_height: u32,
}

/// The ID of a node in a voxel tree, which is either internal (it has child
/// nodes) or external (it refers to a voxel).
#[derive(Copy, Clone, Debug)]
enum VoxelTreeNodeID {
    Internal(VoxelTreeInternalNodeID),
    External(VoxelTreeExternalNodeID),
}

/// Flat storage for all nodes of a specific type in a [`VoxelTree`].
#[derive(Clone, Debug)]
struct VoxelTreeNodeStorage<C> {
    nodes: Vec<C>,
    index_map: KeyIndexMapper<usize, BuildNoHashHasher<usize>>,
    node_id_count: usize,
}

type VoxelTreeInternalNodeStorage<F> = VoxelTreeNodeStorage<VoxelTreeInternalNode<F>>;

type VoxelTreeExternalNodeStorage = VoxelTreeNodeStorage<VoxelTreeExternalNode>;

/// Identifier for a [`VoxelTreeInternalNode`] in a [`VoxelTree`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct VoxelTreeInternalNodeID(usize);

/// Identifier for a [`VoxelTreeExternalNode`] in a [`VoxelTree`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct VoxelTreeExternalNodeID(usize);

/// An internal node in a voxel tree. It has one child for each octant of the
/// region of the grid the node covers.
#[derive(Clone, Debug)]
struct VoxelTreeInternalNode<F: Float> {
    child_ids: [Option<VoxelTreeNodeID>; 8],
    aabb: AxisAlignedBox<F>,
}

/// An external node in a voxel tree. It represents a voxel, which may either be
/// unmerged (if the node is at the bottom of the tree) or a merged group of
/// adjacent identical voxels (if the node is not at the bottom).
#[derive(Clone, Debug)]
struct VoxelTreeExternalNode {
    voxel_type: VoxelType,
    voxel_indices: VoxelIndices,
    voxel_scale: u32,
    adjacent_voxels: Vec<(VoxelIndices, VoxelTreeExternalNodeID)>,
    exposed_face_areas: [u32; 6],
}

/// One of the six faces of a voxel.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum VoxelFace {
    LowerX = 0,
    UpperX = 1,
    LowerY = 2,
    UpperY = 3,
    LowerZ = 4,
    UpperZ = 5,
}

/// Helper type used for constructing a voxel tree. Like [`VoxelTreeNodeID`],
/// but uses a [`VoxelTreeExternalBuildNode`] instead of a
/// [`VoxelTreeExternalNodeID`].
#[derive(Clone, Debug)]
enum VoxelTreeBuildNode {
    Internal(VoxelTreeInternalNodeID),
    External(VoxelTreeExternalBuildNode),
}

/// Temporary representation of an external node in a voxel tree, used during
/// construction to avoid adding external nodes to the storage that will end up
/// being merged.
#[derive(Clone, Debug)]
struct VoxelTreeExternalBuildNode {
    indices: VoxelTreeIndices,
    voxel_type: VoxelType,
}

/// Indices in the voxel grid at the level of detail of a particular depth in a
/// voxel tree.
#[derive(Clone, Debug, PartialEq, Eq)]
struct VoxelTreeIndices {
    tree_height: VoxelTreeHeight,
    depth: u32,
    i: usize,
    j: usize,
    k: usize,
}

/// Indices in the voxel grid at the bottom of a voxel tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct VoxelIndices {
    i: usize,
    j: usize,
    k: usize,
}

/// An iterator over the sequence of octants that must be followed from the root
/// of a voxel tree to reach the voxel a given set of [`VoxelIndices`].
struct OctantIterator {
    indices: VoxelIndices,
    octant_size: usize,
    dividing_i: usize,
    dividing_j: usize,
    dividing_k: usize,
}

/// An octant in a voxel tree. The number associated with each variant is the
/// index of the corresponding child node of a [`VoxelTreeInternalNode`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Octant {
    BackBottomLeft = 0,
    FrontBottomLeft = 1,
    BackTopLeft = 2,
    FrontTopLeft = 3,
    BackBottomRight = 4,
    FrontBottomRight = 5,
    BackTopRight = 6,
    FrontTopRight = 7,
}

impl VoxelType {
    /// Returns an iterator over each voxel type in the order of their index.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..=0).map(|idx| Self::from_usize(idx).unwrap())
    }
}

impl<F: Float> VoxelTree<F> {
    /// Builds a new [`VoxelTree`] using the given [`VoxelGenerator`]. Groups of
    /// eight adjacent voxels of the same type will be recursively merged into
    /// larger voxels.
    pub fn build<G>(generator: &G) -> Option<Self>
    where
        G: VoxelGenerator<F>,
    {
        let voxel_extent = generator.voxel_extent();

        let tree_height = VoxelTreeHeight::from_shape(generator.grid_shape());

        let mut internal_nodes = VoxelTreeNodeStorage::new();
        let mut external_nodes = VoxelTreeNodeStorage::new();

        let build_node = VoxelTreeBuildNode::build(
            &mut internal_nodes,
            &mut external_nodes,
            generator,
            VoxelTreeIndices::at_root(tree_height),
        );

        VoxelTreeNodeID::from_build_node(&mut external_nodes, build_node).map(|root_node_id| {
            let mut tree = Self {
                voxel_extent,
                tree_height,
                root_node_id,
                internal_nodes,
                external_nodes,
            };
            tree.update_internal_node_aabbs();
            tree.update_adjacent_voxels_for_all_external_nodes();
            tree
        })
    }

    /// Returns the extent of single unmerged voxel in the tree.
    pub fn voxel_extent(&self) -> F {
        self.voxel_extent
    }

    /// Returns the full height of the tree. The unmerged voxels reside at
    /// height zero. The grid size decreases by half for each successive height,
    /// down to one at the full height of the tree.
    pub fn tree_height(&self) -> u32 {
        self.tree_height.value()
    }

    /// Returns the number of unmerged voxels along one axis of the grid. The
    /// grid size is always a power of two.
    pub fn grid_size(&self) -> usize {
        self.tree_height.grid_size_at_height(0)
    }

    /// Computes a sphere bounding the entire voxel tree by aggregating the
    /// bounding spheres of nodes down to the give depth. A larger maximum depth
    /// gives a tighter bounding sphere.
    ///
    /// # Panics
    /// If `max_depth` exceeds the tree height.
    pub fn compute_bounding_sphere(&self, max_depth: u32) -> Sphere<F> {
        assert!(self.tree_height.depth_is_valid(max_depth));
        self.root_node_id.compute_bounding_sphere(
            self,
            max_depth,
            VoxelTreeIndices::at_root(self.tree_height),
        )
    }

    /// Computes the transform of each (potentially merged) voxel in the tree.
    pub fn compute_voxel_transforms(&self) -> Vec<VoxelTransform<F>> {
        let mut transforms = Vec::new();
        self.root_node_id.add_voxel_transforms(
            self,
            &mut transforms,
            VoxelTreeIndices::at_root(self.tree_height),
            &|_| true,
        );
        transforms
    }

    /// Computes the transform of each (potentially merged) voxel in the tree
    /// that has at least one face not fully obscured by adjacent voxels.
    pub fn compute_exposed_voxel_transforms(&self) -> Vec<VoxelTransform<F>> {
        let mut transforms = Vec::new();
        self.root_node_id.add_voxel_transforms(
            self,
            &mut transforms,
            VoxelTreeIndices::at_root(self.tree_height),
            &|node_id| !self.external_node(node_id).has_only_fully_obscured_faces(),
        );
        transforms
    }

    /// Returns the type of the voxel at the given indices in the voxel grid, or
    /// [`None`] if the voxel is empty or the indices are outside the bounds of
    /// the grid.
    pub fn find_voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
        self.find_external_node_id_at_indices(i, j, k)
            .map(|node_id| self.external_node(node_id).voxel_type)
    }

    /// Rebuilds the list of adjacent voxels for every external node in the
    /// tree.
    ///
    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction. It also does not remove previously registered
    /// adjacent voxels.
    fn update_adjacent_voxels_for_all_external_nodes(&mut self) {
        for node_idx in 0..self.external_nodes.n_nodes() {
            self.update_adjacent_voxels_for_external_node(node_idx);
        }
    }

    /// Updates the axis-aligned bounding box (AABB) of every internal node in
    /// the tree based on the AABBs of its children.
    ///
    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    pub fn update_internal_node_aabbs(&mut self) {
        self.root_node_id
            .clone()
            .update_internal_node_aabbs(self, VoxelTreeIndices::at_root(self.tree_height));
    }

    /// Returns the ID of the root node of the tree.
    fn root_node_id(&self) -> &VoxelTreeNodeID {
        &self.root_node_id
    }

    fn internal_node(&self, id: VoxelTreeInternalNodeID) -> &VoxelTreeInternalNode<F> {
        self.internal_nodes.node(id)
    }

    fn internal_node_mut(&mut self, id: VoxelTreeInternalNodeID) -> &mut VoxelTreeInternalNode<F> {
        self.internal_nodes.node_mut(id)
    }

    fn external_node(&self, id: VoxelTreeExternalNodeID) -> &VoxelTreeExternalNode {
        self.external_nodes.node(id)
    }

    fn find_external_node_at_indices(
        &self,
        i: usize,
        j: usize,
        k: usize,
    ) -> Option<&VoxelTreeExternalNode> {
        self.find_external_node_id_at_indices(i, j, k)
            .map(|node_id| self.external_node(node_id))
    }

    fn find_external_node_id_at_indices(
        &self,
        i: usize,
        j: usize,
        k: usize,
    ) -> Option<VoxelTreeExternalNodeID> {
        self.find_external_node_id_at_indices_generic(VoxelIndices::new(i, j, k), |node_id| {
            self.internal_node(node_id)
        })
    }

    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    fn find_external_node_idx_at_indices(&self, indices: VoxelIndices) -> Option<usize> {
        self.find_external_node_id_at_indices_generic(indices, |node_id| {
            self.internal_nodes.node_at_idx(node_id.number())
        })
        .map(|node_id| node_id.number())
    }

    fn find_external_node_id_at_indices_generic<'a>(
        &'a self,
        indices: VoxelIndices,
        internal_node_from_id: impl Fn(VoxelTreeInternalNodeID) -> &'a VoxelTreeInternalNode<F>,
    ) -> Option<VoxelTreeExternalNodeID> {
        if let Some(octants) = indices.octants(self.tree_height.value()) {
            let mut node_id = Some(self.root_node_id());

            for octant in octants {
                match node_id {
                    Some(VoxelTreeNodeID::External(_)) => {
                        break;
                    }
                    Some(VoxelTreeNodeID::Internal(internal_id)) => {
                        node_id =
                            internal_node_from_id(*internal_id).child_ids[octant.idx()].as_ref();
                    }
                    None => {
                        return None;
                    }
                }
            }

            node_id.map(|node_id| node_id.as_external().unwrap())
        } else {
            None
        }
    }

    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    fn update_adjacent_voxels_for_external_node(&mut self, node_idx: usize) {
        let node = self.external_nodes.node_at_idx(node_idx);
        let voxel_scale = node.voxel_scale;
        let voxel_indices = node.voxel_indices;

        if voxel_scale == 1 {
            self.update_adjacent_voxels_for_unmerged_voxel(node_idx, voxel_indices);
        } else {
            self.update_adjacent_voxels_for_merged_voxel(node_idx, voxel_scale, voxel_indices);
        }
    }

    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    fn update_adjacent_voxels_for_unmerged_voxel(
        &mut self,
        node_idx: usize,
        voxel_indices: VoxelIndices,
    ) {
        let grid_size = self.grid_size();

        if voxel_indices.i > 0 {
            self.update_adjacent_voxel_for_unmerged_voxel_on_one_side(
                node_idx,
                voxel_indices,
                VoxelFace::LowerX,
                VoxelIndices::new(voxel_indices.i - 1, voxel_indices.j, voxel_indices.k),
            );
        }
        if voxel_indices.i + 1 < grid_size {
            self.update_adjacent_voxel_for_unmerged_voxel_on_one_side(
                node_idx,
                voxel_indices,
                VoxelFace::UpperX,
                VoxelIndices::new(voxel_indices.i + 1, voxel_indices.j, voxel_indices.k),
            );
        }
        if voxel_indices.j > 0 {
            self.update_adjacent_voxel_for_unmerged_voxel_on_one_side(
                node_idx,
                voxel_indices,
                VoxelFace::LowerY,
                VoxelIndices::new(voxel_indices.i, voxel_indices.j - 1, voxel_indices.k),
            );
        }
        if voxel_indices.j + 1 < grid_size {
            self.update_adjacent_voxel_for_unmerged_voxel_on_one_side(
                node_idx,
                voxel_indices,
                VoxelFace::UpperY,
                VoxelIndices::new(voxel_indices.i, voxel_indices.j + 1, voxel_indices.k),
            );
        }
        if voxel_indices.k > 0 {
            self.update_adjacent_voxel_for_unmerged_voxel_on_one_side(
                node_idx,
                voxel_indices,
                VoxelFace::LowerZ,
                VoxelIndices::new(voxel_indices.i, voxel_indices.j, voxel_indices.k - 1),
            );
        }
        if voxel_indices.k + 1 < grid_size {
            self.update_adjacent_voxel_for_unmerged_voxel_on_one_side(
                node_idx,
                voxel_indices,
                VoxelFace::UpperZ,
                VoxelIndices::new(voxel_indices.i, voxel_indices.j, voxel_indices.k + 1),
            );
        }
    }

    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    fn update_adjacent_voxel_for_unmerged_voxel_on_one_side(
        &mut self,
        node_idx: usize,
        voxel_indices: VoxelIndices,
        face: VoxelFace,
        adjacent_indices: VoxelIndices,
    ) {
        // We only need to search for the node at the adjacent indices if we do
        // not already have a neighbor registered at this side
        if !self
            .external_nodes
            .node_at_idx(node_idx)
            .face_is_fully_obscured(face)
        {
            if let Some(adjacent_node_idx) =
                self.find_external_node_idx_at_indices(adjacent_indices)
            {
                let adjacent_node = self.external_nodes.node_at_idx_mut(adjacent_node_idx);

                // These are the indices of the adjacent voxel's origin,
                // which may be different from the indices we searched at
                let adjacent_voxel_indices = adjacent_node.voxel_indices;

                // Add this voxel as an adjacent voxel to the adjacent voxel
                adjacent_node.adjacent_voxels.push((
                    voxel_indices,
                    VoxelTreeExternalNodeID::from_number(node_idx),
                ));

                adjacent_node.add_obscured_face_area(face.opposite_face(), 1);

                let node = self.external_nodes.node_at_idx_mut(node_idx);

                // Add the adjacent voxel as an adjacent voxel to this voxel
                node.adjacent_voxels.push((
                    adjacent_voxel_indices,
                    VoxelTreeExternalNodeID::from_number(adjacent_node_idx),
                ));

                node.add_obscured_face_area(face, 1);
            }
        }
    }

    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    fn update_adjacent_voxels_for_merged_voxel(
        &mut self,
        node_idx: usize,
        voxel_scale: u32,
        voxel_indices: VoxelIndices,
    ) {
        let grid_size = self.grid_size();

        let face_area = voxel_scale.pow(2);
        let voxel_scale = voxel_scale as usize;

        let mut covered = vec![false; face_area as usize];

        if voxel_indices.i > 0 {
            self.update_adjacent_voxels_for_merged_voxel_on_one_side(
                node_idx,
                voxel_scale,
                face_area,
                voxel_indices,
                VoxelFace::LowerX,
                &mut covered,
                |delta_j, delta_k| {
                    VoxelIndices::new(
                        voxel_indices.i - 1,
                        voxel_indices.j + delta_j,
                        voxel_indices.k + delta_k,
                    )
                },
            );
        }
        if voxel_indices.i + voxel_scale < grid_size {
            covered.fill(false);
            self.update_adjacent_voxels_for_merged_voxel_on_one_side(
                node_idx,
                voxel_scale,
                face_area,
                voxel_indices,
                VoxelFace::UpperX,
                &mut covered,
                |delta_j, delta_k| {
                    VoxelIndices::new(
                        voxel_indices.i + voxel_scale,
                        voxel_indices.j + delta_j,
                        voxel_indices.k + delta_k,
                    )
                },
            );
        }
        if voxel_indices.j > 0 {
            covered.fill(false);
            self.update_adjacent_voxels_for_merged_voxel_on_one_side(
                node_idx,
                voxel_scale,
                face_area,
                voxel_indices,
                VoxelFace::LowerY,
                &mut covered,
                |delta_i, delta_k| {
                    VoxelIndices::new(
                        voxel_indices.i + delta_i,
                        voxel_indices.j - 1,
                        voxel_indices.k + delta_k,
                    )
                },
            );
        }
        if voxel_indices.j + voxel_scale < grid_size {
            covered.fill(false);
            self.update_adjacent_voxels_for_merged_voxel_on_one_side(
                node_idx,
                voxel_scale,
                face_area,
                voxel_indices,
                VoxelFace::UpperY,
                &mut covered,
                |delta_i, delta_k| {
                    VoxelIndices::new(
                        voxel_indices.i + delta_i,
                        voxel_indices.j + voxel_scale,
                        voxel_indices.k + delta_k,
                    )
                },
            );
        }
        if voxel_indices.k > 0 {
            covered.fill(false);
            self.update_adjacent_voxels_for_merged_voxel_on_one_side(
                node_idx,
                voxel_scale,
                face_area,
                voxel_indices,
                VoxelFace::LowerZ,
                &mut covered,
                |delta_i, delta_j| {
                    VoxelIndices::new(
                        voxel_indices.i + delta_i,
                        voxel_indices.j + delta_j,
                        voxel_indices.k - 1,
                    )
                },
            );
        }
        if voxel_indices.k + voxel_scale < grid_size {
            covered.fill(false);
            self.update_adjacent_voxels_for_merged_voxel_on_one_side(
                node_idx,
                voxel_scale,
                face_area,
                voxel_indices,
                VoxelFace::UpperZ,
                &mut covered,
                |delta_i, delta_j| {
                    VoxelIndices::new(
                        voxel_indices.i + delta_i,
                        voxel_indices.j + delta_j,
                        voxel_indices.k + voxel_scale,
                    )
                },
            );
        }
    }

    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    fn update_adjacent_voxels_for_merged_voxel_on_one_side(
        &mut self,
        node_idx: usize,
        voxel_scale: usize,
        face_area: u32,
        voxel_indices: VoxelIndices,
        face: VoxelFace,
        covered: &mut [bool],
        get_adjacent_indices: impl Fn(usize, usize) -> VoxelIndices,
    ) {
        let mut delta_n = 0;
        let mut delta_m = 0;
        let mut idx = 0;

        'outer: while delta_n < voxel_scale {
            while delta_m < voxel_scale {
                if !covered[idx] {
                    let adjacent_indices = get_adjacent_indices(delta_n, delta_m);

                    let adjacent_voxel_scale = if let Some(adjacent_node_id) = self
                        .external_nodes
                        .node_at_idx(node_idx)
                        .adjacent_voxel(adjacent_indices)
                    {
                        // If there is already a voxel registered at the
                        // adjacent indices, we only need to obtain its scale to
                        // update the `covered` map
                        Some(
                            self.external_nodes
                                .node_at_idx(adjacent_node_id.number())
                                .voxel_scale as usize,
                        )
                    } else if let Some(adjacent_node_idx) =
                        self.find_external_node_idx_at_indices(adjacent_indices)
                    {
                        let adjacent_node = self.external_nodes.node_at_idx_mut(adjacent_node_idx);
                        let adjacent_voxel_scale = adjacent_node.voxel_scale as usize;

                        // If the scale of the adjacent voxel is larger than
                        // one, it could already be registered as an adjacent
                        // voxel to us, just not at the exact indices we
                        // searched at. Now that we have the adjacent node, we
                        // can check this and make sure to only register the
                        // voxels as neighbors if they truly have not been
                        // registered before.
                        if adjacent_voxel_scale == 1
                            || !adjacent_node.is_adjacent_to_voxel(voxel_indices)
                        {
                            // These are the indices of the adjacent voxel's
                            // origin, which may be different from the indices
                            // we searched at
                            let adjacent_voxel_indices = adjacent_node.voxel_indices;

                            // Either this voxel will completely obscure the
                            // adjacent voxel or vice versa, so we should add
                            // the minimum of the face areas of the two voxels
                            // as the obscured area for both
                            let obscured_area = u32::min(face_area, adjacent_node.face_area());

                            // Add this voxel as an adjacent voxel to the
                            // adjacent voxel
                            adjacent_node.adjacent_voxels.push((
                                voxel_indices,
                                VoxelTreeExternalNodeID::from_number(node_idx),
                            ));

                            adjacent_node
                                .add_obscured_face_area(face.opposite_face(), obscured_area);

                            let node = self.external_nodes.node_at_idx_mut(node_idx);

                            // Add the adjacent voxel as an adjacent voxel to
                            // this voxel
                            node.adjacent_voxels.push((
                                adjacent_voxel_indices,
                                VoxelTreeExternalNodeID::from_number(adjacent_node_idx),
                            ));

                            node.add_obscured_face_area(face, obscured_area);
                        }

                        Some(adjacent_voxel_scale)
                    } else {
                        None
                    };

                    if let Some(adjacent_voxel_scale) = adjacent_voxel_scale {
                        if adjacent_voxel_scale >= voxel_scale {
                            // If the neighbor is not smaller than us, there is
                            // no room for more neighbors on this side
                            break 'outer;
                        } else if adjacent_voxel_scale > 1 {
                            // If the neighbor is merged but smaller than
                            // us, we mark all the later locations covered
                            // by the neighbor so as not to query them later
                            for n in delta_n..(delta_n + adjacent_voxel_scale) {
                                for m in delta_m..(delta_m + adjacent_voxel_scale) {
                                    covered[n * voxel_scale + m] = true;
                                }
                            }
                            // We can immediately skip ahead to the end of the
                            // neighbor
                            delta_m += adjacent_voxel_scale - 1;
                            idx += adjacent_voxel_scale - 1;
                        }
                    }
                }

                delta_m += 1;
                idx += 1;
            }
            delta_m = 0;
            delta_n += 1;
        }
    }

    fn compute_bounding_sphere_of_voxel(&self, indices: VoxelTreeIndices) -> Sphere<F> {
        let (voxel_scale, center) = indices.voxel_scale_and_center_offset(self.voxel_extent());
        let radius = F::ONE_HALF * F::sqrt(F::THREE) * self.voxel_extent() * voxel_scale;
        Sphere::new(center.into(), radius)
    }

    fn compute_aabb_of_voxel(&self, indices: VoxelTreeIndices) -> AxisAlignedBox<F> {
        let (lower_corner, upper_corner) =
            indices.voxel_lower_and_upper_corner(self.voxel_extent());
        AxisAlignedBox::new(lower_corner, upper_corner)
    }
}

impl<F: Float> VoxelTransform<F> {
    /// Creates a new voxel transform with the given translation and scaling.
    pub fn new(translation: Vector3<F>, scaling: F) -> Self {
        Self {
            translation,
            scaling,
        }
    }

    /// Creates a new identity voxel transform.
    pub fn identity() -> Self {
        Self {
            translation: Vector3::zeros(),
            scaling: F::ONE,
        }
    }

    /// Returns a reference to the translational part of the voxel transform.
    pub fn translation(&self) -> &Vector3<F> {
        &self.translation
    }

    /// Returns the scaling part of the voxel transform.
    pub fn scaling(&self) -> F {
        self.scaling
    }

    /// Applies the given transform from the space of the voxel tree to camera
    /// space, yielding the model view transform of the voxel.
    pub fn transform_into_model_view_transform(
        &self,
        transform_from_tree_to_camera_space: &Similarity3<F>,
    ) -> InstanceModelViewTransform
    where
        F: SubsetOf<fre>,
    {
        let scaling_from_tree_to_camera_space = transform_from_tree_to_camera_space.scaling();
        let rotation_from_tree_to_camera_space =
            transform_from_tree_to_camera_space.isometry.rotation;
        let translation_from_tree_to_camera_space = transform_from_tree_to_camera_space
            .isometry
            .translation
            .vector;

        let new_scaling = scaling_from_tree_to_camera_space * self.scaling;

        let new_translation = translation_from_tree_to_camera_space
            + rotation_from_tree_to_camera_space.transform_vector(&self.translation)
                * scaling_from_tree_to_camera_space;

        InstanceModelViewTransform {
            rotation: rotation_from_tree_to_camera_space.cast::<fre>(),
            translation: new_translation.cast::<fre>(),
            scaling: fre::from_subset(&new_scaling),
        }
    }
}

impl<F: Float> Default for VoxelTransform<F> {
    fn default() -> Self {
        Self::identity()
    }
}

impl<F> AbsDiffEq for VoxelTransform<F>
where
    F: Float + AbsDiffEq,
    <F as AbsDiffEq>::Epsilon: Clone,
{
    type Epsilon = <F as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        <F as AbsDiffEq>::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Vector3::abs_diff_eq(&self.translation, &other.translation, epsilon)
            && F::abs_diff_eq(&self.scaling, &other.scaling, epsilon)
    }
}

impl VoxelTreeHeight {
    fn new(tree_height: u32) -> Self {
        Self { tree_height }
    }

    fn from_shape([shape_x, shape_y, shape_z]: [usize; 3]) -> Self {
        let tree_height = shape_x
            .max(shape_y)
            .max(shape_z)
            .checked_next_power_of_two()
            .unwrap()
            .trailing_zeros();

        Self { tree_height }
    }

    fn value(&self) -> u32 {
        self.tree_height
    }

    fn depth_is_valid(&self, depth: u32) -> bool {
        depth <= self.tree_height
    }

    fn depth_is_max(&self, depth: u32) -> bool {
        depth == self.tree_height
    }

    fn voxel_scale_at_depth(&self, depth: u32) -> u32 {
        Self::voxel_scale_at_height(self.depth_to_height(depth))
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

    fn grid_size_at_depth(depth: u32) -> usize {
        1_usize.checked_shl(depth).unwrap()
    }

    fn voxel_scale_at_height(height: u32) -> u32 {
        1_u32.checked_shl(height).unwrap()
    }
}

impl VoxelTreeNodeID {
    fn from_build_node(
        external_nodes: &mut VoxelTreeExternalNodeStorage,
        build_node: Option<VoxelTreeBuildNode>,
    ) -> Option<Self> {
        match build_node {
            Some(VoxelTreeBuildNode::External(external_build_node)) => Some(Self::External(
                VoxelTreeExternalNodeID::from_build_node(external_nodes, external_build_node),
            )),
            Some(VoxelTreeBuildNode::Internal(internal_id)) => Some(Self::Internal(internal_id)),
            None => None,
        }
    }

    fn is_external(&self) -> bool {
        self.as_external().is_some()
    }

    fn is_internal(&self) -> bool {
        self.as_internal().is_some()
    }

    fn as_external(&self) -> Option<VoxelTreeExternalNodeID> {
        if let Self::External(external_id) = self {
            Some(*external_id)
        } else {
            None
        }
    }

    fn as_internal(&self) -> Option<VoxelTreeInternalNodeID> {
        if let Self::Internal(internal_id) = self {
            Some(*internal_id)
        } else {
            None
        }
    }

    fn update_internal_node_aabbs<F: Float>(
        &self,
        tree: &mut VoxelTree<F>,
        current_indices: VoxelTreeIndices,
    ) -> AxisAlignedBox<F> {
        match self {
            Self::External(_) => tree.compute_aabb_of_voxel(current_indices),
            Self::Internal(internal_id) => {
                let child_ids = tree.internal_node(*internal_id).child_ids;

                let mut aggregate_aabb: Option<AxisAlignedBox<F>> = None;

                for (child_id, next_indices) in
                    child_ids.iter().zip(current_indices.for_next_depth())
                {
                    let child_aabb = child_id
                        .as_ref()
                        .map(|child_id| child_id.update_internal_node_aabbs(tree, next_indices));

                    match (&mut aggregate_aabb, child_aabb) {
                        (Some(aggregate_aabb), Some(child_aabb)) => {
                            *aggregate_aabb =
                                AxisAlignedBox::aabb_from_pair(&*aggregate_aabb, &child_aabb);
                        }
                        (None, Some(child_aabb)) => {
                            aggregate_aabb = Some(child_aabb);
                        }
                        _ => {}
                    };
                }

                let aggregate_aabb = aggregate_aabb.unwrap();

                tree.internal_node_mut(*internal_id).aabb = aggregate_aabb.clone();

                aggregate_aabb
            }
        }
    }

    fn compute_bounding_sphere<F: Float>(
        &self,
        tree: &VoxelTree<F>,
        max_depth: u32,
        current_indices: VoxelTreeIndices,
    ) -> Sphere<F> {
        match self {
            Self::External(_) => tree.compute_bounding_sphere_of_voxel(current_indices),
            Self::Internal(internal_id) => {
                if current_indices.depth() < max_depth {
                    let child_ids = &tree.internal_node(*internal_id).child_ids;

                    let mut aggregate_bounding_sphere: Option<Sphere<F>> = None;

                    for (child_id, next_indices) in
                        child_ids.iter().zip(current_indices.for_next_depth())
                    {
                        let child_bounding_sphere = child_id.as_ref().map(|child_id| {
                            child_id.compute_bounding_sphere(tree, max_depth, next_indices)
                        });

                        match (&mut aggregate_bounding_sphere, child_bounding_sphere) {
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
                    aggregate_bounding_sphere.unwrap()
                } else {
                    tree.compute_bounding_sphere_of_voxel(current_indices)
                }
            }
        }
    }

    fn add_voxel_transforms<F: Float>(
        &self,
        tree: &VoxelTree<F>,
        transforms: &mut Vec<VoxelTransform<F>>,
        current_indices: VoxelTreeIndices,
        criterion: &impl Fn(VoxelTreeExternalNodeID) -> bool,
    ) {
        match self {
            Self::External(external_id) => {
                if criterion(*external_id) {
                    let (voxel_scale, voxel_center_offset) =
                        current_indices.voxel_scale_and_center_offset(tree.voxel_extent());

                    transforms.push(VoxelTransform::new(voxel_center_offset, voxel_scale));
                }
            }
            Self::Internal(internal_id) => {
                let child_ids = &tree.internal_node(*internal_id).child_ids;

                for (child_id, next_indices) in
                    child_ids.iter().zip(current_indices.for_next_depth())
                {
                    if let Some(child_id) = child_id.as_ref() {
                        child_id.add_voxel_transforms(tree, transforms, next_indices, criterion);
                    }
                }
            }
        }
    }
}

impl<C: VoxelTreeNode> VoxelTreeNodeStorage<C> {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            index_map: KeyIndexMapper::default(),
            node_id_count: 0,
        }
    }

    fn n_nodes(&self) -> usize {
        self.nodes.len()
    }

    fn has_node(&self, node_id: C::ID) -> bool {
        self.index_map.contains_key(node_id.number())
    }

    fn node(&self, node_id: C::ID) -> &C {
        let idx = self.index_map.idx(node_id.number());
        self.node_at_idx(idx)
    }

    fn node_at_idx(&self, idx: usize) -> &C {
        &self.nodes[idx]
    }

    fn node_mut(&mut self, node_id: C::ID) -> &mut C {
        let idx = self.index_map.idx(node_id.number());
        self.node_at_idx_mut(idx)
    }

    fn node_at_idx_mut(&mut self, idx: usize) -> &mut C {
        &mut self.nodes[idx]
    }

    fn nodes(&self) -> impl Iterator<Item = &C> {
        self.nodes.iter()
    }

    fn nodes_mut(&mut self) -> impl Iterator<Item = &mut C> {
        self.nodes.iter_mut()
    }

    fn add_node(&mut self, node: C) -> C::ID {
        let node_id = self.create_new_node_id();
        self.index_map.push_key(node_id.number());
        self.nodes.push(node);
        node_id
    }

    fn remove_node(&mut self, node_id: C::ID) {
        let idx = self.index_map.swap_remove_key(node_id.number());
        self.nodes.swap_remove(idx);
    }

    fn create_new_node_id(&mut self) -> C::ID {
        let node_id = C::ID::from_number(self.node_id_count);
        self.node_id_count += 1;
        node_id
    }
}

impl VoxelTreeNodeStorageID for VoxelTreeInternalNodeID {
    fn from_number(number: usize) -> Self {
        Self(number)
    }

    fn number(&self) -> usize {
        self.0
    }
}

impl VoxelTreeExternalNodeID {
    fn from_build_node(
        external_nodes: &mut VoxelTreeExternalNodeStorage,
        build_node: VoxelTreeExternalBuildNode,
    ) -> Self {
        let (voxel_scale, voxel_indices) = build_node.indices.voxel_scale_and_indices();
        external_nodes.add_node(VoxelTreeExternalNode::new(
            build_node.voxel_type,
            voxel_indices,
            voxel_scale,
        ))
    }
}

impl VoxelTreeNodeStorageID for VoxelTreeExternalNodeID {
    fn from_number(number: usize) -> Self {
        Self(number)
    }

    fn number(&self) -> usize {
        self.0
    }
}

impl<F: Float> VoxelTreeInternalNode<F> {
    fn new(child_ids: [Option<VoxelTreeNodeID>; 8]) -> Self {
        Self {
            child_ids,
            aabb: AxisAlignedBox::new(Point3::origin(), Point3::origin()),
        }
    }

    fn child_ids(&self) -> impl Iterator<Item = VoxelTreeNodeID> + '_ {
        self.child_ids.iter().filter_map(|child_id| *child_id)
    }

    fn internal_child_ids(&self) -> impl Iterator<Item = VoxelTreeInternalNodeID> + '_ {
        self.child_ids()
            .filter_map(|child_id| child_id.as_internal())
    }

    fn external_child_ids(&self) -> impl Iterator<Item = VoxelTreeExternalNodeID> + '_ {
        self.child_ids()
            .filter_map(|child_id| child_id.as_external())
    }

    fn n_children(&self) -> usize {
        self.child_ids().count()
    }

    fn n_internal_children(&self) -> usize {
        self.internal_child_ids().count()
    }

    fn n_external_children(&self) -> usize {
        self.external_child_ids().count()
    }

    fn aabb(&self) -> &AxisAlignedBox<F> {
        &self.aabb
    }
}

impl<F: Float> VoxelTreeNode for VoxelTreeInternalNode<F> {
    type ID = VoxelTreeInternalNodeID;
}

impl VoxelTreeExternalNode {
    fn new(voxel_type: VoxelType, voxel_indices: VoxelIndices, voxel_scale: u32) -> Self {
        Self {
            voxel_type,
            voxel_indices,
            voxel_scale,
            adjacent_voxels: Vec::new(),
            exposed_face_areas: [voxel_scale.pow(2); 6],
        }
    }

    /// Returns the number of unmerged voxels that would cover one face of this
    /// voxel.
    fn face_area(&self) -> u32 {
        self.voxel_scale.pow(2)
    }

    /// Returns the number of unmerged voxels that are exposed at the given face
    /// of this voxel.
    fn exposed_face_area(&self, face: VoxelFace) -> u32 {
        self.exposed_face_areas[face as usize]
    }

    /// Whether the given face of this voxel is completely obscured by adjacent
    /// voxels.
    fn face_is_fully_obscured(&self, face: VoxelFace) -> bool {
        self.exposed_face_area(face) == 0
    }

    /// Whether the given face of this voxel is completely exposed.
    fn face_is_fully_exposed(&self, face: VoxelFace) -> bool {
        self.exposed_face_area(face) == self.face_area()
    }

    /// Whether any of the faces of this voxel is completely obscured by
    /// adjacent voxels.
    fn has_fully_obscured_face(&self) -> bool {
        self.exposed_face_areas
            .iter()
            .any(|exposed_area| *exposed_area == 0)
    }

    /// Whether all the faces of this voxel are completely obscured by adjacent
    /// voxels.
    fn has_only_fully_obscured_faces(&self) -> bool {
        self.exposed_face_areas
            .iter()
            .all(|exposed_area| *exposed_area == 0)
    }

    /// Whether all the faces of this voxel are completely exposed.
    fn has_only_fully_exposed_faces(&self) -> bool {
        let face_area = self.face_area();
        self.exposed_face_areas
            .iter()
            .all(|exposed_area| *exposed_area == face_area)
    }

    /// Returns the ID of the external node for the adjacent voxel with the
    /// given indices, or [`None`] if the voxel with the given indices is not
    /// adjacent to this one.
    fn adjacent_voxel(&self, voxel_indices: VoxelIndices) -> Option<VoxelTreeExternalNodeID> {
        self.adjacent_voxels
            .iter()
            .find(|(adjacent_voxel_indices, _)| adjacent_voxel_indices == &voxel_indices)
            .map(|(_, adjacent_node_id)| *adjacent_node_id)
    }

    /// Whether the voxel with the given indices is adjacent to this voxel.
    fn is_adjacent_to_voxel(&self, voxel_indices: VoxelIndices) -> bool {
        self.adjacent_voxels
            .iter()
            .any(|(adjacent_voxel_indices, _)| adjacent_voxel_indices == &voxel_indices)
    }

    /// Reduces the exposed area of the given face of this voxel by the given
    /// area.
    fn add_obscured_face_area(&mut self, face: VoxelFace, obscured_area: u32) {
        let exposed_area = &mut self.exposed_face_areas[face as usize];
        *exposed_area = exposed_area.checked_sub(obscured_area).unwrap();
    }
}

impl VoxelTreeNode for VoxelTreeExternalNode {
    type ID = VoxelTreeExternalNodeID;
}

impl VoxelFace {
    fn opposite_face(&self) -> Self {
        match *self {
            Self::LowerX => Self::UpperX,
            Self::UpperX => Self::LowerX,
            Self::LowerY => Self::UpperY,
            Self::UpperY => Self::LowerY,
            Self::LowerZ => Self::UpperZ,
            Self::UpperZ => Self::LowerZ,
        }
    }
}

impl VoxelTreeBuildNode {
    fn build<F, G>(
        internal_nodes: &mut VoxelTreeInternalNodeStorage<F>,
        external_nodes: &mut VoxelTreeExternalNodeStorage,
        generator: &G,
        current_indices: VoxelTreeIndices,
    ) -> Option<Self>
    where
        F: Float,
        G: VoxelGenerator<F>,
    {
        if current_indices.are_at_max_depth() {
            VoxelTreeExternalBuildNode::from_generator(generator, current_indices)
                .map(Self::External)
        } else {
            let mut has_children = false;
            let mut has_common_child_voxel_type = true;
            let mut common_child_voxel_type = None;

            let children = current_indices.for_next_depth().map(|next_indices| {
                let child = Self::build(internal_nodes, external_nodes, generator, next_indices);

                Self::check_child(
                    child.as_ref(),
                    &mut has_children,
                    &mut has_common_child_voxel_type,
                    &mut common_child_voxel_type,
                );

                child
            });

            if has_children {
                Some(match common_child_voxel_type {
                    Some(common_child_voxel_type) if has_common_child_voxel_type => Self::External(
                        VoxelTreeExternalBuildNode::new(current_indices, common_child_voxel_type),
                    ),
                    _ => {
                        let child_ids = children
                            .map(|child| VoxelTreeNodeID::from_build_node(external_nodes, child));

                        let id = internal_nodes.add_node(VoxelTreeInternalNode::new(child_ids));

                        Self::Internal(id)
                    }
                })
            } else {
                None
            }
        }
    }

    fn check_child(
        child: Option<&Self>,
        has_children: &mut bool,
        has_common_child_voxel_type: &mut bool,
        common_child_voxel_type: &mut Option<VoxelType>,
    ) {
        match child {
            None => {
                *has_common_child_voxel_type = false;
            }
            Some(Self::External(child)) if *has_common_child_voxel_type => {
                *has_children = true;

                if let Some(common_child_voxel_type) = *common_child_voxel_type {
                    *has_common_child_voxel_type = child.voxel_type == common_child_voxel_type;
                } else {
                    *common_child_voxel_type = Some(child.voxel_type);
                }
            }
            _ => {
                *has_children = true;
                *has_common_child_voxel_type = false;
            }
        }
    }
}

impl VoxelTreeExternalBuildNode {
    fn new(indices: VoxelTreeIndices, voxel_type: VoxelType) -> Self {
        Self {
            indices,
            voxel_type,
        }
    }

    fn from_generator<F, G>(generator: &G, indices: VoxelTreeIndices) -> Option<Self>
    where
        F: Float,
        G: VoxelGenerator<F>,
    {
        generator
            .voxel_at_indices(indices.i, indices.j, indices.k)
            .map(|voxel_type| Self {
                indices,
                voxel_type,
            })
    }
}

impl VoxelTreeIndices {
    fn new(tree_height: VoxelTreeHeight, depth: u32, i: usize, j: usize, k: usize) -> Self {
        assert!(tree_height.depth_is_valid(depth));
        Self {
            tree_height,
            depth,
            i,
            j,
            k,
        }
    }

    fn at_root(tree_height: VoxelTreeHeight) -> Self {
        Self::new(tree_height, 0, 0, 0, 0)
    }

    fn at_max_depth(tree_height: VoxelTreeHeight, i: usize, j: usize, k: usize) -> Self {
        Self::new(tree_height, tree_height.value(), i, j, k)
    }

    fn tree_height(&self) -> VoxelTreeHeight {
        self.tree_height
    }

    fn depth(&self) -> u32 {
        self.depth
    }

    fn are_at_max_depth(&self) -> bool {
        self.tree_height.depth_is_max(self.depth)
    }

    fn for_next_depth(&self) -> [Self; 8] {
        let next_depth = self.depth + 1;
        assert!(self.tree_height.depth_is_valid(next_depth));

        let i0 = 2 * self.i;
        let i1 = i0 + 1;
        let j0 = 2 * self.j;
        let j1 = j0 + 1;
        let k0 = 2 * self.k;
        let k1 = k0 + 1;
        [
            self.for_child(next_depth, i0, j0, k0),
            self.for_child(next_depth, i0, j0, k1),
            self.for_child(next_depth, i0, j1, k0),
            self.for_child(next_depth, i0, j1, k1),
            self.for_child(next_depth, i1, j0, k0),
            self.for_child(next_depth, i1, j0, k1),
            self.for_child(next_depth, i1, j1, k0),
            self.for_child(next_depth, i1, j1, k1),
        ]
    }

    fn for_child(&self, next_depth: u32, i: usize, j: usize, k: usize) -> Self {
        Self::new(self.tree_height, next_depth, i, j, k)
    }

    fn voxel_scale(&self) -> u32 {
        self.tree_height.voxel_scale_at_depth(self.depth)
    }

    fn voxel_scale_and_indices(&self) -> (u32, VoxelIndices) {
        let voxel_scale = self.voxel_scale();
        let voxel_scale_usize = voxel_scale as usize;
        let voxel_indices = VoxelIndices::new(
            self.i * voxel_scale_usize,
            self.j * voxel_scale_usize,
            self.k * voxel_scale_usize,
        );
        (voxel_scale, voxel_indices)
    }

    fn voxel_scale_and_center_offset<F: Float>(&self, voxel_extent: F) -> (F, Vector3<F>) {
        let voxel_scale = F::from_u32(self.voxel_scale()).unwrap();

        let scaled_voxel_extent = voxel_extent * voxel_scale;
        let half_scaled_voxel_extent = F::ONE_HALF * scaled_voxel_extent;

        let voxel_center_offset = vector![
            F::from_usize(self.i).unwrap() * scaled_voxel_extent + half_scaled_voxel_extent,
            F::from_usize(self.j).unwrap() * scaled_voxel_extent + half_scaled_voxel_extent,
            F::from_usize(self.k).unwrap() * scaled_voxel_extent + half_scaled_voxel_extent
        ];

        (voxel_scale, voxel_center_offset)
    }

    fn voxel_lower_and_upper_corner<F: Float>(&self, voxel_extent: F) -> (Point3<F>, Point3<F>) {
        let voxel_scale = F::from_u32(self.voxel_scale()).unwrap();
        let scaled_voxel_extent = voxel_extent * voxel_scale;

        let lower_corner = point![
            F::from_usize(self.i).unwrap() * scaled_voxel_extent,
            F::from_usize(self.j).unwrap() * scaled_voxel_extent,
            F::from_usize(self.k).unwrap() * scaled_voxel_extent
        ];

        let upper_corner = point![
            lower_corner.x + scaled_voxel_extent,
            lower_corner.y + scaled_voxel_extent,
            lower_corner.z + scaled_voxel_extent
        ];

        (lower_corner, upper_corner)
    }
}

impl VoxelIndices {
    fn new(i: usize, j: usize, k: usize) -> Self {
        Self { i, j, k }
    }

    fn are_inside_grid(&self, grid_size: usize) -> bool {
        self.i < grid_size && self.j < grid_size && self.k < grid_size
    }

    fn octants(self, tree_height: u32) -> Option<impl Iterator<Item = Octant>> {
        OctantIterator::new(tree_height, self)
    }
}

impl OctantIterator {
    fn new(tree_height: u32, indices: VoxelIndices) -> Option<Self> {
        let grid_size = VoxelTreeHeight::grid_size_at_depth(tree_height);

        if indices.are_inside_grid(grid_size) {
            let octant_size = grid_size / 2;
            Some(Self {
                indices,
                octant_size,
                dividing_i: octant_size,
                dividing_j: octant_size,
                dividing_k: octant_size,
            })
        } else {
            None
        }
    }
}

impl Iterator for OctantIterator {
    type Item = Octant;

    fn next(&mut self) -> Option<Self::Item> {
        if self.octant_size < 1 {
            return None;
        }

        self.octant_size /= 2;

        let to_left = if self.indices.i < self.dividing_i {
            self.dividing_i -= self.octant_size;
            true
        } else {
            self.dividing_i += self.octant_size;
            false
        };
        let at_bottom = if self.indices.j < self.dividing_j {
            self.dividing_j -= self.octant_size;
            true
        } else {
            self.dividing_j += self.octant_size;
            false
        };
        let in_back = if self.indices.k < self.dividing_k {
            self.dividing_k -= self.octant_size;
            true
        } else {
            self.dividing_k += self.octant_size;
            false
        };

        let octant = match (to_left, at_bottom, in_back) {
            (true, true, true) => Octant::BackBottomLeft,
            (true, true, false) => Octant::FrontBottomLeft,
            (true, false, true) => Octant::BackTopLeft,
            (true, false, false) => Octant::FrontTopLeft,
            (false, true, true) => Octant::BackBottomRight,
            (false, true, false) => Octant::FrontBottomRight,
            (false, false, true) => Octant::BackTopRight,
            (false, false, false) => Octant::FrontTopRight,
        };

        Some(octant)
    }
}

impl Octant {
    fn idx(&self) -> usize {
        *self as usize
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::geometry::AxisAlignedBox;
    use approx::{abs_diff_eq, assert_abs_diff_eq};
    use nalgebra::{point, Point3, UnitQuaternion};
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

    struct ManualVoxelGenerator<const N: usize> {
        voxels: [[[u8; N]; N]; N],
    }

    impl VoxelGenerator<f64> for EmptyVoxelGenerator {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            self.shape
        }

        fn voxel_at_indices(&self, _i: usize, _j: usize, _k: usize) -> Option<VoxelType> {
            None
        }
    }

    impl VoxelGenerator<f64> for DefaultVoxelGenerator {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            self.shape
        }

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
            if i < self.shape[0] && j < self.shape[1] && k < self.shape[2] {
                Some(VoxelType::Default)
            } else {
                None
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

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
            self.call_counts
                .lock()
                .unwrap()
                .entry((i, j, k))
                .and_modify(|count| *count += 1)
                .or_insert(1);

            if i < self.shape[0] && j < self.shape[1] && k < self.shape[2] {
                Some(VoxelType::Default)
            } else {
                None
            }
        }
    }

    impl<const N: usize> VoxelGenerator<f64> for ManualVoxelGenerator<N> {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            [N; 3]
        }

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
            if i < N && j < N && k < N && self.voxels[i][j][k] != 0 {
                Some(VoxelType::Default)
            } else {
                None
            }
        }
    }

    #[test]
    fn should_get_no_tree_from_empty_voxel_generator() {
        let generator = EmptyVoxelGenerator { shape: [0; 3] };
        assert!(VoxelTree::build(&generator).is_none());
    }

    #[test]
    fn should_get_no_tree_for_zero_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [0; 3] });
        assert!(tree.is_none());
    }

    #[test]
    fn should_get_voxel_extent_of_generator() {
        let generator = DefaultVoxelGenerator { shape: [1; 3] };
        let tree = VoxelTree::build(&generator).unwrap();
        assert_eq!(tree.voxel_extent(), generator.voxel_extent());
    }

    #[test]
    fn should_build_tree_with_grid_size_one_for_single_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1; 3] }).unwrap();
        assert_eq!(tree.tree_height(), 0);
        assert_eq!(tree.grid_size(), 1);
    }

    #[test]
    fn should_build_tree_with_grid_size_two_for_two_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [2, 1, 1] }).unwrap();
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);

        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1, 2, 1] }).unwrap();
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);

        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1, 1, 2] }).unwrap();
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);
    }

    #[test]
    fn should_build_tree_with_grid_size_four_for_three_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [3, 1, 1] }).unwrap();
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);

        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1, 3, 1] }).unwrap();
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);

        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1, 1, 3] }).unwrap();
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);
    }

    #[test]
    fn should_query_one_voxel_generator_once() {
        let generator = RecordingVoxelGenerator::new([1; 3]);
        VoxelTree::build(&generator).unwrap();
        assert_eq!(generator.n_unique_queries(), 1);
    }

    #[test]
    fn should_perform_8_unique_queries_on_two_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([2, 1, 1]);
        VoxelTree::build(&generator).unwrap();
        assert_eq!(generator.n_unique_queries(), 8);
    }

    #[test]
    fn should_perform_64_unique_queries_on_three_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([3, 1, 1]);
        VoxelTree::build(&generator).unwrap();
        assert_eq!(generator.n_unique_queries(), 64);
    }

    #[test]
    fn should_not_query_same_indices_twice_for_one_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([1; 3]);
        VoxelTree::build(&generator).unwrap();
        assert!(generator.count_is_one_for_all_queries());
    }

    #[test]
    fn should_not_query_same_indices_twice_for_two_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([2, 1, 1]);
        VoxelTree::build(&generator).unwrap();
        assert!(generator.count_is_one_for_all_queries());
    }

    #[test]
    fn should_not_query_same_indices_twice_for_three_voxel_generator() {
        let generator = RecordingVoxelGenerator::new([3, 1, 1]);
        VoxelTree::build(&generator).unwrap();
        assert!(generator.count_is_one_for_all_queries());
    }

    #[test]
    fn should_have_external_root_node_for_single_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1; 3] }).unwrap();
        assert!(tree.root_node_id().is_external());
    }

    #[test]
    fn should_have_default_external_root_node_for_single_default_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1; 3] }).unwrap();
        let root_node = tree.external_node(tree.root_node_id().as_external().unwrap());
        assert_eq!(root_node.voxel_type, VoxelType::Default);
    }

    #[test]
    fn should_have_internal_root_node_with_two_external_children_for_two_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [2, 1, 1] }).unwrap();
        let root_node = tree.internal_node(tree.root_node_id().as_internal().unwrap());
        assert_eq!(root_node.n_children(), 2);
        assert_eq!(root_node.n_external_children(), 2);
        assert_eq!(root_node.n_internal_children(), 0);
    }

    #[test]
    fn should_have_default_external_root_node_for_8_default_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [2; 3] }).unwrap();
        let root_node = tree.external_node(tree.root_node_id().as_external().unwrap());
        assert_eq!(root_node.voxel_type, VoxelType::Default);
    }

    #[test]
    fn should_have_internal_root_node_with_correct_internal_and_external_children_for_12_voxel_generator(
    ) {
        let tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [2, 2, 3] }).unwrap();
        let root_node = tree.internal_node(tree.root_node_id().as_internal().unwrap());
        assert_eq!(root_node.n_children(), 2);
        assert_eq!(root_node.n_external_children(), 1);
        assert_eq!(root_node.n_internal_children(), 1);
        let internal_child = tree.internal_node(root_node.internal_child_ids().next().unwrap());
        assert_eq!(internal_child.n_children(), 4);
        assert_eq!(internal_child.n_external_children(), 4);
        assert_eq!(internal_child.n_internal_children(), 0);
    }

    #[test]
    fn should_have_correct_aabb_in_two_voxel_tree() {
        let generator = DefaultVoxelGenerator { shape: [1, 2, 1] };
        let tree = VoxelTree::build(&generator).unwrap();

        let root_node = tree.internal_node(tree.root_node_id().as_internal().unwrap());

        let root_aabb = AxisAlignedBox::new(
            Point3::origin(),
            point![
                generator.voxel_extent(),
                2.0 * generator.voxel_extent(),
                generator.voxel_extent()
            ],
        );
        assert_abs_diff_eq!(root_node.aabb(), &root_aabb);
    }

    #[test]
    fn should_have_correct_aabbs_in_three_voxel_tree() {
        let generator = DefaultVoxelGenerator { shape: [1, 1, 3] };
        let tree = VoxelTree::build(&generator).unwrap();

        let internal_child = |parent_node: &VoxelTreeInternalNode<f64>, octant| {
            tree.internal_node(
                parent_node.child_ids[octant as usize]
                    .unwrap()
                    .as_internal()
                    .unwrap(),
            )
        };

        let check_aabb = |node: &VoxelTreeInternalNode<f64>,
                          [l0, l1, l2]: [u32; 3],
                          [u0, u1, u2]: [u32; 3]| {
            let child_aabb = AxisAlignedBox::new(
                Point3::from(
                    vector![f64::from(l0), f64::from(l1), f64::from(l2)] * generator.voxel_extent(),
                ),
                Point3::from(
                    vector![f64::from(u0), f64::from(u1), f64::from(u2)] * generator.voxel_extent(),
                ),
            );
            assert_abs_diff_eq!(node.aabb(), &child_aabb);
        };

        let root_node = tree.internal_node(tree.root_node_id().as_internal().unwrap());

        check_aabb(root_node, [0, 0, 0], [1, 1, 3]);

        check_aabb(
            internal_child(root_node, Octant::BackBottomLeft),
            [0, 0, 0],
            [1, 1, 2],
        );

        check_aabb(
            internal_child(root_node, Octant::FrontBottomLeft),
            [0, 0, 2],
            [1, 1, 3],
        );
    }

    #[test]
    fn should_have_correct_aabbs_in_three_by_two_by_five_voxel_tree() {
        let generator = DefaultVoxelGenerator { shape: [3, 2, 5] };
        let tree = VoxelTree::build(&generator).unwrap();

        let internal_child = |parent_node: &VoxelTreeInternalNode<f64>, octant| {
            tree.internal_node(
                parent_node.child_ids[octant as usize]
                    .unwrap()
                    .as_internal()
                    .unwrap(),
            )
        };

        let check_aabb = |node: &VoxelTreeInternalNode<f64>,
                          [l0, l1, l2]: [u32; 3],
                          [u0, u1, u2]: [u32; 3]| {
            let child_aabb = AxisAlignedBox::new(
                Point3::from(
                    vector![f64::from(l0), f64::from(l1), f64::from(l2)] * generator.voxel_extent(),
                ),
                Point3::from(
                    vector![f64::from(u0), f64::from(u1), f64::from(u2)] * generator.voxel_extent(),
                ),
            );
            assert_abs_diff_eq!(node.aabb(), &child_aabb);
        };

        let root_node = tree.internal_node(tree.root_node_id().as_internal().unwrap());

        check_aabb(root_node, [0, 0, 0], [3, 2, 5]);

        check_aabb(
            internal_child(root_node, Octant::BackBottomLeft),
            [0, 0, 0],
            [3, 2, 4],
        );

        check_aabb(
            internal_child(root_node, Octant::FrontBottomLeft),
            [0, 0, 4],
            [3, 2, 5],
        );

        check_aabb(
            internal_child(
                internal_child(root_node, Octant::BackBottomLeft),
                Octant::BackBottomRight,
            ),
            [2, 0, 0],
            [3, 2, 2],
        );

        check_aabb(
            internal_child(
                internal_child(root_node, Octant::BackBottomLeft),
                Octant::FrontBottomRight,
            ),
            [2, 0, 2],
            [3, 2, 4],
        );

        check_aabb(
            internal_child(
                internal_child(root_node, Octant::FrontBottomLeft),
                Octant::BackBottomLeft,
            ),
            [0, 0, 4],
            [2, 2, 5],
        );

        check_aabb(
            internal_child(
                internal_child(root_node, Octant::FrontBottomLeft),
                Octant::BackBottomRight,
            ),
            [2, 0, 4],
            [3, 2, 5],
        );
    }

    #[test]
    fn should_compute_valid_bounding_sphere_for_single_voxel_tree() {
        let generator = DefaultVoxelGenerator { shape: [1; 3] };
        let tree = VoxelTree::build(&generator).unwrap();

        let aabb = AxisAlignedBox::new(
            Point3::origin(),
            point![
                generator.voxel_extent(),
                generator.voxel_extent(),
                generator.voxel_extent()
            ],
        );

        let bounding_sphere = tree.compute_bounding_sphere(0);
        let bumped_bounding_sphere =
            Sphere::new(*bounding_sphere.center(), bounding_sphere.radius() + 1e-9);

        assert!(bumped_bounding_sphere.contains_axis_aligned_box(&aabb));
    }

    #[test]
    fn should_compute_valid_bounding_sphere_for_two_voxel_tree() {
        let generator = DefaultVoxelGenerator { shape: [1, 2, 1] };
        let tree = VoxelTree::build(&generator).unwrap();

        let aabb = AxisAlignedBox::new(
            Point3::origin(),
            point![
                generator.voxel_extent(),
                2.0 * generator.voxel_extent(),
                generator.voxel_extent()
            ],
        );

        for max_depth in 0..tree.tree_height() {
            let bounding_sphere = tree.compute_bounding_sphere(max_depth);
            let bumped_bounding_sphere =
                Sphere::new(*bounding_sphere.center(), bounding_sphere.radius() + 1e-9);

            assert!(bumped_bounding_sphere.contains_axis_aligned_box(&aabb));
        }
    }

    #[test]
    fn should_compute_valid_bounding_sphere_for_five_voxel_tree() {
        let generator = DefaultVoxelGenerator { shape: [1, 1, 5] };
        let tree = VoxelTree::build(&generator).unwrap();

        let aabb = AxisAlignedBox::new(
            Point3::origin(),
            point![
                generator.voxel_extent(),
                generator.voxel_extent(),
                5.0 * generator.voxel_extent()
            ],
        );

        for max_depth in 0..tree.tree_height() {
            let bounding_sphere = tree.compute_bounding_sphere(max_depth);
            let bumped_bounding_sphere =
                Sphere::new(*bounding_sphere.center(), bounding_sphere.radius() + 1e-9);

            assert!(bumped_bounding_sphere.contains_axis_aligned_box(&aabb));
        }
    }

    #[test]
    fn should_compute_valid_bounding_sphere_for_16_voxel_tree() {
        let generator = DefaultVoxelGenerator { shape: [4, 2, 2] };
        let tree = VoxelTree::build(&generator).unwrap();

        let aabb = AxisAlignedBox::new(
            Point3::origin(),
            point![
                4.0 * generator.voxel_extent(),
                2.0 * generator.voxel_extent(),
                2.0 * generator.voxel_extent()
            ],
        );

        for max_depth in 0..tree.tree_height() {
            let bounding_sphere = tree.compute_bounding_sphere(max_depth);
            let bumped_bounding_sphere =
                Sphere::new(*bounding_sphere.center(), bounding_sphere.radius() + 1e-9);

            assert!(bumped_bounding_sphere.contains_axis_aligned_box(&aabb));
        }
    }

    #[test]
    fn should_compute_tighter_bounding_sphere_for_higher_max_depth() {
        let generator = DefaultVoxelGenerator { shape: [5, 1, 1] };
        let tree = VoxelTree::build(&generator).unwrap();

        let bounding_sphere_0 = tree.compute_bounding_sphere(0);
        let bounding_sphere_1 = tree.compute_bounding_sphere(1);
        let bounding_sphere_2 = tree.compute_bounding_sphere(2);
        let bounding_sphere_3 = tree.compute_bounding_sphere(3);

        assert!(bounding_sphere_0.radius() > bounding_sphere_1.radius());
        assert!(bounding_sphere_1.radius() > bounding_sphere_2.radius());
        assert!(bounding_sphere_2.radius() > bounding_sphere_3.radius());
    }

    #[test]
    fn should_compute_correct_transform_for_single_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [1; 3] };
        let tree = VoxelTree::build(&generator).unwrap();
        let transforms = tree.compute_voxel_transforms();

        assert_eq!(transforms.len(), 1);
        let transform = &transforms[0];

        let half_voxel_extent = 0.5 * generator.voxel_extent();
        let correct_translation = vector![half_voxel_extent, half_voxel_extent, half_voxel_extent];
        assert_abs_diff_eq!(transform.translation(), &correct_translation);
        assert_abs_diff_eq!(transform.scaling(), 1.0);
    }

    #[test]
    fn should_compute_correct_transform_for_8_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [2; 3] };
        let tree = VoxelTree::build(&generator).unwrap();
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
    fn should_compute_correct_transform_for_64_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [4; 3] };
        let tree = VoxelTree::build(&generator).unwrap();
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
    fn should_compute_correct_transforms_for_12_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [2, 2, 3] };
        let tree = VoxelTree::build(&generator).unwrap();
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

    #[test]
    fn should_get_no_octant_iterator_for_indices_outside_voxel_grid() {
        assert!(VoxelIndices::new(0, 0, 1).octants(0).is_none());
        assert!(VoxelIndices::new(0, 1, 0).octants(0).is_none());
        assert!(VoxelIndices::new(1, 0, 0).octants(0).is_none());
        assert!(VoxelIndices::new(0, 0, 2).octants(1).is_none());
        assert!(VoxelIndices::new(0, 2, 0).octants(1).is_none());
        assert!(VoxelIndices::new(2, 0, 0).octants(1).is_none());
    }

    #[test]
    fn should_get_empty_octant_iterator_for_height_zero_tree() {
        let octants = VoxelIndices::new(0, 0, 0).octants(0);
        assert!(octants.is_some());
        assert!(octants.unwrap().next().is_none());
    }

    #[test]
    fn should_get_correct_single_octant_iterators_for_height_one_tree() {
        let check_octant = |i, j, k, octant| {
            assert_eq!(
                VoxelIndices::new(i, j, k)
                    .octants(1)
                    .unwrap()
                    .collect::<Vec<_>>(),
                vec![octant]
            );
        };

        check_octant(0, 0, 0, Octant::BackBottomLeft);
        check_octant(0, 0, 1, Octant::FrontBottomLeft);
        check_octant(0, 1, 0, Octant::BackTopLeft);
        check_octant(0, 1, 1, Octant::FrontTopLeft);
        check_octant(1, 0, 0, Octant::BackBottomRight);
        check_octant(1, 0, 1, Octant::FrontBottomRight);
        check_octant(1, 1, 0, Octant::BackTopRight);
        check_octant(1, 1, 1, Octant::FrontTopRight);
    }

    #[test]
    fn should_get_correct_octant_iterators_for_height_two_tree() {
        use Octant::{
            BackBottomLeft as BBL, BackBottomRight as BBR, BackTopLeft as BTL, BackTopRight as BTR,
            FrontBottomLeft as FBL, FrontBottomRight as FBR, FrontTopLeft as FTL,
            FrontTopRight as FTR,
        };

        let check_octants = |i, j, k, octants: [Octant; 2]| {
            assert_eq!(
                VoxelIndices::new(i, j, k)
                    .octants(2)
                    .unwrap()
                    .collect::<Vec<_>>(),
                octants.to_vec(),
            );
        };

        let check_octants_for_offset = |i_offset, j_offset, k_offset, first_octant| {
            check_octants(i_offset, j_offset, k_offset, [first_octant, BBL]);
            check_octants(i_offset, j_offset, k_offset + 1, [first_octant, FBL]);
            check_octants(i_offset, j_offset + 1, k_offset, [first_octant, BTL]);
            check_octants(i_offset, j_offset + 1, k_offset + 1, [first_octant, FTL]);
            check_octants(i_offset + 1, j_offset, k_offset, [first_octant, BBR]);
            check_octants(i_offset + 1, j_offset, k_offset + 1, [first_octant, FBR]);
            check_octants(i_offset + 1, j_offset + 1, k_offset, [first_octant, BTR]);
            check_octants(
                i_offset + 1,
                j_offset + 1,
                k_offset + 1,
                [first_octant, FTR],
            );
        };

        check_octants_for_offset(0, 0, 0, BBL);
        check_octants_for_offset(0, 0, 2, FBL);
        check_octants_for_offset(0, 2, 0, BTL);
        check_octants_for_offset(0, 2, 2, FTL);
        check_octants_for_offset(2, 0, 0, BBR);
        check_octants_for_offset(2, 0, 2, FBR);
        check_octants_for_offset(2, 2, 0, BTR);
        check_octants_for_offset(2, 2, 2, FTR);
    }

    #[test]
    fn should_find_no_voxel_at_indices_outside_grid() {
        let generator = DefaultVoxelGenerator { shape: [1; 3] };
        let tree = VoxelTree::build(&generator).unwrap();
        assert!(tree.find_voxel_at_indices(1, 0, 0).is_none());
        assert!(tree.find_voxel_at_indices(0, 1, 0).is_none());
        assert!(tree.find_voxel_at_indices(0, 0, 1).is_none());
    }

    #[test]
    fn should_find_root_voxel_at_zero_indices_for_single_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [1; 3] };
        let tree = VoxelTree::build(&generator).unwrap();
        assert_eq!(
            tree.find_voxel_at_indices(0, 0, 0).unwrap(),
            VoxelType::Default
        );
    }

    #[test]
    fn should_find_same_voxel_types_as_provided_by_generator() {
        let generator = DefaultVoxelGenerator { shape: [1, 3, 2] };
        let tree = VoxelTree::build(&generator).unwrap();

        for i in 0..tree.grid_size() {
            for j in 0..tree.grid_size() {
                for k in 0..tree.grid_size() {
                    assert_eq!(
                        tree.find_voxel_at_indices(i, j, k),
                        generator.voxel_at_indices(i, j, k)
                    );
                }
            }
        }
    }

    #[test]
    fn should_obtain_correct_external_node_indices_and_scales_for_12_voxel_generator() {
        let generator = DefaultVoxelGenerator { shape: [2, 2, 3] };
        let tree = VoxelTree::build(&generator).unwrap();

        let check_node = |i, j, k, indices, scale| {
            let node = tree.find_external_node_at_indices(i, j, k).unwrap();
            assert_eq!(node.voxel_indices, indices);
            assert_eq!(node.voxel_scale, scale);
        };

        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    check_node(i, j, k, VoxelIndices::new(0, 0, 0), 2);
                }
            }
        }
        check_node(0, 0, 2, VoxelIndices::new(0, 0, 2), 1);
        check_node(0, 1, 2, VoxelIndices::new(0, 1, 2), 1);
        check_node(1, 0, 2, VoxelIndices::new(1, 0, 2), 1);
        check_node(1, 1, 2, VoxelIndices::new(1, 1, 2), 1);
    }

    #[test]
    fn should_get_no_adjacent_voxels_for_single_voxel_tree() {
        let mut tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1; 3] }).unwrap();
        tree.update_adjacent_voxels_for_all_external_nodes();
        let node = tree.find_external_node_at_indices(0, 0, 0).unwrap();
        assert!(node.adjacent_voxels.is_empty());
        assert!(node.has_only_fully_exposed_faces());
    }

    #[test]
    fn should_get_no_adjacent_voxels_for_single_merged_voxel_tree() {
        let mut tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [4; 3] }).unwrap();
        tree.update_adjacent_voxels_for_all_external_nodes();
        let node = tree.find_external_node_at_indices(2, 2, 2).unwrap();
        assert!(node.adjacent_voxels.is_empty());
        assert!(node.has_only_fully_exposed_faces());
    }

    #[test]
    fn should_get_correct_voxels_adjacent_to_unmerged_voxel_in_four_voxel_tree() {
        let mut tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [1, 2, 2] }).unwrap();
        tree.update_adjacent_voxels_for_all_external_nodes();
        let node = tree.find_external_node_at_indices(0, 1, 1).unwrap();

        let check_neighbor_present = |i, j, k| {
            assert!(node
                .adjacent_voxels
                .iter()
                .any(|(adjacent_voxel_indices, _)| adjacent_voxel_indices
                    == &VoxelIndices::new(i, j, k)));
        };

        assert_eq!(node.adjacent_voxels.len(), 2);
        check_neighbor_present(0, 1, 0);
        check_neighbor_present(0, 0, 1);
        assert!(node.face_is_fully_exposed(VoxelFace::LowerX));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperX));
        assert!(node.face_is_fully_obscured(VoxelFace::LowerY));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperY));
        assert!(node.face_is_fully_obscured(VoxelFace::LowerZ));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperZ));
    }

    #[test]
    fn should_get_correct_voxels_adjacent_to_unmerged_voxel_in_9_voxel_tree() {
        let mut tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [3; 3] }).unwrap();
        tree.update_adjacent_voxels_for_all_external_nodes();
        let node = tree.find_external_node_at_indices(2, 1, 1).unwrap();

        let check_neighbor_present = |i, j, k| {
            assert!(node
                .adjacent_voxels
                .iter()
                .any(|(adjacent_voxel_indices, _)| adjacent_voxel_indices
                    == &VoxelIndices::new(i, j, k)));
        };

        assert_eq!(node.adjacent_voxels.len(), 5);
        check_neighbor_present(0, 0, 0);
        check_neighbor_present(2, 0, 1);
        check_neighbor_present(2, 2, 1);
        check_neighbor_present(2, 1, 0);
        check_neighbor_present(2, 1, 2);
        assert!(node.face_is_fully_obscured(VoxelFace::LowerX));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperX));
        assert!(node.face_is_fully_obscured(VoxelFace::LowerY));
        assert!(node.face_is_fully_obscured(VoxelFace::UpperY));
        assert!(node.face_is_fully_obscured(VoxelFace::LowerZ));
        assert!(node.face_is_fully_obscured(VoxelFace::UpperZ));
    }

    #[test]
    fn should_get_correct_voxels_adjacent_to_merged_voxel_in_9_voxel_tree() {
        let mut tree = VoxelTree::build(&DefaultVoxelGenerator { shape: [3; 3] }).unwrap();
        tree.update_adjacent_voxels_for_all_external_nodes();
        let node = tree.find_external_node_at_indices(0, 0, 0).unwrap();

        let check_neighbor_present = |i, j, k| {
            assert!(node
                .adjacent_voxels
                .iter()
                .any(|(adjacent_voxel_indices, _)| adjacent_voxel_indices
                    == &VoxelIndices::new(i, j, k)));
        };

        assert_eq!(node.adjacent_voxels.len(), 12);
        check_neighbor_present(2, 0, 0);
        check_neighbor_present(2, 0, 1);
        check_neighbor_present(2, 1, 0);
        check_neighbor_present(2, 1, 1);
        check_neighbor_present(0, 2, 0);
        check_neighbor_present(0, 2, 1);
        check_neighbor_present(1, 2, 0);
        check_neighbor_present(1, 2, 1);
        check_neighbor_present(0, 0, 2);
        check_neighbor_present(0, 1, 2);
        check_neighbor_present(1, 0, 2);
        check_neighbor_present(1, 1, 2);
        assert!(node.face_is_fully_exposed(VoxelFace::LowerX));
        assert!(node.face_is_fully_obscured(VoxelFace::UpperX));
        assert!(node.face_is_fully_exposed(VoxelFace::LowerY));
        assert!(node.face_is_fully_obscured(VoxelFace::UpperY));
        assert!(node.face_is_fully_exposed(VoxelFace::LowerZ));
        assert!(node.face_is_fully_obscured(VoxelFace::UpperZ));
    }

    #[test]
    fn should_get_correct_voxels_adjacent_to_merged_voxel_in_complex_voxel_tree() {
        let empty = [0; 8];
        let empty_by_empty = [empty; 8];
        let four = [1, 1, 1, 1, 0, 0, 0, 0];
        let four_by_four = [four, four, four, four, empty, empty, empty, empty];

        let voxels = [
            four_by_four,
            four_by_four,
            four_by_four,
            four_by_four,
            [
                [1, 1, 0, 1, 0, 0, 0, 0],
                [1, 1, 1, 0, 0, 0, 0, 0],
                [0, 0, 1, 1, 0, 0, 0, 0],
                [1, 0, 1, 1, 0, 0, 0, 0],
                empty,
                empty,
                empty,
                empty,
            ],
            [
                [1, 1, 0, 0, 0, 0, 0, 0],
                [1, 1, 0, 0, 0, 0, 0, 0],
                [0, 0, 1, 1, 0, 0, 0, 0],
                [0, 0, 1, 1, 0, 0, 0, 0],
                empty,
                empty,
                empty,
                empty,
            ],
            empty_by_empty,
            empty_by_empty,
        ];

        let generator = ManualVoxelGenerator { voxels };
        let mut tree = VoxelTree::build(&generator).unwrap();
        tree.update_adjacent_voxels_for_all_external_nodes();

        let check_neighbor_present = |node: &VoxelTreeExternalNode, i, j, k| {
            assert!(node
                .adjacent_voxels
                .iter()
                .any(|(adjacent_voxel_indices, _)| adjacent_voxel_indices
                    == &VoxelIndices::new(i, j, k)));
        };

        let node = tree.find_external_node_at_indices(0, 0, 0).unwrap();

        assert_eq!(node.adjacent_voxels.len(), 5);
        check_neighbor_present(node, 4, 0, 0);
        check_neighbor_present(node, 4, 0, 3);
        check_neighbor_present(node, 4, 1, 2);
        check_neighbor_present(node, 4, 2, 2);
        check_neighbor_present(node, 4, 3, 0);
        assert!(node.face_is_fully_exposed(VoxelFace::LowerX));
        assert_eq!(node.exposed_face_area(VoxelFace::UpperX), 5);
        assert!(node.face_is_fully_exposed(VoxelFace::LowerY));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperY));
        assert!(node.face_is_fully_exposed(VoxelFace::LowerZ));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperZ));

        let node = tree.find_external_node_at_indices(4, 2, 2).unwrap();

        assert_eq!(node.adjacent_voxels.len(), 2);
        check_neighbor_present(node, 0, 0, 0);
        check_neighbor_present(node, 4, 1, 2);
        assert!(node.face_is_fully_obscured(VoxelFace::LowerX));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperX));
        assert_eq!(node.exposed_face_area(VoxelFace::LowerY), 3);
        assert!(node.face_is_fully_exposed(VoxelFace::UpperY));
        assert!(node.face_is_fully_exposed(VoxelFace::LowerZ));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperZ));

        let node = tree.find_external_node_at_indices(4, 1, 2).unwrap();

        assert_eq!(node.adjacent_voxels.len(), 3);
        check_neighbor_present(node, 0, 0, 0);
        check_neighbor_present(node, 4, 0, 0);
        check_neighbor_present(node, 4, 2, 2);
        assert!(node.face_is_fully_obscured(VoxelFace::LowerX));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperX));
        assert!(node.face_is_fully_exposed(VoxelFace::LowerY));
        assert!(node.face_is_fully_obscured(VoxelFace::UpperY));
        assert!(node.face_is_fully_obscured(VoxelFace::LowerZ));
        assert!(node.face_is_fully_exposed(VoxelFace::UpperZ));
    }

    #[test]
    fn should_correctly_transform_voxel_transform() {
        let translation = vector![0.1, -0.2, 0.3];
        let scaling = 0.8;

        let voxel_transform = VoxelTransform::new(translation, scaling);

        let voxel_similarity =
            Similarity3::from_parts(translation.into(), UnitQuaternion::identity(), scaling);

        let transform_from_tree_to_camera_space = Similarity3::from_parts(
            vector![-1.2, 9.7, 0.4].into(),
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 1.1),
            2.7,
        );

        let model_view_transform = voxel_transform
            .transform_into_model_view_transform(&transform_from_tree_to_camera_space);

        let correct_model_view_transform = transform_from_tree_to_camera_space * voxel_similarity;

        assert_abs_diff_eq!(
            model_view_transform.translation,
            correct_model_view_transform.isometry.translation.vector
        );
        assert_abs_diff_eq!(
            model_view_transform.rotation,
            correct_model_view_transform.isometry.rotation
        );
        assert_abs_diff_eq!(
            model_view_transform.scaling,
            correct_model_view_transform.scaling()
        );
    }
}
