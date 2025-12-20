//! Scene graph implementation.

use crate::{
    SceneEntityFlags,
    camera::SceneCamera,
    model::{ModelID, ModelInstanceManager},
};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_containers::{HashMap, SlotKey, SlotMap, hash_map::Entry};
use impact_geometry::{Frustum, Sphere, projection::CubemapFace};
use impact_light::{
    LightFlags, LightManager, MAX_SHADOW_MAP_CASCADES, ShadowableOmnidirectionalLight,
    ShadowableUnidirectionalLight, shadow_map::CascadeIdx,
};
use impact_material::MaterialRegistry;
use impact_math::transform::{Isometry3, Similarity3};
use impact_model::{
    InstanceFeature, InstanceFeatureID, InstanceFeatureTypeID,
    transform::{
        InstanceModelLightTransform, InstanceModelViewTransform,
        InstanceModelViewTransformWithPrevious,
    },
};
use roc_integration::roc;
use std::{
    mem,
    sync::atomic::{AtomicU32, Ordering},
};
use tinyvec::TinyVec;

/// A tree structure that defines a spatial hierarchy of objects in the world
/// and enables useful operations on them.
///
/// The scene graph can contain leaf nodes representing model instances and
/// cameras. Every leaf node has a parent "group" node, which itself has a group
/// node as a parent and may have any number and type of children. Each node
/// holds a transform from the model space of the object or group it represents
/// to the space of the parent.
#[derive(Debug)]
pub struct SceneGraph {
    root_node_id: GroupNodeID,
    group_nodes: NodeStorage<GroupNode>,
    model_instance_nodes: NodeStorage<ModelInstanceNode>,
    camera_nodes: NodeStorage<CameraNode>,
    model_metadata: ModelMetadata,
}

/// Flat storage for all the [`SceneGraph`] nodes of a given
/// type.
#[derive(Clone, Debug, Default)]
pub struct NodeStorage<N> {
    nodes: SlotMap<N>,
}

#[derive(Debug)]
struct ModelMetadata {
    feature_type_ids_for_shadow_mapping: HashMap<ModelID, Vec<InstanceFeatureTypeID>>,
}

/// Represents a type of node in a [`SceneGraph`].
pub trait SceneGraphNode {
    /// Type of the node's ID.
    type ID: SceneGraphNodeID;
}

/// Represents the ID of a type of node in a [`SceneGraph`].
pub trait SceneGraphNodeID: NodeIDToSlotKey + SlotKeyToNodeID + Pod {}

/// Identifier for a group node in a [`SceneGraph`].
#[roc(parents = "Scene")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct GroupNodeID(SlotKey);

/// Identifier for a [`ModelInstanceNode`] in a [`SceneGraph`].
#[roc(parents = "Scene")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ModelInstanceNodeID(SlotKey);

/// Identifier for a [`CameraNode`] in a [`SceneGraph`].
#[roc(parents = "Scene")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct CameraNodeID(SlotKey);

/// Represents a type of node identifier that may provide an associated
/// [`SlotKey`].
pub trait NodeIDToSlotKey {
    /// Returns the [`SlotKey`] corresponding to the node ID.
    fn key(&self) -> SlotKey;
}

/// Represents a type of node identifier that may be created from an associated
/// [`SlotKey`].
pub trait SlotKeyToNodeID {
    /// Creates the node ID corresponding to the given [`SlotKey`].
    fn from_key(key: SlotKey) -> Self;
}

/// Type alias for a collection of child group node IDs with inline capacity of 8.
type ChildGroupNodeIds = TinyVec<[GroupNodeID; 8]>;

/// Type alias for a collection of child model instance node IDs with inline capacity of 8.
type ChildModelInstanceNodeIds = TinyVec<[ModelInstanceNodeID; 8]>;

/// Type alias for a collection of child camera node IDs with inline capacity of 8.
type ChildCameraNodeIds = TinyVec<[CameraNodeID; 8]>;

/// A [`SceneGraph`] node that has a group of other nodes as children. The
/// children may be [`ModelInstanceNode`]s, [`CameraNode`]s and/or other group
/// nodes. It holds a transform representing group's spatial relationship with
/// its parent group.
#[derive(Clone, Debug)]
pub struct GroupNode {
    parent_node_id: Option<GroupNodeID>,
    group_to_parent_transform: Isometry3,
    child_group_node_ids: ChildGroupNodeIds,
    child_model_instance_node_ids: ChildModelInstanceNodeIds,
    child_camera_node_ids: ChildCameraNodeIds,
    bounding_sphere: Option<Sphere>,
    group_to_root_transform: Isometry3,
}

/// A [`SceneGraph`] leaf node representing a model instance. It holds a
/// transform representing the instance's spatial relationship with its parent
/// group, as well as a list of instance feature IDs.
#[derive(Debug)]
pub struct ModelInstanceNode {
    parent_node_id: GroupNodeID,
    model_bounding_sphere: Option<Sphere>,
    model_to_parent_transform: Similarity3,
    model_id: ModelID,
    feature_ids_for_rendering: FeatureIDSet,
    feature_ids_for_shadow_mapping: FeatureIDSet,
    flags: ModelInstanceFlags,
    frame_number_when_last_visible: AtomicU32,
}

pub type FeatureIDSet = TinyVec<[InstanceFeatureID; 4]>;

/// A [`SceneGraph`] leaf node representing a [`Camera`](impact_camera::Camera).
/// It holds at transform representing the camera's spatial relationship with
/// its parent group.
#[derive(Clone, Debug)]
pub struct CameraNode {
    parent_node_id: GroupNodeID,
    camera_to_parent_transform: Isometry3,
}

#[derive(Clone, Debug)]
enum BoundingSphereUpdateOperation {
    VisitChildren(GroupNodeID),
    ComputeBounds(GroupNodeID),
}

bitflags! {
    /// Bitflags encoding a set of binary states or properties for a model instance.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct ModelInstanceFlags: u8 {
        /// The model instance should not be rendered.
        const IS_HIDDEN     = 1 << 0;
        /// The model instance should not participate in shadow maps.
        const CASTS_NO_SHADOWS = 1 << 1;
    }
}

impl SceneGraph {
    /// Creates a new empty scene graph.
    pub fn new() -> Self {
        let mut group_nodes = NodeStorage::new();
        let model_instance_nodes = NodeStorage::new();
        let camera_nodes = NodeStorage::new();

        let model_metadata = ModelMetadata::new();

        let root_node_id = group_nodes.add_node(GroupNode::root());

        Self {
            root_node_id,
            group_nodes,
            model_instance_nodes,
            camera_nodes,
            model_metadata,
        }
    }

    /// Returns the ID of the root group node.
    pub fn root_node_id(&self) -> GroupNodeID {
        self.root_node_id
    }

    /// Returns a reference to the storage of group nodes in the scene graph.
    pub fn group_nodes(&self) -> &NodeStorage<GroupNode> {
        &self.group_nodes
    }

    /// Returns a reference to the storage of model instance nodes in the scene
    /// graph.
    pub fn model_instance_nodes(&self) -> &NodeStorage<ModelInstanceNode> {
        &self.model_instance_nodes
    }

    /// Returns a reference to the storage of camera nodes in the scene graph.
    pub fn camera_nodes(&self) -> &NodeStorage<CameraNode> {
        &self.camera_nodes
    }

