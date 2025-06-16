//! Simple visual elements drawn over the scene to provide technical
//! information.

pub mod components;
pub mod entity;
pub mod mesh;
pub mod model;
pub mod systems;
pub mod tasks;

use crate::model::{
    InstanceFeature, InstanceFeatureManager, transform::InstanceModelViewTransform,
};
use bitflags::{Flags, bitflags};
use bytemuck::{Pod, Zeroable};
use model::{GizmoModel, gizmo_models};
use serde::{Deserialize, Serialize};

/// A specific gizmo type.
///
/// Gizmos are simple visual elements drawn over the scene to provide technical
/// information.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GizmoType {
    ReferenceFrameAxes = 0,
    BoundingSphere = 1,
    LightSphere = 2,
    ShadowCubemapFaces = 3,
    ShadowMapCascades = 4,
}

bitflags! {
    /// Bitflags encoding a set of different gizmo types.
    ///
    /// Gizmos are simple visual elements drawn over the scene to provide technical
    /// information.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct GizmoSet: u16 {
        const REFERENCE_FRAME_AXES = 1 << 0;
        const BOUNDING_SPHERE      = 1 << 1;
        const LIGHT_SPHERE         = 1 << 2;
        const SHADOW_CUBEMAP_FACES = 1 << 3;
        const SHADOW_MAP_CASCADES  = 1 << 4;
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
    pub reference_frame_axes_visibility: GizmoVisibility,
    /// The visibility of the gizmo showing bounding spheres.
    ///
    /// When visible, the bounding spheres of models in the scene graph will
    /// be rendered as semi-transparent cyan spheres.
    pub bounding_sphere_visibility: GizmoVisibility,
    /// The visibility of the gizmo showing spheres of influence for
    /// omnidirectional lights.
    ///
    /// When visible, the boundaries at which the light from omnidirectional
    /// light sources is cut off will be rendered as semi-transparent yellow
    /// spheres."
    pub light_sphere_visibility: GizmoVisibility,
    /// The visibility of the gizmo indicating the faces of shadow cubemaps for
    /// omnidirectional lights.
    ///
    /// When visible, the far and near planes of the six "view" frusta used when
    /// rendering the shadow cubemap of each omnidirectional light are rendered
    /// in different semi-transparent colors, and the edges of the frusta are
    /// shown as white lines.
    pub shadow_cubemap_face_visibility: GizmoVisibility,
    /// The visibility of the gizmo visualizing the partition of the view
    /// frustum for the cascaded shadow maps for unidirectional lights.
    ///
    /// When visible, a semi-transparent colored plane is rendered at each view
    /// distance corresponding to a partition between the cascades used for the
    /// shadow map of a unidirectional light. The result is that geometry
    /// falling within each cascade will be tinted red (closest), yellow, green
    /// or cyan (farthest).
    pub shadow_map_cascade_visibility: GizmoVisibility,
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

/// Whether a gizmo should be visible through obscuring geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GizmoObscurability {
    /// The gizmo is can be obscured by geometry in front of it.
    Obscurable,
    /// The gizmo is can be seen through geometry in front of it.
    NonObscurable,
}

impl GizmoType {
    /// The number of different gizmo types.
    pub const fn count() -> usize {
        Self::all().len()
    }

    /// The array containing each gizmo type.
    pub const fn all() -> [Self; 5] {
        [
            Self::ReferenceFrameAxes,
            Self::BoundingSphere,
            Self::LightSphere,
            Self::ShadowCubemapFaces,
            Self::ShadowMapCascades,
        ]
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
            Self::LightSphere => GizmoSet::LIGHT_SPHERE,
            Self::ShadowCubemapFaces => GizmoSet::SHADOW_CUBEMAP_FACES,
            Self::ShadowMapCascades => GizmoSet::SHADOW_MAP_CASCADES,
        }
    }

    /// A human-friendly name for the gizmo.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::ReferenceFrameAxes => "Reference frame axes",
            Self::BoundingSphere => "Bounding spheres",
            Self::LightSphere => "Light spheres",
            Self::ShadowCubemapFaces => "Shadow cubemap faces",
            Self::ShadowMapCascades => "Shadow map cascades",
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
                rendered as semi-transparent cyan spheres."
            }
            Self::LightSphere => {
                "\
                When enabled, the boundaries at which the light from omnidirectional \
                light sources is cut off will be rendered as semi-transparent yellow \
                spheres."
            }
            Self::ShadowCubemapFaces => {
                "\
                When enabled, the far and near planes of the six \"view\" frusta used when \
                rendering the shadow cubemap of each omnidirectional light are rendered \
                in different semi-transparent colors, and the edges of the frusta are \
                shown as white lines."
            }
            Self::ShadowMapCascades => {
                "\
                When enabled, a semi-transparent colored plane is rendered at each view \
                distance corresponding to a partition between the cascades used for the \
                shadow map of a unidirectional light. The result is that geometry \
                falling within each cascade will be tinted red (closest), yellow, green \
                or cyan (farthest)."
            }
            _ => "",
        }
    }

    /// Returns the [`GizmoModel`]s defining the geometric and visual attributes
    /// of this gizmo.
    pub fn models(&self) -> &'static [GizmoModel] {
        &gizmo_models()[*self as usize]
    }
}

impl GizmoConfig {
    /// Returns the visibility of the given gizmo.
    pub fn visibility(&self, gizmo: GizmoType) -> GizmoVisibility {
        match gizmo {
            GizmoType::ReferenceFrameAxes => self.reference_frame_axes_visibility,
            GizmoType::BoundingSphere => self.bounding_sphere_visibility,
            GizmoType::LightSphere => self.light_sphere_visibility,
            GizmoType::ShadowCubemapFaces => self.shadow_cubemap_face_visibility,
            GizmoType::ShadowMapCascades => self.shadow_map_cascade_visibility,
        }
    }

    /// Returns a mutable reference to the visibility of the given gizmo.
    pub fn visibility_mut(&mut self, gizmo: GizmoType) -> &mut GizmoVisibility {
        match gizmo {
            GizmoType::ReferenceFrameAxes => &mut self.reference_frame_axes_visibility,
            GizmoType::BoundingSphere => &mut self.bounding_sphere_visibility,
            GizmoType::LightSphere => &mut self.light_sphere_visibility,
            GizmoType::ShadowCubemapFaces => &mut self.shadow_cubemap_face_visibility,
            GizmoType::ShadowMapCascades => &mut self.shadow_map_cascade_visibility,
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

/// Initializes the instance buffers used for the model-view transforms of the
/// gizmo instances.
pub fn initialize_buffers_for_gizmo_models(instance_feature_manager: &mut InstanceFeatureManager) {
    for model_id in gizmo_models().iter().flatten().map(|model| model.model_id) {
        instance_feature_manager
            .initialize_instance_buffer(model_id, &[InstanceModelViewTransform::FEATURE_TYPE_ID]);
    }
}
