//! Scene graph implementation.

mod components;

pub use components::{
    register_scene_graph_components, ParentComp, SceneGraphCameraNodeComp, SceneGraphGroupComp,
    SceneGraphGroupNodeComp, SceneGraphModelInstanceNodeComp, SceneGraphNodeComp,
    SceneGraphParentNodeComp, UncullableComp,
};

use crate::{
    geometry::{
        CubemapFace, Frustum, InstanceFeature, InstanceFeatureID, InstanceModelLightTransform,
        InstanceModelViewTransform, Sphere,
    },
    num::Float,
    rendering::{fre, CascadeIdx},
    scene::{
        InstanceFeatureManager, LightStorage, ModelID, OmnidirectionalLight, SceneCamera,
        UnidirectionalLight, MAX_SHADOW_MAP_CASCADES,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{GenerationalIdx, GenerationalReusingVec};
use nalgebra::Similarity3;
use std::collections::HashSet;

/// A tree structure that defines a spatial hierarchy of objects in the world
/// and enables useful operations on them.
///
/// The scene graph can contain leaf nodes representing model instances and
/// cameras. Every leaf node has a parent "group" node, which itself has a group
/// node as a parent and may have any number and type of children. Each node
/// holds a transform from the model space of the object or group it represents
/// to the space of the parent.
#[derive(Clone, Debug)]
pub struct SceneGraph<F: Float> {
    root_node_id: GroupNodeID,
    group_nodes: NodeStorage<GroupNode<F>>,
    model_instance_nodes: NodeStorage<ModelInstanceNode<F>>,
    model_instance_cluster_nodes: NodeStorage<ModelInstanceClusterNode<F>>,
    camera_nodes: NodeStorage<CameraNode<F>>,
}

/// Flat storage for all the [`SceneGraph`] nodes of a given
/// type.
#[derive(Clone, Debug, Default)]
pub struct NodeStorage<N> {
    nodes: GenerationalReusingVec<N>,
}

/// Type of the transform used by scene graph nodes.
pub type NodeTransform<F> = Similarity3<F>;

/// Represents a type of node in a [`SceneGraph`].
pub trait SceneGraphNode {
    /// Type of the node's ID.
    type ID: SceneGraphNodeID;
    /// Floating point type used for the node's transform.
    type F: Float;
}

/// Represents the ID of a type of node in a [`SceneGraph`].
pub trait SceneGraphNodeID: NodeIDToIdx + IdxToNodeID + Pod {}

/// Identifier for a [`GroupNode`] in a [`SceneGraph`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct GroupNodeID(GenerationalIdx);

/// Identifier for a [`ModelInstanceNode`] in a [`SceneGraph`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ModelInstanceNodeID(GenerationalIdx);

/// Identifier for a [`ModelInstanceClusterNode`] in a [`SceneGraph`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ModelInstanceClusterNodeID(GenerationalIdx);

/// Identifier for a [`CameraNode`] in a [`SceneGraph`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct CameraNodeID(GenerationalIdx);

/// Represents a type of node identifier that may provide an associated index.
pub trait NodeIDToIdx {
    /// Returns the index corresponding to the node ID.
    fn idx(&self) -> GenerationalIdx;
}

/// Represents a type of node identifier that may be created from an associated
/// index.
pub trait IdxToNodeID {
    /// Creates the node ID corresponding to the given index.
    fn from_idx(idx: GenerationalIdx) -> Self;
}

/// A [`SceneGraph`] node that has a group of other nodes as children. The
/// children may be [`ModelInstanceNode`]s, [`ModelInstanceClusterNode`]s,
/// [`CameraNode`]s and/or other group nodes. It holds a transform representing
/// group's spatial relationship with its parent group.
#[derive(Clone, Debug)]
pub struct GroupNode<F: Float> {
    parent_node_id: Option<GroupNodeID>,
    group_to_parent_transform: NodeTransform<F>,
    child_group_node_ids: HashSet<GroupNodeID>,
    child_model_instance_node_ids: HashSet<ModelInstanceNodeID>,
    child_model_instance_cluster_node_ids: HashSet<ModelInstanceClusterNodeID>,
    child_camera_node_ids: HashSet<CameraNodeID>,
    bounding_sphere: Option<Sphere<F>>,
    group_to_root_transform: NodeTransform<F>,
}

/// A [`SceneGraph`] leaf node representing a model instance. It holds a
/// transform representing the instance's spatial relationship with its parent
/// group, as well as a list of instance feature IDs.
#[derive(Clone, Debug)]
pub struct ModelInstanceNode<F: Float> {
    parent_node_id: GroupNodeID,
    model_bounding_sphere: Option<Sphere<F>>,
    model_to_parent_transform: NodeTransform<F>,
    model_id: ModelID,
    feature_ids: Vec<InstanceFeatureID>,
}

/// A [`SceneGraph`] leaf node representing a cluster of model instance. It
/// holds a list of transforms representing the spatial relationship of each
/// instance in the cluster with their parent group, as well as a nested list of
/// instance feature IDs, one list for each feature type. It also holds a
/// bounding sphere for the entire cluster, defined in the parent space.
///
/// The usecase for this node type compared to [`ModelInstanceNode`]s is to
/// efficiently handle a large group of instances, like a block of voxels, that
/// should not be culled individually.
#[derive(Clone, Debug)]
pub struct ModelInstanceClusterNode<F: Float> {
    parent_node_id: GroupNodeID,
    cluster_bounding_sphere: Option<Sphere<F>>,
    model_to_parent_transforms: Vec<NodeTransform<F>>,
    model_id: ModelID,
    feature_ids: Vec<Vec<InstanceFeatureID>>,
}

/// A [`SceneGraph`] leaf node representing a
/// [`Camera`](crate::geometry::Camera). It holds at transform representing the
/// camera's spatial relationship with its parent group.
#[derive(Clone, Debug)]
pub struct CameraNode<F: Float> {
    parent_node_id: GroupNodeID,
    camera_to_parent_transform: NodeTransform<F>,
}

impl<F: Float> SceneGraph<F> {
    /// Creates a new empty scene graph.
    pub fn new() -> Self {
        let mut group_nodes = NodeStorage::new();
        let model_instance_nodes = NodeStorage::new();
        let model_instance_cluster_nodes = NodeStorage::new();
        let camera_nodes = NodeStorage::new();

        let root_node_id = group_nodes.add_node(GroupNode::root(Similarity3::identity()));

        Self {
            root_node_id,
            group_nodes,
            model_instance_nodes,
            model_instance_cluster_nodes,
            camera_nodes,
        }
    }

    /// Returns the ID of the root group node.
    pub fn root_node_id(&self) -> GroupNodeID {
        self.root_node_id
    }

    /// Returns a reference to the storage of group nodes in the scene graph.
    pub fn group_nodes(&self) -> &NodeStorage<GroupNode<F>> {
        &self.group_nodes
    }

    /// Returns a reference to the storage of model instance nodes in the scene
    /// graph.
    pub fn model_instance_nodes(&self) -> &NodeStorage<ModelInstanceNode<F>> {
        &self.model_instance_nodes
    }

    /// Returns a reference to the storage of model instance cluter nodes in the
    /// scene graph.
    pub fn model_instance_cluster_nodes(&self) -> &NodeStorage<ModelInstanceClusterNode<F>> {
        &self.model_instance_cluster_nodes
    }

    /// Returns a reference to the storage of camera nodes in the scene graph.
    pub fn camera_nodes(&self) -> &NodeStorage<CameraNode<F>> {
        &self.camera_nodes
    }

    /// Returns a mutable reference to the model instance cluster node with the
    /// given ID.
    ///
    /// # Panics
    /// If no model instance cluster node with the given ID exists.
    pub fn model_instance_cluster_node_mut(
        &mut self,
        model_instance_cluster_node_id: ModelInstanceClusterNodeID,
    ) -> &mut ModelInstanceClusterNode<F> {
        self.model_instance_cluster_nodes
            .node_mut(model_instance_cluster_node_id)
    }

    /// Creates a new empty [`GroupNode`] with the given parent and
    /// parent-to-model transform and includes it in the scene graph.
    ///
    /// # Returns
    /// The ID of the created group node.
    ///
    /// # Panics
    /// If the specified parent group node does not exist.
    pub fn create_group_node(
        &mut self,
        parent_node_id: GroupNodeID,
        group_to_parent_transform: NodeTransform<F>,
    ) -> GroupNodeID {
        let group_node = GroupNode::non_root(parent_node_id, group_to_parent_transform);
        let group_node_id = self.group_nodes.add_node(group_node);
        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_group_node(group_node_id);
        group_node_id
    }

    /// Creates a new [`ModelInstanceNode`] for an instance of the model with
    /// the given ID, bounding sphere for frustum culling and feature IDs. It is
    /// included in the scene graph with the given transform relative to the the
    /// given parent node.
    ///
    /// If no bounding sphere is provided, the model will not be frustum culled.
    ///
    /// # Returns
    /// The ID of the created model instance node.
    ///
    /// # Panics
    /// - If the specified parent group node does not exist.
    /// - If no bounding sphere is provided when the parent node is not the root
    ///   node.
    pub fn create_model_instance_node(
        &mut self,
        parent_node_id: GroupNodeID,
        model_to_parent_transform: NodeTransform<F>,
        model_id: ModelID,
        frustum_culling_bounding_sphere: Option<Sphere<F>>,
        feature_ids: Vec<InstanceFeatureID>,
    ) -> ModelInstanceNodeID {
        // Since we don't guarantee that any other parent node than the root is
        // never culled, allowing a non-root node to have an uncullable child
        // could lead to unexpected behavior, so we disallow it
        assert!(
            frustum_culling_bounding_sphere.is_some() || parent_node_id == self.root_node_id(),
            "Tried to create model instance node without bounding sphere and with a non-root parent"
        );

        let model_instance_node = ModelInstanceNode::new(
            parent_node_id,
            frustum_culling_bounding_sphere,
            model_to_parent_transform,
            model_id,
            feature_ids,
        );

        let model_instance_node_id = self.model_instance_nodes.add_node(model_instance_node);

        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_model_instance_node(model_instance_node_id);

        model_instance_node_id
    }

    /// Creates a new [`ModelInstanceClusterNode`] for a collection of instances
    /// of the model with the given ID and feature IDs, using the given bounding
    /// sphere, defined in the cluster's reference frame, for frustum culling.
    /// The node is included in the scene graph with the given collection of
    /// transforms, one for each instance, relative to the the given parent
    /// node.
    ///
    /// If no bounding sphere is provided, the collection of instances will not
    /// be frustum culled.
    ///
    /// # Returns
    /// The ID of the created model instance cluster node.
    ///
    /// # Panics
    /// - If the specified parent group node does not exist.
    /// - If no bounding sphere is provided when the parent node is not the root
    ///   node.
    /// - If the number of elements in each feature ID [`Vec`] is not equal to
    ///   the given number of transforms.
    pub fn create_model_instance_cluster_node(
        &mut self,
        parent_node_id: GroupNodeID,
        model_to_parent_transforms: Vec<NodeTransform<F>>,
        model_id: ModelID,
        cluster_bounding_sphere: Option<Sphere<F>>,
        feature_ids: Vec<Vec<InstanceFeatureID>>,
    ) -> ModelInstanceClusterNodeID {
        // Since we don't guarantee that any other parent node than the root is
        // never culled, allowing a non-root node to have an uncullable child
        // could lead to unexpected behavior, so we disallow it
        assert!(
            cluster_bounding_sphere.is_some() || parent_node_id == self.root_node_id(),
            "Tried to create model instance cluster node without bounding sphere and with a non-root parent"
        );

        let model_instance_cluster_node = ModelInstanceClusterNode::new(
            parent_node_id,
            cluster_bounding_sphere,
            model_to_parent_transforms,
            model_id,
            feature_ids,
        );

        let model_instance_cluster_node_id = self
            .model_instance_cluster_nodes
            .add_node(model_instance_cluster_node);

        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_model_instance_cluster_node(model_instance_cluster_node_id);

        model_instance_cluster_node_id
    }

    /// Creates a new [`CameraNode`] with the given transform relative to the
    /// the given parent node.
    ///
    /// # Returns
    /// The ID of the created camera node.
    ///
    /// # Panics
    /// If the specified parent group node does not exist.
    pub fn create_camera_node(
        &mut self,
        parent_node_id: GroupNodeID,
        camera_to_parent_transform: NodeTransform<F>,
    ) -> CameraNodeID {
        let camera_node = CameraNode::new(parent_node_id, camera_to_parent_transform);
        let camera_node_id = self.camera_nodes.add_node(camera_node);
        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_camera_node(camera_node_id);
        camera_node_id
    }

    /// Removes the [`GroupNode`] with the given ID and all its children from
    /// the scene graph.
    ///
    /// # Panics
    /// - If the specified group node does not exist.
    /// - If the specified group node is the root node.
    pub fn remove_group_node(&mut self, group_node_id: GroupNodeID) {
        let group_node = self.group_nodes.node(group_node_id);

        let parent_node_id = group_node.parent_node_id();

        let (
            child_group_node_ids,
            child_model_instance_node_ids,
            child_model_instance_cluster_node_ids,
            child_camera_node_ids,
        ) = group_node.obtain_child_node_ids();

        for child_group_node_id in child_group_node_ids {
            self.remove_group_node(child_group_node_id);
        }
        for child_model_instance_node_id in child_model_instance_node_ids {
            self.remove_model_instance_node(child_model_instance_node_id);
        }
        for child_model_instance_cluster_node_id in child_model_instance_cluster_node_ids {
            self.remove_model_instance_cluster_node(child_model_instance_cluster_node_id);
        }
        for child_camera_node_id in child_camera_node_ids {
            self.remove_camera_node(child_camera_node_id);
        }

        self.group_nodes.remove_node(group_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_group_node(group_node_id);
    }

    /// Removes the [`ModelInstanceNode`] with the given ID from the scene
    /// graph.
    ///
    /// # Panics
    /// If the specified model instance node does not exist.
    pub fn remove_model_instance_node(
        &mut self,
        model_instance_node_id: ModelInstanceNodeID,
    ) -> ModelID {
        let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);
        let model_id = model_instance_node.model_id();
        let parent_node_id = model_instance_node.parent_node_id();
        self.model_instance_nodes
            .remove_node(model_instance_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_model_instance_node(model_instance_node_id);
        model_id
    }

    /// Removes the [`ModelInstanceClusterNode`] with the given ID from the
    /// scene graph.
    ///
    /// # Panics
    /// If the specified model instance cluster node does not exist.
    pub fn remove_model_instance_cluster_node(
        &mut self,
        model_instance_cluster_node_id: ModelInstanceClusterNodeID,
    ) -> ModelID {
        let model_instance_cluster_node = self
            .model_instance_cluster_nodes
            .node(model_instance_cluster_node_id);
        let model_id = model_instance_cluster_node.model_id();
        let parent_node_id = model_instance_cluster_node.parent_node_id();
        self.model_instance_cluster_nodes
            .remove_node(model_instance_cluster_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_model_instance_cluster_node(model_instance_cluster_node_id);
        model_id
    }

    /// Removes the [`CameraNode`] with the given ID from the scene graph.
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

    /// Sets the given transform as the parent-to-model transform for the
    /// [`GroupNode`] with the given ID.
    pub fn set_group_to_parent_transform(
        &mut self,
        group_node_id: GroupNodeID,
        transform: NodeTransform<<GroupNode<F> as SceneGraphNode>::F>,
    ) {
        self.group_nodes
            .node_mut(group_node_id)
            .set_group_to_parent_transform(transform);
    }

    /// Sets the given transform as the model-to-parent transform for the
    /// [`ModelInstanceNode`] with the given ID.
    pub fn set_model_to_parent_transform(
        &mut self,
        model_instance_node_id: ModelInstanceNodeID,
        transform: NodeTransform<<ModelInstanceNode<F> as SceneGraphNode>::F>,
    ) {
        self.model_instance_nodes
            .node_mut(model_instance_node_id)
            .set_model_to_parent_transform(transform);
    }

    /// Sets the given transform as the camera-to-parent transform for the
    /// [`CameraNode`] with the given ID.
    pub fn set_camera_to_parent_transform(
        &mut self,
        camera_node_id: CameraNodeID,
        transform: NodeTransform<<CameraNode<F> as SceneGraphNode>::F>,
    ) {
        self.camera_nodes
            .node_mut(camera_node_id)
            .set_camera_to_parent_transform(transform);
    }

    /// Updates the transform from local space to the space of the root node for
    /// all group nodes in the scene graph.
    pub fn update_all_group_to_root_transforms(&mut self) {
        self.update_group_to_root_transforms(self.root_node_id(), NodeTransform::identity());
    }

    /// Updates the world-to-camera transform of the given scene camera based on
    /// the transforms of its node and parent nodes.
    ///
    /// # Warning
    /// Make sure to [`update_all_group_to_root_transforms`]  before calling
    /// this method if any group nodes have changed.
    pub fn sync_camera_view_transform(&self, scene_camera: &mut SceneCamera<F>) {
        let camera_node = self.camera_nodes.node(scene_camera.scene_graph_node_id());
        let view_transform = self.compute_view_transform(camera_node);
        scene_camera.set_view_transform(view_transform);
    }

    /// Updates the bounding spheres of all nodes in the scene graph.
    pub fn update_all_bounding_spheres(&mut self) {
        self.update_bounding_spheres(self.root_node_id());
    }

    /// Computes the model-to-camera space transforms of all the model instances
    /// in the scene graph that are visible with the specified camera and adds
    /// them to the given instance feature manager.
    ///
    /// # Warning
    /// Make sure to [`update_all_bounding_spheres`] and
    /// [`compute_view_transform`] before calling this method if any nodes have
    /// changed.
    pub fn buffer_transforms_of_visible_model_instances(
        &self,
        instance_feature_manager: &mut InstanceFeatureManager,
        scene_camera: &SceneCamera<F>,
    ) where
        InstanceModelViewTransform: InstanceFeature,
        F: simba::scalar::SubsetOf<fre>,
    {
        let root_node = self.group_nodes.node(self.root_node_id());

        let camera_space_view_frustum = scene_camera.camera().view_frustum();
        let root_to_camera_transform = scene_camera.view_transform();

        for &group_node_id in root_node.child_group_node_ids() {
            let group_node = self.group_nodes.node(group_node_id);

            let group_to_camera_transform =
                root_to_camera_transform * group_node.group_to_parent_transform();

            let should_buffer = if let Some(bounding_sphere) = group_node.get_bounding_sphere() {
                let bounding_sphere_camera_space =
                    bounding_sphere.transformed(&group_to_camera_transform);

                !camera_space_view_frustum.sphere_lies_outside(&bounding_sphere_camera_space)
            } else {
                // If the group has no bounding sphere, buffer it unconditionally
                true
            };

            if should_buffer {
                self.buffer_transforms_of_visible_model_instances_in_group(
                    instance_feature_manager,
                    camera_space_view_frustum,
                    group_node,
                    &group_to_camera_transform,
                );
            }
        }

        for &model_instance_node_id in root_node.child_model_instance_node_ids() {
            let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

            let model_view_transform =
                root_to_camera_transform * model_instance_node.model_to_parent_transform();

            let should_buffer = if let Some(bounding_sphere) =
                model_instance_node.get_model_bounding_sphere()
            {
                let child_bounding_sphere_camera_space =
                    bounding_sphere.transformed(&model_view_transform);

                !camera_space_view_frustum.sphere_lies_outside(&child_bounding_sphere_camera_space)
            } else {
                // If the model has no bounding sphere, buffer it unconditionally
                true
            };

            if should_buffer {
                Self::buffer_model_view_transform_of_model_instance(
                    instance_feature_manager,
                    model_instance_node,
                    &model_view_transform,
                );
            }
        }

        for &model_instance_cluster_node_id in root_node.child_model_instance_cluster_node_ids() {
            let model_instance_cluster_node = self
                .model_instance_cluster_nodes
                .node(model_instance_cluster_node_id);

            let should_buffer = if let Some(bounding_sphere) =
                model_instance_cluster_node.get_cluster_bounding_sphere()
            {
                // The cluster bounding sphere is defined in its parent space
                let cluster_bounding_sphere_camera_space =
                    bounding_sphere.transformed(&root_to_camera_transform);

                !camera_space_view_frustum
                    .sphere_lies_outside(&cluster_bounding_sphere_camera_space)
            } else {
                // If the model has no bounding sphere, buffer it unconditionally
                true
            };

            if should_buffer {
                Self::buffer_model_view_transforms_of_model_instance_cluster(
                    instance_feature_manager,
                    model_instance_cluster_node,
                    &root_to_camera_transform,
                );
            }
        }
    }

    /// Updates the transform from local space to the space of the root node for
    /// the specified group node and all its children, by concatenating their
    /// group-to-parent transforms recursively.
    ///
    /// # Panics
    /// If the specified group node does not exist.
    fn update_group_to_root_transforms(
        &mut self,
        group_node_id: GroupNodeID,
        parent_to_root_transform: NodeTransform<F>,
    ) {
        let group_node = self.group_nodes.node_mut(group_node_id);

        let group_to_root_transform =
            parent_to_root_transform * group_node.group_to_parent_transform();

        group_node.set_group_to_root_transform(group_to_root_transform);

        for child_group_node_id in group_node.obtain_child_group_node_ids() {
            self.update_group_to_root_transforms(child_group_node_id, group_to_root_transform);
        }
    }

    /// Computes the transform from the scene graph's root node space to the
    /// space of the given camera node.
    fn compute_view_transform(&self, camera_node: &CameraNode<F>) -> NodeTransform<F> {
        let parent_node = self.group_nodes.node(camera_node.parent_node_id());
        camera_node.parent_to_camera_transform() * parent_node.root_to_group_transform()
    }

    /// Updates the bounding sphere of the specified group node and all its
    /// children. Each bounding sphere is defined in the local space of its
    /// group node.
    ///
    /// # Returns
    /// The bounding sphere of the specified group node, defined in the space of
    /// its parent group node (used for recursion).
    ///
    /// # Panics
    /// If the specified group node does not exist.
    fn update_bounding_spheres(&mut self, group_node_id: GroupNodeID) -> Option<Sphere<F>> {
        let group_node = self.group_nodes.node(group_node_id);

        let child_group_node_ids = group_node.obtain_child_group_node_ids();
        let model_instance_node_ids = group_node.obtain_child_model_instance_node_ids();
        let model_instance_cluster_node_ids =
            group_node.obtain_child_model_instance_cluster_node_ids();

        let mut child_bounding_spheres = Vec::with_capacity(
            child_group_node_ids.len()
                + model_instance_node_ids.len()
                + model_instance_cluster_node_ids.len(),
        );

        child_bounding_spheres.extend(
            child_group_node_ids
                .into_iter()
                .filter_map(|group_node_id| self.update_bounding_spheres(group_node_id)),
        );

        child_bounding_spheres.extend(model_instance_node_ids.into_iter().filter_map(
            |model_instance_node_id| {
                let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

                model_instance_node
                    .get_model_bounding_sphere()
                    .map(|bounding_sphere| {
                        bounding_sphere.transformed(model_instance_node.model_to_parent_transform())
                    })
            },
        ));

        child_bounding_spheres.extend(model_instance_cluster_node_ids.into_iter().filter_map(
            |model_instance_cluster_node_id| {
                // The cluster bounding sphere is defined in its parent space,
                // so we do not transform it
                self.model_instance_cluster_nodes
                    .node(model_instance_cluster_node_id)
                    .get_cluster_bounding_sphere()
                    .cloned()
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
                bounding_sphere.transformed(group_node.group_to_parent_transform());

            group_node.set_bounding_sphere(Some(bounding_sphere));

            Some(bounding_sphere_in_parent_space)
        }
    }

    /// Determines the group/model-to-camera transforms of the group nodes,
    /// model instance nodes and model instance cluster nodes that are children
    /// of the specified group node and whose bounding spheres lie within the
    /// given camera frustum. The given group-to-camera transform is prepended
    /// to the transforms of the children. For the children that are model
    /// instance nodes or model instance cluster nodes, their final
    /// model-to-camera transforms are added to the given instance feature
    /// manager.
    ///
    /// # Panics
    /// If any of the child nodes of the group node does not exist.
    fn buffer_transforms_of_visible_model_instances_in_group(
        &self,
        instance_feature_manager: &mut InstanceFeatureManager,
        camera_space_view_frustum: &Frustum<F>,
        group_node: &GroupNode<F>,
        group_to_camera_transform: &NodeTransform<F>,
    ) where
        InstanceModelViewTransform: InstanceFeature,
        F: simba::scalar::SubsetOf<fre>,
    {
        for &child_group_node_id in group_node.child_group_node_ids() {
            let child_group_node = self.group_nodes.node(child_group_node_id);

            let child_group_to_camera_transform =
                group_to_camera_transform * child_group_node.group_to_parent_transform();

            let should_buffer = if let Some(child_bounding_sphere) =
                child_group_node.get_bounding_sphere()
            {
                let child_bounding_sphere_camera_space =
                    child_bounding_sphere.transformed(&child_group_to_camera_transform);

                !camera_space_view_frustum.sphere_lies_outside(&child_bounding_sphere_camera_space)
            } else {
                // If the group has no bounding sphere, buffer it unconditionally
                true
            };

            if should_buffer {
                self.buffer_transforms_of_visible_model_instances_in_group(
                    instance_feature_manager,
                    camera_space_view_frustum,
                    child_group_node,
                    &child_group_to_camera_transform,
                );
            }
        }

        for &child_model_instance_node_id in group_node.child_model_instance_node_ids() {
            let child_model_instance_node =
                self.model_instance_nodes.node(child_model_instance_node_id);

            let child_model_view_transform =
                group_to_camera_transform * child_model_instance_node.model_to_parent_transform();

            let should_buffer = if let Some(child_bounding_sphere) =
                child_model_instance_node.get_model_bounding_sphere()
            {
                let child_bounding_sphere_camera_space =
                    child_bounding_sphere.transformed(&child_model_view_transform);

                !camera_space_view_frustum.sphere_lies_outside(&child_bounding_sphere_camera_space)
            } else {
                // If the model has no bounding sphere, buffer it unconditionally
                true
            };

            if should_buffer {
                Self::buffer_model_view_transform_of_model_instance(
                    instance_feature_manager,
                    child_model_instance_node,
                    &child_model_view_transform,
                );
            }
        }

        for &child_model_instance_cluster_node_id in
            group_node.child_model_instance_cluster_node_ids()
        {
            let child_model_instance_cluster_node = self
                .model_instance_cluster_nodes
                .node(child_model_instance_cluster_node_id);

            let should_buffer = if let Some(child_bounding_sphere) =
                child_model_instance_cluster_node.get_cluster_bounding_sphere()
            {
                // The cluster bounding sphere is defined in its parent space
                let cluster_bounding_sphere_camera_space =
                    child_bounding_sphere.transformed(group_to_camera_transform);

                !camera_space_view_frustum
                    .sphere_lies_outside(&cluster_bounding_sphere_camera_space)
            } else {
                // If the model has no bounding sphere, buffer it unconditionally
                true
            };

            if should_buffer {
                Self::buffer_model_view_transforms_of_model_instance_cluster(
                    instance_feature_manager,
                    child_model_instance_cluster_node,
                    group_to_camera_transform,
                );
            }
        }
    }

    /// Adds an instance corresponding to the given model instance node with the
    /// given model-to-camera transform to the given instance feature manager.
    ///
    /// # Panics
    /// If the specified model instance node does not exist.
    fn buffer_model_view_transform_of_model_instance(
        instance_feature_manager: &mut InstanceFeatureManager,
        model_instance_node: &ModelInstanceNode<F>,
        model_view_transform: &NodeTransform<F>,
    ) where
        InstanceModelViewTransform: InstanceFeature,
        F: simba::scalar::SubsetOf<fre>,
    {
        let instance_model_view_transform =
            InstanceModelViewTransform::with_model_view_transform(model_view_transform.cast());

        instance_feature_manager.buffer_instance(
            model_instance_node.model_id(),
            &instance_model_view_transform,
            model_instance_node.feature_ids(),
        );
    }

    /// Adds instances corresponding to the given model instance cluster node
    /// with the given model-to-camera transforms to the given instance feature
    /// manager.
    ///
    /// Prepends the given parent group-to-camera transform to each
    /// model-to-parent transform in the specified model instance cluster node
    /// and adds instances with each resulting transform to the given instance
    /// feature manager.
    ///
    /// # Panics
    /// If the specified model instance cluster node does not exist.
    fn buffer_model_view_transforms_of_model_instance_cluster(
        instance_feature_manager: &mut InstanceFeatureManager,
        model_instance_cluster_node: &ModelInstanceClusterNode<F>,
        parent_to_camera_transform: &NodeTransform<F>,
    ) where
        InstanceModelViewTransform: InstanceFeature,
        F: simba::scalar::SubsetOf<fre>,
    {
        let instance_model_view_transforms: Vec<_> = model_instance_cluster_node
            .model_to_parent_transforms()
            .iter()
            .map(|transform| {
                InstanceModelViewTransform::with_model_view_transform(
                    (parent_to_camera_transform * transform).cast(),
                )
            })
            .collect();

        instance_feature_manager.buffer_multiple_instances(
            model_instance_cluster_node.model_id(),
            &instance_model_view_transforms,
            model_instance_cluster_node.feature_ids(),
        );
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
    fn node_has_model_instance_cluster_node_as_child(
        &self,
        group_node_id: GroupNodeID,
        child_model_instance_cluster_node_id: ModelInstanceClusterNodeID,
    ) -> bool {
        self.group_nodes
            .node(group_node_id)
            .child_model_instance_cluster_node_ids()
            .contains(&child_model_instance_cluster_node_id)
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

impl SceneGraph<fre> {
    /// Goes through all omnidirectional lights in the given light storage and
    /// updates their cubemap orientations and distance spans to encompass all
    /// model instances that may cast visible shadows in a way that preserves
    /// quality and efficiency. Then the model to cubemap face space transform
    /// of every such shadow casting model instance is computed for the relevant
    /// cube faces of each light and copied to the model's instance transform
    /// buffer in new ranges dedicated to the faces of the cubemap of the
    /// particular light.
    ///
    /// # Warning
    /// Make sure to [`buffer_transforms_of_visible_model_instances`] before
    /// calling this method, so that the ranges of model to cubemap face
    /// transforms in the model instance buffers come after the initial range
    /// containing model to camera transforms.
    pub fn bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
        &self,
        light_storage: &mut LightStorage,
        instance_feature_manager: &mut InstanceFeatureManager,
        scene_camera: &SceneCamera<fre>,
    ) {
        let camera_space_view_frustum = scene_camera.camera().view_frustum();
        let view_transform = scene_camera.view_transform();

        let root_node_id = self.root_node_id();
        let root_node = self.group_nodes.node(root_node_id);

        if let Some(world_space_bounding_sphere) = root_node.get_bounding_sphere() {
            let camera_space_bounding_sphere =
                world_space_bounding_sphere.transformed(view_transform);

            for (light_id, omnidirectional_light) in
                light_storage.omnidirectional_lights_with_ids_mut()
            {
                let camera_space_aabb_for_visible_models = camera_space_bounding_sphere
                    .compute_aabb()
                    .union_with(&camera_space_view_frustum.compute_aabb());

                omnidirectional_light.orient_and_scale_cubemap_for_shadow_casting_models(
                    &camera_space_bounding_sphere,
                    camera_space_aabb_for_visible_models.as_ref(),
                );

                for face in CubemapFace::all() {
                    // Begin a new range dedicated for tranforms to the current
                    // cubemap face space for the current light at the end of
                    // each transform buffer, identified by the light's ID plus
                    // a face index offset
                    for buffer in instance_feature_manager.transform_buffers_mut() {
                        buffer.begin_range(
                            light_id.as_instance_feature_buffer_range_id() + face.as_idx_u32(),
                        );
                    }

                    let camera_space_face_frustum =
                        omnidirectional_light.compute_camera_space_frustum_for_face(face);

                    if OmnidirectionalLight::camera_space_frustum_for_face_may_contain_visible_models(
                        camera_space_aabb_for_visible_models.as_ref(),
                        &camera_space_face_frustum,
                    ) {
                        self.buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_omnidirectional_light_cubemap_face(
                            instance_feature_manager,
                            omnidirectional_light,
                            face,
                            &camera_space_face_frustum,
                            root_node,
                            view_transform,
                        );
                    }
                }
            }
        }
    }

    /// Goes through all unidirectional lights in the given light storage and
    /// updates their orthographic transforms to encompass model instances that
    /// may cast visible shadows inside the corresponding cascades in the view
    /// frustum. Then the model to light transform of every such shadow casting
    /// model instance is computed for each light and copied to the model's
    /// instance transform buffer in a new range dedicated to the particular
    /// light and cascade.
    ///
    /// # Warning
    /// Make sure to [`buffer_transforms_of_visible_model_instances`] before
    /// calling this method, so that the ranges of model to light transforms in
    /// the model instance buffers come after the initial range containing model
    /// to camera transforms.
    pub fn bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
        &self,
        light_storage: &mut LightStorage,
        instance_feature_manager: &mut InstanceFeatureManager,
        scene_camera: &SceneCamera<fre>,
    ) {
        let camera_space_view_frustum = scene_camera.camera().view_frustum();
        let view_transform = scene_camera.view_transform();

        let root_node_id = self.root_node_id();
        let root_node = self.group_nodes.node(root_node_id);

        if let Some(world_space_bounding_sphere) = root_node.get_bounding_sphere() {
            let camera_space_bounding_sphere =
                world_space_bounding_sphere.transformed(view_transform);

            for (light_id, unidirectional_light) in
                light_storage.unidirectional_lights_with_ids_mut()
            {
                unidirectional_light.update_cascade_partition_depths(
                    camera_space_view_frustum,
                    &camera_space_bounding_sphere,
                );

                unidirectional_light.bound_orthographic_transforms_to_cascaded_view_frustum(
                    camera_space_view_frustum,
                    &camera_space_bounding_sphere,
                );

                for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                    // Begin a new range dedicated for tranforms to the current
                    // light's space for instances casting shadows in he current
                    // cascade at the end of each transform buffer, identified
                    // by the light's ID plus a cascade index offset
                    for buffer in instance_feature_manager.transform_buffers_mut() {
                        buffer.begin_range(
                            light_id.as_instance_feature_buffer_range_id() + cascade_idx,
                        );
                    }

                    self.buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_unidirectional_light_cascade(
                        instance_feature_manager,
                        unidirectional_light,
                        cascade_idx,
                        root_node,
                        view_transform,
                    );
                }
            }
        }
    }

    fn buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_omnidirectional_light_cubemap_face(
        &self,
        instance_feature_manager: &mut InstanceFeatureManager,
        omnidirectional_light: &OmnidirectionalLight,
        face: CubemapFace,
        camera_space_face_frustum: &Frustum<fre>,
        group_node: &GroupNode<fre>,
        group_to_camera_transform: &NodeTransform<fre>,
    ) {
        for &child_group_node_id in group_node.child_group_node_ids() {
            let child_group_node = self.group_nodes.node(child_group_node_id);

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(child_bounding_sphere) = child_group_node.get_bounding_sphere() {
                let child_group_to_camera_transform =
                    group_to_camera_transform * child_group_node.group_to_parent_transform();

                let child_camera_space_bounding_sphere =
                    child_bounding_sphere.transformed(&child_group_to_camera_transform);

                if !camera_space_face_frustum
                    .sphere_lies_outside(&child_camera_space_bounding_sphere)
                {
                    self.buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_omnidirectional_light_cubemap_face(
                        instance_feature_manager,
                        omnidirectional_light,
                        face,
                        camera_space_face_frustum,
                        child_group_node,
                        &child_group_to_camera_transform,
                    );
                }
            }
        }

        for &model_instance_node_id in group_node.child_model_instance_node_ids() {
            let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(model_instance_bounding_sphere) =
                model_instance_node.get_model_bounding_sphere()
            {
                let model_instance_to_camera_transform =
                    group_to_camera_transform * model_instance_node.model_to_parent_transform();

                let model_instance_camera_space_bounding_sphere =
                    model_instance_bounding_sphere.transformed(&model_instance_to_camera_transform);

                if !camera_space_face_frustum
                    .sphere_lies_outside(&model_instance_camera_space_bounding_sphere)
                {
                    let instance_model_light_transform =
                        InstanceModelLightTransform::with_model_light_transform(
                            omnidirectional_light
                                .create_transform_to_positive_z_cubemap_face_space(
                                    face,
                                    &model_instance_to_camera_transform,
                                ),
                        );

                    instance_feature_manager.buffer_instance_transform(
                        model_instance_node.model_id(),
                        &instance_model_light_transform,
                    );
                }
            }
        }

        for &model_instance_cluster_node_id in group_node.child_model_instance_cluster_node_ids() {
            let model_instance_cluster_node = self
                .model_instance_cluster_nodes
                .node(model_instance_cluster_node_id);

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(model_instance_cluster_bounding_sphere) =
                model_instance_cluster_node.get_cluster_bounding_sphere()
            {
                let model_instance_cluster_camera_space_bounding_sphere =
                    model_instance_cluster_bounding_sphere.transformed(group_to_camera_transform);

                if !camera_space_face_frustum
                    .sphere_lies_outside(&model_instance_cluster_camera_space_bounding_sphere)
                {
                    let group_to_cubemap_face_transform = omnidirectional_light
                        .create_transform_from_camera_space_to_positive_z_cubemap_face_space(face)
                        * group_to_camera_transform;

                    let instance_model_light_transforms: Vec<_> = model_instance_cluster_node
                        .model_to_parent_transforms()
                        .iter()
                        .map(|transform| {
                            InstanceModelLightTransform::with_model_light_transform(
                                group_to_cubemap_face_transform * transform,
                            )
                        })
                        .collect();

                    instance_feature_manager.buffer_multiple_instance_transforms(
                        model_instance_cluster_node.model_id(),
                        &instance_model_light_transforms,
                    );
                }
            }
        }
    }

    fn buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_unidirectional_light_cascade(
        &self,
        instance_feature_manager: &mut InstanceFeatureManager,
        unidirectional_light: &UnidirectionalLight,
        cascade_idx: CascadeIdx,
        group_node: &GroupNode<fre>,
        group_to_camera_transform: &NodeTransform<fre>,
    ) {
        for &child_group_node_id in group_node.child_group_node_ids() {
            let child_group_node = self.group_nodes.node(child_group_node_id);

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(child_bounding_sphere) = child_group_node.get_bounding_sphere() {
                let child_group_to_camera_transform =
                    group_to_camera_transform * child_group_node.group_to_parent_transform();

                let child_camera_space_bounding_sphere =
                    child_bounding_sphere.transformed(&child_group_to_camera_transform);

                if unidirectional_light.bounding_sphere_may_cast_visible_shadow_in_cascade(
                    cascade_idx,
                    &child_camera_space_bounding_sphere,
                ) {
                    self.buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_unidirectional_light_cascade(
                        instance_feature_manager,
                        unidirectional_light,
                        cascade_idx,
                        child_group_node,
                        &child_group_to_camera_transform,
                    );
                }
            }
        }

        for &model_instance_node_id in group_node.child_model_instance_node_ids() {
            let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(model_instance_bounding_sphere) =
                model_instance_node.get_model_bounding_sphere()
            {
                let model_instance_to_camera_transform =
                    group_to_camera_transform * model_instance_node.model_to_parent_transform();

                let model_instance_camera_space_bounding_sphere =
                    model_instance_bounding_sphere.transformed(&model_instance_to_camera_transform);

                if unidirectional_light.bounding_sphere_may_cast_visible_shadow_in_cascade(
                    cascade_idx,
                    &model_instance_camera_space_bounding_sphere,
                ) {
                    let instance_model_light_transform =
                        InstanceModelLightTransform::with_model_light_transform(
                            unidirectional_light.create_transform_to_light_space(
                                &model_instance_to_camera_transform,
                            ),
                        );

                    instance_feature_manager.buffer_instance_transform(
                        model_instance_node.model_id(),
                        &instance_model_light_transform,
                    );
                }
            }
        }

        for &model_instance_cluster_node_id in group_node.child_model_instance_cluster_node_ids() {
            let model_instance_cluster_node = self
                .model_instance_cluster_nodes
                .node(model_instance_cluster_node_id);

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(model_instance_cluster_bounding_sphere) =
                model_instance_cluster_node.get_cluster_bounding_sphere()
            {
                let model_instance_cluster_camera_space_bounding_sphere =
                    model_instance_cluster_bounding_sphere.transformed(group_to_camera_transform);

                if unidirectional_light.bounding_sphere_may_cast_visible_shadow_in_cascade(
                    cascade_idx,
                    &model_instance_cluster_camera_space_bounding_sphere,
                ) {
                    let group_to_light_transform = unidirectional_light
                        .camera_to_light_space_rotation()
                        * group_to_camera_transform;

                    let instance_model_light_transforms: Vec<_> = model_instance_cluster_node
                        .model_to_parent_transforms()
                        .iter()
                        .map(|transform| {
                            InstanceModelLightTransform::with_model_light_transform(
                                group_to_light_transform * transform,
                            )
                        })
                        .collect();

                    instance_feature_manager.buffer_multiple_instance_transforms(
                        model_instance_cluster_node.model_id(),
                        &instance_model_light_transforms,
                    );
                }
            }
        }
    }
}

impl<F: Float> Default for SceneGraph<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: SceneGraphNode> NodeStorage<N> {
    fn new() -> Self {
        Self {
            nodes: GenerationalReusingVec::new(),
        }
    }

    /// Returns the number of nodes in the storage.
    pub fn n_nodes(&self) -> usize {
        self.nodes.n_elements()
    }

    /// Whether a node with the given ID exists in the storage.
    pub fn has_node(&self, node_id: N::ID) -> bool {
        self.nodes.get_element(node_id.idx()).is_some()
    }

    /// Returns a reference to the node with the given ID.
    pub fn node(&self, node_id: N::ID) -> &N {
        self.nodes.element(node_id.idx())
    }

    fn node_mut(&mut self, node_id: N::ID) -> &mut N {
        self.nodes.element_mut(node_id.idx())
    }

    fn add_node(&mut self, node: N) -> N::ID {
        N::ID::from_idx(self.nodes.add_element(node))
    }

    fn remove_node(&mut self, node_id: N::ID) {
        self.nodes.free_element_at_idx(node_id.idx());
    }
}

impl<F: Float> GroupNode<F> {
    /// Returns the group-to-root transform for the node.
    pub fn group_to_root_transform(&self) -> &NodeTransform<F> {
        &self.group_to_root_transform
    }

    fn new(
        parent_node_id: Option<GroupNodeID>,
        group_to_parent_transform: NodeTransform<F>,
    ) -> Self {
        Self {
            parent_node_id,
            group_to_parent_transform,
            child_group_node_ids: HashSet::new(),
            child_model_instance_node_ids: HashSet::new(),
            child_model_instance_cluster_node_ids: HashSet::new(),
            child_camera_node_ids: HashSet::new(),
            bounding_sphere: None,
            group_to_root_transform: NodeTransform::identity(),
        }
    }

    fn root(transform: NodeTransform<F>) -> Self {
        Self::new(None, transform)
    }

    fn non_root(parent_node_id: GroupNodeID, transform: NodeTransform<F>) -> Self {
        Self::new(Some(parent_node_id), transform)
    }

    fn group_to_parent_transform(&self) -> &NodeTransform<F> {
        &self.group_to_parent_transform
    }

    fn root_to_group_transform(&self) -> NodeTransform<F> {
        self.group_to_root_transform.inverse()
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

    fn child_model_instance_cluster_node_ids(&self) -> &HashSet<ModelInstanceClusterNodeID> {
        &self.child_model_instance_cluster_node_ids
    }

    #[cfg(test)]
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

    fn obtain_child_model_instance_cluster_node_ids(&self) -> Vec<ModelInstanceClusterNodeID> {
        self.child_model_instance_cluster_node_ids
            .iter()
            .cloned()
            .collect()
    }

    fn obtain_child_camera_node_ids(&self) -> Vec<CameraNodeID> {
        self.child_camera_node_ids.iter().cloned().collect()
    }

    fn obtain_child_node_ids(
        &self,
    ) -> (
        Vec<GroupNodeID>,
        Vec<ModelInstanceNodeID>,
        Vec<ModelInstanceClusterNodeID>,
        Vec<CameraNodeID>,
    ) {
        (
            self.obtain_child_group_node_ids(),
            self.obtain_child_model_instance_node_ids(),
            self.obtain_child_model_instance_cluster_node_ids(),
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

    fn add_child_model_instance_cluster_node(
        &mut self,
        model_instance_cluster_node_id: ModelInstanceClusterNodeID,
    ) {
        self.child_model_instance_cluster_node_ids
            .insert(model_instance_cluster_node_id);
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

    fn remove_child_model_instance_cluster_node(
        &mut self,
        model_instance_cluster_node_id: ModelInstanceClusterNodeID,
    ) {
        self.child_model_instance_cluster_node_ids
            .remove(&model_instance_cluster_node_id);
    }

    fn remove_child_camera_node(&mut self, camera_node_id: CameraNodeID) {
        self.child_camera_node_ids.remove(&camera_node_id);
    }

    fn set_bounding_sphere(&mut self, bounding_sphere: Option<Sphere<F>>) {
        self.bounding_sphere = bounding_sphere;
    }

    fn set_group_to_root_transform(&mut self, group_to_root_transform: NodeTransform<F>) {
        self.group_to_root_transform = group_to_root_transform;
    }

    fn set_group_to_parent_transform(&mut self, transform: NodeTransform<F>) {
        self.group_to_parent_transform = transform;
    }
}

impl SceneGraphNodeID for GroupNodeID {}

impl<F: Float> SceneGraphNode for GroupNode<F> {
    type ID = GroupNodeID;
    type F = F;
}

impl<F: Float> ModelInstanceNode<F> {
    pub fn set_model_bounding_sphere(&mut self, bounding_sphere: Option<Sphere<F>>) {
        self.model_bounding_sphere = bounding_sphere;
    }

    fn new(
        parent_node_id: GroupNodeID,
        model_bounding_sphere: Option<Sphere<F>>,
        model_to_parent_transform: NodeTransform<F>,
        model_id: ModelID,
        feature_ids: Vec<InstanceFeatureID>,
    ) -> Self {
        Self {
            parent_node_id,
            model_bounding_sphere,
            model_to_parent_transform,
            model_id,
            feature_ids,
        }
    }

    /// Returns the ID of the parent [`GroupNode`].
    fn parent_node_id(&self) -> GroupNodeID {
        self.parent_node_id
    }

    /// Returns the parent-to-model transform for the node.
    pub fn parent_to_model_transform(&self) -> NodeTransform<F> {
        self.model_to_parent_transform.inverse()
    }

    /// Returns the model-to-parent transform for the node.
    pub fn model_to_parent_transform(&self) -> &NodeTransform<F> {
        &self.model_to_parent_transform
    }

    /// Returns the ID of the model the node represents an
    /// instance of.
    pub fn model_id(&self) -> ModelID {
        self.model_id
    }

    /// Returns the IDs of the instance's features.
    pub fn feature_ids(&self) -> &[InstanceFeatureID] {
        &self.feature_ids
    }

    /// Returns the bounding sphere of the model instance, or [`None`] if it has
    /// no bounding sphere.
    fn get_model_bounding_sphere(&self) -> Option<&Sphere<F>> {
        self.model_bounding_sphere.as_ref()
    }

    fn set_model_to_parent_transform(&mut self, transform: NodeTransform<F>) {
        self.model_to_parent_transform = transform;
    }
}

impl SceneGraphNodeID for ModelInstanceNodeID {}

impl<F: Float> SceneGraphNode for ModelInstanceNode<F> {
    type ID = ModelInstanceNodeID;
    type F = F;
}

impl<F: Float> ModelInstanceClusterNode<F> {
    pub fn set_cluster_bounding_sphere(&mut self, bounding_sphere: Option<Sphere<F>>) {
        self.cluster_bounding_sphere = bounding_sphere;
    }

    fn new(
        parent_node_id: GroupNodeID,
        cluster_bounding_sphere: Option<Sphere<F>>,
        model_to_parent_transforms: Vec<NodeTransform<F>>,
        model_id: ModelID,
        feature_ids: Vec<Vec<InstanceFeatureID>>,
    ) -> Self {
        let n_instances = model_to_parent_transforms.len();
        for feature_ids in &feature_ids {
            assert_eq!(
                feature_ids.len(),
                n_instances,
                "Tried to create model instance cluster node with inconsistent numbers of transforms and feature IDs"
            );
        }

        Self {
            parent_node_id,
            cluster_bounding_sphere,
            model_to_parent_transforms,
            model_id,
            feature_ids,
        }
    }

    /// Returns the ID of the parent [`GroupNode`].
    fn parent_node_id(&self) -> GroupNodeID {
        self.parent_node_id
    }

    /// Returns the number of instances in the cluster.
    pub fn n_instances(&self) -> usize {
        self.model_to_parent_transforms.len()
    }

    /// Returns an iterator over the parent-to-model transforms for the node.
    pub fn parent_to_model_transforms(&self) -> impl Iterator<Item = NodeTransform<F>> + '_ {
        self.model_to_parent_transforms
            .iter()
            .map(|transform| transform.inverse())
    }

    /// Returns the model-to-parent transforms for the node.
    pub fn model_to_parent_transforms(&self) -> &[NodeTransform<F>] {
        &self.model_to_parent_transforms
    }

    /// Returns the ID of the model the node represents an cluster of instance
    /// of.
    pub fn model_id(&self) -> ModelID {
        self.model_id
    }

    /// Returns the a instance feature IDs, one [`Vec`] for each feature type.
    pub fn feature_ids(&self) -> &[Vec<InstanceFeatureID>] {
        &self.feature_ids
    }

    /// Adds a model instance represented by the given model-to-parent transform
    /// and set of feature IDs to the cluster. This information will be pushed
    /// to the end of the [`Vec`]s holding the existing information.
    ///
    /// # Panics
    /// If the given number of feature IDs is different from the number of
    /// feature types in the cluster.
    pub fn push_instance(
        &mut self,
        model_to_parent_transform: NodeTransform<F>,
        feature_ids: &[InstanceFeatureID],
    ) {
        assert_eq!(
            feature_ids.len(),
            self.feature_ids.len(),
            "Tried to push instance with inconsistent feature ID count to model instance cluster node"
        );

        self.model_to_parent_transforms
            .push(model_to_parent_transform);

        for (existing_ids, &id) in self.feature_ids.iter_mut().zip(feature_ids.iter()) {
            existing_ids.push(id);
        }
    }

    /// Removes the transformation and feature IDs of the instance at the given
    /// index. The removed instance information will be replaced with that of
    /// the last instance, so this does not preserve ordering.
    ///
    /// # Panics
    /// If the given index is not smaller than the number of instances present.
    pub fn swap_remove_instance(&mut self, instance_idx: usize) {
        self.model_to_parent_transforms.swap_remove(instance_idx);

        for feature_ids in &mut self.feature_ids {
            feature_ids.swap_remove(instance_idx);
        }
    }

    /// Returns the bounding sphere of the model instance cluster, or [`None`]
    /// if it has no bounding sphere.
    fn get_cluster_bounding_sphere(&self) -> Option<&Sphere<F>> {
        self.cluster_bounding_sphere.as_ref()
    }
}

impl SceneGraphNodeID for ModelInstanceClusterNodeID {}

impl<F: Float> SceneGraphNode for ModelInstanceClusterNode<F> {
    type ID = ModelInstanceClusterNodeID;
    type F = F;
}

impl<F: Float> CameraNode<F> {
    fn new(parent_node_id: GroupNodeID, camera_to_parent_transform: NodeTransform<F>) -> Self {
        Self {
            parent_node_id,
            camera_to_parent_transform,
        }
    }

    /// Returns the ID of the parent [`GroupNode`].
    fn parent_node_id(&self) -> GroupNodeID {
        self.parent_node_id
    }

    /// Returns the parent-to-camera transform for the node.
    pub fn parent_to_camera_transform(&self) -> NodeTransform<F> {
        self.camera_to_parent_transform.inverse()
    }

    /// Returns the camera-to-parent transform for the node.
    pub fn camera_to_parent_transform(&self) -> &NodeTransform<F> {
        &self.camera_to_parent_transform
    }

    fn set_camera_to_parent_transform(&mut self, transform: NodeTransform<F>) {
        self.camera_to_parent_transform = transform;
    }
}

impl SceneGraphNodeID for CameraNodeID {}

impl<F: Float> SceneGraphNode for CameraNode<F> {
    type ID = CameraNodeID;
    type F = F;
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
impl_node_id_idx_traits!(ModelInstanceClusterNodeID);
impl_node_id_idx_traits!(CameraNodeID);

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        scene::MaterialID,
        scene::{MaterialHandle, MeshID},
    };
    use approx::assert_abs_diff_eq;
    use impact_utils::hash64;
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
        create_dummy_model_instance_node_with_transform(
            scene_graph,
            parent_node_id,
            Similarity3::identity(),
        )
    }

    fn create_dummy_model_instance_node_with_transform<F: Float>(
        scene_graph: &mut SceneGraph<F>,
        parent_node_id: GroupNodeID,
        model_to_parent_transform: Similarity3<F>,
    ) -> ModelInstanceNodeID {
        scene_graph.create_model_instance_node(
            parent_node_id,
            model_to_parent_transform,
            create_dummy_model_id(""),
            Some(Sphere::new(Point3::origin(), F::one())),
            Vec::new(),
        )
    }

    fn create_dummy_model_instance_cluster_node<F: Float>(
        scene_graph: &mut SceneGraph<F>,
        parent_node_id: GroupNodeID,
    ) -> ModelInstanceClusterNodeID {
        create_dummy_model_instance_cluster_node_with_transforms(
            scene_graph,
            parent_node_id,
            vec![Similarity3::identity(), Similarity3::identity()],
        )
    }

    fn create_dummy_model_instance_cluster_node_with_transforms<F: Float>(
        scene_graph: &mut SceneGraph<F>,
        parent_node_id: GroupNodeID,
        model_to_parent_transforms: Vec<Similarity3<F>>,
    ) -> ModelInstanceClusterNodeID {
        scene_graph.create_model_instance_cluster_node(
            parent_node_id,
            model_to_parent_transforms,
            create_dummy_model_id(""),
            Some(Sphere::new(Point3::origin(), F::one())),
            Vec::new(),
        )
    }

    fn create_dummy_camera_node<F: Float>(
        scene_graph: &mut SceneGraph<F>,
        parent_node_id: GroupNodeID,
    ) -> CameraNodeID {
        scene_graph.create_camera_node(parent_node_id, Similarity3::identity())
    }

    fn create_dummy_model_id<S: AsRef<str>>(tag: S) -> ModelID {
        ModelID::for_mesh_and_material(
            MeshID(hash64!(format!("Test mesh {}", tag.as_ref()))),
            MaterialHandle::new(
                MaterialID(hash64!(format!("Test material {}", tag.as_ref()))),
                None,
                None,
            ),
            None,
        )
    }

    fn create_dummy_transform() -> Similarity3<f64> {
        Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            7.0,
        )
    }

    #[test]
    fn creating_scene_graph_works() {
        let scene_graph = SceneGraph::<f64>::new();

        assert!(scene_graph
            .group_nodes()
            .has_node(scene_graph.root_node_id()));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    fn creating_group_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_group_node(&mut scene_graph, root_id);

        assert!(scene_graph.group_nodes().has_node(id));
        assert!(scene_graph.node_has_group_node_as_child(root_id, id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 2);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    fn creating_model_instance_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_model_instance_node(&mut scene_graph, root_id);

        assert!(scene_graph.model_instance_nodes().has_node(id));
        assert!(scene_graph.node_has_model_instance_node_as_child(root_id, id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    fn creating_model_instance_cluster_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_model_instance_cluster_node(&mut scene_graph, root_id);

        assert!(scene_graph.model_instance_cluster_nodes().has_node(id));
        assert!(scene_graph.node_has_model_instance_cluster_node_as_child(root_id, id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_cluster_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    #[should_panic]
    fn creating_model_instance_cluster_node_with_inconsistent_transform_and_feature_id_counts_fails(
    ) {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        scene_graph.create_model_instance_cluster_node(
            root_id,
            vec![Similarity3::identity()],
            create_dummy_model_id(""),
            Some(Sphere::new(Point3::origin(), 1.0)),
            vec![vec![]],
        );
    }

    #[test]
    fn pushing_instance_to_model_instance_cluster_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        let id = scene_graph.create_model_instance_cluster_node(
            root_id,
            vec![Similarity3::identity()],
            create_dummy_model_id(""),
            Some(Sphere::new(Point3::origin(), 1.0)),
            vec![vec![InstanceFeatureID::not_applicable()]],
        );

        let new_transform = create_dummy_transform();

        let node = scene_graph.model_instance_cluster_node_mut(id);

        node.push_instance(new_transform, &[InstanceFeatureID::not_applicable()]);

        assert_eq!(node.n_instances(), 2);
        assert_abs_diff_eq!(
            node.model_to_parent_transforms(),
            [Similarity3::identity(), new_transform].as_slice()
        );
        assert_eq!(node.feature_ids().len(), 1);
        assert_eq!(
            node.feature_ids()[0].as_slice(),
            [InstanceFeatureID::not_applicable(); 2].as_slice()
        );
    }

    #[test]
    #[should_panic]
    fn pushing_instance_with_inconsistent_feature_id_count_to_model_instance_cluster_node_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        let id = scene_graph.create_model_instance_cluster_node(
            root_id,
            vec![Similarity3::identity()],
            create_dummy_model_id(""),
            Some(Sphere::new(Point3::origin(), 1.0)),
            vec![
                vec![InstanceFeatureID::not_applicable()],
                vec![InstanceFeatureID::not_applicable()],
            ],
        );

        let new_transform = create_dummy_transform();

        let node = scene_graph.model_instance_cluster_node_mut(id);

        node.push_instance(new_transform, &[InstanceFeatureID::not_applicable()]);
    }

    #[test]
    fn swap_removing_instance_from_model_instance_cluster_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        let remaining_transform = create_dummy_transform();

        let id = scene_graph.create_model_instance_cluster_node(
            root_id,
            vec![Similarity3::identity(), remaining_transform],
            create_dummy_model_id(""),
            Some(Sphere::new(Point3::origin(), 1.0)),
            vec![vec![InstanceFeatureID::not_applicable(); 2]],
        );

        let node = scene_graph.model_instance_cluster_node_mut(id);

        node.swap_remove_instance(0);

        assert_eq!(node.n_instances(), 1);
        assert_abs_diff_eq!(
            node.model_to_parent_transforms(),
            [remaining_transform].as_slice()
        );
        assert_eq!(node.feature_ids().len(), 1);
        assert_eq!(node.feature_ids()[0].len(), 1);
    }

    #[test]
    fn creating_camera_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_camera_node(&mut scene_graph, root_id);

        assert!(scene_graph.camera_nodes().has_node(id));
        assert!(scene_graph.node_has_camera_node_as_child(root_id, id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 1);
    }

    #[test]
    fn removing_model_instance_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_model_instance_node(&mut scene_graph, root_id);
        let model_id = scene_graph.remove_model_instance_node(id);

        assert_eq!(model_id, create_dummy_model_id(""));
        assert!(!scene_graph.model_instance_nodes().has_node(id));
        assert!(!scene_graph.node_has_model_instance_node_as_child(root_id, id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    fn removing_model_instance_cluster_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_model_instance_cluster_node(&mut scene_graph, root_id);
        let model_id = scene_graph.remove_model_instance_cluster_node(id);

        assert_eq!(model_id, create_dummy_model_id(""));
        assert!(!scene_graph.model_instance_cluster_nodes().has_node(id));
        assert!(!scene_graph.node_has_model_instance_cluster_node_as_child(root_id, id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_cluster_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    fn removing_camera_node_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_camera_node(&mut scene_graph, root_id);
        scene_graph.remove_camera_node(id);

        assert!(!scene_graph.camera_nodes().has_node(id));
        assert!(!scene_graph.node_has_camera_node_as_child(root_id, id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
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

        assert!(!scene_graph.group_nodes().has_node(group_node_id));
        assert!(!scene_graph.node_has_group_node_as_child(root_id, group_node_id));

        assert!(!scene_graph.group_nodes().has_node(child_group_node_id));
        assert!(!scene_graph.camera_nodes().has_node(child_camera_node_id));
        assert!(!scene_graph
            .model_instance_nodes()
            .has_node(child_model_instance_node_id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
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
    fn creating_model_instance_cluster_node_with_missing_parent_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();

        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);

        create_dummy_model_instance_cluster_node(&mut scene_graph, group_node_id);
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
    fn removing_model_instance_cluster_node_twice_fails() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root_id = scene_graph.root_node_id();
        let model_instance_cluster_node_id =
            create_dummy_model_instance_cluster_node(&mut scene_graph, root_id);
        scene_graph.remove_model_instance_cluster_node(model_instance_cluster_node_id);
        scene_graph.remove_model_instance_cluster_node(model_instance_cluster_node_id);
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
    fn computing_root_to_camera_transform_with_only_camera_transforms_works() {
        let camera_to_root_transform = Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            7.0,
        );

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();
        let camera = scene_graph.create_camera_node(root, camera_to_root_transform);

        let root_to_camera_transform =
            scene_graph.compute_view_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(root_to_camera_transform, camera_to_root_transform.inverse());
    }

    #[test]
    fn computing_root_to_camera_transform_with_only_identity_parent_to_model_transforms_works() {
        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();
        let group_1 = scene_graph.create_group_node(root, Similarity3::identity());
        let group_2 = scene_graph.create_group_node(group_1, Similarity3::identity());
        let group_3 = scene_graph.create_group_node(group_2, Similarity3::identity());
        let camera = scene_graph.create_camera_node(group_3, Similarity3::identity());

        scene_graph.update_all_group_to_root_transforms();

        let transform = scene_graph.compute_view_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(transform, Similarity3::identity());
    }

    #[test]
    fn computing_root_to_camera_transform_with_different_parent_to_model_transforms_works() {
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
        );

        scene_graph.update_all_group_to_root_transforms();

        let root_to_camera_transform =
            scene_graph.compute_view_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(
            root_to_camera_transform.to_homogeneous(),
            Scale3::new(scaling, scaling, scaling)
                .try_inverse()
                .unwrap()
                .to_homogeneous()
                * rotation.inverse().to_homogeneous()
                * translation.inverse().to_homogeneous(),
            epsilon = 1e-9
        );
    }

    fn assert_spheres_equal<F: Float>(sphere_1: &Sphere<F>, sphere_2: &Sphere<F>) {
        assert_abs_diff_eq!(sphere_1.center(), sphere_2.center());
        assert_abs_diff_eq!(sphere_1.radius(), sphere_2.radius());
    }

    #[test]
    fn updating_bounding_spheres_with_one_transformed_instance_in_world_space_works() {
        let model_to_parent_transform = Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            7.0,
        );
        let bounding_sphere = Sphere::new(point![3.9, 5.2, 0.0], 11.1);

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();

        let model_instance_node_id = scene_graph.create_model_instance_node(
            root,
            model_to_parent_transform,
            create_dummy_model_id(""),
            Some(bounding_sphere.clone()),
            Vec::new(),
        );

        let root_bounding_sphere = scene_graph.update_bounding_spheres(root);
        assert_spheres_equal(
            &root_bounding_sphere.unwrap(),
            &bounding_sphere.transformed(&model_to_parent_transform),
        );

        let model_instance_node = scene_graph
            .model_instance_nodes
            .node(model_instance_node_id);
        let model_bounding_sphere = model_instance_node
            .get_model_bounding_sphere()
            .unwrap()
            .transformed(model_instance_node.model_to_parent_transform());

        assert_spheres_equal(
            &model_bounding_sphere,
            &bounding_sphere.transformed(&model_to_parent_transform),
        );
    }

    #[test]
    fn updating_bounding_spheres_with_transformed_instance_cluster_in_group_works() {
        let group_to_parent_transform = Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            7.0,
        );
        let bounding_sphere = Sphere::new(point![3.9, 5.2, 0.0], 11.1);

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();

        let group = scene_graph.create_group_node(root, group_to_parent_transform);

        let model_instance_cluster_node_id = scene_graph.create_model_instance_cluster_node(
            group,
            vec![Similarity3::identity(), Similarity3::identity()],
            create_dummy_model_id(""),
            Some(bounding_sphere.clone()),
            Vec::new(),
        );

        let root_bounding_sphere = scene_graph.update_bounding_spheres(root);
        assert_spheres_equal(
            &root_bounding_sphere.unwrap(),
            &bounding_sphere.transformed(&group_to_parent_transform),
        );

        let model_instance_cluster_node = scene_graph
            .model_instance_cluster_nodes
            .node(model_instance_cluster_node_id);
        let cluster_bounding_sphere = model_instance_cluster_node
            .get_cluster_bounding_sphere()
            .unwrap();

        assert_spheres_equal(&cluster_bounding_sphere, &bounding_sphere);
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
            create_dummy_model_id("1"),
            Some(bounding_sphere_1.clone()),
            Vec::new(),
        );
        scene_graph.create_model_instance_node(
            root,
            Similarity3::identity(),
            create_dummy_model_id("2"),
            Some(bounding_sphere_2.clone()),
            Vec::new(),
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

        let group_1_to_parent_transform = Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::identity().into(),
            2.0,
        );
        let group_2_to_parent_transform = Similarity3::from_parts(
            Translation3::new(0.01, 2.9, 10.1),
            Rotation3::from_euler_angles(1.1, 2.2, 3.3).into(),
            0.2,
        );
        let model_instance_2_to_parent_transform = Similarity3::from_parts(
            Translation3::new(-2.1, 8.9, 1.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            1.0,
        );

        let mut scene_graph = SceneGraph::<f64>::new();
        let root = scene_graph.root_node_id();

        let group_1 = scene_graph.create_group_node(root, group_1_to_parent_transform);
        scene_graph.create_model_instance_node(
            group_1,
            Similarity3::identity(),
            create_dummy_model_id("1"),
            Some(bounding_sphere_1.clone()),
            Vec::new(),
        );
        let group_2 = scene_graph.create_group_node(group_1, group_2_to_parent_transform);
        scene_graph.create_model_instance_node(
            group_2,
            model_instance_2_to_parent_transform,
            create_dummy_model_id("2"),
            Some(bounding_sphere_2.clone()),
            Vec::new(),
        );

        let correct_group_2_bounding_sphere =
            bounding_sphere_2.transformed(&model_instance_2_to_parent_transform);
        let correct_group_1_bounding_sphere = Sphere::bounding_sphere_from_pair(
            &bounding_sphere_1,
            &correct_group_2_bounding_sphere.transformed(&group_2_to_parent_transform),
        );
        let correct_root_bounding_sphere =
            correct_group_1_bounding_sphere.transformed(&group_1_to_parent_transform);

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
