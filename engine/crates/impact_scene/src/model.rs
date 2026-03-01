//! Models defined by a mesh and material.

use crate::graph::{ModelInstanceFlags, ModelInstanceNode, SceneGraph};
use bytemuck::Zeroable;
use impact_camera::Camera;
use impact_geometry::{AxisAlignedBox, OrientedBox};
use impact_intersection::IntersectionManager;
use impact_material::{MaterialID, MaterialRegistry};
use impact_math::{
    hash::{Hash64, compute_hash_64_of_two_hash_64},
    point::Point3,
    transform::Similarity3,
};
use impact_mesh::{LineSegmentMeshID, MeshID, TriangleMeshID};
use impact_model::{
    InstanceFeature, ModelInstanceID,
    transform::{InstanceModelViewTransform, InstanceModelViewTransformWithPrevious},
};
use std::{
    cmp, fmt,
    hash::{Hash, Hasher},
};

/// Identifier for specific models.
///
/// A model is uniquely defined by its mesh and material.
#[derive(Copy, Clone, Debug)]
pub struct ModelID {
    mesh_id: MeshID,
    material_id: MaterialID,
    hash: Hash64,
}

pub type ModelInstanceManager = impact_model::ModelInstanceManager<ModelID>;
pub type ModelInstanceManagerState = impact_model::ModelInstanceManagerState<ModelID>;

pub type ModelInstanceGPUBufferMap = impact_model::gpu_resource::ModelInstanceGPUBufferMap<ModelID>;

impl ModelID {
    /// Creates a new [`ModelID`] for the model comprised of the given triangle
    /// mesh and material.
    pub fn for_triangle_mesh_and_material(
        mesh_id: TriangleMeshID,
        material_id: MaterialID,
    ) -> Self {
        let hash = compute_hash_64_of_two_hash_64(mesh_id.0.hash(), material_id.0.hash());
        Self {
            mesh_id: MeshID::Triangle(mesh_id),
            material_id,
            hash,
        }
    }

    /// Creates a new [`ModelID`] for the model comprised of the given line
    /// segment mesh and material.
    pub fn for_line_segment_mesh_and_material(
        mesh_id: LineSegmentMeshID,
        material_id: MaterialID,
    ) -> Self {
        let hash = compute_hash_64_of_two_hash_64(mesh_id.0.hash(), material_id.0.hash());
        Self {
            mesh_id: MeshID::LineSegment(mesh_id),
            material_id,
            hash,
        }
    }

    /// Creates a new [`ModelID`] with the given hash. The [`ModelID::mesh_id`]
    /// and [`ModelID::material_id`] methods on this `ModelID` will return dummy
    /// values.
    pub fn hash_only(hash: Hash64) -> Self {
        Self {
            mesh_id: MeshID::Triangle(TriangleMeshID::zeroed()),
            material_id: MaterialID::not_applicable(),
            hash,
        }
    }

    /// The ID of the model's mesh.
    pub fn mesh_id(&self) -> MeshID {
        self.mesh_id
    }

    /// The ID of the model's triangle mesh.
    ///
    /// # Panics
    /// If the mesh is not a triangle mesh.
    pub fn triangle_mesh_id(&self) -> TriangleMeshID {
        match self.mesh_id {
            MeshID::Triangle(id) => id,
            MeshID::LineSegment(_) => {
                panic!("Got line segment mesh when expecting triangle mesh in `ModelID`")
            }
        }
    }

    /// The ID of the model's line segment mesh.
    ///
    /// # Panics
    /// If the mesh is not a line segment mesh.
    pub fn line_segment_mesh_id(&self) -> LineSegmentMeshID {
        match self.mesh_id {
            MeshID::LineSegment(id) => id,
            MeshID::Triangle(_) => {
                panic!("Got triangle mesh when expecting line segment mesh in `ModelID`")
            }
        }
    }

    /// The ID of the model's material.
    pub fn material_id(&self) -> MaterialID {
        self.material_id
    }
}

impl fmt::Display for ModelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{mesh: {}, material: {}}}",
            self.mesh_id, self.material_id,
        )
    }
}

impl PartialEq for ModelID {
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash)
    }
}

impl Eq for ModelID {}

