//! Scene graph implementation.

use crate::{
    geometry::{
        Camera, CameraID, CameraRepository, Frustum, ModelID, ModelInstance, ModelInstancePool,
        Sphere,
    },
    num::Float,
    util::VecWithFreeList,
};
use anyhow::{anyhow, Result};
use nalgebra::{Matrix4, Similarity3};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct SceneGraph<F: Float> {
    root_node_id: GroupNodeID,
    group_nodes: NodeStorage<GroupNode<F>>,
    model_instance_nodes: NodeStorage<ModelInstanceNode<F>>,
    camera_nodes: NodeStorage<CameraNode<F>>,
}

#[derive(Clone, Debug, Default)]
pub struct NodeStorage<N> {
    nodes: VecWithFreeList<N>,
}

pub trait SceneGraphNode {
    type ID;
    type Transform;

    fn set_model_transform(&mut self, transform: Self::Transform);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupNodeID(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelInstanceNodeID(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CameraNodeID(usize);

pub trait NodeIDToIdx {
    fn idx(&self) -> usize;
}

trait IdxToNodeID {
    fn from_idx(idx: usize) -> Self;
}

#[derive(Clone, Debug)]
pub struct GroupNode<F: Float> {
    parent_node_id: Option<GroupNodeID>,
    model_transform: Similarity3<F>,
    child_group_node_ids: HashSet<GroupNodeID>,
    child_model_instance_node_ids: HashSet<ModelInstanceNodeID>,
    child_camera_node_ids: HashSet<CameraNodeID>,
    bounding_sphere: Option<Sphere<F>>,
}

#[derive(Clone, Debug)]
pub struct ModelInstanceNode<F: Float> {
    parent_node_id: GroupNodeID,
    model_transform: Similarity3<F>,
    model_id: ModelID,
    model_bounding_sphere: Sphere<F>,
}

#[derive(Clone, Debug)]
pub struct CameraNode<F: Float> {
    parent_node_id: GroupNodeID,
    model_transform: Similarity3<F>,
    camera_id: CameraID,
}

pub type SceneGraphStorages<F> = (
    NodeStorage<GroupNode<F>>,
    NodeStorage<ModelInstanceNode<F>>,
    NodeStorage<CameraNode<F>>,
);

impl<F: Float> SceneGraph<F> {
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

    pub fn root_node_id(&self) -> GroupNodeID {
        self.root_node_id
    }

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

    pub fn create_model_instance_node(
        &mut self,

        parent_node_id: GroupNodeID,
        transform: <ModelInstanceNode<F> as SceneGraphNode>::Transform,
        model_id: ModelID,
    ) -> ModelInstanceNodeID {
        let model_instance_node = ModelInstanceNode::new(parent_node_id, transform, model_id);
        let model_instance_node_id = self.model_instance_nodes.add_node(model_instance_node);
        self.group_nodes
            .node_mut(parent_node_id)
            .add_child_model_instance_node(model_instance_node_id);
        model_instance_node_id
    }

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

    pub fn remove_group_node(&mut self, group_node_id: GroupNodeID) {
        let group_node = self.group_nodes.node(group_node_id);

        let parent_node_id = group_node.parent_node_id();

        let (child_group_node_ids, child_model_instance_node_ids, child_camera_node_ids) =
            group_node.obtain_child_node_ids();

        self.group_nodes.remove_node(group_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_group_node(group_node_id);

        for group_node_id in child_group_node_ids {
            self.remove_group_node(group_node_id);
        }
        for model_instance_node_id in child_model_instance_node_ids {
            self.remove_model_instance_node(model_instance_node_id);
        }
        for camera_node_id in child_camera_node_ids {
            self.remove_camera_node(camera_node_id);
        }
    }

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

    pub fn remove_camera_node(&mut self, camera_node_id: CameraNodeID) {
        let parent_node_id = self.camera_nodes.node(camera_node_id).parent_node_id();
        self.camera_nodes.remove_node(camera_node_id);
        self.group_nodes
            .node_mut(parent_node_id)
            .remove_child_camera_node(camera_node_id);
    }

    pub fn take_storages(&mut self) -> SceneGraphStorages<F> {
        let group_nodes = std::mem::replace(&mut self.group_nodes, NodeStorage::new());
        let model_instance_nodes =
            std::mem::replace(&mut self.model_instance_nodes, NodeStorage::new());
        let camera_nodes = std::mem::replace(&mut self.camera_nodes, NodeStorage::new());
        (group_nodes, model_instance_nodes, camera_nodes)
    }

    pub fn return_storages(&mut self, storages: SceneGraphStorages<F>) {
        let (group_nodes, model_instance_nodes, camera_nodes) = storages;
        self.group_nodes = group_nodes;
        self.model_instance_nodes = model_instance_nodes;
        self.camera_nodes = camera_nodes;
    }

    pub fn update_model_instances(
        &mut self,
        camera_repository: &CameraRepository<F>,
        model_instance_pool: &mut ModelInstancePool<F>,
        camera_node_id: CameraNodeID,
    ) -> Result<()> {
        self.update_bounding_spheres(self.root_node_id());

        let camera_node = self.camera_nodes.node(camera_node_id);

        let camera_id = camera_node.camera_id;
        let camera = camera_repository
            .perspective_cameras
            .get(&camera_id)
            .ok_or_else(|| anyhow!("Camera {} not found", camera_id))?;

        let mut world_view_transform =
            Similarity3::from_isometry(*camera.config().view_transform(), F::one());

        let mut parent_node = self.group_nodes.node(camera_node.parent_node_id());

        while !parent_node.is_root() {
            world_view_transform *= parent_node.model_transform();
            parent_node = self.group_nodes.node(parent_node.parent_node_id());
        }

        let camera_frustum = Frustum::from_transform(camera.projection_transform());

        self.update_model_view_projection_transforms_for_group(
            model_instance_pool,
            &camera_frustum,
            self.root_node_id(),
            &world_view_transform,
        );

        Ok(())
    }

    fn update_model_view_projection_transforms_for_group(
        &self,
        model_instance_pool: &mut ModelInstancePool<F>,
        camera_frustum: &Frustum<F>,
        group_node_id: GroupNodeID,
        parent_model_view_transform: &Similarity3<F>,
    ) {
        let group_node = self.group_nodes.node(group_node_id);

        let model_view_transform = parent_model_view_transform * group_node.model_transform();

        if let Some(bounding_sphere) = group_node.get_bounding_sphere() {
            let bounding_sphere_view_space = bounding_sphere.transformed(&model_view_transform);

            if !camera_frustum.sphere_lies_outside(&bounding_sphere_view_space) {
                for &group_node_id in group_node.child_group_node_ids() {
                    self.update_model_view_projection_transforms_for_group(
                        model_instance_pool,
                        camera_frustum,
                        group_node_id,
                        &model_view_transform,
                    );
                }

                let model_view_projection_transform =
                    camera_frustum.transform_matrix() * model_view_transform.to_homogeneous();

                for &model_instance_node_id in group_node.child_model_instance_node_ids() {
                    self.update_model_view_projection_transform_of_model_instance(
                        model_instance_pool,
                        model_instance_node_id,
                        &model_view_projection_transform,
                    );
                }
            }
        }
    }

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

    fn find_model_instance_bounding_sphere(
        &self,
        model_instance_node_id: ModelInstanceNodeID,
    ) -> Sphere<F> {
        let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

        model_instance_node
            .model_bounding_sphere()
            .transformed(model_instance_node.model_transform())
    }

    fn update_model_view_projection_transform_of_model_instance(
        &self,
        model_instance_pool: &mut ModelInstancePool<F>,
        model_instance_node_id: ModelInstanceNodeID,
        parent_model_view_projection_transform: &Matrix4<F>,
    ) {
        let model_instance_node = self.model_instance_nodes.node(model_instance_node_id);

        if let Some(buffer) = model_instance_pool
            .model_instance_buffers
            .get_mut(&model_instance_node.model_id())
        {
            let model_view_projection_transform = parent_model_view_projection_transform
                * model_instance_node.model_transform().to_homogeneous();

            buffer.add_instance(ModelInstance::with_transform(
                model_view_projection_transform,
            ))
        }
    }
}

impl<F: Float> Default for SceneGraph<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: SceneGraphNode> NodeStorage<N> {
    pub fn set_node_transform(&mut self, node_id: N::ID, transform: N::Transform)
    where
        N::ID: NodeIDToIdx,
    {
        self.node_mut(node_id).set_model_transform(transform);
    }

    fn new() -> Self {
        Self {
            nodes: VecWithFreeList::new(),
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

    fn inverse_model_transform(&self) -> Similarity3<F> {
        self.model_transform.inverse()
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

    fn set_parent_node_id(&mut self, parent_node_id: GroupNodeID) {
        self.parent_node_id = Some(parent_node_id);
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
    fn new(
        parent_node_id: GroupNodeID,
        model_transform: Similarity3<F>,
        model_id: ModelID,
    ) -> Self {
        let model_bounding_sphere = todo!();
        Self {
            parent_node_id,
            model_transform,
            model_id,
            model_bounding_sphere,
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
            fn from_idx(idx: usize) -> Self {
                Self(idx)
            }
        }
        impl NodeIDToIdx for $node_id_type {
            fn idx(&self) -> usize {
                self.0
            }
        }
    };
}

impl_node_id_idx_traits!(GroupNodeID);
impl_node_id_idx_traits!(ModelInstanceNodeID);
impl_node_id_idx_traits!(CameraNodeID);