    /// Creates a new empty group node with the given parent and
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
        group_to_parent_transform: Isometry3,
    ) -> GroupNodeID {
        let group_node = GroupNode::non_root(parent_node_id, group_to_parent_transform);
        let group_node_id = self.group_nodes.add_node(group_node);
        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_group_node(group_node_id);
        group_node_id
    }

    /// Creates a new [`ModelInstanceNode`] for an instance of the model with
    /// the given ID, bounding sphere for frustum culling, feature IDs for
    /// rendering and shadow mapping, and flags. It is included in the scene
    /// graph with the given transform relative to the the given parent node.
    ///
    /// If no bounding sphere is provided, the model will not be frustum culled.
    ///
    /// # Returns
    /// The ID of the created model instance node.
    ///
    /// # Panics
    /// - If the first rendering feature ID is not the
    ///   [`InstanceModelViewTransformWithPrevious`] feature.
    /// - If the first shadow mapping rendering feature ID is not the
    ///   [`InstanceModelLightTransform`] feature.
    /// - If the specified parent group node does not exist.
    /// - If no bounding sphere is provided when the parent node is not the root
    ///   node.
    pub fn create_model_instance_node(
        &mut self,
        parent_node_id: GroupNodeID,
        model_to_parent_transform: Similarity3,
        model_id: ModelID,
        frustum_culling_bounding_sphere: Option<Sphere>,
        feature_ids_for_rendering: FeatureIDSet,
        feature_ids_for_shadow_mapping: FeatureIDSet,
        flags: ModelInstanceFlags,
    ) -> ModelInstanceNodeID {
        if !feature_ids_for_rendering.is_empty() {
            assert_eq!(
                feature_ids_for_rendering[0].feature_type_id(),
                InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID,
                "First rendering feature for model instance node must be the InstanceModelViewTransformWithPrevious feature"
            );
        }
        if !feature_ids_for_shadow_mapping.is_empty() {
            assert_eq!(
                feature_ids_for_shadow_mapping[0].feature_type_id(),
                InstanceModelLightTransform::FEATURE_TYPE_ID,
                "First shadow mapping feature for model instance node must be the InstanceModelLightTransform feature"
            );
        }

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
            feature_ids_for_rendering,
            feature_ids_for_shadow_mapping,
            flags,
        );

        self.model_metadata.register_instance(&model_instance_node);

        let model_instance_node_id = self.model_instance_nodes.add_node(model_instance_node);

        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_model_instance_node(model_instance_node_id);

        model_instance_node_id
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
        camera_to_parent_transform: Isometry3,
    ) -> CameraNodeID {
        let camera_node = CameraNode::new(parent_node_id, camera_to_parent_transform);
        let camera_node_id = self.camera_nodes.add_node(camera_node);
        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_camera_node(camera_node_id);
        camera_node_id
    }

    /// Removes the group node with the given ID and all its children from
    /// the scene graph.
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
        let model_id = *model_instance_node.model_id();
        let parent_node_id = model_instance_node.parent_node_id();
        self.model_instance_nodes
            .remove_node(model_instance_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_model_instance_node(model_instance_node_id);
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

    /// Removes all descendents of the root node from the tree.
    pub fn clear_nodes(&mut self) {
        self.group_nodes.remove_all_nodes();
        self.model_instance_nodes.remove_all_nodes();
        self.camera_nodes.remove_all_nodes();
        self.root_node_id = self.group_nodes.add_node(GroupNode::root());
    }

    /// Sets the given transform as the parent-to-model transform for the
    /// group node with the given ID if it exists.
    pub fn set_group_to_parent_transform(
        &mut self,
        group_node_id: GroupNodeID,
        transform: Isometry3,
    ) {
        if let Some(node) = self.group_nodes.get_node_mut(group_node_id) {
            node.set_group_to_parent_transform(transform);
        }
    }

    /// Sets the given transform as the model-to-parent transform for the
    /// [`ModelInstanceNode`] with the given ID if it exists.
    pub fn set_model_to_parent_transform(
        &mut self,
        model_instance_node_id: ModelInstanceNodeID,
        transform: Similarity3,
    ) {
        if let Some(node) = self
            .model_instance_nodes
            .get_node_mut(model_instance_node_id)
        {
            node.set_model_to_parent_transform(transform);
        }
    }

    /// Sets the given transform and flags as the model-to-parent transform and
    /// flags for the [`ModelInstanceNode`] with the given ID if it exists.
    pub fn set_model_to_parent_transform_and_flags(
        &mut self,
        model_instance_node_id: ModelInstanceNodeID,
        transform: Similarity3,
        flags: ModelInstanceFlags,
    ) {
        if let Some(node) = self
            .model_instance_nodes
            .get_node_mut(model_instance_node_id)
        {
            node.set_model_to_parent_transform(transform);
            node.set_flags(flags);
        }
    }

    /// Sets the given sphere as the bounding sphere for the
    /// [`ModelInstanceNode`] with the given ID if it exists.
    pub fn set_model_instance_bounding_sphere(
        &mut self,
        model_instance_node_id: ModelInstanceNodeID,
        bounding_sphere: Option<Sphere>,
    ) {
        if let Some(node) = self
            .model_instance_nodes
            .get_node_mut(model_instance_node_id)
        {
            node.set_model_bounding_sphere(bounding_sphere);
        }
    }

    /// Sets the given transform as the camera-to-parent transform for the
    /// [`CameraNode`] with the given ID if it exists.
    pub fn set_camera_to_parent_transform(
        &mut self,
        camera_node_id: CameraNodeID,
        transform: Isometry3,
    ) {
        if let Some(node) = self.camera_nodes.get_node_mut(camera_node_id) {
            node.set_camera_to_parent_transform(transform);
        }
    }

    /// Updates the transform from local space to the space of the root node for
    /// all group nodes in the scene graph.
    pub fn update_all_group_to_root_transforms(&mut self) {
        let arena =
            ArenaPool::get_arena_for_capacity(32 * mem::size_of::<(GroupNodeID, Isometry3)>());
        let mut operation_stack = AVec::with_capacity_in(32, &arena);

        operation_stack.push((self.root_node_id, Isometry3::identity()));

        while let Some((node_id, parent_to_root_transform)) = operation_stack.pop() {
            let group_node = self.group_nodes.node_mut(node_id);

            let group_to_root_transform =
                parent_to_root_transform * group_node.group_to_parent_transform();

            group_node.set_group_to_root_transform(group_to_root_transform);

            for child_group_node_id in group_node.child_group_node_ids() {
                operation_stack.push((*child_group_node_id, group_to_root_transform));
            }
        }
    }

    /// Updates the world-to-camera transform of the given scene camera based on
    /// the transforms of its node and parent nodes.
    ///
    /// # Warning
    /// Make sure to [`Self::update_all_group_to_root_transforms`] before calling
    /// this method if any group nodes have changed.
    pub fn sync_camera_view_transform(&self, scene_camera: &mut SceneCamera) {
        let camera_node = self.camera_nodes.node(scene_camera.scene_graph_node_id());
        let view_transform = self.compute_view_transform(camera_node);
        scene_camera.set_view_transform(view_transform);
    }

    /// Updates the bounding spheres of all nodes in the scene graph (excluding
    /// contributions from hidden model instances).
    pub fn update_all_bounding_spheres(&mut self) {
        fn merge_spheres(accum: &mut Option<Sphere>, sphere: Sphere) {
            match accum {
                None => {
                    *accum = Some(sphere);
                }
                Some(accum_sphere) => {
                    *accum = Some(Sphere::bounding_sphere_from_pair(accum_sphere, &sphere));
                }
            }
        }

        let arena =
            ArenaPool::get_arena_for_capacity(32 * mem::size_of::<BoundingSphereUpdateOperation>());
        let mut operation_stack = AVec::with_capacity_in(32, &arena);

        operation_stack.push(BoundingSphereUpdateOperation::VisitChildren(
            self.root_node_id,
        ));

        while let Some(operation) = operation_stack.pop() {
            match operation {
                // We need to update the bounding sphere of each child group node
                // before its parent, so we don't proceed until we have pushed all
                // children on the stack after their parents
                BoundingSphereUpdateOperation::VisitChildren(group_node_id) => {
                    operation_stack
                        .push(BoundingSphereUpdateOperation::ComputeBounds(group_node_id));

                    let group_node = self.group_nodes.node(group_node_id);
                    for child_group_node_id in group_node.child_group_node_ids() {
                        operation_stack.push(BoundingSphereUpdateOperation::VisitChildren(
                            *child_group_node_id,
                        ));
                    }
                }
                BoundingSphereUpdateOperation::ComputeBounds(group_node_id) => {
                    let mut group_bounding_sphere = None;

                    let group_node = self.group_nodes.node(group_node_id);

                    for child_group_node_id in group_node.child_group_node_ids() {
                        let child_group_node = self.group_nodes.node(*child_group_node_id);

                        if let Some(child_group_bounding_sphere) =
                            child_group_node.get_bounding_sphere()
                        {
                            merge_spheres(
                                &mut group_bounding_sphere,
                                child_group_bounding_sphere.translated_and_rotated(
                                    child_group_node.group_to_parent_transform(),
                                ),
                            );
                        }
                    }

                    for model_instance_node_id in group_node.child_model_instance_node_ids() {
                        let model_instance_node =
                            self.model_instance_nodes.node(*model_instance_node_id);

                        // Hidden instances don't affect the parent bounds
                        if model_instance_node
                            .flags()
                            .contains(ModelInstanceFlags::IS_HIDDEN)
                        {
                            continue;
                        }

                        if let Some(model_bounding_sphere) =
                            model_instance_node.get_model_bounding_sphere()
                        {
                            merge_spheres(
                                &mut group_bounding_sphere,
                                model_bounding_sphere
                                    .transformed(model_instance_node.model_to_parent_transform()),
                            );
                        }
                    }

                    self.group_nodes
                        .node_mut(group_node_id)
                        .set_bounding_sphere(group_bounding_sphere);
                }
            }
        }
    }

    /// Computes the model-to-camera space transforms of all the model instances
    /// in the scene graph that are visible with the specified camera and adds
    /// them to the given model instance manager.
    ///
    /// # Warning
    /// Make sure to [`Self::update_all_bounding_spheres`] and
    /// compute the view transform before calling this method if any nodes have
    /// changed.
    pub fn buffer_model_instances_for_rendering(
        &self,
        material_registry: &MaterialRegistry,
        model_instance_manager: &mut ModelInstanceManager,
        scene_camera: &SceneCamera,
        current_frame_number: u32,
    ) where
        InstanceModelViewTransformWithPrevious: InstanceFeature,
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
                    bounding_sphere.translated_and_rotated(&group_to_camera_transform);

                camera_space_view_frustum
                    .could_contain_part_of_sphere(&bounding_sphere_camera_space)
            } else {
                // If the group has no bounding sphere, buffer it unconditionally
                true
            };

            if should_buffer {
                self.buffer_model_instances_in_group_for_rendering(
                    material_registry,
                    model_instance_manager,
                    current_frame_number,
                    camera_space_view_frustum,
                    group_node,
                    &group_to_camera_transform,
                );
            }
        }

        for &model_instance_node_id in root_node.child_model_instance_node_ids() {
            let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

            if model_instance_node
                .flags()
                .contains(ModelInstanceFlags::IS_HIDDEN)
                || model_instance_node.feature_ids_for_rendering().is_empty()
            {
                continue;
            }

            let model_view_transform =
                root_to_camera_transform * model_instance_node.model_to_parent_transform();

            let should_buffer =
                if let Some(bounding_sphere) = model_instance_node.get_model_bounding_sphere() {
                    let child_bounding_sphere_camera_space =
                        bounding_sphere.transformed(&model_view_transform);

                    camera_space_view_frustum
                        .could_contain_part_of_sphere(&child_bounding_sphere_camera_space)
                } else {
                    // If the model has no bounding sphere, buffer it unconditionally
                    true
                };

            if should_buffer {
                Self::buffer_model_instance_for_rendering(
                    material_registry,
                    model_instance_manager,
                    current_frame_number,
                    model_instance_node,
                    &model_view_transform,
                );
            }
        }
    }

    /// Computes the transform from the scene graph's root node space to the
    /// space of the given camera node.
    fn compute_view_transform(&self, camera_node: &CameraNode) -> Isometry3 {
        let parent_node = self.group_nodes.node(camera_node.parent_node_id());
        camera_node.parent_to_camera_transform() * parent_node.root_to_group_transform()
    }

    /// Determines the group/model-to-camera transforms of the group nodes and
    /// model instance nodes that are children of the specified group node and
    /// whose bounding spheres lie within the given camera frustum. The given
    /// group-to-camera transform is prepended to the transforms of the
    /// children. For the children that are model instance nodes, their final
    /// model-to-camera transforms along with other relevant features needed for
    /// rendering are added to the given model instance manager.
    ///
    /// # Panics
    /// If any of the child nodes of the group node does not exist.
    fn buffer_model_instances_in_group_for_rendering(
        &self,
        material_registry: &MaterialRegistry,
        model_instance_manager: &mut ModelInstanceManager,
        current_frame_number: u32,
        camera_space_view_frustum: &Frustum,
        group_node: &GroupNode,
        group_to_camera_transform: &Isometry3,
    ) where
        InstanceModelViewTransformWithPrevious: InstanceFeature,
    {
        for &child_group_node_id in group_node.child_group_node_ids() {
            let child_group_node = self.group_nodes.node(child_group_node_id);

            let child_group_to_camera_transform =
                group_to_camera_transform * child_group_node.group_to_parent_transform();

            let should_buffer =
                if let Some(child_bounding_sphere) = child_group_node.get_bounding_sphere() {
                    let child_bounding_sphere_camera_space = child_bounding_sphere
                        .translated_and_rotated(&child_group_to_camera_transform);

                    camera_space_view_frustum
                        .could_contain_part_of_sphere(&child_bounding_sphere_camera_space)
                } else {
                    // If the group has no bounding sphere, buffer it unconditionally
                    true
                };

            if should_buffer {
                self.buffer_model_instances_in_group_for_rendering(
                    material_registry,
                    model_instance_manager,
                    current_frame_number,
                    camera_space_view_frustum,
                    child_group_node,
                    &child_group_to_camera_transform,
                );
            }
        }

        for &child_model_instance_node_id in group_node.child_model_instance_node_ids() {
            let child_model_instance_node =
                self.model_instance_nodes.node(child_model_instance_node_id);

            if child_model_instance_node
                .flags()
                .contains(ModelInstanceFlags::IS_HIDDEN)
                || child_model_instance_node
                    .feature_ids_for_rendering()
                    .is_empty()
            {
                continue;
            }

            let child_model_view_transform =
                group_to_camera_transform * child_model_instance_node.model_to_parent_transform();

            let should_buffer = if let Some(child_bounding_sphere) =
                child_model_instance_node.get_model_bounding_sphere()
            {
                let child_bounding_sphere_camera_space =
                    child_bounding_sphere.transformed(&child_model_view_transform);

                camera_space_view_frustum
                    .could_contain_part_of_sphere(&child_bounding_sphere_camera_space)
            } else {
                // If the model has no bounding sphere, buffer it unconditionally
                true
            };

            if should_buffer {
                Self::buffer_model_instance_for_rendering(
                    material_registry,
                    model_instance_manager,
                    current_frame_number,
                    child_model_instance_node,
                    &child_model_view_transform,
                );
            }
        }
    }

    fn buffer_model_instance_for_rendering(
        material_registry: &MaterialRegistry,
        model_instance_manager: &mut ModelInstanceManager,
        current_frame_number: u32,
        model_instance_node: &ModelInstanceNode,
        model_view_transform: &Similarity3,
    ) where
        InstanceModelViewTransformWithPrevious: InstanceFeature,
    {
        let instance_model_view_transform = InstanceModelViewTransform::from(*model_view_transform);

        model_instance_manager
            .feature_mut::<InstanceModelViewTransformWithPrevious>(
                model_instance_node.model_view_transform_feature_id(),
            )
            .set_transform_for_new_frame(instance_model_view_transform);

        let model_id = model_instance_node.model_id();

        model_instance_manager.buffer_instance_features_from_storages(
            model_id,
            model_instance_node.feature_ids_for_rendering(),
        );

        if let Some(material) = material_registry.get(model_id.material_id()) {
            material
                .property_values
                .buffer(model_instance_manager, model_id);
        }

        model_instance_node.declare_visible_this_frame(current_frame_number);
    }

    /// Goes through all omnidirectional lights in the given light manager and
    /// updates their cubemap orientations and distance spans to encompass all
    /// model instances that may cast visible shadows in a way that preserves
    /// quality and efficiency. Then the model to cubemap face space transform
    /// of every such shadow casting model instance is computed for the relevant
    /// cube faces of each light and copied to the model's instance transform
    /// buffer in new ranges dedicated to the faces of the cubemap of the
    /// particular light.
    ///
    /// # Warning
    /// Make sure to call [`Self::buffer_model_instances_for_rendering`] before
    /// calling this method, so that the ranges of model to cubemap face
    /// transforms in the model instance buffers come after the initial range
    /// containing model to camera transforms.
    pub fn bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
        &self,
        light_manager: &mut LightManager,
        model_instance_manager: &mut ModelInstanceManager,
        scene_camera: &SceneCamera,
        shadow_mapping_enabled: bool,
    ) {
        let camera_space_view_frustum = scene_camera.camera().view_frustum();
        let view_transform = scene_camera.view_transform();

        let root_node_id = self.root_node_id();
        let root_node = self.group_nodes.node(root_node_id);

        if let Some(world_space_bounding_sphere) = root_node.get_bounding_sphere() {
            let camera_space_bounding_sphere =
                world_space_bounding_sphere.translated_and_rotated(view_transform);

            for (light_id, omnidirectional_light) in
                light_manager.shadowable_omnidirectional_lights_with_ids_mut()
            {
                if omnidirectional_light
                    .flags()
                    .contains(LightFlags::IS_DISABLED)
                {
                    continue;
                }

                let camera_space_aabb_for_visible_models = camera_space_bounding_sphere
                    .compute_aabb()
                    .compute_overlap_with(&camera_space_view_frustum.compute_aabb());

                omnidirectional_light.orient_and_scale_cubemap_for_shadow_casting_models(
                    &camera_space_bounding_sphere,
                    camera_space_aabb_for_visible_models.as_ref(),
                );

                if !shadow_mapping_enabled {
                    continue;
                }

                for face in CubemapFace::all() {
                    // Begin a new range dedicated for tranforms to the current
                    // cubemap face space for the current light at the end of
                    // each transform buffer, identified by the light's ID plus
                    // a face index offset
                    let range_id =
                        crate::light::light_id_to_instance_feature_buffer_range_id(light_id)
                            + face.as_idx_u32();
                    model_instance_manager.begin_range_in_feature_buffers(
                        InstanceModelLightTransform::FEATURE_TYPE_ID,
                        range_id,
                    );

                    for (model_id, feature_type_ids) in self
                        .model_metadata
                        .models_with_feature_type_ids_for_shadow_mapping()
                    {
                        // We have already created a range for the
                        // `InstanceModelLightTransform` feature, which is the
                        // first ID in the list
                        if feature_type_ids.len() > 1 {
                            model_instance_manager.begin_ranges_in_feature_buffers_for_model(
                                model_id,
                                &feature_type_ids[1..],
                                range_id,
                            );
                        }
                    }

                    let camera_space_face_frustum =
                        omnidirectional_light.compute_camera_space_frustum_for_face(face);

                    if ShadowableOmnidirectionalLight::camera_space_frustum_for_face_may_contain_visible_models(
                        camera_space_aabb_for_visible_models.as_ref(),
                        &camera_space_face_frustum,
                    ) {
                        self.buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_omnidirectional_light_cubemap_face(
                            model_instance_manager,
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

    fn buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_omnidirectional_light_cubemap_face(
        &self,
        model_instance_manager: &mut ModelInstanceManager,
        omnidirectional_light: &ShadowableOmnidirectionalLight,
        face: CubemapFace,
        camera_space_face_frustum: &Frustum,
        group_node: &GroupNode,
        group_to_camera_transform: &Isometry3,
    ) {
        for &child_group_node_id in group_node.child_group_node_ids() {
            let child_group_node = self.group_nodes.node(child_group_node_id);

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(child_bounding_sphere) = child_group_node.get_bounding_sphere() {
                let child_group_to_camera_transform =
                    group_to_camera_transform * child_group_node.group_to_parent_transform();

                let child_camera_space_bounding_sphere =
                    child_bounding_sphere.translated_and_rotated(&child_group_to_camera_transform);

                if camera_space_face_frustum
                    .could_contain_part_of_sphere(&child_camera_space_bounding_sphere)
                {
                    self.buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_omnidirectional_light_cubemap_face(
                            model_instance_manager,
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

            if model_instance_node
                .flags()
                .intersects(ModelInstanceFlags::IS_HIDDEN | ModelInstanceFlags::CASTS_NO_SHADOWS)
                | model_instance_node
                    .feature_ids_for_shadow_mapping()
                    .is_empty()
            {
                continue;
            }

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(model_instance_bounding_sphere) =
                model_instance_node.get_model_bounding_sphere()
            {
                let model_instance_to_camera_transform =
                    group_to_camera_transform * model_instance_node.model_to_parent_transform();

                let model_instance_camera_space_bounding_sphere =
                    model_instance_bounding_sphere.transformed(&model_instance_to_camera_transform);

                if camera_space_face_frustum
                    .could_contain_part_of_sphere(&model_instance_camera_space_bounding_sphere)
                {
                    let instance_model_light_transform = InstanceModelLightTransform::from(
                        omnidirectional_light.create_transform_to_positive_z_cubemap_face_space(
                            face,
                            &model_instance_to_camera_transform,
                        ),
                    );

                    Self::buffer_model_instance_for_shadow_mapping(
                        model_instance_manager,
                        model_instance_node,
                        &instance_model_light_transform,
                    );
                }
            }
        }
    }

    /// Goes through all unidirectional lights in the given light manager and
    /// updates their orthographic transforms to encompass model instances that
    /// may cast visible shadows inside the corresponding cascades in the view
    /// frustum. Then the model to light transform of every such shadow casting
    /// model instance is computed for each light and copied to the model's
    /// instance transform buffer in a new range dedicated to the particular
    /// light and cascade.
    ///
    /// # Warning
    /// Make sure to call [`Self::buffer_model_instances_for_rendering`] before
    /// calling this method, so that the ranges of model to light transforms in
    /// the model instance buffers come after the initial range containing model
    /// to camera transforms.
    pub fn bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
        &self,
        light_manager: &mut LightManager,
        model_instance_manager: &mut ModelInstanceManager,
        scene_camera: &SceneCamera,
        shadow_mapping_enabled: bool,
    ) {
        let camera_space_view_frustum = scene_camera.camera().view_frustum();
        let view_transform = scene_camera.view_transform();

        let root_node_id = self.root_node_id();
        let root_node = self.group_nodes.node(root_node_id);

        if let Some(world_space_bounding_sphere) = root_node.get_bounding_sphere() {
            let camera_space_bounding_sphere =
                world_space_bounding_sphere.translated_and_rotated(view_transform);

            for (light_id, unidirectional_light) in
                light_manager.shadowable_unidirectional_lights_with_ids_mut()
            {
                if unidirectional_light
                    .flags()
                    .contains(LightFlags::IS_DISABLED)
                {
                    continue;
                }

                unidirectional_light.update_cascade_partition_depths(
                    camera_space_view_frustum,
                    &camera_space_bounding_sphere,
                );

                unidirectional_light.bound_orthographic_transforms_to_cascaded_view_frustum(
                    camera_space_view_frustum,
                    &camera_space_bounding_sphere,
                );

                if !shadow_mapping_enabled {
                    continue;
                }

                for cascade_idx in 0..MAX_SHADOW_MAP_CASCADES {
                    // Begin a new range dedicated for tranforms to the current
                    // light's space for instances casting shadows in he current
                    // cascade at the end of each transform buffer, identified
                    // by the light's ID plus a cascade index offset
                    let range_id =
                        crate::light::light_id_to_instance_feature_buffer_range_id(light_id)
                            + cascade_idx;
                    model_instance_manager.begin_range_in_feature_buffers(
                        InstanceModelLightTransform::FEATURE_TYPE_ID,
                        range_id,
                    );

                    for (model_id, feature_type_ids) in self
                        .model_metadata
                        .models_with_feature_type_ids_for_shadow_mapping()
                    {
                        // We have already created a range for the
                        // `InstanceModelLightTransform` feature, which is the
                        // first ID in the list
                        if feature_type_ids.len() > 1 {
                            model_instance_manager.begin_ranges_in_feature_buffers_for_model(
                                model_id,
                                &feature_type_ids[1..],
                                range_id,
                            );
                        }
                    }

                    self.buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_unidirectional_light_cascade(
                        model_instance_manager,
                        unidirectional_light,
                        cascade_idx,
                        root_node,
                        view_transform,
                    );
                }
            }
        }
    }

    fn buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_unidirectional_light_cascade(
        &self,
        model_instance_manager: &mut ModelInstanceManager,
        unidirectional_light: &ShadowableUnidirectionalLight,
        cascade_idx: CascadeIdx,
        group_node: &GroupNode,
        group_to_camera_transform: &Isometry3,
    ) {
        for &child_group_node_id in group_node.child_group_node_ids() {
            let child_group_node = self.group_nodes.node(child_group_node_id);

            // We assume that only objects with bounding spheres will cast shadows
            if let Some(child_bounding_sphere) = child_group_node.get_bounding_sphere() {
                let child_group_to_camera_transform =
                    group_to_camera_transform * child_group_node.group_to_parent_transform();

                let child_camera_space_bounding_sphere =
                    child_bounding_sphere.translated_and_rotated(&child_group_to_camera_transform);

                if unidirectional_light.bounding_sphere_may_cast_visible_shadow_in_cascade(
                    cascade_idx,
                    &child_camera_space_bounding_sphere,
                ) {
                    self.buffer_transforms_of_visibly_shadow_casting_model_instances_in_group_for_unidirectional_light_cascade(
                        model_instance_manager,
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

            if model_instance_node
                .flags()
                .intersects(ModelInstanceFlags::IS_HIDDEN | ModelInstanceFlags::CASTS_NO_SHADOWS)
                | model_instance_node
                    .feature_ids_for_shadow_mapping()
                    .is_empty()
            {
                continue;
            }

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
                    let instance_model_light_transform = InstanceModelLightTransform::from(
                        unidirectional_light
                            .create_transform_to_light_space(&model_instance_to_camera_transform),
                    );

                    Self::buffer_model_instance_for_shadow_mapping(
                        model_instance_manager,
                        model_instance_node,
                        &instance_model_light_transform,
                    );
                }
            }
        }
    }

    fn buffer_model_instance_for_shadow_mapping(
        model_instance_manager: &mut ModelInstanceManager,
        model_instance_node: &ModelInstanceNode,
        instance_model_light_transform: &InstanceModelLightTransform,
    ) where
        InstanceModelLightTransform: InstanceFeature,
    {
        model_instance_manager.buffer_instance_feature(
            model_instance_node.model_id(),
            instance_model_light_transform,
        );

        let feature_ids_for_shadow_mapping = model_instance_node.feature_ids_for_shadow_mapping();

        if feature_ids_for_shadow_mapping.len() > 1 {
            model_instance_manager.buffer_instance_features_from_storages(
                model_instance_node.model_id(),
                &feature_ids_for_shadow_mapping[1..],
            );
        }
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

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: SceneGraphNode> NodeStorage<N> {
    fn new() -> Self {
        Self {
            nodes: SlotMap::new(),
        }
    }

    /// Returns the number of nodes in the storage.
    pub fn n_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Whether a node with the given ID exists in the storage.
    pub fn has_node(&self, node_id: N::ID) -> bool {
        self.nodes.get_value(node_id.key()).is_some()
    }

    /// Returns a reference to the node with the given ID.
    pub fn node(&self, node_id: N::ID) -> &N {
        self.nodes.value(node_id.key())
    }

    fn get_node_mut(&mut self, node_id: N::ID) -> Option<&mut N> {
        self.nodes.get_value_mut(node_id.key())
    }

    fn node_mut(&mut self, node_id: N::ID) -> &mut N {
        self.nodes.value_mut(node_id.key())
    }

    fn add_node(&mut self, node: N) -> N::ID {
        N::ID::from_key(self.nodes.insert(node))
    }

    fn remove_node(&mut self, node_id: N::ID) {
        self.nodes.remove(node_id.key());
    }

    fn remove_all_nodes(&mut self) {
        self.nodes.clear();
    }
}

impl ModelMetadata {
    fn new() -> Self {
        Self {
            feature_type_ids_for_shadow_mapping: HashMap::default(),
        }
    }

    fn register_instance(&mut self, model_instance_node: &ModelInstanceNode) {
        match self
            .feature_type_ids_for_shadow_mapping
            .entry(*model_instance_node.model_id())
        {
            Entry::Vacant(entry) => {
                entry.insert(
                    model_instance_node
                        .feature_ids_for_shadow_mapping()
                        .iter()
                        .map(InstanceFeatureID::feature_type_id)
                        .collect(),
                );
            }
            Entry::Occupied(entry) => {
                assert!(
                    entry.get().iter().copied().eq(model_instance_node
                        .feature_ids_for_shadow_mapping()
                        .iter()
                        .map(InstanceFeatureID::feature_type_id)),
                    "Got inconsistent list of feature types for shadow mapping between instances of the same model"
                );
            }
        }
    }

    fn models_with_feature_type_ids_for_shadow_mapping(
        &self,
    ) -> impl Iterator<Item = (&ModelID, &[InstanceFeatureTypeID])> {
        self.feature_type_ids_for_shadow_mapping
            .iter()
            .map(|(model_id, feature_type_ids)| (model_id, feature_type_ids.as_slice()))
    }
}

impl GroupNode {
    /// Returns the group-to-root transform for the node.
    pub fn group_to_root_transform(&self) -> &Isometry3 {
        &self.group_to_root_transform
    }

    fn new(parent_node_id: Option<GroupNodeID>, group_to_parent_transform: Isometry3) -> Self {
        Self {
            parent_node_id,
            group_to_parent_transform,
            child_group_node_ids: ChildGroupNodeIds::default(),
            child_model_instance_node_ids: ChildModelInstanceNodeIds::default(),
            child_camera_node_ids: ChildCameraNodeIds::default(),
            bounding_sphere: None,
            group_to_root_transform: Isometry3::identity(),
        }
    }

    fn root() -> Self {
        Self::new(None, Isometry3::identity())
    }

    fn non_root(parent_node_id: GroupNodeID, transform: Isometry3) -> Self {
        Self::new(Some(parent_node_id), transform)
    }

    fn group_to_parent_transform(&self) -> &Isometry3 {
        &self.group_to_parent_transform
    }

    fn root_to_group_transform(&self) -> Isometry3 {
        self.group_to_root_transform.inverse()
    }

    fn parent_node_id(&self) -> GroupNodeID {
        self.parent_node_id.unwrap()
    }

    fn child_group_node_ids(&self) -> &ChildGroupNodeIds {
        &self.child_group_node_ids
    }

    fn child_model_instance_node_ids(&self) -> &ChildModelInstanceNodeIds {
        &self.child_model_instance_node_ids
    }

    #[cfg(test)]
    fn child_camera_node_ids(&self) -> &ChildCameraNodeIds {
        &self.child_camera_node_ids
    }

    fn get_bounding_sphere(&self) -> Option<&Sphere> {
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
        self.child_group_node_ids.push(group_node_id);
    }

    fn add_child_model_instance_node(&mut self, model_instance_node_id: ModelInstanceNodeID) {
        self.child_model_instance_node_ids
            .push(model_instance_node_id);
    }

    fn add_child_camera_node(&mut self, camera_node_id: CameraNodeID) {
        self.child_camera_node_ids.push(camera_node_id);
    }

    fn remove_child_group_node(&mut self, group_node_id: GroupNodeID) {
        if let Some(pos) = self
            .child_group_node_ids
            .iter()
            .position(|&id| id == group_node_id)
        {
            self.child_group_node_ids.remove(pos);
        }
    }

    fn remove_child_model_instance_node(&mut self, model_instance_node_id: ModelInstanceNodeID) {
        if let Some(pos) = self
            .child_model_instance_node_ids
            .iter()
            .position(|&id| id == model_instance_node_id)
        {
            self.child_model_instance_node_ids.remove(pos);
        }
    }

    fn remove_child_camera_node(&mut self, camera_node_id: CameraNodeID) {
        if let Some(pos) = self
            .child_camera_node_ids
            .iter()
            .position(|&id| id == camera_node_id)
        {
            self.child_camera_node_ids.remove(pos);
        }
    }

    fn set_bounding_sphere(&mut self, bounding_sphere: Option<Sphere>) {
        self.bounding_sphere = bounding_sphere;
    }

    fn set_group_to_root_transform(&mut self, group_to_root_transform: Isometry3) {
        self.group_to_root_transform = group_to_root_transform;
    }

    fn set_group_to_parent_transform(&mut self, transform: Isometry3) {
        self.group_to_parent_transform = transform;
    }
}

impl SceneGraphNodeID for GroupNodeID {}

impl SceneGraphNode for GroupNode {
    type ID = GroupNodeID;
}

impl ModelInstanceNode {
    pub fn set_model_bounding_sphere(&mut self, bounding_sphere: Option<Sphere>) {
        self.model_bounding_sphere = bounding_sphere;
    }

    fn new(
        parent_node_id: GroupNodeID,
        model_bounding_sphere: Option<Sphere>,
        model_to_parent_transform: Similarity3,
        model_id: ModelID,
        feature_ids_for_rendering: FeatureIDSet,
        feature_ids_for_shadow_mapping: FeatureIDSet,
        flags: ModelInstanceFlags,
    ) -> Self {
        Self {
            parent_node_id,
            model_bounding_sphere,
            model_to_parent_transform,
            model_id,
            feature_ids_for_rendering,
            feature_ids_for_shadow_mapping,
            flags,
            frame_number_when_last_visible: AtomicU32::new(0),
        }
    }

    /// Returns the ID of the parent [`GroupNode`].
    fn parent_node_id(&self) -> GroupNodeID {
        self.parent_node_id
    }

    /// Returns the parent-to-model transform for the node.
    pub fn parent_to_model_transform(&self) -> Similarity3 {
        self.model_to_parent_transform.inverse()
    }

    /// Returns the model-to-parent transform for the node.
    pub fn model_to_parent_transform(&self) -> &Similarity3 {
        &self.model_to_parent_transform
    }

    /// Returns the ID of the model the node represents an
    /// instance of.
    pub fn model_id(&self) -> &ModelID {
        &self.model_id
    }

    /// Returns the ID of the instance's model-to-camera transform feature.
    pub fn model_view_transform_feature_id(&self) -> InstanceFeatureID {
        self.feature_ids_for_rendering[0]
    }

    /// Returns the IDs of the instance's features needed for rendering.
    pub fn feature_ids_for_rendering(&self) -> &[InstanceFeatureID] {
        &self.feature_ids_for_rendering
    }

    /// Returns the IDs of the instance's features needed for shadow mapping.
    pub fn feature_ids_for_shadow_mapping(&self) -> &[InstanceFeatureID] {
        &self.feature_ids_for_shadow_mapping
    }

    /// Returns the flags for the model instance.
    pub fn flags(&self) -> ModelInstanceFlags {
        self.flags
    }

    /// Returns the frame number when the model instance was last visible.
    pub fn frame_number_when_last_visible(&self) -> u32 {
        self.frame_number_when_last_visible.load(Ordering::Relaxed)
    }

    /// Returns the bounding sphere of the model instance, or [`None`] if it has
    /// no bounding sphere.
    pub fn get_model_bounding_sphere(&self) -> Option<&Sphere> {
        self.model_bounding_sphere.as_ref()
    }

    fn set_model_to_parent_transform(&mut self, transform: Similarity3) {
        self.model_to_parent_transform = transform;
    }

    fn set_flags(&mut self, flags: ModelInstanceFlags) {
        self.flags = flags;
    }

    fn declare_visible_this_frame(&self, current_frame_number: u32) {
        self.frame_number_when_last_visible
            .store(current_frame_number, Ordering::Relaxed);
    }
}

impl SceneGraphNodeID for ModelInstanceNodeID {}

impl SceneGraphNode for ModelInstanceNode {
    type ID = ModelInstanceNodeID;
}

impl CameraNode {
    fn new(parent_node_id: GroupNodeID, camera_to_parent_transform: Isometry3) -> Self {
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
    pub fn parent_to_camera_transform(&self) -> Isometry3 {
        self.camera_to_parent_transform.inverse()
    }

    /// Returns the camera-to-parent transform for the node.
    pub fn camera_to_parent_transform(&self) -> &Isometry3 {
        &self.camera_to_parent_transform
    }

    fn set_camera_to_parent_transform(&mut self, transform: Isometry3) {
        self.camera_to_parent_transform = transform;
    }
}

impl SceneGraphNodeID for CameraNodeID {}

impl SceneGraphNode for CameraNode {
    type ID = CameraNodeID;
}

macro_rules! impl_node_id_key_traits {
    ($node_id_type:ty) => {
        impl SlotKeyToNodeID for $node_id_type {
            fn from_key(key: SlotKey) -> Self {
                Self(key)
            }
        }
        impl NodeIDToSlotKey for $node_id_type {
            fn key(&self) -> SlotKey {
                self.0
            }
        }
    };
}

impl_node_id_key_traits!(GroupNodeID);
impl_node_id_key_traits!(ModelInstanceNodeID);
impl_node_id_key_traits!(CameraNodeID);

impl From<SceneEntityFlags> for ModelInstanceFlags {
    fn from(scene_entity_flags: SceneEntityFlags) -> Self {
        let mut model_instance_flags = Self::empty();
        if scene_entity_flags.contains(SceneEntityFlags::IS_DISABLED) {
            model_instance_flags |= Self::IS_HIDDEN;
        }
        if scene_entity_flags.contains(SceneEntityFlags::CASTS_NO_SHADOWS) {
            model_instance_flags |= Self::CASTS_NO_SHADOWS;
        }
        model_instance_flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_math::{hash::Hash64, quaternion::UnitQuaternion};
    use impact_model::InstanceFeatureStorage;
    use nalgebra::{Point3, Vector3};

    fn create_dummy_group_node(
        scene_graph: &mut SceneGraph,
        parent_node_id: GroupNodeID,
    ) -> GroupNodeID {
        scene_graph.create_group_node(parent_node_id, Isometry3::identity())
    }

    fn create_dummy_model_instance_node(
        scene_graph: &mut SceneGraph,
        parent_node_id: GroupNodeID,
    ) -> ModelInstanceNodeID {
        create_dummy_model_instance_node_with_transform(
            scene_graph,
            parent_node_id,
            Similarity3::identity(),
        )
    }

    fn create_dummy_model_instance_node_with_transform(
        scene_graph: &mut SceneGraph,
        parent_node_id: GroupNodeID,
        model_to_parent_transform: Similarity3,
    ) -> ModelInstanceNodeID {
        scene_graph.create_model_instance_node(
            parent_node_id,
            model_to_parent_transform,
            create_dummy_model_id(""),
            Some(Sphere::new(Point3::origin(), 1.0)),
            create_dummy_model_instance_rendering_feature_ids(),
            FeatureIDSet::new(),
            ModelInstanceFlags::empty(),
        )
    }

    fn create_dummy_model_instance_rendering_feature_ids() -> FeatureIDSet {
        let id_1 = InstanceFeatureStorage::new::<InstanceModelViewTransformWithPrevious>()
            .add_feature(&InstanceModelViewTransformWithPrevious::zeroed());
        let id_2 = InstanceFeatureStorage::new::<InstanceModelLightTransform>()
            .add_feature(&InstanceModelLightTransform::zeroed());
        FeatureIDSet::from_iter([id_1, id_2])
    }

    fn create_dummy_camera_node(
        scene_graph: &mut SceneGraph,
        parent_node_id: GroupNodeID,
    ) -> CameraNodeID {
        scene_graph.create_camera_node(parent_node_id, Isometry3::identity())
    }

    fn create_dummy_model_id<S: AsRef<str>>(tag: S) -> ModelID {
        ModelID::hash_only(Hash64::from_str(tag.as_ref()))
    }

    #[test]
    fn creating_scene_graph_works() {
        let scene_graph = SceneGraph::new();

        assert!(
            scene_graph
                .group_nodes()
                .has_node(scene_graph.root_node_id())
        );

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    fn creating_group_node_works() {
        let mut scene_graph = SceneGraph::new();
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
        let mut scene_graph = SceneGraph::new();
        let root_id = scene_graph.root_node_id();
        let id = create_dummy_model_instance_node(&mut scene_graph, root_id);

        assert!(scene_graph.model_instance_nodes().has_node(id));
        assert!(scene_graph.node_has_model_instance_node_as_child(root_id, id));

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    fn creating_camera_node_works() {
        let mut scene_graph = SceneGraph::new();
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
        let mut scene_graph = SceneGraph::new();
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
    fn removing_camera_node_works() {
        let mut scene_graph = SceneGraph::new();
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
        let mut scene_graph = SceneGraph::new();
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
        assert!(
            !scene_graph
                .model_instance_nodes()
                .has_node(child_model_instance_node_id)
        );

        assert_eq!(scene_graph.group_nodes().n_nodes(), 1);
        assert_eq!(scene_graph.model_instance_nodes().n_nodes(), 0);
        assert_eq!(scene_graph.camera_nodes().n_nodes(), 0);
    }

    #[test]
    #[should_panic]
    fn creating_group_node_with_missing_parent_fails() {
        let mut scene_graph = SceneGraph::new();
        let root_id = scene_graph.root_node_id();

        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);

        create_dummy_group_node(&mut scene_graph, group_node_id);
    }

    #[test]
    #[should_panic]
    fn creating_model_instance_node_with_missing_parent_fails() {
        let mut scene_graph = SceneGraph::new();
        let root_id = scene_graph.root_node_id();

        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);

        create_dummy_model_instance_node(&mut scene_graph, group_node_id);
    }

    #[test]
    #[should_panic]
    fn creating_camera_node_with_missing_parent_fails() {
        let mut scene_graph = SceneGraph::new();
        let root_id = scene_graph.root_node_id();

        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);

        create_dummy_camera_node(&mut scene_graph, group_node_id);
    }

    #[test]
    #[should_panic]
    fn removing_root_node_fails() {
        let mut scene_graph = SceneGraph::new();
        scene_graph.remove_group_node(scene_graph.root_node_id());
    }

    #[test]
    #[should_panic]
    fn removing_group_node_twice_fails() {
        let mut scene_graph = SceneGraph::new();
        let root_id = scene_graph.root_node_id();
        let group_node_id = create_dummy_group_node(&mut scene_graph, root_id);
        scene_graph.remove_group_node(group_node_id);
        scene_graph.remove_group_node(group_node_id);
    }

    #[test]
    #[should_panic]
    fn removing_model_instance_node_twice_fails() {
        let mut scene_graph = SceneGraph::new();
        let root_id = scene_graph.root_node_id();
        let model_instance_node_id = create_dummy_model_instance_node(&mut scene_graph, root_id);
        scene_graph.remove_model_instance_node(model_instance_node_id);
        scene_graph.remove_model_instance_node(model_instance_node_id);
    }

    #[test]
    #[should_panic]
    fn removing_camera_node_twice_fails() {
        let mut scene_graph = SceneGraph::new();
        let root_id = scene_graph.root_node_id();
        let camera_node_id = create_dummy_camera_node(&mut scene_graph, root_id);
        scene_graph.remove_camera_node(camera_node_id);
        scene_graph.remove_camera_node(camera_node_id);
    }

    #[test]
    fn computing_root_to_camera_transform_with_only_camera_transforms_works() {
        let camera_to_root_transform = Isometry3::from_parts(
            Vector3::new(2.1, -5.9, 0.01),
            UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3),
        );

        let mut scene_graph = SceneGraph::new();
        let root = scene_graph.root_node_id();
        let camera = scene_graph.create_camera_node(root, camera_to_root_transform);

        let root_to_camera_transform =
            scene_graph.compute_view_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(root_to_camera_transform, camera_to_root_transform.inverse());
    }

    #[test]
    fn computing_root_to_camera_transform_with_only_identity_parent_to_model_transforms_works() {
        let mut scene_graph = SceneGraph::new();
        let root = scene_graph.root_node_id();
        let group_1 = scene_graph.create_group_node(root, Isometry3::identity());
        let group_2 = scene_graph.create_group_node(group_1, Isometry3::identity());
        let group_3 = scene_graph.create_group_node(group_2, Isometry3::identity());
        let camera = scene_graph.create_camera_node(group_3, Isometry3::identity());

        scene_graph.update_all_group_to_root_transforms();

        let transform = scene_graph.compute_view_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(transform, Isometry3::identity());
    }

    #[test]
    fn computing_root_to_camera_transform_with_different_parent_to_model_transforms_works() {
        let translation = Vector3::new(2.1, -5.9, 0.01);
        let rotation = UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3);

        let mut scene_graph = SceneGraph::new();
        let root = scene_graph.root_node_id();
        let group_1 = scene_graph.create_group_node(
            root,
            Isometry3::from_parts(translation, UnitQuaternion::identity()),
        );
        let group_2 = scene_graph
            .create_group_node(group_1, Isometry3::from_parts(Vector3::zeros(), rotation));
        let camera = scene_graph.create_camera_node(
            group_2,
            Isometry3::from_parts(Vector3::zeros(), UnitQuaternion::identity()),
        );

        scene_graph.update_all_group_to_root_transforms();

        let root_to_camera_transform =
            scene_graph.compute_view_transform(scene_graph.camera_nodes.node(camera));

        assert_abs_diff_eq!(
            root_to_camera_transform,
            Isometry3::from_parts(translation, rotation).inverse(),
            epsilon = 1e-7
        );
    }

    fn assert_spheres_equal(sphere_1: &Sphere, sphere_2: &Sphere) {
        assert_abs_diff_eq!(sphere_1.center(), sphere_2.center());
        assert_abs_diff_eq!(sphere_1.radius(), sphere_2.radius());
    }

    #[test]
    fn updating_bounding_spheres_with_one_transformed_instance_in_world_space_works() {
        let model_to_parent_transform = Similarity3::from_parts(
            Vector3::new(2.1, -5.9, 0.01),
            UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3),
            7.0,
        );
        let bounding_sphere = Sphere::new(Point3::new(3.9, 5.2, 0.0), 11.1);

        let mut scene_graph = SceneGraph::new();
        let root = scene_graph.root_node_id();

        let model_instance_node_id = scene_graph.create_model_instance_node(
            root,
            model_to_parent_transform,
            create_dummy_model_id(""),
            Some(bounding_sphere),
            create_dummy_model_instance_rendering_feature_ids(),
            FeatureIDSet::new(),
            ModelInstanceFlags::empty(),
        );

        scene_graph.update_all_bounding_spheres();
        let root_bounding_sphere = scene_graph.group_nodes().node(root).get_bounding_sphere();
        assert_spheres_equal(
            root_bounding_sphere.unwrap(),
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
    fn updating_bounding_spheres_with_two_instances_in_world_space_works() {
        let bounding_sphere_1 = Sphere::new(Point3::new(3.9, 5.2, 0.0), 11.1);
        let bounding_sphere_2 = Sphere::new(Point3::new(-0.4, 7.7, 2.9), 4.8);

        let mut scene_graph = SceneGraph::new();
        let root = scene_graph.root_node_id();

        scene_graph.create_model_instance_node(
            root,
            Similarity3::identity(),
            create_dummy_model_id("1"),
            Some(bounding_sphere_1),
            create_dummy_model_instance_rendering_feature_ids(),
            FeatureIDSet::new(),
            ModelInstanceFlags::empty(),
        );
        scene_graph.create_model_instance_node(
            root,
            Similarity3::identity(),
            create_dummy_model_id("2"),
            Some(bounding_sphere_2),
            create_dummy_model_instance_rendering_feature_ids(),
            FeatureIDSet::new(),
            ModelInstanceFlags::empty(),
        );

        scene_graph.update_all_bounding_spheres();
        let root_bounding_sphere = scene_graph.group_nodes().node(root).get_bounding_sphere();
        assert_spheres_equal(
            root_bounding_sphere.unwrap(),
            &Sphere::bounding_sphere_from_pair(&bounding_sphere_1, &bounding_sphere_2),
        );
    }

    #[test]
    fn updating_bounding_spheres_with_nested_instances_works() {
        let bounding_sphere_1 = Sphere::new(Point3::new(3.9, 5.2, 0.0), 11.1);
        let bounding_sphere_2 = Sphere::new(Point3::new(-0.4, 7.7, 2.9), 4.8);

        let group_1_to_parent_transform =
            Isometry3::from_parts(Vector3::new(2.1, -5.9, 0.01), UnitQuaternion::identity());
        let group_2_to_parent_transform = Isometry3::from_parts(
            Vector3::new(0.01, 2.9, 10.1),
            UnitQuaternion::from_euler_angles(1.1, 2.2, 3.3),
        );
        let model_instance_2_to_parent_transform = Similarity3::from_parts(
            Vector3::new(-2.1, 8.9, 1.01),
            UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3),
            1.0,
        );

        let mut scene_graph = SceneGraph::new();
        let root = scene_graph.root_node_id();

        let group_1 = scene_graph.create_group_node(root, group_1_to_parent_transform);
        scene_graph.create_model_instance_node(
            group_1,
            Similarity3::identity(),
            create_dummy_model_id("1"),
            Some(bounding_sphere_1),
            create_dummy_model_instance_rendering_feature_ids(),
            FeatureIDSet::new(),
            ModelInstanceFlags::empty(),
        );
        let group_2 = scene_graph.create_group_node(group_1, group_2_to_parent_transform);
        scene_graph.create_model_instance_node(
            group_2,
            model_instance_2_to_parent_transform,
            create_dummy_model_id("2"),
            Some(bounding_sphere_2),
            create_dummy_model_instance_rendering_feature_ids(),
            FeatureIDSet::new(),
            ModelInstanceFlags::empty(),
        );

        let correct_group_2_bounding_sphere =
            bounding_sphere_2.transformed(&model_instance_2_to_parent_transform);
        let correct_group_1_bounding_sphere = Sphere::bounding_sphere_from_pair(
            &bounding_sphere_1,
            &correct_group_2_bounding_sphere.translated_and_rotated(&group_2_to_parent_transform),
        );
        let correct_root_bounding_sphere =
            correct_group_1_bounding_sphere.translated_and_rotated(&group_1_to_parent_transform);

        scene_graph.update_all_bounding_spheres();
        let root_bounding_sphere = scene_graph.group_nodes().node(root).get_bounding_sphere();

        assert_spheres_equal(root_bounding_sphere.unwrap(), &correct_root_bounding_sphere);

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
        let mut scene_graph = SceneGraph::new();
        let root = scene_graph.root_node_id();
        let group_1 = scene_graph.create_group_node(root, Isometry3::identity());
        let group_2 = scene_graph.create_group_node(group_1, Isometry3::identity());
        scene_graph.update_all_bounding_spheres();
        let root_bounding_sphere = scene_graph.group_nodes().node(root).get_bounding_sphere();
        assert!(root_bounding_sphere.is_none());
        assert!(
            scene_graph
                .group_nodes
                .node(group_1)
                .get_bounding_sphere()
                .is_none()
        );
        assert!(
            scene_graph
                .group_nodes
                .node(group_2)
                .get_bounding_sphere()
                .is_none()
        );
    }
}
