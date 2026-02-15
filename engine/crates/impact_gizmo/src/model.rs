//! Gizmo models.

use crate::{GizmoDepthClipping, GizmoObscurability, GizmoType};
use impact_math::hash64;
use impact_mesh::{LineSegmentMeshID, MeshID, MeshPrimitive, TriangleMeshID};
use impact_scene::model::ModelID;
use std::sync::LazyLock;

/// A model defining the geometric and visual attributes of a gizmo or part of a
/// gizmo.
#[derive(Clone, Debug)]
pub struct GizmoModel {
    /// The ID of the mesh used for the model.
    pub mesh_id: MeshID,
    /// The model ID used by this gizmo model. It is the key under which the
    /// model-view transforms to apply to the mesh during rendering are buffered
    /// in the model instance manager.
    pub model_id: ModelID,
    /// The geometric primitive used for this gizmo model's mesh.
    pub mesh_primitive: MeshPrimitive,
    /// Whether this gizmo mode can be obscured by geometry in front of it.
    pub obscurability: GizmoObscurability,
    /// Whether this gizmo should be clipped against the camera's near and far
    /// plane.
    pub depth_clipping: GizmoDepthClipping,
}

impl GizmoModel {
    pub fn mesh_id(&self) -> MeshID {
        self.mesh_id
    }

    pub fn model_id(&self) -> &ModelID {
        &self.model_id
    }

    pub fn triangle_mesh_id(&self) -> TriangleMeshID {
        match self.mesh_id {
            MeshID::Triangle(id) => id,
            MeshID::LineSegment(_) => {
                panic!("Got line segment mesh when expecting triangle mesh in `GizmoModel`")
            }
        }
    }

    pub fn line_segment_mesh_id(&self) -> LineSegmentMeshID {
        match self.mesh_id {
            MeshID::LineSegment(id) => id,
            MeshID::Triangle(_) => {
                panic!("Got triangle mesh when expecting line segment mesh in `GizmoModel`")
            }
        }
    }
}

/// Returns the set of models used for each gizmo.
pub fn gizmo_models() -> &'static [Vec<GizmoModel>; GizmoType::count()] {
    &GIZMO_MODELS
}

/// Returns each gizmo model whose mesh is of the given type and that has the
/// given obscurability and depth clipping.
pub fn select_gizmo_models(
    primitive: MeshPrimitive,
    obscurability: GizmoObscurability,
    depth_clipping: GizmoDepthClipping,
) -> impl IntoIterator<Item = &'static GizmoModel> {
    gizmo_models().iter().flatten().filter(move |model| {
        model.mesh_primitive == primitive
            && model.obscurability == obscurability
            && model.depth_clipping == depth_clipping
    })
}

static GIZMO_MODELS: LazyLock<[Vec<GizmoModel>; GizmoType::count()]> =
    LazyLock::new(|| GizmoType::all().map(define_models_for_gizmo));

fn define_models_for_gizmo(gizmo: GizmoType) -> Vec<GizmoModel> {
    match gizmo {
        GizmoType::ReferenceFrameAxes
        | GizmoType::LinearVelocity
        | GizmoType::AngularVelocity
        | GizmoType::AngularMomentum
        | GizmoType::Force
        | GizmoType::Torque => {
            vec![define_non_obscurable_line_segment_model(gizmo.label())]
        }
        GizmoType::BoundingVolume => {
            vec![define_obscurable_triangle_model(gizmo.label())]
        }
        GizmoType::LightSphere => {
            vec![define_obscurable_triangle_model_with_unclipped_depth(
                gizmo.label(),
            )]
        }
        GizmoType::CenterOfMass | GizmoType::Anchors | GizmoType::VoxelIntersections => {
            vec![define_non_obscurable_triangle_model(gizmo.label())]
        }
        GizmoType::DynamicCollider | GizmoType::StaticCollider | GizmoType::PhantomCollider => {
            vec![
                define_non_obscurable_triangle_model(format!("{} sphere", gizmo.label())),
                define_non_obscurable_triangle_model(format!("{} plane", gizmo.label())),
                define_obscurable_triangle_model(format!("{} voxel sphere", gizmo.label())),
            ]
        }
        GizmoType::VoxelChunks => {
            vec![
                define_obscurable_triangle_model(format!(
                    "{} (uniform, obscurable)",
                    gizmo.label()
                )),
                define_obscurable_triangle_model(format!(
                    "{} (non-uniform, obscurable)",
                    gizmo.label()
                )),
                define_obscurable_triangle_model(format!("{} (empty, obscurable)", gizmo.label())),
                define_non_obscurable_triangle_model(format!(
                    "{} (uniform, non-obscurable)",
                    gizmo.label()
                )),
                define_non_obscurable_triangle_model(format!(
                    "{} (non-uniform, non-obscurable)",
                    gizmo.label()
                )),
                define_non_obscurable_triangle_model(format!(
                    "{} (empty, non-obscurable)",
                    gizmo.label()
                )),
            ]
        }
        GizmoType::ShadowCubemapFaces => {
            vec![
                define_obscurable_triangle_model_with_unclipped_depth(format!(
                    "{} planes",
                    gizmo.label()
                )),
                define_non_obscurable_line_segment_model_with_unclipped_depth(format!(
                    "{} outlines",
                    gizmo.label()
                )),
            ]
        }
        GizmoType::ShadowMapCascades => {
            vec![
                define_obscurable_triangle_model_with_unclipped_depth(format!(
                    "{} plane 0",
                    gizmo.label()
                )),
                define_obscurable_triangle_model_with_unclipped_depth(format!(
                    "{} plane 1",
                    gizmo.label()
                )),
                define_obscurable_triangle_model_with_unclipped_depth(format!(
                    "{} plane 2",
                    gizmo.label()
                )),
                define_obscurable_triangle_model_with_unclipped_depth(format!(
                    "{} plane 3",
                    gizmo.label()
                )),
            ]
        }
    }
}

