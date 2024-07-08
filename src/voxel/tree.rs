//! Voxel trees.

use super::{
    CoordinateAxes, GroupedVoxelInstanceStorage, InstanceGroupID, VoxelInstanceID,
    VoxelInstanceStorage, VoxelTransform,
};
use crate::{
    geometry::{Angle, AxisAlignedBox, Frustum, Radians, Sphere},
    gpu::rendering::fre,
    num::Float,
    voxel::{VoxelFace, VoxelGenerator, VoxelType, N_VOXEL_TYPES},
};
use impact_utils::KeyIndexMapper;
use nalgebra::{point, vector, Point3, Similarity3, Vector3};
use nohash_hasher::BuildNoHashHasher;
use num_traits::FromPrimitive;
use simba::scalar::SubsetOf;
use std::{array, num::NonZeroU32};

/// Controller for determining the level of detail at which to render voxels.
#[derive(Clone, Debug)]
pub struct VoxelTreeLODController<F> {
    min_angular_voxel_extent: Radians<F>,
}

/// An octree representation of a grid of voxels.
#[derive(Debug)]
pub struct VoxelTree<F: Float> {
    properties: VoxelTreeProperties<F>,
    root_node: InternalNode<F>,
    external_node_aux_storage: ExternalNodeAuxiliaryStorage,
    voxel_instances: GroupedVoxelInstanceStorage<F>,
}

/// The basic properties of a voxel tree.
#[derive(Clone, Debug)]
struct VoxelTreeProperties<F> {
    voxel_extent: F,
    height: VoxelTreeHeight,
    instance_group_height: u32,
}

/// The total number of levels in a voxel tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct VoxelTreeHeight {
    tree_height: u32,
}

/// Storage for auxiliary data associated with individual external nodes in a
/// [`VoxelTree`].
#[derive(Clone, Debug)]
struct ExternalNodeAuxiliaryStorage {
    data: Vec<ExternalNodeAuxiliaryData>,
    index_map: KeyIndexMapper<usize, BuildNoHashHasher<usize>>,
    id_count: usize,
}

/// An internal node in a voxel tree. It has one child for each non-empty octant
/// of the region of the grid the node covers.
#[derive(Clone, Debug)]
struct InternalNode<F: Float> {
    internal_children: Vec<InternalNode<F>>,
    external_children: Vec<ExternalNode>,
    octant_child_indices: [OctantChildNodeIdx; 8],
    /// ID of the group of [`VoxelInstanceStorage`]s in the
    /// [`GroupedVoxelInstanceStorage`] that the voxel instances for this node's
    /// external children (and this node's instance for LOD) belong to.
    instance_group_id: InstanceGroupID,
    /// ID of the voxel instance representing this internal node, used for
    /// rendering when the desired LOD matches this node's height in the tree.
    voxel_instance_id_for_lod: VoxelInstanceID,
    height: u32,
    voxel_indices: VoxelIndices,
    dominating_voxel_type: Option<VoxelType>,
    aabb: AxisAlignedBox<F>,
    exposed_descendant_count: usize,
    has_exposed_descendants: bool,
    directional_obscuredness: DirectionalObscurednessLookupTable,
}

/// Encodes the type and index of the child node in a specific octant of an
/// internal node.
#[derive(Copy, Clone, Debug)]
enum OctantChildNodeIdx {
    /// The internal node has no child for this octant.
    None,
    /// The internal node has an internal child node for this octant, located at
    /// this index in the `internal_children` vector.
    Internal(usize),
    /// The internal node has an external child node for this octant, located at
    /// this index in the `external_children` vector.
    External(usize),
}

/// An external node in a voxel tree. It represents a voxel, which may either be
/// unmerged (if the node is at the bottom of the tree) or a merged group of
/// adjacent identical voxels (if the node is not at the bottom).
#[derive(Clone, Debug)]
struct ExternalNode {
    node_id: ExternalNodeID,
    #[allow(dead_code)]
    voxel_instance_id: VoxelInstanceID,
    voxel_type: VoxelType,
    voxel_indices: VoxelIndices,
    voxel_scale: u32,
    is_exposed: bool,
    directional_obscuredness: DirectionalObscurednessLookupTable,
}

/// Identifier for an [`ExternalNode`] in a [`VoxelTree`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ExternalNodeID(usize);

/// Auxiliary data for a specific [`ExternalNode`].
#[derive(Clone, Debug)]
struct ExternalNodeAuxiliaryData {
    voxel_indices: VoxelIndices,
    voxel_scale: u32,
    adjacent_voxels: Vec<(VoxelIndices, ExternalNodeID)>,
    exposed_face_areas: [u32; 6],
}

/// Lookup table for whether a voxel or group of voxels is fully obscured by
/// adjacent voxels when viewed from a specific direction. A rectangular region
/// of voxels is guranteed to be obscured from a direction when the three faces
/// of the region visible from the direction are fully covered by adjacent
/// voxels.
#[derive(Copy, Clone, Debug)]
struct DirectionalObscurednessLookupTable {
    is_obscured_from_direction: [[[bool; 2]; 2]; 2],
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
/// of a voxel tree to reach the voxel at a given set of [`VoxelIndices`].
struct OctantIterator {
    indices: VoxelIndices,
    octant_size: usize,
    dividing_i: usize,
    dividing_j: usize,
    dividing_k: usize,
}

/// An octant in a voxel tree. The number associated with each variant is the
/// index of the corresponding child node of an [`InternalNode`].
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

impl<F: Float> VoxelTreeLODController<F> {
    /// Computes the minimum angular voxel extent corresponding to the given
    /// vertical window dimension (in number of pixels), vertical field of view
    /// and minimum number of pixels per voxel.
    pub fn compute_min_angular_voxel_extent<A: Angle<F>>(
        vertical_window_dimension: NonZeroU32,
        vertical_field_of_view: A,
        min_pixels_per_voxel: F,
    ) -> Radians<F> {
        let angular_pixel_extent = vertical_field_of_view.as_radians()
            / F::from_u32(vertical_window_dimension.into()).unwrap();

        let min_angular_voxel_extent = angular_pixel_extent * min_pixels_per_voxel;

        min_angular_voxel_extent
    }

    /// Creates a new LOD controller with the given minimum angular voxel
    /// extent.
    pub fn new<A: Angle<F>>(min_angular_voxel_extent: A) -> Self {
        Self {
            min_angular_voxel_extent: min_angular_voxel_extent.as_radians(),
        }
    }

    /// Scales the minimum angular voxel extent by the given factor. The extent
    /// should be scaled to remain proportional to the field of view and
    /// inversely proportional to the number of pixels across the window.
    pub fn scale_min_angular_voxel_extent(&mut self, scale: F) {
        self.min_angular_voxel_extent = self.min_angular_voxel_extent * scale;
    }

    /// Computes the desired level of detail for the given (unmerged) voxel
    /// extent and distance from the camera.
    pub fn compute_lod_at_distance(&self, voxel_extent: F, distance: F) -> u32 {
        let min_voxel_extent = self.compute_min_voxel_extent_at_distance(distance);
        let min_voxel_scale = min_voxel_extent / voxel_extent;
        assert!(min_voxel_scale >= F::ZERO);

        let lod_voxel_scale = F::floor(min_voxel_scale)
            .to_u32()
            .unwrap()
            .next_power_of_two();

        VoxelTreeHeight::height_with_voxel_scale(lod_voxel_scale)
    }

    pub fn compute_lod_for_tree_space_displacement_and_view_transform(
        &self,
        voxel_tree: &VoxelTree<F>,
        tree_space_voxel_displacement_from_camera: &Vector3<F>,
        view_transform: &Similarity3<F>,
    ) -> u32 {
        self.compute_lod_at_distance(
            voxel_tree.voxel_extent(),
            tree_space_voxel_displacement_from_camera.norm() * view_transform.scaling(),
        )
    }

