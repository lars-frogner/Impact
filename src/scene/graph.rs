//! Scene graph implementation.

use crate::{
    geometry::{Frustum, Sphere},
    num::Float,
    scene::{CameraID, CameraRepository, ModelID, ModelInstance, ModelInstancePool},
    util::{GenerationalIdx, GenerationalReusingVec},
};
use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Projective3, Similarity3};
use std::collections::HashSet;

/// A tree structure that defines a spatial hierarchy of
/// objects in the world and enables useful operations on them.
///
/// The scene graph can contain leaf nodes representing
/// [`ModelInstance`]s and [`Camera`](crate::geometry::Camera)s.
/// Every leaf node has a parent "group" node, which itself
/// has a group node as a parent and may have any number and
/// type of children. Each node holds a transform from the
/// space of the parent to the model space of the object or
/// group it represents.
#[derive(Clone, Debug)]
pub struct SceneGraph<F: Float> {
    root_node_id: GroupNodeID,
    group_nodes: NodeStorage<GroupNode<F>>,
    model_instance_nodes: NodeStorage<ModelInstanceNode<F>>,
    camera_nodes: NodeStorage<CameraNode<F>>,
}

/// Flat storage for all the [`SceneGraph`] nodes of a given
/// type.
#[derive(Clone, Debug, Default)]
pub struct NodeStorage<N> {
    nodes: GenerationalReusingVec<N>,
}

/// Represents a type of node in a [`SceneGraph`].
pub trait SceneGraphNode {
    /// Type of the node's ID.
    type ID;
    /// Type of the node's transform.
    type Transform;

    /// Sets the given transform as the transform from
    /// the space of the node's parent to the model
    /// space of the group or object the node represents.
    fn set_model_transform(&mut self, transform: Self::Transform);
}

/// Identifier for a [`GroupNode`] in a [`SceneGraph`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct GroupNodeID(GenerationalIdx);

/// Identifier for a [`ModelInstanceNode`] in a [`SceneGraph`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ModelInstanceNodeID(GenerationalIdx);

/// Identifier for a [`CameraNode`] in a [`SceneGraph`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct CameraNodeID(GenerationalIdx);

/// Represents a type of node identifier that may provide
/// an associated index.
pub trait NodeIDToIdx {
    /// Returns the index corresponding to the node ID.
    fn idx(&self) -> GenerationalIdx;
}

/// Represents a type of node identifier that may be created
/// from an associated index.
trait IdxToNodeID {
    /// Creates the node ID corresponding to the given index.
    fn from_idx(idx: GenerationalIdx) -> Self;
}

/// A [`SceneGraph`] node that has a group of other nodes as children.
/// The children may be [`ModelInstanceNode`]s, [`CameraNode`]s and/or
/// other group nodes. It holds a transform representing group's spatial
/// relationship with its parent group.
#[derive(Clone, Debug)]
pub struct GroupNode<F: Float> {
    parent_node_id: Option<GroupNodeID>,
    model_transform: Similarity3<F>,
    child_group_node_ids: HashSet<GroupNodeID>,
    child_model_instance_node_ids: HashSet<ModelInstanceNodeID>,
    child_camera_node_ids: HashSet<CameraNodeID>,
    bounding_sphere: Option<Sphere<F>>,
}

/// A [`SceneGraph`] leaf node representing a [`ModelInstance`].
/// It holds a transform representing the instance's spatial
/// relationship with its parent group.
#[derive(Clone, Debug)]
pub struct ModelInstanceNode<F: Float> {
    parent_node_id: GroupNodeID,
    model_bounding_sphere: Sphere<F>,
    model_transform: Similarity3<F>,
    model_id: ModelID,
}

/// A [`SceneGraph`] leaf node representing a
/// [`Camera`](crate::geometry::Camera). It holds at transform
/// representing the camera's spatial relationship with its parent
/// group.
#[derive(Clone, Debug)]
pub struct CameraNode<F: Float> {
    parent_node_id: GroupNodeID,
    model_transform: Similarity3<F>,
    camera_id: CameraID,
}

/// Storages for each [`SceneGraph`] node type.
pub type SceneGraphStorages<F> = (
    NodeStorage<GroupNode<F>>,
    NodeStorage<ModelInstanceNode<F>>,
    NodeStorage<CameraNode<F>>,
);

impl<F: Float> SceneGraph<F> {
    /// Creates a new empty scene graph.
    pub fn new() -> Self {
        let mut group_nodes = NodeStorage::new();
        let model_instance_nodes = NodeStorage::new();
        let camera_nodes = NodeStorage::new();

        let root_node_id = group_nodes.add_node(GroupNode::root(Similarity3::identity()));

        Self {
            root_node_id,
            group_nodes,
            model_instance_nodes,
            camera_nodes,
        }
    }

    /// Returns the ID of the root group node.
    pub fn root_node_id(&self) -> GroupNodeID {
        self.root_node_id
    }

    /// Returns the number of group nodes in the scene graph.
    pub fn n_group_nodes(&self) -> usize {
        self.group_nodes.n_nodes()
    }

    /// Returns the number of model instance nodes in the scene graph.
    pub fn n_model_instance_nodes(&self) -> usize {
        self.model_instance_nodes.n_nodes()
    }

