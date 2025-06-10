//! Simple visual elements drawn over the scene to provide technical
//! information.

pub mod components;
pub mod entity;
pub mod systems;
pub mod tasks;

use crate::{
    material::MaterialHandle,
    mesh,
    model::{
        InstanceFeature, InstanceFeatureManager, ModelID, transform::InstanceModelViewTransform,
    },
};
use bitflags::{Flags, bitflags};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Manager controlling the display of gizmos.
///
/// Gizmos are simple visual elements drawn over the scene to provide technical
/// information.
#[derive(Debug)]
pub struct GizmoManager {
    config: GizmoConfig,
    gizmos_with_new_global_visibility: GizmoSet,
}

/// Configuration parameters for gizmos.
///
/// Gizmos are simple visual elements drawn over the scene to provide technical
/// information.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GizmoConfig {
    /// The visibility of the gizmo indicating reference frame axes.
    ///
    /// When visible, a red, green and blue line segment representing the x- y-
    /// and z-axis (respectively) of the local reference frame will be shown
    /// atop applicable entities. The lines are of unit length in the local
    /// reference frame.
    pub reference_frame_visibility: GizmoVisibility,
}

/// The scope of visibility for a gizmo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GizmoVisibility {
    /// The gizmo is hidden for all entities.
    Hidden,
    /// The gizmo is visible for all applicable entities.
    VisibleForAll,
    /// The gizmo is visible for a selection of applicable entities.
    VisibleForSelected,
}

bitflags! {
    /// Bitflags encoding a set of different gizmo types.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct GizmoSet: u8 {
        /// Line segments representing the axes of a local reference frame.
        const REFERENCE_FRAME_AXES = 1 << 0;
    }
}

static REFERENCE_FRAME_AXES_MODEL_ID: LazyLock<ModelID> = LazyLock::new(|| {
    ModelID::for_mesh_and_material(
        mesh::reference_frame_axes_mesh_id(),
        MaterialHandle::not_applicable(),
    )
});

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

    /// Sets the visibility of the gizmo indicating reference frame axes.
    pub fn set_visibility_for_reference_frame_gizmo(&mut self, visibility: GizmoVisibility) {
        if self
            .config
            .reference_frame_visibility
            .gets_gobally_altered(visibility)
        {
            self.gizmos_with_new_global_visibility
                .insert(GizmoSet::REFERENCE_FRAME_AXES);
        }
        self.config.reference_frame_visibility = visibility;
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

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            reference_frame_visibility: GizmoVisibility::Hidden,
        }
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

/// The model ID used by each gizmo. It holds the ID of the line segment mesh
/// used for the gizmo. It is also the key under which the model-view transforms
/// to apply to the mesh during rendering are buffered in the instance feature
/// manager.
pub fn gizmo_model_ids() -> [&'static ModelID; 1] {
    [reference_frame_axes_model_id()]
}

pub fn reference_frame_axes_model_id() -> &'static ModelID {
    &REFERENCE_FRAME_AXES_MODEL_ID
}

/// Initializes the instance buffers used for the model-view transforms of the
/// gizmo instances.
pub fn initialize_buffers_for_gizmo_models(instance_feature_manager: &mut InstanceFeatureManager) {
    instance_feature_manager.initialize_instance_buffer(
        *reference_frame_axes_model_id(),
        &[InstanceModelViewTransform::FEATURE_TYPE_ID],
    );
}