    fn compute_min_voxel_extent_at_distance(&self, distance: F) -> F {
        distance * self.min_angular_voxel_extent.radians()
    }
}

impl<F: Float> Default for VoxelTreeLODController<F> {
    fn default() -> Self {
        Self::new(Radians(F::ZERO))
    }
}

impl<F: Float> VoxelTree<F> {
    /// Builds a new [`VoxelTree`] using the given [`VoxelGenerator`]. Groups of
    /// eight adjacent voxels of the same type will be recursively merged into
    /// larger voxels. Returns [`None`] if the resulting voxel tree would be
    /// empty or have a height of zero (have a 1 x 1 x 1 voxel grid), in which
    /// case there is no need for a tree.
    pub fn build<G>(generator: &G) -> Option<Self>
    where
        G: VoxelGenerator<F>,
    {
        let properties = VoxelTreeProperties::from_generator(generator);

        let mut external_node_aux_storage = ExternalNodeAuxiliaryStorage::new();
        let mut voxel_instances = GroupedVoxelInstanceStorage::new();

        InternalNode::build_root_node(
            &mut external_node_aux_storage,
            &mut voxel_instances,
            generator,
            properties.tree_height(),
        )
        .map(|root_node| {
            let mut tree = Self {
                properties,
                root_node,
                external_node_aux_storage,
                voxel_instances,
            };

            // The order here is important: we must update adjacent voxels first
            // since this information is used when updating derived node data
            // and creating instances for unexposed voxels
            tree.update_adjacent_voxels_for_all_external_nodes();
            tree.update_derived_node_data();
            tree.store_instances_for_unexposed_voxels();

            tree
        })
    }

    /// Returns the extent of single unmerged voxel in the tree.
    pub fn voxel_extent(&self) -> F {
        self.properties.voxel_extent()
    }

    /// Returns the full height of the tree. The unmerged voxels reside at
    /// height zero. The grid size decreases by half for each successive height,
    /// down to one at the full height of the tree.
    pub fn tree_height(&self) -> u32 {
        self.properties.tree_height().value()
    }

    /// Returns the number of unmerged voxels along one axis of the grid. The
    /// grid size is always a power of two.
    pub fn grid_size(&self) -> usize {
        self.properties.grid_size()
    }

    /// Computes a sphere bounding the entire voxel tree by aggregating the
    /// bounding spheres of nodes down to the given depth.
    ///
    /// # Panics
    /// If `max_depth` exceeds the tree height.
    pub fn compute_bounding_sphere(&self, max_depth: u32) -> Sphere<F> {
        assert!(self.properties.tree_height().depth_is_valid(max_depth));
        self.root_node
            .compute_bounding_sphere(&self.properties, max_depth, 0)
    }

    /// Returns the type of the voxel at the given indices in the voxel grid, or
    /// [`None`] if the voxel is empty or the indices are outside the bounds of
    /// the grid.
    pub fn find_voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
        self.root_node
            .find_external_node_at_indices(self.tree_height(), VoxelIndices::new(i, j, k))
            .map(|external_node| external_node.voxel_type)
    }

    /// Determines the voxels that may be visible based on the given camera view
    /// frustum and passes the objects storing their instance data to the given
    /// `buffer_instances` closure.
    ///
    /// The view frustum is assumed to be represented in the space of the voxel
    /// tree.
    ///
    /// The `direction_to_voxel` closure should return the direction vector from
    /// the camera to the voxel given the voxel position (in tree space). The
    /// vector does not have to be normalized.
    ///
    /// Depending on their requested level of detail, provided by the
    /// `requested_lod` closure (it takes the voxel position and the direction
    /// returned from `direction_to_voxel`), groups of adjacent voxels may be
    /// represented by a single instance.
    pub fn buffer_visible_voxel_instances(
        &self,
        tree_space_view_frustum: &Frustum<F>,
        view_transform: &Similarity3<F>,
        direction_to_voxel: &impl Fn(&Point3<F>) -> Vector3<F>,
        requested_lod: &impl Fn(&Point3<F>, &Vector3<F>) -> u32,
        buffer_instances: &mut impl FnMut(&VoxelInstanceStorage<F>, &CoordinateAxes<F>),
    ) where
        F: SubsetOf<fre>,
    {
        let camera_space_axes_in_tree_space =
            VoxelTransform::compute_camera_space_axes_in_model_space(view_transform);

        let root_voxel_instance_group = self
            .voxel_instances()
            .group(self.root_node().instance_group_id());

        // We buffer all instances in the root group (external nodes at heights
        // from the `instance_group_height` and up) unconditionally, as
        // evaluating their individual visibility creates more work than it
        // saves
        buffer_instances(root_voxel_instance_group, &camera_space_axes_in_tree_space);

        // If we have instance groups in addition to the root group, we traverse
        // the tree and buffer all the instances in each group that may be
        // visible
        if self.properties.has_intermediate_instance_group_height() {
            self.root_node().buffer_instances_for_internal_nodes(
                &self.properties,
                &mut |internal_node: &InternalNode<F>| {
                    if internal_node.has_exposed_descendants() {
                        let direction = direction_to_voxel(&internal_node.aabb().center());
                        !internal_node.is_fully_obscured_from_direction(&direction)
                            && tree_space_view_frustum
                                .could_contain_part_of_axis_aligned_box(internal_node.aabb())
                    } else {
                        false
                    }
                },
                &mut |internal_node: &InternalNode<F>| {
                    if internal_node.has_exposed_descendants() {
                        let position = internal_node.aabb().center();
                        let direction = direction_to_voxel(&position);

                        if !internal_node.is_fully_obscured_from_direction(&direction)
                            && tree_space_view_frustum
                                .could_contain_part_of_axis_aligned_box(internal_node.aabb())
                        {
                            let lod = u32::min(
                                requested_lod(&position, &direction),
                                internal_node.height(),
                            );

                            let voxel_instance_group = self
                                .voxel_instances()
                                .group_at_level_of_detail(internal_node.instance_group_id(), lod);

                            buffer_instances(
                                voxel_instance_group,
                                &camera_space_axes_in_tree_space,
                            );
                        }
                    }
                },
            );
        }
    }

    /// Rebuilds the list of adjacent voxels for every external node in the
    /// tree.
    ///
    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the
    /// auxiliary data storage, which is only valid if no nodes have been
    /// removed from the tree after construction. It also does not remove
    /// previously registered adjacent voxels.
    fn update_adjacent_voxels_for_all_external_nodes(&mut self) {
        for node_idx in 0..self.external_node_aux_storage.n_nodes() {
            self.update_adjacent_voxels_for_external_node(node_idx);
        }
    }

    /// Updates the derived data of every node (not including the lists of
    /// adjacent voxels for external nodes) based on the current state of the
    /// tree. Should be called after
    /// [`Self::update_adjacent_voxels_for_all_external_nodes`].
    fn update_derived_node_data(&mut self) {
        self.root_node
            .update_derived_node_data(&self.properties, &self.external_node_aux_storage);
    }

    /// Computes the transform of each (potentially merged) voxel in the tree
    /// that has at least one face not fully obscured by adjacent voxels and
    /// adds it along with the voxel type in the voxel instance storage under
    /// the external node ID.
    fn store_instances_for_unexposed_voxels(&mut self) {
        self.root_node.add_voxel_instances(
            &mut self.voxel_instances,
            &self.properties,
            &|external_node| external_node.is_exposed(),
            &|internal_node| internal_node.has_exposed_descendants(),
        );
    }

    /// Returns the root node (which is an internal node) of the tree.
    fn root_node(&self) -> &InternalNode<F> {
        &self.root_node
    }

    #[cfg(test)]
    fn external_node_aux_data(&self, id: ExternalNodeID) -> &ExternalNodeAuxiliaryData {
        self.external_node_aux_storage.data(id)
    }

    fn voxel_instances(&self) -> &GroupedVoxelInstanceStorage<F> {
        &self.voxel_instances
    }

    fn find_external_node_at_indices(&self, indices: VoxelIndices) -> Option<&ExternalNode> {
        self.root_node
            .find_external_node_at_indices(self.tree_height(), indices)
    }

    fn find_external_node_id_at_indices(&self, indices: VoxelIndices) -> Option<ExternalNodeID> {
        self.find_external_node_at_indices(indices)
            .map(|external_node| external_node.node_id())
    }

    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    fn find_external_node_idx_at_indices(&self, indices: VoxelIndices) -> Option<usize> {
        self.find_external_node_id_at_indices(indices)
            .map(|id| id.number())
    }