    /// Returns the number of camera nodes in the scene graph.
    pub fn n_camera_nodes(&self) -> usize {
        self.camera_nodes.n_nodes()
    }

    /// Whether a group node with the given ID exists in the
    /// scene graph.
    pub fn has_group_node(&self, group_node_id: GroupNodeID) -> bool {
        self.group_nodes.has_node(group_node_id)
    }

    /// Whether a model instance node with the given ID exists
    /// in the scene graph.
    pub fn has_model_instance_node(&self, model_instance_node_id: ModelInstanceNodeID) -> bool {
        self.model_instance_nodes.has_node(model_instance_node_id)
    }

    /// Whether a camera node with the given ID exists in the
    /// scene graph.
    pub fn has_camera_node(&self, camera_node_id: CameraNodeID) -> bool {
        self.camera_nodes.has_node(camera_node_id)
    }

    /// Finds the [`CameraID`] of the camera represented by the
    /// camera node with the given ID.
    ///
    /// # Panics
    /// If the camera node with the given ID does not exist.
    pub fn camera_id(&self, camera_node_id: CameraNodeID) -> CameraID {
        self.camera_nodes.node(camera_node_id).camera_id()
    }

    /// Creates a new empty [`GroupNode`] with the given parent
    /// and model transform and includes it in the scene graph.
    ///
    /// # Returns
    /// The ID of the created group node.
    ///
    /// # Panics
    /// If the specified parent group node does not exist.
    pub fn create_group_node(
        &mut self,
        parent_node_id: GroupNodeID,
        transform: <GroupNode<F> as SceneGraphNode>::Transform,
    ) -> GroupNodeID {
        let group_node = GroupNode::non_root(parent_node_id, transform);
        let group_node_id = self.group_nodes.add_node(group_node);
        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_group_node(group_node_id);
        group_node_id
    }

    /// Creates a new [`ModelInstanceNode`] for an instance of the
    /// model with the given ID and bounding sphere. It is included
    /// in the scene graph with the given transform relative to the
    /// the given parent node.
    ///
    /// # Returns
    /// The ID of the created model instance node.
    ///
    /// # Panics
    /// If the specified parent group node does not exist.
    pub fn create_model_instance_node(
        &mut self,
        parent_node_id: GroupNodeID,
        transform: <ModelInstanceNode<F> as SceneGraphNode>::Transform,
        model_id: ModelID,
        bounding_sphere: Sphere<F>,
    ) -> ModelInstanceNodeID {
        let model_instance_node =
            ModelInstanceNode::new(parent_node_id, bounding_sphere, transform, model_id);
        let model_instance_node_id = self.model_instance_nodes.add_node(model_instance_node);
        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_model_instance_node(model_instance_node_id);
        model_instance_node_id
    }

    /// Creates a new [`CameraNode`] for the camera with the given ID.
    /// It is included in the scene graph with the given transform
    /// relative to the the given parent node.
    ///
    /// # Returns
    /// The ID of the created camera node.
    ///
    /// # Panics
    /// If the specified parent group node does not exist.
    pub fn create_camera_node(
        &mut self,
        parent_node_id: GroupNodeID,
        transform: <CameraNode<F> as SceneGraphNode>::Transform,
        camera_id: CameraID,
    ) -> CameraNodeID {
        let camera_node = CameraNode::new(parent_node_id, transform, camera_id);
        let camera_node_id = self.camera_nodes.add_node(camera_node);
        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_camera_node(camera_node_id);
        camera_node_id
    }