pub const SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX: usize = 0;
pub const SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX: usize = 1;

pub const COLLIDER_GIZMO_SPHERE_MODEL_IDX: usize = 0;
pub const COLLIDER_GIZMO_PLANE_MODEL_IDX: usize = 1;
pub const COLLIDER_GIZMO_VOXEL_SPHERE_MODEL_IDX: usize = 2;

pub const VOXEL_CHUNKS_GIZMO_OBSCURABLE_UNIFORM_MODEL_IDX: usize = 0;
pub const VOXEL_CHUNKS_GIZMO_OBSCURABLE_NON_UNIFORM_MODEL_IDX: usize = 1;
pub const VOXEL_CHUNKS_GIZMO_OBSCURABLE_EMPTY_MODEL_IDX: usize = 2;
pub const VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_UNIFORM_MODEL_IDX: usize = 3;
pub const VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_NON_UNIFORM_MODEL_IDX: usize = 4;
pub const VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_EMPTY_MODEL_IDX: usize = 5;

fn define_obscurable_triangle_model(label: impl AsRef<str>) -> GizmoModel {
    let (mesh_id, model_id) = create_triangle_mesh_and_model_id(label);
    GizmoModel {
        mesh_id,
        model_id,
        mesh_primitive: MeshPrimitive::Triangle,
        obscurability: GizmoObscurability::Obscurable,
        depth_clipping: GizmoDepthClipping::Enabled,
    }
}

fn define_obscurable_triangle_model_with_unclipped_depth(label: impl AsRef<str>) -> GizmoModel {
    let (mesh_id, model_id) = create_triangle_mesh_and_model_id(label);
    GizmoModel {
        mesh_id,
        model_id,
        mesh_primitive: MeshPrimitive::Triangle,
        obscurability: GizmoObscurability::Obscurable,
        depth_clipping: GizmoDepthClipping::Disabled,
    }
}

fn define_non_obscurable_triangle_model(label: impl AsRef<str>) -> GizmoModel {
    let (mesh_id, model_id) = create_triangle_mesh_and_model_id(label);
    GizmoModel {
        mesh_id,
        model_id,
        mesh_primitive: MeshPrimitive::Triangle,
        obscurability: GizmoObscurability::NonObscurable,
        depth_clipping: GizmoDepthClipping::Enabled,
    }
}

fn define_non_obscurable_line_segment_model(label: impl AsRef<str>) -> GizmoModel {
    let (mesh_id, model_id) = create_line_segment_mesh_and_model_id(label);
    GizmoModel {
        mesh_id,
        model_id,
        mesh_primitive: MeshPrimitive::LineSegment,
        obscurability: GizmoObscurability::NonObscurable,
        depth_clipping: GizmoDepthClipping::Enabled,
    }
}

fn define_non_obscurable_line_segment_model_with_unclipped_depth(
    label: impl AsRef<str>,
) -> GizmoModel {
    let (mesh_id, model_id) = create_line_segment_mesh_and_model_id(label);
    GizmoModel {
        mesh_id,
        model_id,
        mesh_primitive: MeshPrimitive::LineSegment,
        obscurability: GizmoObscurability::NonObscurable,
        depth_clipping: GizmoDepthClipping::Disabled,
    }
}

fn create_triangle_mesh_and_model_id(label: impl AsRef<str>) -> (MeshID, ModelID) {
    let hash = hash64!(label.as_ref());
    (
        MeshID::Triangle(TriangleMeshID(hash)),
        ModelID::hash_only(hash.hash()),
    )
}

fn create_line_segment_mesh_and_model_id(label: impl AsRef<str>) -> (MeshID, ModelID) {
    let hash = hash64!(label.as_ref());
    (
        MeshID::LineSegment(LineSegmentMeshID(hash)),
        ModelID::hash_only(hash.hash()),
    )
}
