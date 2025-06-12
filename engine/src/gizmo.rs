//! Simple visual elements drawn over the scene to provide technical
//! information.

pub mod components;
pub mod entity;
pub mod systems;
pub mod tasks;

use crate::{
    material::MaterialHandle,
    mesh::{self, MeshID},
    model::{
        InstanceFeature, InstanceFeatureManager, ModelID, transform::InstanceModelViewTransform,
    },
};
use bitflags::{Flags, bitflags};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// A specific gizmo type.
///
/// Gizmos are simple visual elements drawn over the scene to provide technical
/// information.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GizmoType {
    ReferenceFrameAxes = 0,
    BoundingSphere = 1,
}

bitflags! {
    /// Bitflags encoding a set of different gizmo types.
    ///
    /// Gizmos are simple visual elements drawn over the scene to provide technical
    /// information.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct GizmoSet: u8 {
        const REFERENCE_FRAME_AXES = 1 << 0;
        const BOUNDING_SPHERE      = 1 << 1;
    }
}

/// Configuration parameters for gizmos.
///
/// Gizmos are simple visual elements drawn over the scene to provide technical
/// information.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GizmoConfig {
    /// The visibility of the gizmo indicating reference frame axes.
    ///
    /// When visible, a red, green and blue line segment representing the x- y-
    /// and z-axis (respectively) of the local reference frame will be shown
    /// atop applicable entities. The lines are of unit length in the local
    /// reference frame. They meet at the original origin of the entity, so any
    /// origin offset (typically used to shift the origin to the center of mass)
    /// is not accounted for.
    pub reference_frame_visibility: GizmoVisibility,
    /// The visibility of the gizmo showing bounding spheres.
    ///
    /// When visible, the bounding spheres of models in the scene graph will be
    /// outlined by orthogonal yellow circles.
    pub bounding_sphere_visibility: GizmoVisibility,
}

/// Manager controlling the display of gizmos.
///
/// Gizmos are simple visual elements drawn over the scene to provide technical
/// information.
#[derive(Debug)]
pub struct GizmoManager {
    config: GizmoConfig,
    gizmos_with_new_global_visibility: GizmoSet,
}

/// The scope of visibility for a gizmo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GizmoVisibility {
    /// The gizmo is hidden for all entities.
    #[default]
    Hidden,
    /// The gizmo is visible for all applicable entities.
    VisibleForAll,
    /// The gizmo is visible for a selection of applicable entities.
    VisibleForSelected,
}

impl GizmoType {
    /// The number of different gizmo types.
    pub const fn count() -> usize {
        Self::all().len()
    }

    /// The array containing each gizmo type.
    pub const fn all() -> [Self; 2] {
        [Self::ReferenceFrameAxes, Self::BoundingSphere]
    }

    /// Returns an iterator over all gizmos in the given set.
    pub fn all_in_set(set: GizmoSet) -> impl Iterator<Item = Self> {
        Self::all()
            .into_iter()
            .filter(move |gizmo| set.contains(gizmo.as_set()))
    }

    /// Returns the [`GizmoSet`] containing only this gizmo type.
    pub const fn as_set(&self) -> GizmoSet {
        match self {
            Self::ReferenceFrameAxes => GizmoSet::REFERENCE_FRAME_AXES,
            Self::BoundingSphere => GizmoSet::BOUNDING_SPHERE,
        }
    }

    /// The ID of the line segment mesh used for the gizmo.
    pub fn mesh_id(&self) -> MeshID {
        match self {
            Self::ReferenceFrameAxes => mesh::reference_frame_axes_mesh_id(),
            Self::BoundingSphere => mesh::bounding_sphere_mesh_id(),
        }
    }