    /// Removes the [`GroupNode`] with the given ID and all its
    /// children from the scene graph.
    ///
    /// # Panics
    /// - If the specified group node does not exist.
    /// - If the specified group node is the root node.
    pub fn remove_group_node(&mut self, group_node_id: GroupNodeID) {
        let group_node = self.group_nodes.node(group_node_id);

        let parent_node_id = group_node.parent_node_id();

        let (child_group_node_ids, child_model_instance_node_ids, child_camera_node_ids) =
            group_node.obtain_child_node_ids();

        for child_group_node_id in child_group_node_ids {
            self.remove_group_node(child_group_node_id);
        }
        for child_model_instance_node_id in child_model_instance_node_ids {
            self.remove_model_instance_node(child_model_instance_node_id);
        }
        for child_camera_node_id in child_camera_node_ids {
            self.remove_camera_node(child_camera_node_id);
        }

        self.group_nodes.remove_node(group_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_group_node(group_node_id);
    }

    /// Removes the [`ModelInstanceNode`] with the given ID from the
    /// scene graph.
    ///
    /// # Panics
    /// If the specified model instance node does not exist.
    pub fn remove_model_instance_node(&mut self, model_instance_node_id: ModelInstanceNodeID) {
        let parent_node_id = self
            .model_instance_nodes
            .node(model_instance_node_id)
            .parent_node_id();
        self.model_instance_nodes
            .remove_node(model_instance_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_model_instance_node(model_instance_node_id);
    }

    /// Removes the [`CameraNode`] with the given ID from the scene
    /// graph.
    ///
    /// # Panics
    /// If the specified camera node does not exist.
    pub fn remove_camera_node(&mut self, camera_node_id: CameraNodeID) {
        let parent_node_id = self.camera_nodes.node(camera_node_id).parent_node_id();
        self.camera_nodes.remove_node(camera_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_camera_node(camera_node_id);
    }

    /// Returns all the storages of the scene graph's nodes by value,
    /// leaving only empty storages in the scene graph.
    /// Use [`return_storages`](Self::return_storages) to put the
    /// storages back into the scene graph.
    pub fn take_storages(&mut self) -> SceneGraphStorages<F> {
        let group_nodes = std::mem::replace(&mut self.group_nodes, NodeStorage::new());
        let model_instance_nodes =
            std::mem::replace(&mut self.model_instance_nodes, NodeStorage::new());
        let camera_nodes = std::mem::replace(&mut self.camera_nodes, NodeStorage::new());
        (group_nodes, model_instance_nodes, camera_nodes)
    }

    /// Takes the given storages of scene graph nodes (typically
    /// returned from [`take_storages`](Self::take_storages)) and
    /// uses them as the storages for this scene graph.
    pub fn return_storages(&mut self, storages: SceneGraphStorages<F>) {
        let (group_nodes, model_instance_nodes, camera_nodes) = storages;
        self.group_nodes = group_nodes;
        self.model_instance_nodes = model_instance_nodes;
        self.camera_nodes = camera_nodes;
    }

    /// Computes the model-to-camera space transforms of all the model
    /// instances in the scene graph that are visible with the specified
    /// camera and adds them in the given model instance pool. If no camera
    /// is specified, the computed transforms will be model-to-root space
    /// transforms instead, and view culling is performed for the identity
    /// view projection transform.
    ///
    /// # Errors
    /// Returns an error if the specified camera is not present in the
    /// given camera repository.
    ///
    /// # Panics
    /// If the specified camera node does not exist.
    pub fn sync_visible_model_instances(
        &mut self,
        model_instance_pool: &mut ModelInstancePool<F>,
        camera_repository: &CameraRepository<F>,
        camera_node_id: Option<CameraNodeID>,
    ) -> Result<()> {
        let root_node_id = self.root_node_id();

        let (view_projection_transform, root_to_camera_transform) = match camera_node_id {
            Some(camera_node_id) => {
                let camera_node = self.camera_nodes.node(camera_node_id);
                let camera_id = camera_node.camera_id();
                let camera = camera_repository
                    .get_camera(camera_id)
                    .ok_or_else(|| anyhow!("Camera {} not found", camera_id))?;
                (
                    camera.compute_view_projection_transform(),
                    self.compute_root_to_camera_transform(camera_node),
                )
            }
            None => (
                Projective3::identity(),
                *self.group_nodes.node(root_node_id).model_transform(),
            ),
        };

        let camera_frustum = Frustum::from_transform(&view_projection_transform);

        self.update_bounding_spheres(root_node_id);

        self.update_model_transforms_for_group(
            model_instance_pool,
            &camera_frustum,
            root_node_id,
            &root_to_camera_transform,
        );

        Ok(())
    }

    /// Computes the transform from the scene graph's root node space
    /// to the space of the given camera node.
    fn compute_root_to_camera_transform(&self, camera_node: &CameraNode<F>) -> Similarity3<F> {
        let mut root_to_camera_transform = *camera_node.model_transform();
        let mut parent_node = self.group_nodes.node(camera_node.parent_node_id());

        // Walk up the tree and append transforms until reaching the root
        loop {
            root_to_camera_transform *= parent_node.model_transform();

            if parent_node.is_root() {
                break;
            } else {
                parent_node = self.group_nodes.node(parent_node.parent_node_id());
            }
        }

        root_to_camera_transform
    }

    /// Updates the model transforms of the group and model instance
    /// nodes that are children of the specified group node and
    /// whose bounding spheres lie within the given camera frustum.
    /// The given parent model transform and the model transform
    /// of the specified group node are prepended to the transforms
    /// of the children. For the children that are model instance
    /// nodes, their final model-to-camera transforms are added in
    /// the given model instance pool.
    ///
    /// # Panics
    /// If the specified group node does not exist.
    fn update_model_transforms_for_group(
        &self,
        model_instance_pool: &mut ModelInstancePool<F>,
        camera_frustum: &Frustum<F>,
        group_node_id: GroupNodeID,
        parent_model_transform: &Similarity3<F>,
    ) {
        let group_node = self.group_nodes.node(group_node_id);

        let model_transform = parent_model_transform * group_node.model_transform();

        if let Some(bounding_sphere) = group_node.get_bounding_sphere() {
            let bounding_sphere_world_space = bounding_sphere.transformed(&model_transform);

            if !camera_frustum.sphere_lies_outside(&bounding_sphere_world_space) {
                for &group_node_id in group_node.child_group_node_ids() {
                    self.update_model_transforms_for_group(
                        model_instance_pool,
                        camera_frustum,
                        group_node_id,
                        &model_transform,
                    );
                }

                for &model_instance_node_id in group_node.child_model_instance_node_ids() {
                    self.update_model_transform_of_model_instance(
                        model_instance_pool,
                        model_instance_node_id,
                        &model_transform,
                    );
                }
            }
        }
    }

    /// Prepends the given parent model transform to the
    /// model transform of the specified model instance node
    /// and adds an instance with the resulting transform
    /// in the given model instance pool.
    ///
    /// # Panics
    /// If the specified model instance node does not exist.
    fn update_model_transform_of_model_instance(
        &self,
        model_instance_pool: &mut ModelInstancePool<F>,
        model_instance_node_id: ModelInstanceNodeID,
        parent_model_transform: &Similarity3<F>,
    ) {
        let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

        if let Some(buffer) = model_instance_pool
            .model_instance_buffers
            .get_mut(&model_instance_node.model_id())
        {
            let model_transform = parent_model_transform * model_instance_node.model_transform();

            buffer.add_instance(ModelInstance::with_transform(
                model_transform.to_homogeneous(),
            ))
        }
    }

    /// Updates the bounding sphere of the specified group node
    /// and all its children. Each bounding sphere is defined
    /// in the local space of its group node.
    ///
    /// # Returns
    /// The bounding sphere of the specified group node, defined
    /// in the space of its parent group node (used for recursion).
    ///
    /// # Panics
    /// If the specified group node does not exist.
    fn update_bounding_spheres(&mut self, group_node_id: GroupNodeID) -> Option<Sphere<F>> {
        let group_node = self.group_nodes.node(group_node_id);

        let child_group_node_ids = group_node.obtain_child_group_node_ids();
        let model_instance_node_ids = group_node.obtain_child_model_instance_node_ids();

        let mut child_bounding_spheres =
            Vec::with_capacity(child_group_node_ids.len() + model_instance_node_ids.len());

        child_bounding_spheres.extend(
            child_group_node_ids
                .into_iter()
                .filter_map(|group_node_id| self.update_bounding_spheres(group_node_id)),
        );

        child_bounding_spheres.extend(model_instance_node_ids.into_iter().map(
            |model_instance_node_id| {
                self.find_model_instance_bounding_sphere(model_instance_node_id)
            },
        ));

        let group_node = self.group_nodes.node_mut(group_node_id);

        if child_bounding_spheres.is_empty() {
            group_node.set_bounding_sphere(None);
            None
        } else {
            let bounding_sphere = child_bounding_spheres.pop().unwrap();
            let bounding_sphere = bounding_sphere.bounding_sphere_with(&child_bounding_spheres);

            let bounding_sphere_in_parent_space =
                bounding_sphere.transformed(group_node.model_transform());

            group_node.set_bounding_sphere(Some(bounding_sphere));

            Some(bounding_sphere_in_parent_space)
        }
    }

    /// Returns the bounding sphere of the model of the
    /// specified model instance node, defined in the space
    /// of the model instance.
    fn find_model_instance_bounding_sphere(
        &self,
        model_instance_node_id: ModelInstanceNodeID,
    ) -> Sphere<F> {
        let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

        model_instance_node
            .model_bounding_sphere()
            .transformed(model_instance_node.model_transform())
    }

    #[cfg(test)]
    fn node_has_group_node_as_child(
        &self,
        group_node_id: GroupNodeID,
        child_group_node_id: GroupNodeID,
    ) -> bool {
        self.group_nodes
            .node(group_node_id)
            .child_group_node_ids()
            .contains(&child_group_node_id)
    }

    #[cfg(test)]
    fn node_has_model_instance_node_as_child(
        &self,
        group_node_id: GroupNodeID,
        child_model_instance_node_id: ModelInstanceNodeID,
    ) -> bool {
        self.group_nodes
            .node(group_node_id)
            .child_model_instance_node_ids()
            .contains(&child_model_instance_node_id)
    }

    #[cfg(test)]
    fn node_has_camera_node_as_child(
        &self,
        group_node_id: GroupNodeID,
        child_camera_node_id: CameraNodeID,
    ) -> bool {
        self.group_nodes
            .node(group_node_id)
            .child_camera_node_ids()
            .contains(&child_camera_node_id)
    }
}

impl<F: Float> Default for SceneGraph<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: SceneGraphNode> NodeStorage<N> {
    /// Sets the given transform as the model transform for
    /// the node with the given ID.
    pub fn set_node_transform(&mut self, node_id: N::ID, transform: N::Transform)
    where
        N::ID: NodeIDToIdx,
    {
        self.node_mut(node_id).set_model_transform(transform);
    }

    /// Whether a node with the given ID exists in the storage.
    pub fn has_node(&self, node_id: N::ID) -> bool
    where
        N::ID: NodeIDToIdx,
    {
        self.nodes.get_element(node_id.idx()).is_some()
    }

    fn new() -> Self {
        Self {
            nodes: GenerationalReusingVec::new(),
        }
    }

    fn n_nodes(&self) -> usize {
        self.nodes.n_elements()
    }

    fn node(&self, node_id: N::ID) -> &N
    where
        N::ID: NodeIDToIdx,
    {
        self.nodes.element(node_id.idx())
    }

    fn node_mut(&mut self, node_id: N::ID) -> &mut N
    where
        N::ID: NodeIDToIdx,
    {
        self.nodes.element_mut(node_id.idx())
    }

    fn add_node(&mut self, node: N) -> N::ID
    where
        N::ID: IdxToNodeID,
    {
        N::ID::from_idx(self.nodes.add_element(node))
    }

    fn remove_node(&mut self, node_id: N::ID)
    where
        N::ID: NodeIDToIdx,
    {
        self.nodes.free_element_at_idx(node_id.idx());
    }
}

impl<F: Float> GroupNode<F> {
    fn new(parent_node_id: Option<GroupNodeID>, transform: Similarity3<F>) -> Self {
        Self {
            parent_node_id,
            model_transform: transform,
            child_group_node_ids: HashSet::new(),
            child_model_instance_node_ids: HashSet::new(),
            child_camera_node_ids: HashSet::new(),
            bounding_sphere: None,
        }
    }

    fn root(transform: Similarity3<F>) -> Self {
        Self::new(None, transform)
    }

    fn non_root(parent_node_id: GroupNodeID, transform: Similarity3<F>) -> Self {
        Self::new(Some(parent_node_id), transform)
    }

    fn is_root(&self) -> bool {
        self.parent_node_id.is_none()
    }

    fn model_transform(&self) -> &Similarity3<F> {
        &self.model_transform
    }

    fn parent_node_id(&self) -> GroupNodeID {
        self.parent_node_id.unwrap()
    }

    fn child_group_node_ids(&self) -> &HashSet<GroupNodeID> {
        &self.child_group_node_ids
    }

    fn child_model_instance_node_ids(&self) -> &HashSet<ModelInstanceNodeID> {
        &self.child_model_instance_node_ids
    }

    #[allow(dead_code)]
    fn child_camera_node_ids(&self) -> &HashSet<CameraNodeID> {
        &self.child_camera_node_ids
    }

    fn get_bounding_sphere(&self) -> Option<&Sphere<F>> {
        self.bounding_sphere.as_ref()
    }

    fn obtain_child_group_node_ids(&self) -> Vec<GroupNodeID> {
        self.child_group_node_ids.iter().cloned().collect()
    }

    fn obtain_child_model_instance_node_ids(&self) -> Vec<ModelInstanceNodeID> {
        self.child_model_instance_node_ids.iter().cloned().collect()
    }

    fn obtain_child_camera_node_ids(&self) -> Vec<CameraNodeID> {
        self.child_camera_node_ids.iter().cloned().collect()
    }

    fn obtain_child_node_ids(
        &self,
    ) -> (
        Vec<GroupNodeID>,
        Vec<ModelInstanceNodeID>,
        Vec<CameraNodeID>,
    ) {
        (
            self.obtain_child_group_node_ids(),
            self.obtain_child_model_instance_node_ids(),
            self.obtain_child_camera_node_ids(),
        )
    }

    fn add_child_group_node(&mut self, group_node_id: GroupNodeID) {
        self.child_group_node_ids.insert(group_node_id);
    }

    fn add_child_model_instance_node(&mut self, model_instance_node_id: ModelInstanceNodeID) {
        self.child_model_instance_node_ids
            .insert(model_instance_node_id);
    }

    fn add_child_camera_node(&mut self, camera_node_id: CameraNodeID) {
        self.child_camera_node_ids.insert(camera_node_id);
    }

    fn remove_child_group_node(&mut self, group_node_id: GroupNodeID) {
        self.child_group_node_ids.remove(&group_node_id);
    }

    fn remove_child_model_instance_node(&mut self, model_instance_node_id: ModelInstanceNodeID) {
        self.child_model_instance_node_ids
            .remove(&model_instance_node_id);
    }

    fn remove_child_camera_node(&mut self, camera_node_id: CameraNodeID) {
        self.child_camera_node_ids.remove(&camera_node_id);
    }

    fn set_bounding_sphere(&mut self, bounding_sphere: Option<Sphere<F>>) {
        self.bounding_sphere = bounding_sphere;
    }
}

impl<F: Float> SceneGraphNode for GroupNode<F> {
    type ID = GroupNodeID;
    type Transform = Similarity3<F>;

    fn set_model_transform(&mut self, transform: Self::Transform) {
        self.model_transform = transform;
    }
}

impl<F: Float> ModelInstanceNode<F> {
    pub fn set_model_bounding_sphere(&mut self, bounding_sphere: Sphere<F>) {
        self.model_bounding_sphere = bounding_sphere;
    }

    fn new(
        parent_node_id: GroupNodeID,
        model_bounding_sphere: Sphere<F>,
        model_transform: Similarity3<F>,
        model_id: ModelID,
    ) -> Self {
        Self {
            parent_node_id,
            model_bounding_sphere,
            model_transform,
            model_id,
        }
    }

    fn parent_node_id(&self) -> GroupNodeID {
        self.parent_node_id
    }

    fn model_transform(&self) -> &Similarity3<F> {
        &self.model_transform
    }

    fn model_id(&self) -> ModelID {
        self.model_id
    }

    fn model_bounding_sphere(&self) -> &Sphere<F> {
        &self.model_bounding_sphere
    }
}

impl<F: Float> SceneGraphNode for ModelInstanceNode<F> {
    type ID = ModelInstanceNodeID;
    type Transform = Similarity3<F>;

    fn set_model_transform(&mut self, transform: Self::Transform) {
        self.model_transform = transform;
    }
}

impl<F: Float> CameraNode<F> {
    fn new(
        parent_node_id: GroupNodeID,
        model_transform: Similarity3<F>,
        camera_id: CameraID,
    ) -> Self {
        Self {
            parent_node_id,
            model_transform,
            camera_id,
        }
    }

    fn parent_node_id(&self) -> GroupNodeID {
        self.parent_node_id
    }

    fn model_transform(&self) -> &Similarity3<F> {
        &self.model_transform
    }

    fn camera_id(&self) -> CameraID {
        self.camera_id
    }
}

impl<F: Float> SceneGraphNode for CameraNode<F> {
    type ID = CameraNodeID;
    type Transform = Similarity3<F>;

    fn set_model_transform(&mut self, transform: Self::Transform) {
        self.model_transform = transform;
    }
}

macro_rules! impl_node_id_idx_traits {
    ($node_id_type:ty) => {
        impl IdxToNodeID for $node_id_type {
            fn from_idx(idx: GenerationalIdx) -> Self {
                Self(idx)
            }
        }
        impl NodeIDToIdx for $node_id_type {
            fn idx(&self) -> GenerationalIdx {
                self.0
            }
        }
    };
}

impl_node_id_idx_traits!(GroupNodeID);
impl_node_id_idx_traits!(ModelInstanceNodeID);
impl_node_id_idx_traits!(CameraNodeID);

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::{point, Point3, Rotation3, Scale3, Translation3};

    fn create_dummy_group_node<F: Float>(
        scene_graph: &mut SceneGraph<F>,
        parent_node_id: GroupNodeID,
    ) -> GroupNodeID {
        scene_graph.create_group_node(parent_node_id, Similarity3::identity())
    }

    fn create_dummy_model_instance_node<F: Float>(
        scene_graph: &mut SceneGraph<F>,
        parent_node_id: GroupNodeID,
    ) -> ModelInstanceNodeID {
        scene_graph.create_model_instance_node(
            parent_node_id,
            Similarity3::identity(),
            ModelID(hash!("Test model")),
            Sphere::new(Point3::origin(), F::one()),
        )
    }

    fn create_dummy_camera_node<F: Float>(
        scene_graph: &mut SceneGraph<F>,
        parent_node_id: GroupNodeID,
    ) -> CameraNodeID {
        scene_graph.create_camera_node(
            parent_node_id,
            Similarity3::identity(),
            CameraID(hash!("Test camera")),
        )
    }

    #[test]
    fn creating_scene_graph_works() {
        let scene_graph = SceneGraph::<f64>::new();

        assert!(scene_graph.has_group_node(scene_graph.root_node_id()));

        assert_eq!(scene_graph.n_group_nodes(), 1);
        assert_eq!(scene_graph.n_model_instance_nodes(), 0);
        assert_eq!(scene_graph.n_camera_nodes(), 0);
    }

    #[test]
    fn creating_group_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_group_node(&mut scene_graph, root_id);

        assert!(scene_graph.has_group_node(id));
        assert!(scene_graph.node_has_group_node_as_child(root_id, id));

        assert_eq!(scene_graph.n_group_nodes(), 2);
        assert_eq!(scene_graph.n_model_instance_nodes(), 0);
        assert_eq!(scene_graph.n_camera_nodes(), 0);
    }

    #[test]
    fn creating_model_instance_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_model_instance_node(&mut scene_graph, root_id);

        assert!(scene_graph.has_model_instance_node(id));
        assert!(scene_graph.node_has_model_instance_node_as_child(root_id, id));

        assert_eq!(scene_graph.n_group_nodes(), 1);
        assert_eq!(scene_graph.n_model_instance_nodes(), 1);
        assert_eq!(scene_graph.n_camera_nodes(), 0);
    }