impl Ord for ModelID {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl PartialOrd for ModelID {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for ModelID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

/// Computes the model-to-camera space transforms of all the model instances
/// in the scene graph that are visible with the specified camera and adds
/// them to the model instance manager.
///
/// # Returns
/// The camera-space AABB encompassing all the buffered model instances, or
/// [`None`] if there are no visible model instances.
///
/// # Warning
/// Make sure to call [`SceneGraph::sync_camera_view_transform`] and build the
/// bounding volume hierarchy before calling this method.
pub fn buffer_model_instances_for_rendering(
    material_registry: &MaterialRegistry,
    model_instance_manager: &mut ModelInstanceManager,
    intersection_manager: &IntersectionManager,
    scene_graph: &SceneGraph,
    camera: &Camera,
    current_frame_number: u32,
) -> Option<AxisAlignedBox> {
    let world_space_view_frustum = camera.compute_world_space_view_frustum();

    let world_to_camera_transform = camera.view_transform();

    let mut min_camera_space_coords = Point3::same(f32::INFINITY);
    let mut max_camera_space_coords = Point3::same(f32::NEG_INFINITY);
    let mut found_visible_instances = false;

    intersection_manager.for_each_bounding_volume_maybe_in_frustum(
        &world_space_view_frustum,
        |id, aabb| {
            let model_instance_id = ModelInstanceID::from_entity_id(id.as_entity_id());
            let Some(model_instance_node) = scene_graph
                .model_instance_nodes()
                .get_node(model_instance_id)
            else {
                return;
            };

            let camera_space_obb = OrientedBox::from_axis_aligned_box(&aabb.aligned())
                .iso_transformed(world_to_camera_transform);

            // TODO: Optimize with Jim Arvo method
            for corner in camera_space_obb.compute_corners() {
                min_camera_space_coords = min_camera_space_coords.min_with(&corner);
                max_camera_space_coords = max_camera_space_coords.max_with(&corner);
            }

            found_visible_instances = true;

            buffer_model_instance_for_rendering(
                material_registry,
                model_instance_manager,
                scene_graph,
                camera,
                current_frame_number,
                model_instance_node,
            );
        },
    );

    if found_visible_instances {
        Some(AxisAlignedBox::new(
            min_camera_space_coords,
            max_camera_space_coords,
        ))
    } else {
        None
    }
}

fn buffer_model_instance_for_rendering(
    material_registry: &MaterialRegistry,
    model_instance_manager: &mut ModelInstanceManager,
    scene_graph: &SceneGraph,
    camera: &Camera,
    current_frame_number: u32,
    model_instance_node: &ModelInstanceNode,
) {
    let model_view_transform =
        compute_model_view_transform_for_node(scene_graph, camera, model_instance_node);

    let instance_model_view_transform = InstanceModelViewTransform::from(&model_view_transform);

    model_instance_manager
        .feature_mut::<InstanceModelViewTransformWithPrevious>(
            model_instance_node
                .get_rendering_feature_id_of_type(
                    InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID,
                )
                .unwrap(),
        )
        .set_transform_for_new_frame(instance_model_view_transform);

    let model_id = model_instance_node.model_id();

    model_instance_manager.buffer_instance_features_from_storages(
        model_id,
        model_instance_node.feature_ids_for_rendering(),
    );

    if !model_instance_node
        .flags()
        .contains(ModelInstanceFlags::HAS_INDEPENDENT_MATERIAL_VALUES)
        && let Some(material) = material_registry.get(model_id.material_id())
    {
        material
            .property_values
            .buffer(model_instance_manager, model_id);
    }

    model_instance_node.declare_visible_this_frame(current_frame_number);
}

fn compute_model_view_transform_for_node(
    scene_graph: &SceneGraph,
    camera: &Camera,
    model_instance_node: &ModelInstanceNode,
) -> Similarity3 {
    let model_to_parent_transform = model_instance_node.model_to_parent_transform().aligned();

    let model_to_world_transform =
        if model_instance_node.parent_group_id() == scene_graph.root_node_id() {
            model_to_parent_transform
        } else {
            let parent_to_world_transform = scene_graph
                .group_nodes()
                .node(model_instance_node.parent_group_id())
                .group_to_root_transform()
                .aligned();

            parent_to_world_transform * model_to_parent_transform
        };

    camera.view_transform() * model_to_world_transform
}