    /// A human-friendly name for the gizmo.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::ReferenceFrameAxes => "Reference frame axes",
            Self::BoundingSphere => "Bounding spheres",
        }
    }

    /// An explanation of the gizmo.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::ReferenceFrameAxes => {
                "\
                When enabled, a red, green and blue line segment representing the x- y- \
                and z-axis (respectively) of the local reference frame will be shown \
                atop applicable entities. The lines are of unit length in the local \
                reference frame. They meet at the original origin of the entity, so any \
                origin offset (typically used to shift the origin to the center of mass) \
                is not accounted for."
            }
            Self::BoundingSphere => {
                "\
                When enabled, the bounding spheres of models in the scene graph will be \
                outlined by orthogonal yellow circles."
            }
        }
    }

    /// The model ID used by this gizmo type. It holds the ID of the line
    /// segment mesh used for the gizmo. It is also the key under which the
    /// model-view transforms to apply to the mesh during rendering are buffered
    /// in the instance feature manager.
    pub fn model_id(&self) -> &'static ModelID {
        &gizmo_model_ids()[*self as usize]
    }
}

impl GizmoConfig {
    /// Returns the visibility of the given gizmo.
    pub fn visibility(&self, gizmo: GizmoType) -> GizmoVisibility {
        match gizmo {
            GizmoType::ReferenceFrameAxes => self.reference_frame_visibility,
            GizmoType::BoundingSphere => self.bounding_sphere_visibility,
        }
    }

    /// Returns a mutable reference to the visibility of the given gizmo.
    pub fn visibility_mut(&mut self, gizmo: GizmoType) -> &mut GizmoVisibility {
        match gizmo {
            GizmoType::ReferenceFrameAxes => &mut self.reference_frame_visibility,
            GizmoType::BoundingSphere => &mut self.bounding_sphere_visibility,
        }
    }
}

impl GizmoManager {
    pub fn new(config: GizmoConfig) -> Self {
        Self {
            config,
            gizmos_with_new_global_visibility: GizmoSet::all(),
        }
    }

    pub fn config(&self) -> &GizmoConfig {
        &self.config
    }

    /// Sets the visibility of the specified gizmo.
    pub fn set_visibility_for_gizmo(&mut self, gizmo: GizmoType, visibility: GizmoVisibility) {
        let current_visibility = self.config.visibility_mut(gizmo);

        if current_visibility.gets_gobally_altered(visibility) {
            self.gizmos_with_new_global_visibility
                .insert(gizmo.as_set());
        }

        *current_visibility = visibility;
    }

    /// Whether the global visibility of any of the specified gizmo types has
    /// changed since the last call to
    /// [`Self::declare_visibilities_synchronized`].
    pub fn global_visibility_changed_for_any_of_gizmos(&self, gizmos: GizmoSet) -> bool {
        self.gizmos_with_new_global_visibility.intersects(gizmos)
    }

    /// Declares to the manager that all changes in global visibility made with
    /// the `set_visibility_for_*` methods have been propagated to the affected
    /// systems.
    pub fn declare_visibilities_synchronized(&mut self) {
        self.gizmos_with_new_global_visibility.clear();
    }
}

impl GizmoVisibility {
    pub fn is_hidden(self) -> bool {
        self == Self::Hidden
    }

    pub fn is_visible_for_all(self) -> bool {
        self == Self::VisibleForAll
    }

    pub fn is_visible_for_selected(self) -> bool {
        self == Self::VisibleForSelected
    }

    fn gets_gobally_altered(self, new: Self) -> bool {
        new != self && new != Self::VisibleForSelected
    }
}

/// The model ID used by each gizmo.
pub fn gizmo_model_ids() -> &'static [ModelID; GizmoType::count()] {
    &GIZMO_MODEL_IDS
}

static GIZMO_MODEL_IDS: LazyLock<[ModelID; GizmoType::count()]> = LazyLock::new(|| {
    GizmoType::all().map(|gizmo| {
        ModelID::for_mesh_and_material(gizmo.mesh_id(), MaterialHandle::not_applicable())
    })
});

/// Initializes the instance buffers used for the model-view transforms of the
/// gizmo instances.
pub fn initialize_buffers_for_gizmo_models(instance_feature_manager: &mut InstanceFeatureManager) {
    for model_id in gizmo_model_ids() {
        instance_feature_manager
            .initialize_instance_buffer(*model_id, &[InstanceModelViewTransform::FEATURE_TYPE_ID]);
    }
}