    #[test]
    fn creating_camera_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_camera_node(&mut scene_graph, root_id);

        assert!(scene_graph.has_camera_node(id));
        assert!(scene_graph.node_has_camera_node_as_child(root_id, id));

        assert_eq!(scene_graph.n_group_nodes(), 1);
        assert_eq!(scene_graph.n_model_instance_nodes(), 0);
        assert_eq!(scene_graph.n_camera_nodes(), 1);
    }

    #[test]
    fn removing_model_instance_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_model_instance_node(&mut scene_graph, root_id);
        scene_graph.remove_model_instance_node(id);

        assert!(!scene_graph.has_model_instance_node(id));
        assert!(!scene_graph.node_has_model_instance_node_as_child(root_id, id));

        assert_eq!(scene_graph.n_group_nodes(), 1);
        assert_eq!(scene_graph.n_model_instance_nodes(), 0);
        assert_eq!(scene_graph.n_camera_nodes(), 0);
    }

    #[test]
    fn removing_camera_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_camera_node(&mut scene_graph, root_id);
        scene_graph.remove_camera_node(id);

        assert!(!scene_graph.has_camera_node(id));
        assert!(!scene_graph.node_has_camera_node_as_child(root_id, id));

        assert_eq!(scene_graph.n_group_nodes(), 1);
        assert_eq!(scene_graph.n_model_instance_nodes(), 0);
        assert_eq!(scene_graph.n_camera_nodes(), 0);
    }

    #[test]
    fn removing_group_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        let child_group_node_id = create_dummy_group_node(&mut scene_graph, group_node_id);
        let child_camera_node_id = create_dummy_camera_node(&mut scene_graph, group_node_id);
        let child_model_instance_node_id =
            create_dummy_model_instance_node(&mut scene_graph, group_node_id);

        scene_graph.remove_group_node(group_node_id);

        assert!(!scene_graph.has_group_node(group_node_id));
        assert!(!scene_graph.node_has_group_node_as_child(root_id, group_node_id));

        assert!(!scene_graph.has_group_node(child_group_node_id));
        assert!(!scene_graph.has_camera_node(child_camera_node_id));
        assert!(!scene_graph.has_model_instance_node(child_model_instance_node_id));

        assert_eq!(scene_graph.n_group_nodes(), 1);
        assert_eq!(scene_graph.n_model_instance_nodes(), 0);
        assert_eq!(scene_graph.n_camera_nodes(), 0);
    }

    #[test]
    #[should_panic]
    fn creating_group_node_with_missing_parent_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);

        create_dummy_group_node(&mut scene_graph, group_node_id);
    }

    #[test]
    #[should_panic]
    fn creating_model_instance_node_with_missing_parent_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);

        create_dummy_model_instance_node(&mut scene_graph, group_node_id);
    }

    #[test]
    #[should_panic]
    fn creating_camera_node_with_missing_parent_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);

        create_dummy_camera_node(&mut scene_graph, group_node_id);
    }

    #[test]
    #[should_panic]
    fn removing_root_node_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        scene_graph.remove_group_node(scene_graph.root_node_id());
    }

    #[test]
    #[should_panic]
    fn removing_group_node_twice_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);
        scene_graph.remove_group_node(group_node_id);
    }

    #[test]
    #[should_panic]
    fn removing_model_instance_node_twice_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let model_instance_node_id = create_dummy_model_instance_node(&mut scene_graph, root_id);
        scene_graph.remove_model_instance_node(model_instance_node_id);
        scene_graph.remove_model_instance_node(model_instance_node_id);
    }

    #[test]
    #[should_panic]
    fn removing_camera_node_twice_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let camera_node_id = create_dummy_camera_node(&mut scene_graph, root_id);
        scene_graph.remove_camera_node(camera_node_id);
        scene_graph.remove_camera_node(camera_node_id);
    }

    #[test]
    fn taking_and_returning_storages_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        create_dummy_group_node(&mut scene_graph, root_id);
        create_dummy_group_node(&mut scene_graph, root_id);
        create_dummy_camera_node(&mut scene_graph, root_id);
        create_dummy_camera_node(&mut scene_graph, root_id);
        create_dummy_model_instance_node(&mut scene_graph, root_id);
        create_dummy_model_instance_node(&mut scene_graph, root_id);

        assert_eq!(scene_graph.n_group_nodes(), 3);
        assert_eq!(scene_graph.n_model_instance_nodes(), 2);
        assert_eq!(scene_graph.n_camera_nodes(), 2);

        let (group_node_storage, model_instance_node_storage, camera_node_storage) =
            scene_graph.take_storages();

        assert_eq!(group_node_storage.n_nodes(), 3);
        assert_eq!(model_instance_node_storage.n_nodes(), 2);
        assert_eq!(camera_node_storage.n_nodes(), 2);

        assert_eq!(scene_graph.n_group_nodes(), 0);
        assert_eq!(scene_graph.n_model_instance_nodes(), 0);
        assert_eq!(scene_graph.n_camera_nodes(), 0);

        scene_graph.return_storages((
            group_node_storage,
            model_instance_node_storage,
            camera_node_storage,
        ));

        assert_eq!(scene_graph.n_group_nodes(), 3);
        assert_eq!(scene_graph.n_model_instance_nodes(), 2);
        assert_eq!(scene_graph.n_camera_nodes(), 2);
    }

    #[test]
    fn computing_root_to_camera_transform_with_only_camera_transforms_works() {
        let camera_transform = Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            7.0,
        );

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();
        let camera =
            scene_graph.create_camera_node(root, camera_transform, CameraID(hash!("Test camera")));

        let transform =
            scene_graph.compute_root_to_camera_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(transform, camera_transform);
    }

    #[test]
    fn computing_root_to_camera_transform_with_only_identity_model_transforms_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();
        let group_1 = scene_graph.create_group_node(root, Similarity3::identity());
        let group_2 = scene_graph.create_group_node(group_1, Similarity3::identity());
        let group_3 = scene_graph.create_group_node(group_2, Similarity3::identity());
        let camera = scene_graph.create_camera_node(
            group_3,
            Similarity3::identity(),
            CameraID(hash!("Test camera")),
        );

        let transform =
            scene_graph.compute_root_to_camera_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(transform, Similarity3::identity());
    }

    #[test]
    fn computing_root_to_camera_transform_with_different_model_transforms_works() {
        let translation = Translation3::new(2.1, -5.9, 0.01);
        let rotation = Rotation3::from_euler_angles(0.1, 0.2, 0.3);
        let scaling = 7.0;

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();
        let group_1 = scene_graph.create_group_node(
            root,
            Similarity3::from_parts(translation, Rotation3::identity().into(), 1.0),
        );
        let group_2 = scene_graph.create_group_node(
            group_1,
            Similarity3::from_parts(Translation3::identity(), rotation.into(), 1.0),
        );
        let camera = scene_graph.create_camera_node(
            group_2,
            Similarity3::from_parts(
                Translation3::identity(),
                Rotation3::identity().into(),
                scaling,
            ),
            CameraID(hash!("Test camera")),
        );

        let transform =
            scene_graph.compute_root_to_camera_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(
            transform.to_homogeneous(),
            Scale3::new(scaling, scaling, scaling).to_homogeneous()
                * rotation.to_homogeneous()
                * translation.to_homogeneous(),
            epsilon = 1e-9
        );
    }

    fn assert_spheres_equal<F: Float>(sphere_1: &Sphere<F>, sphere_2: &Sphere<F>) {
        assert_abs_diff_eq!(sphere_1.center(), sphere_2.center());
        assert_abs_diff_eq!(sphere_1.radius(), sphere_2.radius());
    }

    #[test]
    fn updating_bounding_spheres_with_one_transformed_instance_in_world_space_works() {
        let instance_transform = Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            7.0,
        );
        let bounding_sphere = Sphere::new(point![3.9, 5.2, 0.0], 11.1);

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();

        let instance = scene_graph.create_model_instance_node(
            root,
            instance_transform,
            ModelID(hash!("Test model")),
            bounding_sphere.clone(),
        );

        let root_bounding_sphere = scene_graph.update_bounding_spheres(root);
        assert_spheres_equal(
            &root_bounding_sphere.unwrap(),
            &bounding_sphere.transformed(&instance_transform),
        );
        assert_spheres_equal(
            &scene_graph.find_model_instance_bounding_sphere(instance),
            &bounding_sphere.transformed(&instance_transform),
        )
    }

    #[test]
    fn updating_bounding_spheres_with_two_instances_in_world_space_works() {
        let bounding_sphere_1 = Sphere::new(point![3.9, 5.2, 0.0], 11.1);
        let bounding_sphere_2 = Sphere::new(point![-0.4, 7.7, 2.9], 4.8);

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();

        scene_graph.create_model_instance_node(
            root,
            Similarity3::identity(),
            ModelID(hash!("Test model 1")),
            bounding_sphere_1.clone(),
        );
        scene_graph.create_model_instance_node(
            root,
            Similarity3::identity(),
            ModelID(hash!("Test model 2")),
            bounding_sphere_2.clone(),
        );

        let root_bounding_sphere = scene_graph.update_bounding_spheres(root);
        assert_spheres_equal(
            &root_bounding_sphere.unwrap(),
            &Sphere::bounding_sphere_from_pair(&bounding_sphere_1, &bounding_sphere_2),
        );
    }

    #[test]
    fn updating_bounding_spheres_with_nested_instances_works() {
        let bounding_sphere_1 = Sphere::new(point![3.9, 5.2, 0.0], 11.1);
        let bounding_sphere_2 = Sphere::new(point![-0.4, 7.7, 2.9], 4.8);

        let group_1_transform = Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::identity().into(),
            2.0,
        );
        let group_2_transform = Similarity3::from_parts(
            Translation3::new(0.01, 2.9, 10.1),
            Rotation3::from_euler_angles(1.1, 2.2, 3.3).into(),
            0.2,
        );
        let instance_2_transform = Similarity3::from_parts(
            Translation3::new(-2.1, 8.9, 1.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            1.0,
        );

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();

        let group_1 = scene_graph.create_group_node(root, group_1_transform);
        scene_graph.create_model_instance_node(
            group_1,
            Similarity3::identity(),
            ModelID(hash!("Test model 1")),
            bounding_sphere_1.clone(),
        );
        let group_2 = scene_graph.create_group_node(group_1, group_2_transform);
        scene_graph.create_model_instance_node(
            group_2,
            instance_2_transform,
            ModelID(hash!("Test model 2")),
            bounding_sphere_2.clone(),
        );

        let correct_group_2_bounding_sphere = bounding_sphere_2.transformed(&instance_2_transform);
        let correct_group_1_bounding_sphere = Sphere::bounding_sphere_from_pair(
            &bounding_sphere_1,
            &correct_group_2_bounding_sphere.transformed(&group_2_transform),
        );
        let correct_root_bounding_sphere =
            correct_group_1_bounding_sphere.transformed(&group_1_transform);

        let root_bounding_sphere = scene_graph.update_bounding_spheres(root);

        assert_spheres_equal(
            &root_bounding_sphere.unwrap(),
            &correct_root_bounding_sphere,
        );

        assert_spheres_equal(
            scene_graph
                .group_nodes
                .node(group_1)
                .get_bounding_sphere()
                .unwrap(),
            &correct_group_1_bounding_sphere,
        );

        assert_spheres_equal(
            scene_graph
                .group_nodes
                .node(group_2)
                .get_bounding_sphere()
                .unwrap(),
            &correct_group_2_bounding_sphere,
        );
    }

    #[test]
    fn branch_without_model_instance_child_has_no_bounding_spheres() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();
        let group_1 = scene_graph.create_group_node(root, Similarity3::identity());
        let group_2 = scene_graph.create_group_node(group_1, Similarity3::identity());
        let root_bounding_sphere = scene_graph.update_bounding_spheres(root);
        assert!(root_bounding_sphere.is_none());
        assert!(scene_graph
            .group_nodes
            .node(group_1)
            .get_bounding_sphere()
            .is_none());
        assert!(scene_graph
            .group_nodes
            .node(group_2)
            .get_bounding_sphere()
            .is_none());
    }
}