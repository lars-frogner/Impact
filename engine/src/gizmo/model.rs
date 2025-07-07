//! Gizmo models.

use crate::gizmo::{GizmoObscurability, GizmoType};
use impact_material::MaterialHandle;
use impact_math::hash64;
use impact_mesh::{LineSegmentMeshID, MeshID, MeshPrimitive, TriangleMeshID};
use impact_scene::model::ModelID;
use std::sync::LazyLock;

/// A model defining the geometric and visual attributes of a gizmo or part of a
/// gizmo.
#[derive(Clone, Debug)]
pub struct GizmoModel {
    /// The model ID used by this gizmo model. It holds the ID of the mesh used
    /// for the model. It is also the key under which the model-view transforms
    /// to apply to the mesh during rendering are buffered in the instance
    /// feature manager.
    pub model_id: ModelID,
    /// The geometric primitive used for this gizmo model's mesh.
    pub mesh_primitive: MeshPrimitive,
    /// Whether this gizmo mode can be obscured by geometry in front of it.
    pub obscurability: GizmoObscurability,
}

impl GizmoModel {
    pub fn mesh_id(&self) -> MeshID {
        self.model_id.mesh_id()
    }

    pub fn triangle_mesh_id(&self) -> TriangleMeshID {
        self.model_id.triangle_mesh_id()
    }

    pub fn line_segment_mesh_id(&self) -> LineSegmentMeshID {
        self.model_id.line_segment_mesh_id()
    }
}

/// Returns the set of models used for each gizmo.
pub fn gizmo_models() -> &'static [Vec<GizmoModel>; GizmoType::count()] {
    &GIZMO_MODELS
}

/// Returns the model ID used by each gizmo model whose mesh is of the given
/// type and that has the given obscurability.
pub fn gizmo_model_ids_for_mesh_primitive_and_obscurability(
    primitive: MeshPrimitive,
    obscurability: GizmoObscurability,
) -> impl IntoIterator<Item = &'static ModelID> {
    gizmo_models().iter().flatten().filter_map(move |model| {
        if model.mesh_primitive == primitive && model.obscurability == obscurability {
            Some(&model.model_id)
        } else {
            None
        }
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
        GizmoType::BoundingSphere | GizmoType::LightSphere => {
            vec![define_obscurable_triangle_model(gizmo.label())]
        }
        GizmoType::CenterOfMass => {
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
                define_non_obscurable_triangle_model(format!(
                    "{} (uniform, non-obscurable)",
                    gizmo.label()
                )),
                define_non_obscurable_triangle_model(format!(
                    "{} (non-uniform, non-obscurable)",
                    gizmo.label()
                )),
            ]
        }
        GizmoType::ShadowCubemapFaces => {
            vec![
                define_non_obscurable_triangle_model(format!("{} planes", gizmo.label())),
                define_non_obscurable_line_segment_model(format!("{} outlines", gizmo.label())),
            ]
        }
        GizmoType::ShadowMapCascades => {
            vec![
                define_obscurable_triangle_model(format!("{} plane 0", gizmo.label())),
                define_obscurable_triangle_model(format!("{} plane 1", gizmo.label())),
                define_obscurable_triangle_model(format!("{} plane 2", gizmo.label())),
                define_obscurable_triangle_model(format!("{} plane 3", gizmo.label())),
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
pub const VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_UNIFORM_MODEL_IDX: usize = 2;
pub const VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_NON_UNIFORM_MODEL_IDX: usize = 3;

fn define_obscurable_triangle_model(label: impl AsRef<str>) -> GizmoModel {
    GizmoModel {
        model_id: create_triangle_model_id(label),
        mesh_primitive: MeshPrimitive::Triangle,
        obscurability: GizmoObscurability::Obscurable,
    }
}

fn define_non_obscurable_triangle_model(label: impl AsRef<str>) -> GizmoModel {
    GizmoModel {
        model_id: create_triangle_model_id(label),
        mesh_primitive: MeshPrimitive::Triangle,
        obscurability: GizmoObscurability::NonObscurable,
    }
}

fn define_non_obscurable_line_segment_model(label: impl AsRef<str>) -> GizmoModel {
    GizmoModel {
        model_id: create_line_segment_model_id(label),
        mesh_primitive: MeshPrimitive::LineSegment,
        obscurability: GizmoObscurability::NonObscurable,
    }
}

fn create_triangle_model_id(label: impl AsRef<str>) -> ModelID {
    ModelID::for_triangle_mesh_and_material(
        TriangleMeshID(hash64!(label.as_ref())),
        MaterialHandle::not_applicable(),
    )
}

fn create_line_segment_model_id(label: impl AsRef<str>) -> ModelID {
    ModelID::for_line_segment_mesh_and_material(
        LineSegmentMeshID(hash64!(label.as_ref())),
        MaterialHandle::not_applicable(),
    )
}