    /// # Warning
    /// This method uses the raw value of the node IDs as indices into the node
    /// storage, which is only valid if no nodes have been removed from the tree
    /// after construction.
    fn update_adjacent_voxels_for_external_node(&mut self, node_idx: usize) {
        let aux_data = self.external_node_aux_storage.data_at_idx(node_idx);
        let voxel_scale = aux_data.voxel_scale;
        let voxel_indices = aux_data.voxel_indices;

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
            .external_node_aux_storage
            .data_at_idx(node_idx)
            .face_is_fully_obscured(face)
        {
            if let Some(adjacent_node_idx) =
                self.find_external_node_idx_at_indices(adjacent_indices)
            {
                let adjacent_node_data = self
                    .external_node_aux_storage
                    .data_at_idx_mut(adjacent_node_idx);

                // These are the indices of the adjacent voxel's origin,
                // which may be different from the indices we searched at
                let adjacent_voxel_indices = adjacent_node_data.voxel_indices;

                // Add this voxel as an adjacent voxel to the adjacent voxel
                adjacent_node_data
                    .adjacent_voxels
                    .push((voxel_indices, ExternalNodeID::from_number(node_idx)));

                adjacent_node_data.add_obscured_face_area(face.opposite_face(), 1);

                let node_data = self.external_node_aux_storage.data_at_idx_mut(node_idx);

                // Add the adjacent voxel as an adjacent voxel to this voxel
                node_data.adjacent_voxels.push((
                    adjacent_voxel_indices,
                    ExternalNodeID::from_number(adjacent_node_idx),
                ));

                node_data.add_obscured_face_area(face, 1);
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
                        .external_node_aux_storage
                        .data_at_idx(node_idx)
                        .adjacent_voxel(adjacent_indices)
                    {
                        // If there is already a voxel registered at the
                        // adjacent indices, we only need to obtain its scale to
                        // update the `covered` map
                        Some(
                            self.external_node_aux_storage
                                .data_at_idx(adjacent_node_id.number())
                                .voxel_scale as usize,
                        )
                    } else if let Some(adjacent_node_idx) =
                        self.find_external_node_idx_at_indices(adjacent_indices)
                    {
                        let adjacent_node_data = self
                            .external_node_aux_storage
                            .data_at_idx_mut(adjacent_node_idx);
                        let adjacent_voxel_scale = adjacent_node_data.voxel_scale as usize;

                        // If the scale of the adjacent voxel is larger than
                        // one, it could already be registered as an adjacent
                        // voxel to us, just not at the exact indices we
                        // searched at. Now that we have the adjacent node, we
                        // can check this and make sure to only register the
                        // voxels as neighbors if they truly have not been
                        // registered before.
                        if adjacent_voxel_scale == 1
                            || !adjacent_node_data.is_adjacent_to_voxel(voxel_indices)
                        {
                            // These are the indices of the adjacent voxel's
                            // origin, which may be different from the indices
                            // we searched at
                            let adjacent_voxel_indices = adjacent_node_data.voxel_indices;

                            // Either this voxel will completely obscure the
                            // adjacent voxel or vice versa, so we should add
                            // the minimum of the face areas of the two voxels
                            // as the obscured area for both
                            let obscured_area = u32::min(face_area, adjacent_node_data.face_area());

                            // Add this voxel as an adjacent voxel to the
                            // adjacent voxel
                            adjacent_node_data
                                .adjacent_voxels
                                .push((voxel_indices, ExternalNodeID::from_number(node_idx)));

                            adjacent_node_data
                                .add_obscured_face_area(face.opposite_face(), obscured_area);

                            let node_data =
                                self.external_node_aux_storage.data_at_idx_mut(node_idx);

                            // Add the adjacent voxel as an adjacent voxel to
                            // this voxel
                            node_data.adjacent_voxels.push((
                                adjacent_voxel_indices,
                                ExternalNodeID::from_number(adjacent_node_idx),
                            ));

                            node_data.add_obscured_face_area(face, obscured_area);
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
}

impl<F: Float> VoxelTreeProperties<F> {
    fn from_generator<G>(generator: &G) -> Self
    where
        G: VoxelGenerator<F>,
    {
        let voxel_extent = generator.voxel_extent();
        let height = VoxelTreeHeight::from_shape(generator.grid_shape());
        let instance_group_height = generator.instance_group_height();

        Self {
            voxel_extent,
            height,
            instance_group_height,
        }
    }

    fn voxel_extent(&self) -> F {
        self.voxel_extent
    }

    fn tree_height(&self) -> VoxelTreeHeight {
        self.height
    }

    fn instance_group_height(&self) -> u32 {
        self.instance_group_height
    }

    fn grid_size(&self) -> usize {
        self.height.grid_size_at_height(0)
    }

    fn has_intermediate_instance_group_height(&self) -> bool {
        self.instance_group_height > 0 && self.instance_group_height < self.height.value()
    }
}

impl VoxelTreeHeight {
    fn new(tree_height: u32) -> Self {
        Self { tree_height }
    }

    fn from_shape([shape_x, shape_y, shape_z]: [usize; 3]) -> Self {
        let tree_height = Self::height_with_grid_size(
            shape_x
                .max(shape_y)
                .max(shape_z)
                .checked_next_power_of_two()
                .unwrap(),
        );

        Self::new(tree_height)
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

    fn height_with_grid_size(grid_size: usize) -> u32 {
        grid_size.trailing_zeros()
    }

    fn height_with_voxel_scale(voxel_scale: u32) -> u32 {
        voxel_scale.trailing_zeros()
    }
}

#[allow(dead_code)]
impl ExternalNodeAuxiliaryStorage {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            index_map: KeyIndexMapper::default(),
            id_count: 0,
        }
    }

    fn n_nodes(&self) -> usize {
        self.data.len()
    }

    fn has_data_for_node(&self, node_id: ExternalNodeID) -> bool {
        self.index_map.contains_key(node_id.number())
    }

    fn data(&self, node_id: ExternalNodeID) -> &ExternalNodeAuxiliaryData {
        let idx = self.index_map.idx(node_id.number());
        self.data_at_idx(idx)
    }

    fn data_at_idx(&self, idx: usize) -> &ExternalNodeAuxiliaryData {
        &self.data[idx]
    }

    fn data_mut(&mut self, node_id: ExternalNodeID) -> &mut ExternalNodeAuxiliaryData {
        let idx = self.index_map.idx(node_id.number());
        self.data_at_idx_mut(idx)
    }

    fn data_at_idx_mut(&mut self, idx: usize) -> &mut ExternalNodeAuxiliaryData {
        &mut self.data[idx]
    }

    fn add_data(&mut self, node_id: ExternalNodeID, node: ExternalNodeAuxiliaryData) {
        self.index_map.push_key(node_id.number());
        self.data.push(node);
    }

    fn remove_data(&mut self, node_id: ExternalNodeID) {
        let idx = self.index_map.swap_remove_key(node_id.number());
        self.data.swap_remove(idx);
    }

    fn create_new_external_node_id(&mut self) -> ExternalNodeID {
        let node_id = ExternalNodeID::from_number(self.id_count);
        self.id_count += 1;
        node_id
    }
}

impl<F: Float> InternalNode<F> {
    fn build_root_node<G>(
        external_node_aux_storage: &mut ExternalNodeAuxiliaryStorage,
        voxel_instances: &mut GroupedVoxelInstanceStorage<F>,
        generator: &G,
        tree_height: VoxelTreeHeight,
    ) -> Option<Self>
    where
        G: VoxelGenerator<F>,
    {
        let current_indices = VoxelTreeIndices::at_root(tree_height);

        if current_indices.are_at_max_depth() {
            return None;
        }

        let mut internal_children = Vec::new();
        let mut external_children = Vec::new();

        // This instance group will contain the instances for all external nodes
        // down to the `instance_group_height`
        let instance_group_id = voxel_instances.create_group();

        let octant_child_indices = current_indices.for_next_depth().map(|next_indices| {
            create_voxel_tree_node(
                external_node_aux_storage,
                voxel_instances,
                &mut internal_children,
                &mut external_children,
                generator,
                instance_group_id,
                next_indices,
            )
        });

        if internal_children.is_empty() && external_children.is_empty() {
            None
        } else {
            let (dominating_voxel_type, _) =
                determine_dominating_voxel_type_with_count(&external_children, &internal_children);

            for external_child in &mut external_children {
                external_child.create_aux_storage_entry(external_node_aux_storage);
            }

            Some(InternalNode::new(
                octant_child_indices,
                internal_children,
                external_children,
                instance_group_id,
                voxel_instances.create_new_voxel_instance_id(),
                current_indices.height(),
                current_indices.voxel_indices(),
                dominating_voxel_type,
            ))
        }
    }

    fn new(
        octant_child_indices: [OctantChildNodeIdx; 8],
        internal_children: Vec<InternalNode<F>>,
        external_children: Vec<ExternalNode>,
        instance_group_id: InstanceGroupID,
        voxel_instance_id_for_lod: VoxelInstanceID,
        height: u32,
        voxel_indices: VoxelIndices,
        dominating_voxel_type: Option<VoxelType>,
    ) -> Self {
        Self {
            octant_child_indices,
            internal_children,
            external_children,
            instance_group_id,
            voxel_instance_id_for_lod,
            height,
            voxel_indices,
            dominating_voxel_type,
            aabb: AxisAlignedBox::new(Point3::origin(), Point3::origin()),
            exposed_descendant_count: 0,
            has_exposed_descendants: false,
            directional_obscuredness: DirectionalObscurednessLookupTable::fully_obscured(),
        }
    }

    fn octant_child_idx(&self, octant: Octant) -> OctantChildNodeIdx {
        self.octant_child_indices[octant.idx()]
    }

    fn internal_children(&self) -> &[InternalNode<F>] {
        &self.internal_children
    }

    fn external_children(&self) -> &[ExternalNode] {
        &self.external_children
    }

    fn internal_child_at_idx(&self, idx: usize) -> &InternalNode<F> {
        &self.internal_children[idx]
    }

    fn external_child_at_idx(&self, idx: usize) -> &ExternalNode {
        &self.external_children[idx]
    }

    #[cfg(test)]
    fn get_internal_child_in_octant(&self, octant: Octant) -> Option<&InternalNode<F>> {
        self.octant_child_idx(octant)
            .as_internal()
            .map(|idx| self.internal_child_at_idx(idx))
    }

    #[cfg(test)]
    fn n_children(&self) -> usize {
        self.n_internal_children() + self.n_external_children()
    }

    #[cfg(test)]
    fn n_internal_children(&self) -> usize {
        self.internal_children.len()
    }

    #[cfg(test)]
    fn n_external_children(&self) -> usize {
        self.external_children.len()
    }

    fn instance_group_id(&self) -> InstanceGroupID {
        self.instance_group_id
    }

    fn voxel_instance_id_for_lod(&self) -> VoxelInstanceID {
        self.voxel_instance_id_for_lod
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn voxel_scale(&self) -> u32 {
        VoxelTreeHeight::voxel_scale_at_height(self.height())
    }

    fn voxel_indices(&self) -> VoxelIndices {
        self.voxel_indices
    }

    fn dominating_voxel_type(&self) -> Option<VoxelType> {
        self.dominating_voxel_type
    }

    fn aabb(&self) -> &AxisAlignedBox<F> {
        &self.aabb
    }

    fn exposed_descendant_count(&self) -> usize {
        self.exposed_descendant_count
    }

    fn has_exposed_descendants(&self) -> bool {
        self.has_exposed_descendants
    }

    fn directional_obscuredness(&self) -> &DirectionalObscurednessLookupTable {
        &self.directional_obscuredness
    }

    /// Whether the node is fully obscured when viewed from the given direction.
    /// The direction is from the viewer towards the node and does not have to
    /// be normalized.
    fn is_fully_obscured_from_direction(&self, direction: &Vector3<F>) -> bool {
        self.directional_obscuredness
            .is_fully_obscured_from_direction(direction)
    }

    fn compute_transform(&self, voxel_extent: F) -> VoxelTransform<F> {
        let voxel_scale = F::from_u32(self.voxel_scale()).unwrap();

        let voxel_center_offset = self
            .voxel_indices()
            .voxel_center_offset(voxel_extent, voxel_scale);

        VoxelTransform::new(voxel_center_offset, voxel_scale)
    }

    fn add_voxel_instance_entry_for_lod(
        &self,
        voxel_instances: &mut GroupedVoxelInstanceStorage<F>,
        voxel_extent: F,
    ) {
        if let Some(voxel_type) = self.dominating_voxel_type() {
            voxel_instances
                .group_at_level_of_detail_mut(self.instance_group_id(), self.height())
                .add_instance(
                    self.voxel_instance_id_for_lod(),
                    voxel_type,
                    self.compute_transform(voxel_extent),
                );
        }
    }

    fn set_aabb(&mut self, aabb: AxisAlignedBox<F>) {
        self.aabb = aabb;
    }

    fn set_exposed_descendant_count(&mut self, exposed_descendant_count: usize) {
        self.exposed_descendant_count = exposed_descendant_count;
        self.has_exposed_descendants = exposed_descendant_count > 0;
    }

    fn set_directional_obscuredness(
        &mut self,
        directional_obscuredness: DirectionalObscurednessLookupTable,
    ) {
        self.directional_obscuredness = directional_obscuredness;
    }

    fn find_external_node_at_indices(
        &self,
        tree_height: u32,
        indices: VoxelIndices,
    ) -> Option<&ExternalNode> {
        if let Some(octants) = indices.octants(tree_height) {
            let mut internal_node = self;

            for octant in octants {
                match internal_node.octant_child_idx(octant) {
                    OctantChildNodeIdx::External(idx) => {
                        return Some(internal_node.external_child_at_idx(idx));
                    }
                    OctantChildNodeIdx::Internal(idx) => {
                        internal_node = internal_node.internal_child_at_idx(idx);
                    }
                    OctantChildNodeIdx::None => {
                        return None;
                    }
                }
            }
            None
        } else {
            None
        }
    }

    fn update_derived_node_data(
        &mut self,
        tree_properties: &VoxelTreeProperties<F>,
        aux_storage: &ExternalNodeAuxiliaryStorage,
    ) {
        let mut external_children = self.external_children.iter_mut();
        let mut internal_children = self.internal_children.iter_mut();

        let (mut aggregate_aabb, mut exposed_descendant_count, mut merged_directional_obscuredness) =
            if let Some(external_child) = external_children.next() {
                external_child.update_exposedness(aux_storage);

                let aggregate_aabb = external_child.compute_aabb(tree_properties.voxel_extent());

                let exposed_descendant_count = usize::from(external_child.is_exposed());

                let merged_directional_obscuredness = *external_child.directional_obscuredness();

                (
                    aggregate_aabb,
                    exposed_descendant_count,
                    merged_directional_obscuredness,
                )
            } else if let Some(internal_child) = internal_children.next() {
                internal_child.update_derived_node_data(tree_properties, aux_storage);

                let aggregate_aabb = internal_child.aabb().clone();

                let exposed_descendant_count = internal_child.exposed_descendant_count();

                let merged_directional_obscuredness = *internal_child.directional_obscuredness();

                (
                    aggregate_aabb,
                    exposed_descendant_count,
                    merged_directional_obscuredness,
                )
            } else {
                // All internal nodes should have at least one child
                unreachable!();
            };

        for external_child in external_children {
            external_child.update_exposedness(aux_storage);

            aggregate_aabb = AxisAlignedBox::aabb_from_pair(
                &aggregate_aabb,
                &external_child.compute_aabb(tree_properties.voxel_extent()),
            );

            exposed_descendant_count += usize::from(external_child.is_exposed());

            merged_directional_obscuredness.merge_with(external_child.directional_obscuredness());
        }

        for internal_child in internal_children {
            internal_child.update_derived_node_data(tree_properties, aux_storage);

            aggregate_aabb = AxisAlignedBox::aabb_from_pair(&aggregate_aabb, internal_child.aabb());

            exposed_descendant_count += internal_child.exposed_descendant_count();

            merged_directional_obscuredness.merge_with(internal_child.directional_obscuredness());
        }

        self.set_aabb(aggregate_aabb);
        self.set_exposed_descendant_count(exposed_descendant_count);
        self.set_directional_obscuredness(merged_directional_obscuredness);
    }

    fn add_voxel_instances(
        &self,
        voxel_instances: &mut GroupedVoxelInstanceStorage<F>,
        tree_properties: &VoxelTreeProperties<F>,
        external_node_criterion: &impl Fn(&ExternalNode) -> bool,
        internal_lod_node_criterion: &impl Fn(&InternalNode<F>) -> bool,
    ) {
        let mut stack = vec![self];

        while let Some(internal_node) = stack.pop() {
            let max_lod = if tree_properties.has_intermediate_instance_group_height()
                && internal_node.height() <= tree_properties.instance_group_height()
                && internal_lod_node_criterion(internal_node)
            {
                internal_node.add_voxel_instance_entry_for_lod(
                    voxel_instances,
                    tree_properties.voxel_extent(),
                );

                tree_properties.instance_group_height()
            } else {
                0
            };

            for external_child in internal_node.external_children() {
                if external_node_criterion(external_child) {
                    external_child.add_voxel_instance_entries(
                        internal_node.instance_group_id(),
                        voxel_instances,
                        tree_properties.voxel_extent(),
                        max_lod,
                    );
                }
            }

            stack.extend(internal_node.internal_children().iter());
        }
    }

    fn compute_bounding_sphere(
        &self,
        tree_properties: &VoxelTreeProperties<F>,
        max_depth: u32,
        current_depth: u32,
    ) -> Sphere<F> {
        let next_depth = current_depth + 1;

        if next_depth <= max_depth {
            let mut external_children = self.external_children().iter();
            let mut internal_children = self.internal_children().iter();

            let aggregate_bounding_sphere = if let Some(external_child) = external_children.next() {
                external_child.compute_bounding_sphere(tree_properties.voxel_extent())
            } else if let Some(internal_child) = internal_children.next() {
                internal_child.compute_bounding_sphere(tree_properties, max_depth, next_depth)
            } else {
                // All internal nodes should have at least one child
                unreachable!();
            };

            aggregate_bounding_sphere.bounding_sphere_with(
                external_children
                    .map(|external_child| {
                        external_child.compute_bounding_sphere(tree_properties.voxel_extent())
                    })
                    .chain(internal_children.map(|internal_child| {
                        internal_child.compute_bounding_sphere(
                            tree_properties,
                            max_depth,
                            next_depth,
                        )
                    })),
            )
        } else {
            Sphere::bounding_sphere_from_aabb(self.aabb())
        }
    }

    fn buffer_instances_for_internal_nodes(
        &self,
        tree_properties: &VoxelTreeProperties<F>,
        should_process_child: &mut impl FnMut(&InternalNode<F>) -> bool,
        buffer_instances: &mut impl FnMut(&InternalNode<F>),
    ) {
        let mut stack = vec![self];

        while let Some(internal_node) = stack.pop() {
            if internal_node.height() == tree_properties.instance_group_height() + 1 {
                for internal_child in internal_node.internal_children() {
                    buffer_instances(internal_child);
                }
            } else {
                for internal_child in internal_node.internal_children() {
                    if should_process_child(internal_child) {
                        stack.push(internal_child);
                    }
                }
            }
        }
    }
}

impl OctantChildNodeIdx {
    #[cfg(test)]
    fn as_internal(&self) -> Option<usize> {
        if let Self::Internal(idx) = self {
            Some(*idx)
        } else {
            None
        }
    }
}

impl ExternalNode {
    fn from_generator<F, G>(
        generator: &G,
        voxel_instance_id: VoxelInstanceID,
        indices: VoxelTreeIndices,
    ) -> Option<Self>
    where
        F: Float,
        G: VoxelGenerator<F>,
    {
        generator
            .voxel_at_indices(indices.i, indices.j, indices.k)
            .map(|voxel_type| Self::new(voxel_instance_id, voxel_type, indices))
    }

    fn new(
        voxel_instance_id: VoxelInstanceID,
        voxel_type: VoxelType,
        indices: VoxelTreeIndices,
    ) -> Self {
        let (voxel_scale, voxel_indices) = indices.voxel_scale_and_indices();
        Self {
            node_id: ExternalNodeID::not_applicable(),
            voxel_instance_id,
            voxel_type,
            voxel_indices,
            voxel_scale,
            is_exposed: true,
            directional_obscuredness: DirectionalObscurednessLookupTable::fully_exposed(),
        }
    }

    fn create_aux_storage_entry(&mut self, aux_storage: &mut ExternalNodeAuxiliaryStorage) {
        let aux_data = ExternalNodeAuxiliaryData::new(self.voxel_indices, self.voxel_scale);
        self.node_id = aux_storage.create_new_external_node_id();
        aux_storage.add_data(self.node_id, aux_data);
    }

    fn voxel_type(&self) -> VoxelType {
        self.voxel_type
    }

    fn voxel_indices(&self) -> &VoxelIndices {
        &self.voxel_indices
    }

    fn voxel_scale(&self) -> u32 {
        self.voxel_scale
    }

    fn height(&self) -> u32 {
        VoxelTreeHeight::height_with_voxel_scale(self.voxel_scale())
    }

    fn node_id(&self) -> ExternalNodeID {
        self.node_id
    }

    fn voxel_instance_id(&self) -> VoxelInstanceID {
        self.node_id.as_voxel_instance_id()
    }

    fn directional_obscuredness(&self) -> &DirectionalObscurednessLookupTable {
        &self.directional_obscuredness
    }

    fn is_exposed(&self) -> bool {
        self.is_exposed
    }

    fn aux_data<'a>(
        &self,
        aux_storage: &'a ExternalNodeAuxiliaryStorage,
    ) -> &'a ExternalNodeAuxiliaryData {
        aux_storage.data(self.node_id())
    }

    fn update_exposedness(&mut self, aux_storage: &ExternalNodeAuxiliaryStorage) {
        let aux_data = self.aux_data(aux_storage);

        self.directional_obscuredness =
            DirectionalObscurednessLookupTable::for_external_node(aux_data);

        self.is_exposed = !aux_data.has_only_fully_obscured_faces();
    }

    fn add_voxel_instance_entries<F: Float>(
        &self,
        instance_group_id: InstanceGroupID,
        voxel_instances: &mut GroupedVoxelInstanceStorage<F>,
        voxel_extent: F,
        max_lod: u32,
    ) {
        let transform = self.compute_transform(voxel_extent);

        for lod in 0..=u32::min(max_lod, self.height()) {
            voxel_instances
                .group_at_level_of_detail_mut(instance_group_id, lod)
                .add_instance(
                    self.voxel_instance_id(),
                    self.voxel_type(),
                    transform.clone(),
                );
        }
    }

    fn compute_aabb<F: Float>(&self, voxel_extent: F) -> AxisAlignedBox<F> {
        let (lower_corner, upper_corner) = self
            .voxel_indices()
            .voxel_lower_and_upper_corner(voxel_extent, self.voxel_scale());
        AxisAlignedBox::new(lower_corner, upper_corner)
    }

    fn compute_transform<F: Float>(&self, voxel_extent: F) -> VoxelTransform<F> {
        let voxel_scale = F::from_u32(self.voxel_scale()).unwrap();

        let voxel_center_offset = self
            .voxel_indices()
            .voxel_center_offset(voxel_extent, voxel_scale);

        VoxelTransform::new(voxel_center_offset, voxel_scale)
    }

    fn compute_bounding_sphere<F: Float>(&self, voxel_extent: F) -> Sphere<F> {
        let voxel_scale = F::from_u32(self.voxel_scale()).unwrap();

        let center = self
            .voxel_indices()
            .voxel_center_offset(voxel_extent, voxel_scale);

        let radius = F::ONE_HALF * F::sqrt(F::THREE) * voxel_extent * voxel_scale;
        Sphere::new(center.into(), radius)
    }
}

impl ExternalNodeID {
    fn not_applicable() -> Self {
        Self(usize::MAX)
    }

    fn from_number(number: usize) -> Self {
        Self(number)
    }

    fn number(&self) -> usize {
        self.0
    }

    fn as_voxel_instance_id(&self) -> VoxelInstanceID {
        VoxelInstanceID(self.number())
    }
}

impl ExternalNodeAuxiliaryData {
    fn new(voxel_indices: VoxelIndices, voxel_scale: u32) -> Self {
        Self {
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
    #[cfg(test)]
    fn face_is_fully_exposed(&self, face: VoxelFace) -> bool {
        self.exposed_face_area(face) == self.face_area()
    }

    /// Whether all the faces of this voxel are completely obscured by adjacent
    /// voxels.
    fn has_only_fully_obscured_faces(&self) -> bool {
        self.exposed_face_areas
            .iter()
            .all(|exposed_area| *exposed_area == 0)
    }

    /// Whether all the faces of this voxel are completely exposed.
    #[cfg(test)]
    fn has_only_fully_exposed_faces(&self) -> bool {
        let face_area = self.face_area();
        self.exposed_face_areas
            .iter()
            .all(|exposed_area| *exposed_area == face_area)
    }

    /// Returns the ID of the external node for the adjacent voxel with the
    /// given indices, or [`None`] if the voxel with the given indices is not
    /// adjacent to this one.
    fn adjacent_voxel(&self, voxel_indices: VoxelIndices) -> Option<ExternalNodeID> {
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

    /// The number of voxels adjacent to this one.
    #[cfg(test)]
    fn n_adjacent_voxels(&self) -> usize {
        self.adjacent_voxels.len()
    }

    /// Whether this voxel has any adjacent voxels.
    #[cfg(test)]
    fn has_adjacent_voxels(&self) -> bool {
        !self.adjacent_voxels.is_empty()
    }

    /// Reduces the exposed area of the given face of this voxel by the given
    /// area.
    fn add_obscured_face_area(&mut self, face: VoxelFace, obscured_area: u32) {
        let exposed_area = &mut self.exposed_face_areas[face as usize];
        *exposed_area = exposed_area.checked_sub(obscured_area).unwrap();
    }
}

impl DirectionalObscurednessLookupTable {
    fn fully_exposed() -> Self {
        Self {
            is_obscured_from_direction: [[[false; 2]; 2]; 2],
        }
    }

    fn fully_obscured() -> Self {
        Self {
            is_obscured_from_direction: [[[true; 2]; 2]; 2],
        }
    }

    fn for_external_node(aux_data: &ExternalNodeAuxiliaryData) -> Self {
        let is_obscured_from_direction = array::from_fn(|i| {
            let obscured_x = aux_data.face_is_fully_obscured(VoxelFace::X_FACES[i]);
            array::from_fn(|j| {
                let obscured_y = aux_data.face_is_fully_obscured(VoxelFace::Y_FACES[j]);
                array::from_fn(|k| {
                    let obscured_z = aux_data.face_is_fully_obscured(VoxelFace::Z_FACES[k]);
                    obscured_x && obscured_y && obscured_z
                })
            })
        });

        Self {
            is_obscured_from_direction,
        }
    }

    /// Whether the voxel or group of voxels is fully obscured when viewed from
    /// the given direction. The direction is from the viewer towards the
    /// voxel(s) and does not have to be normalized.
    fn is_fully_obscured_from_direction<F: Float>(&self, direction: &Vector3<F>) -> bool {
        self.is_obscured_from_direction[usize::from(direction.x.is_negative())]
            [usize::from(direction.y.is_negative())][usize::from(direction.z.is_negative())]
    }

    fn merge_with(&mut self, other: &Self) {
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    self.is_obscured_from_direction[i][j][k] = self.is_obscured_from_direction[i]
                        [j][k]
                        && other.is_obscured_from_direction[i][j][k];
                }
            }
        }
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

    fn depth(&self) -> u32 {
        self.depth
    }

    fn height(&self) -> u32 {
        self.tree_height.depth_to_height(self.depth())
    }

    fn are_at_max_depth(&self) -> bool {
        self.tree_height.depth_is_max(self.depth())
    }

    fn for_next_depth(&self) -> [Self; 8] {
        let next_depth = self.depth() + 1;
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
        self.tree_height.voxel_scale_at_depth(self.depth())
    }

    fn voxel_indices(&self) -> VoxelIndices {
        self.voxel_scale_and_indices().1
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

    fn voxel_lower_and_upper_corner<F: Float>(
        &self,
        voxel_extent: F,
        voxel_scale: u32,
    ) -> (Point3<F>, Point3<F>) {
        let voxel_scale = F::from_u32(voxel_scale).unwrap();
        let scaled_voxel_extent = voxel_extent * voxel_scale;

        let lower_corner = point![
            F::from_usize(self.i).unwrap() * voxel_extent,
            F::from_usize(self.j).unwrap() * voxel_extent,
            F::from_usize(self.k).unwrap() * voxel_extent
        ];

        let upper_corner = point![
            lower_corner.x + scaled_voxel_extent,
            lower_corner.y + scaled_voxel_extent,
            lower_corner.z + scaled_voxel_extent
        ];

        (lower_corner, upper_corner)
    }

    fn voxel_center_offset<F: Float>(&self, voxel_extent: F, voxel_scale: F) -> Vector3<F> {
        let scaled_voxel_extent = voxel_extent * voxel_scale;
        let half_scaled_voxel_extent = F::ONE_HALF * scaled_voxel_extent;

        let voxel_center_offset = vector![
            F::from_usize(self.i).unwrap() * voxel_extent + half_scaled_voxel_extent,
            F::from_usize(self.j).unwrap() * voxel_extent + half_scaled_voxel_extent,
            F::from_usize(self.k).unwrap() * voxel_extent + half_scaled_voxel_extent
        ];

        voxel_center_offset
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

fn create_voxel_tree_node<F, G>(
    external_node_aux_storage: &mut ExternalNodeAuxiliaryStorage,
    voxel_instances: &mut GroupedVoxelInstanceStorage<F>,
    internal_nodes: &mut Vec<InternalNode<F>>,
    external_nodes: &mut Vec<ExternalNode>,
    generator: &G,
    parent_instance_group_id: InstanceGroupID,
    current_indices: VoxelTreeIndices,
) -> OctantChildNodeIdx
where
    F: Float,
    G: VoxelGenerator<F>,
{
    if current_indices.are_at_max_depth() {
        if let Some(external_node) = ExternalNode::from_generator(
            generator,
            voxel_instances.create_new_voxel_instance_id(),
            current_indices,
        ) {
            let idx = external_nodes.len();
            external_nodes.push(external_node);
            OctantChildNodeIdx::External(idx)
        } else {
            OctantChildNodeIdx::None
        }
    } else {
        let height = current_indices.height();

        // If the current internal node is at the `instance_group_height`, we
        // need to create a new group for the instances of its external
        // descendants
        let create_new_instance_group = height == generator.instance_group_height();
        let instance_group_id = if create_new_instance_group {
            voxel_instances.create_group_with_multiple_levels_of_detail(height)
        } else {
            parent_instance_group_id
        };

        let mut internal_children = Vec::new();
        let mut external_children = Vec::new();

        let octant_child_indices = current_indices.for_next_depth().map(|next_indices| {
            create_voxel_tree_node(
                external_node_aux_storage,
                voxel_instances,
                &mut internal_children,
                &mut external_children,
                generator,
                instance_group_id,
                next_indices,
            )
        });

        let n_internal_children = internal_children.len();
        let n_external_children = external_children.len();

        if n_internal_children + n_external_children > 0 {
            let (dominating_voxel_type, dominating_voxel_type_count) =
                determine_dominating_voxel_type_with_count(&external_children, &internal_children);

            if n_external_children == 8 && dominating_voxel_type_count == 8 {
                // If it turns out that we can make this an external node,
                // we must remove the instance group we created for this
                // node
                if create_new_instance_group {
                    voxel_instances.remove_group(instance_group_id);
                }

                let idx = external_nodes.len();
                external_nodes.push(ExternalNode::new(
                    voxel_instances.create_new_voxel_instance_id(),
                    dominating_voxel_type.unwrap(),
                    current_indices,
                ));

                return OctantChildNodeIdx::External(idx);
            }

            for external_child in &mut external_children {
                external_child.create_aux_storage_entry(external_node_aux_storage);
            }

            let idx = internal_nodes.len();
            internal_nodes.push(InternalNode::new(
                octant_child_indices,
                internal_children,
                external_children,
                instance_group_id,
                voxel_instances.create_new_voxel_instance_id(),
                height,
                current_indices.voxel_indices(),
                dominating_voxel_type,
            ));

            OctantChildNodeIdx::Internal(idx)
        } else {
            OctantChildNodeIdx::None
        }
    }
}

fn determine_dominating_voxel_type_with_count<F: Float>(
    external_children: &[ExternalNode],
    internal_children: &[InternalNode<F>],
) -> (Option<VoxelType>, usize) {
    let mut voxel_type_counts = [0_usize; N_VOXEL_TYPES];
    let mut total_voxel_count = external_children.len();

    for external_child in external_children {
        voxel_type_counts[external_child.voxel_type() as usize] += 1;
    }

    for internal_child in internal_children {
        if let Some(voxel_type) = internal_child.dominating_voxel_type() {
            voxel_type_counts[voxel_type as usize] += 1;
            total_voxel_count += 1;
        }
    }

    let (dominating_voxel_type_idx, dominating_voxel_type_count) = voxel_type_counts
        .iter()
        .enumerate()
        .max_by_key(|&(_, &count)| count)
        .unwrap();

    let dominating_voxel_type = if total_voxel_count > 0 {
        Some(VoxelType::from_usize(dominating_voxel_type_idx).unwrap())
    } else {
        None
    };

    (dominating_voxel_type, *dominating_voxel_type_count)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::geometry::AxisAlignedBox;
    use approx::assert_abs_diff_eq;
    use nalgebra::{point, Point3, UnitQuaternion};
    use std::{collections::HashMap, sync::Mutex};

    struct EmptyVoxelGenerator {
        shape: [usize; 3],
    }

    struct DefaultVoxelGenerator {
        shape: [usize; 3],
        instance_group_height: u32,
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

    impl DefaultVoxelGenerator {
        fn new(shape: [usize; 3]) -> Self {
            Self::new_with_instance_group_height(shape, 0)
        }

        fn new_with_instance_group_height(shape: [usize; 3], instance_group_height: u32) -> Self {
            Self {
                shape,
                instance_group_height,
            }
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

        fn instance_group_height(&self) -> u32 {
            self.instance_group_height
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
    fn should_get_no_tree_from_two_cubed_empty_voxel_generator() {
        assert!(VoxelTree::build(&EmptyVoxelGenerator { shape: [2; 3] }).is_none());
    }

    #[test]
    fn should_get_no_tree_from_three_cubed_empty_voxel_generator() {
        assert!(VoxelTree::build(&EmptyVoxelGenerator { shape: [3; 3] }).is_none());
    }

    #[test]
    fn should_get_no_tree_for_single_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([1; 3]));
        assert!(tree.is_none());
    }

    #[test]
    fn should_get_tree_for_more_than_one_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([2, 1, 1]));
        assert!(tree.is_some());
    }

    #[test]
    fn should_get_voxel_extent_of_generator() {
        let generator = DefaultVoxelGenerator::new([2; 3]);
        let tree = VoxelTree::build(&generator).unwrap();
        assert_eq!(tree.voxel_extent(), generator.voxel_extent());
    }

    #[test]
    fn should_build_tree_with_grid_size_two_for_two_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([2, 1, 1])).unwrap();
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);

        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([1, 2, 1])).unwrap();
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);

        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([1, 1, 2])).unwrap();
        assert_eq!(tree.tree_height(), 1);
        assert_eq!(tree.grid_size(), 2);
    }

    #[test]
    fn should_build_tree_with_grid_size_four_for_three_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([3, 1, 1])).unwrap();
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);

        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([1, 3, 1])).unwrap();
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);

        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([1, 1, 3])).unwrap();
        assert_eq!(tree.tree_height(), 2);
        assert_eq!(tree.grid_size(), 4);
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
    fn should_have_root_node_with_two_external_children_for_two_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([2, 1, 1])).unwrap();
        let root_node = tree.root_node();
        assert_eq!(root_node.n_children(), 2);
        assert_eq!(root_node.n_external_children(), 2);
        assert_eq!(root_node.n_internal_children(), 0);
    }

    #[test]
    fn should_have_root_node_with_8_external_children_for_two_cubed_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([2; 3])).unwrap();
        let root_node = tree.root_node();
        assert_eq!(root_node.n_children(), 8);
        assert_eq!(root_node.n_external_children(), 8);
        assert_eq!(root_node.n_internal_children(), 0);
    }

    #[test]
    fn should_have_root_node_with_correct_internal_and_external_children_for_12_voxel_generator() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([2, 2, 3])).unwrap();
        let root_node = tree.root_node();
        assert_eq!(root_node.n_children(), 2);
        assert_eq!(root_node.n_external_children(), 1);
        assert_eq!(root_node.n_internal_children(), 1);
        let internal_child = root_node.internal_child_at_idx(0);
        assert_eq!(internal_child.n_children(), 4);
        assert_eq!(internal_child.n_external_children(), 4);
        assert_eq!(internal_child.n_internal_children(), 0);
    }

    #[test]
    fn should_have_correct_aabb_in_two_voxel_tree() {
        let generator = DefaultVoxelGenerator::new([1, 2, 1]);
        let tree = VoxelTree::build(&generator).unwrap();

        let root_node = tree.root_node();

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
        let generator = DefaultVoxelGenerator::new([1, 1, 3]);
        let tree = VoxelTree::build(&generator).unwrap();

        let check_aabb = |node: &InternalNode<f64>,
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

        let root_node = tree.root_node();

        check_aabb(root_node, [0, 0, 0], [1, 1, 3]);

        check_aabb(
            root_node
                .get_internal_child_in_octant(Octant::BackBottomLeft)
                .unwrap(),
            [0, 0, 0],
            [1, 1, 2],
        );

        check_aabb(
            root_node
                .get_internal_child_in_octant(Octant::FrontBottomLeft)
                .unwrap(),
            [0, 0, 2],
            [1, 1, 3],
        );
    }

    #[test]
    fn should_have_correct_aabbs_in_three_by_two_by_five_voxel_tree() {
        let generator = DefaultVoxelGenerator::new([3, 2, 5]);
        let tree = VoxelTree::build(&generator).unwrap();

        let check_aabb = |node: &InternalNode<f64>,
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

        let root_node = tree.root_node();

        check_aabb(root_node, [0, 0, 0], [3, 2, 5]);

        check_aabb(
            root_node
                .get_internal_child_in_octant(Octant::BackBottomLeft)
                .unwrap(),
            [0, 0, 0],
            [3, 2, 4],
        );

        check_aabb(
            root_node
                .get_internal_child_in_octant(Octant::FrontBottomLeft)
                .unwrap(),
            [0, 0, 4],
            [3, 2, 5],
        );

        check_aabb(
            root_node
                .get_internal_child_in_octant(Octant::BackBottomLeft)
                .unwrap()
                .get_internal_child_in_octant(Octant::BackBottomRight)
                .unwrap(),
            [2, 0, 0],
            [3, 2, 2],
        );

        check_aabb(
            root_node
                .get_internal_child_in_octant(Octant::BackBottomLeft)
                .unwrap()
                .get_internal_child_in_octant(Octant::FrontBottomRight)
                .unwrap(),
            [2, 0, 2],
            [3, 2, 4],
        );

        check_aabb(
            root_node
                .get_internal_child_in_octant(Octant::FrontBottomLeft)
                .unwrap()
                .get_internal_child_in_octant(Octant::BackBottomLeft)
                .unwrap(),
            [0, 0, 4],
            [2, 2, 5],
        );

        check_aabb(
            root_node
                .get_internal_child_in_octant(Octant::FrontBottomLeft)
                .unwrap()
                .get_internal_child_in_octant(Octant::BackBottomRight)
                .unwrap(),
            [2, 0, 4],
            [3, 2, 5],
        );
    }

    #[test]
    fn should_have_correct_exposed_descendant_count_in_two_voxel_tree() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([1, 2, 1])).unwrap();
        let root_node = tree.root_node();
        assert_eq!(root_node.exposed_descendant_count(), 2);
    }

    #[test]
    fn should_have_correct_exposed_descendant_count_in_spherical_voxel_tree() {
        let voxels = [
            [[0, 0, 0, 0], [0, 1, 1, 0], [0, 1, 1, 0], [0, 0, 0, 0]],
            [[0, 1, 1, 0], [1, 1, 1, 1], [1, 1, 1, 1], [0, 1, 1, 0]],
            [[0, 1, 1, 0], [1, 1, 1, 1], [1, 1, 1, 1], [0, 1, 1, 0]],
            [[0, 0, 0, 0], [0, 1, 1, 0], [0, 1, 1, 0], [0, 0, 0, 0]],
        ];
        let tree = VoxelTree::build(&ManualVoxelGenerator { voxels }).unwrap();
        let root_node = tree.root_node();
        assert_eq!(root_node.exposed_descendant_count(), 24);
    }

    #[test]
    fn should_compute_valid_bounding_sphere_for_two_voxel_tree() {
        let generator = DefaultVoxelGenerator::new([1, 2, 1]);
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
        let generator = DefaultVoxelGenerator::new([1, 1, 5]);
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
    fn should_compute_valid_bounding_sphere_for_four_by_two_by_two_voxel_tree() {
        let generator = DefaultVoxelGenerator::new([4, 2, 2]);
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
    fn should_have_correct_voxel_transforms_for_two_by_two_by_three_voxel_generator() {
        let generator = DefaultVoxelGenerator::new([2, 2, 3]);
        let tree = VoxelTree::build(&generator).unwrap();
        let voxel_instances = tree
            .voxel_instances()
            .group(tree.root_node().instance_group_id());

        assert_eq!(voxel_instances.n_instances(), 5);

        let check_transform = |i, j, k, x, y, z, scaling| {
            let node = tree
                .find_external_node_at_indices(VoxelIndices::new(i, j, k))
                .unwrap();
            let transform = voxel_instances.transform(node.voxel_instance_id());

            let correct_translation = vector![
                x * generator.voxel_extent(),
                y * generator.voxel_extent(),
                z * generator.voxel_extent()
            ];

            assert_abs_diff_eq!(transform.translation(), &correct_translation);
            assert_abs_diff_eq!(transform.scaling(), scaling);
        };

        check_transform(0, 0, 0, 1.0, 1.0, 1.0, 2.0);
        check_transform(0, 0, 2, 0.5, 0.5, 2.5, 1.0);
        check_transform(0, 1, 2, 0.5, 1.5, 2.5, 1.0);
        check_transform(1, 0, 2, 1.5, 0.5, 2.5, 1.0);
        check_transform(1, 1, 2, 1.5, 1.5, 2.5, 1.0);
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
        let generator = DefaultVoxelGenerator::new([2; 3]);
        let tree = VoxelTree::build(&generator).unwrap();
        assert!(tree.find_voxel_at_indices(2, 0, 0).is_none());
        assert!(tree.find_voxel_at_indices(0, 2, 0).is_none());
        assert!(tree.find_voxel_at_indices(0, 0, 2).is_none());
    }

    #[test]
    fn should_find_same_voxel_types_as_provided_by_generator() {
        let generator = DefaultVoxelGenerator::new([1, 3, 2]);
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
        let generator = DefaultVoxelGenerator::new([2, 2, 3]);
        let tree = VoxelTree::build(&generator).unwrap();

        let check_node = |i, j, k, indices, scale| {
            let node = tree
                .find_external_node_at_indices(VoxelIndices::new(i, j, k))
                .unwrap();
            assert_eq!(node.voxel_indices(), &indices);
            assert_eq!(node.voxel_scale(), scale);
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
        let tree = VoxelTree::build(&ManualVoxelGenerator {
            voxels: [[[1, 0], [0, 0]], [[0, 0], [0, 0]]],
        })
        .unwrap();
        let aux_data = tree.external_node_aux_data(
            tree.find_external_node_id_at_indices(VoxelIndices::new(0, 0, 0))
                .unwrap(),
        );
        assert!(!aux_data.has_adjacent_voxels());
        assert!(aux_data.has_only_fully_exposed_faces());
    }

    #[test]
    fn should_get_correct_voxels_adjacent_to_unmerged_voxel_in_one_by_two_by_two_voxel_tree() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([1, 2, 2])).unwrap();
        let aux_data = tree.external_node_aux_data(
            tree.find_external_node_id_at_indices(VoxelIndices::new(0, 1, 1))
                .unwrap(),
        );

        let check_neighbor_present = |i, j, k| {
            assert!(aux_data.is_adjacent_to_voxel(VoxelIndices::new(i, j, k)));
        };

        assert_eq!(aux_data.n_adjacent_voxels(), 2);
        check_neighbor_present(0, 1, 0);
        check_neighbor_present(0, 0, 1);
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerX));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperX));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::LowerY));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperY));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::LowerZ));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperZ));
    }

    #[test]
    fn should_get_correct_voxels_adjacent_to_unmerged_voxel_in_three_cubed_voxel_tree() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([3; 3])).unwrap();
        let aux_data = tree.external_node_aux_data(
            tree.find_external_node_id_at_indices(VoxelIndices::new(2, 1, 1))
                .unwrap(),
        );

        let check_neighbor_present = |i, j, k| {
            assert!(aux_data.is_adjacent_to_voxel(VoxelIndices::new(i, j, k)));
        };

        assert_eq!(aux_data.n_adjacent_voxels(), 5);
        check_neighbor_present(0, 0, 0);
        check_neighbor_present(2, 0, 1);
        check_neighbor_present(2, 2, 1);
        check_neighbor_present(2, 1, 0);
        check_neighbor_present(2, 1, 2);
        assert!(aux_data.face_is_fully_obscured(VoxelFace::LowerX));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperX));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::LowerY));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::UpperY));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::LowerZ));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::UpperZ));
    }

    #[test]
    fn should_get_correct_voxels_adjacent_to_merged_voxel_in_three_cubed_voxel_tree() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new([3; 3])).unwrap();
        let aux_data = tree.external_node_aux_data(
            tree.find_external_node_id_at_indices(VoxelIndices::new(0, 0, 0))
                .unwrap(),
        );

        let check_neighbor_present = |i, j, k| {
            assert!(aux_data.is_adjacent_to_voxel(VoxelIndices::new(i, j, k)));
        };

        assert_eq!(aux_data.n_adjacent_voxels(), 12);
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
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerX));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::UpperX));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerY));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::UpperY));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerZ));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::UpperZ));
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
        let tree = VoxelTree::build(&generator).unwrap();

        let check_neighbor_present = |aux_data: &ExternalNodeAuxiliaryData, i, j, k| {
            assert!(aux_data.is_adjacent_to_voxel(VoxelIndices::new(i, j, k)));
        };

        let aux_data = tree.external_node_aux_data(
            tree.find_external_node_id_at_indices(VoxelIndices::new(0, 0, 0))
                .unwrap(),
        );

        assert_eq!(aux_data.n_adjacent_voxels(), 5);
        check_neighbor_present(aux_data, 4, 0, 0);
        check_neighbor_present(aux_data, 4, 0, 3);
        check_neighbor_present(aux_data, 4, 1, 2);
        check_neighbor_present(aux_data, 4, 2, 2);
        check_neighbor_present(aux_data, 4, 3, 0);
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerX));
        assert_eq!(aux_data.exposed_face_area(VoxelFace::UpperX), 5);
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerY));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperY));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerZ));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperZ));

        let aux_data = tree.external_node_aux_data(
            tree.find_external_node_id_at_indices(VoxelIndices::new(4, 2, 2))
                .unwrap(),
        );

        assert_eq!(aux_data.n_adjacent_voxels(), 2);
        check_neighbor_present(aux_data, 0, 0, 0);
        check_neighbor_present(aux_data, 4, 1, 2);
        assert!(aux_data.face_is_fully_obscured(VoxelFace::LowerX));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperX));
        assert_eq!(aux_data.exposed_face_area(VoxelFace::LowerY), 3);
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperY));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerZ));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperZ));

        let aux_data = tree.external_node_aux_data(
            tree.find_external_node_id_at_indices(VoxelIndices::new(4, 1, 2))
                .unwrap(),
        );

        assert_eq!(aux_data.n_adjacent_voxels(), 3);
        check_neighbor_present(aux_data, 0, 0, 0);
        check_neighbor_present(aux_data, 4, 0, 0);
        check_neighbor_present(aux_data, 4, 2, 2);
        assert!(aux_data.face_is_fully_obscured(VoxelFace::LowerX));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperX));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::LowerY));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::UpperY));
        assert!(aux_data.face_is_fully_obscured(VoxelFace::LowerZ));
        assert!(aux_data.face_is_fully_exposed(VoxelFace::UpperZ));
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

        let model_view_transform = voxel_transform.transform_into_model_view_transform(
            &transform_from_tree_to_camera_space,
            &VoxelTransform::compute_camera_space_axes_in_model_space(
                &transform_from_tree_to_camera_space,
            ),
        );

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

    #[test]
    fn should_have_single_instance_group_for_instance_group_height_zero() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new_with_instance_group_height(
            [4, 1, 1],
            0,
        ))
        .unwrap();
        assert_eq!(tree.voxel_instances().n_groups(), 1);
    }

    #[test]
    fn should_have_single_instance_group_for_instance_group_height_equal_to_tree_height() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new_with_instance_group_height(
            [4, 1, 1],
            2,
        ))
        .unwrap();
        assert_eq!(tree.voxel_instances().n_groups(), 1);
    }

    #[test]
    fn should_have_single_instance_group_for_instance_group_height_exceeding_tree_height() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new_with_instance_group_height(
            [4, 1, 1],
            3,
        ))
        .unwrap();
        assert_eq!(tree.voxel_instances().n_groups(), 1);
    }

    #[test]
    fn should_have_9_instance_groups_for_instance_group_height_one_with_tree_height_two() {
        let tree = VoxelTree::build(&DefaultVoxelGenerator::new_with_instance_group_height(
            [4, 1, 1],
            1,
        ))
        .unwrap();
        assert_eq!(tree.voxel_instances().n_groups(), 9);
    }
}
