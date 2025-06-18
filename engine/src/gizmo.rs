//! Simple visual elements drawn over the scene to provide technical
//! information.

pub mod components;
pub mod entity;
pub mod mesh;
pub mod model;
pub mod systems;
pub mod tasks;

use crate::{
    mesh::MeshID,
    model::{
        InstanceFeature, InstanceFeatureManager, ModelID, transform::InstanceModelViewTransform,
    },
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
    CenterOfMass = 5,
    LinearVelocity = 6,
    AngularVelocity = 7,
    AngularMomentum = 8,
    Force = 9,
    Torque = 10,
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
        const CENTER_OF_MASS       = 1 << 5;
        const LINEAR_VELOCITY      = 1 << 6;
        const ANGULAR_VELOCITY     = 1 << 7;
        const ANGULAR_MOMENTUM     = 1 << 8;
        const FORCE                = 1 << 9;
        const TORQUE               = 1 << 10;
    }
}

/// Configuration options for gizmos.
///
/// Gizmos are simple visual elements drawn over the scene to provide technical
/// information.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GizmoConfig {
    pub visibilities: GizmoVisibilities,
    pub parameters: GizmoParameters,
}

/// The [`GizmoVisibility`] of each gizmo type.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GizmoVisibilities {
    /// The visibility of the gizmo indicating reference frame axes.
    ///
    /// When visible, a red, green and blue line segment representing the x- y-
    /// and z-axis (respectively) of the local reference frame will be shown
    /// atop applicable entities. The lines are of unit length in the local
    /// reference frame. They meet at the original origin of the entity, so any
    /// origin offset (typically used to shift the origin to the center of mass)
    /// is not accounted for.
    pub reference_frame_axes: GizmoVisibility,
    /// The visibility of the gizmo showing bounding spheres.
    ///
    /// When visible, the bounding spheres of models in the scene graph will
    /// be rendered as semi-transparent cyan spheres.
    pub bounding_sphere: GizmoVisibility,
    /// The visibility of the gizmo showing spheres of influence for
    /// omnidirectional lights.
    ///
    /// When visible, the boundaries at which the light from omnidirectional
    /// light sources is cut off will be rendered as semi-transparent yellow
    /// spheres."
    pub light_sphere: GizmoVisibility,
    /// The visibility of the gizmo indicating the faces of shadow cubemaps for
    /// omnidirectional lights.
    ///
    /// When visible, the far and near planes of the six "view" frusta used when
    /// rendering the shadow cubemap of each omnidirectional light are rendered
    /// in different semi-transparent colors, and the edges of the frusta are
    /// shown as white lines.
    pub shadow_cubemap_face: GizmoVisibility,
    /// The visibility of the gizmo visualizing the partition of the view
    /// frustum for the cascaded shadow maps for unidirectional lights.
    ///
    /// When visible, a semi-transparent colored plane is rendered at each view
    /// distance corresponding to a partition between the cascades used for the
    /// shadow map of a unidirectional light. The result is that geometry
    /// falling within each cascade will be tinted red (closest), yellow, green
    /// or cyan (farthest).
    pub shadow_map_cascade: GizmoVisibility,
    /// The visibility of the gizmo visualizing the centers of mass of rigid
    /// bodies.
    ///
    /// When visible, a semi-transparent blue sphere is rendered at the center
    /// of mass of each rigid body. The volume of the sphere is proportional to
    /// the mass of the body, with the proportionality factor (the sphere's
    /// density) being equal to [`Self::center_of_mass_sphere_density`].
    pub center_of_mass: GizmoVisibility,
    /// The visibility of the gizmo visualizing the linear velocity vector of
    /// moving entities.
    ///
    /// When visible, a red arrow aligned with the linear velocity direction is
    /// rendered from the local origin (typically the center of mass) of moving
    /// entities. The length of the arrow is proportional to the magnitude of
    /// the velocity, with the proportionality factor being equal to
    /// [`Self::linear_velocity_scale`].
    pub linear_velocity: GizmoVisibility,
    /// The visibility of the gizmo visualizing the angular velocity vector of
    /// rotating entities.
    ///
    /// When visible, a yellow arrow aligned with the angular velocity axis is
    /// rendered from the local origin (typically the center of mass) of
    /// rotating entities. The length of the arrow is proportional to the
    /// magnitude of the angular velocity, with the proportionality factor being
    /// equal to [`Self::angular_velocity_scale`].
    pub angular_velocity: GizmoVisibility,
    /// The visibility of the gizmo visualizing the angular momentum vector of
    /// rotating rigid bodies.
    ///
    /// When visible, a magenta arrow aligned with the angular momentum axis
    /// is rendered from the center of mass of rotating rigid bodies. The length
    /// of the arrow is proportional to the magnitude of the angular momentum,
    /// with the proportionality factor being equal to
    /// [`Self::angular_momentum_scale`].
    pub angular_momentum: GizmoVisibility,
    /// The visibility of the gizmo visualizing the total force on the center of
    /// mass of rigid bodies.
    ///
    /// When visible, a green arrow aligned with the force direction is rendered
    /// from the center of mass of rigid bodies. The length of the arrow is
    /// proportional to the magnitude of the force, with the proportionality
    /// factor being equal to [`Self::force_scale`].
    pub force: GizmoVisibility,
    /// The visibility of the gizmo visualizing the total torque around the
    /// center of mass of rigid bodies.
    ///
    /// When visible, a cyan arrow aligned with the torque axis is rendered
    /// from the center of mass of rigid bodies. The length of the arrow is
    /// proportional to the magnitude of the axis, with the proportionality
    /// factor being equal to [`Self::torque_scale`].
    pub torque: GizmoVisibility,
    // pub sphere_collider_visibility: GizmoVisibility,
    // pub plane_collider_visibility: GizmoVisibility,
    // pub voxel_chunks_visibility: GizmoVisibility,
}

/// The configuration parameters associated with each gizmo type.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GizmoParameters {
    /// The density used to calculate the size of the center of mass sphere from
    /// the mass of the body.
    pub center_of_mass_sphere_density: f64,
    /// The scale factor used to calculate the length of the linear velocity
    /// arrow based on the entity's speed.
    pub linear_velocity_scale: f64,
    /// The scale factor used to calculate the length of the angular velocity
    /// arrow based on the entity's angular speed.
    pub angular_velocity_scale: f64,
    /// The scale factor used to calculate the length of the angular momentum
    /// arrow based on the magnitude of the body's angular momentum.
    pub angular_momentum_scale: f64,
    /// The scale factor used to calculate the length of the force arrow based
    /// on the magnitude of the force on the body.
    pub force_scale: f64,
    /// The scale factor used to calculate the length of the torque arrow based
    /// on the magnitude of the torque on the body.
    pub torque_scale: f64,
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

/// Manager controlling the display of gizmos.
///
/// Gizmos are simple visual elements drawn over the scene to provide technical
/// information.
#[derive(Debug)]
pub struct GizmoManager {
    config: GizmoConfig,
    gizmos_with_new_global_visibility: GizmoSet,
}

impl GizmoType {
    /// The number of different gizmo types.
    pub const fn count() -> usize {
        Self::all().len()
    }

    /// The array containing each gizmo type.
    pub const fn all() -> [Self; 11] {
        [
            Self::ReferenceFrameAxes,
            Self::BoundingSphere,
            Self::LightSphere,
            Self::ShadowCubemapFaces,
            Self::ShadowMapCascades,
            Self::CenterOfMass,
            Self::LinearVelocity,
            Self::AngularVelocity,
            Self::AngularMomentum,
            Self::Force,
            Self::Torque,
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
            Self::CenterOfMass => GizmoSet::CENTER_OF_MASS,
            Self::LinearVelocity => GizmoSet::LINEAR_VELOCITY,
            Self::AngularVelocity => GizmoSet::ANGULAR_VELOCITY,
            Self::AngularMomentum => GizmoSet::ANGULAR_MOMENTUM,
            Self::Force => GizmoSet::FORCE,
            Self::Torque => GizmoSet::TORQUE,
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
            Self::CenterOfMass => "Centers of mass",
            Self::LinearVelocity => "Linear velocities",
            Self::AngularVelocity => "Angular velocities",
            Self::AngularMomentum => "Angular momenta",
            Self::Force => "Forces",
            Self::Torque => "Torques",
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
            Self::CenterOfMass => {
                "\
                When enabled, a semi-transparent blue sphere is rendered at the center \
                of mass of each rigid body. The volume of the sphere is proportional to \
                the mass of the body, with the proportionality factor (the sphere's \
                density) set by the `Center of mass sphere density` parameter."
            }
            Self::LinearVelocity => {
                "\
                When enabled, a red arrow aligned with the linear velocity direction is \
                rendered from the local origin (typically the center of mass) of moving \
                entities. The length of the arrow is proportional to the magnitude of \
                the velocity, with the proportionality factor being set by the `Linear \
                velocity scale` parameter."
            }
            Self::AngularVelocity => {
                "\
                When enabled, a yellow arrow aligned with the angular velocity axis is \
                rendered from the local origin (typically the center of mass) of \
                rotating entities. The length of the arrow is proportional to the \
                magnitude of the angular velocity, with the proportionality factor being \
                set by the `Angular velocity scale` parameter."
            }
            Self::AngularMomentum => {
                "\
                When enabled, a magenta arrow aligned with the angular momentum axis \
                is rendered from the center of mass of rotating rigid bodies. The length \
                of the arrow is proportional to the magnitude of the angular momentum, \
                with the proportionality factor being set by the `Angular momentum \
                scale` parameter."
            }
            Self::Force => {
                "\
                When enabled, a green arrow aligned with the force direction is rendered \
                from the center of mass of rigid bodies. The length of the arrow is \
                proportional to the magnitude of the force, with the proportionality \
                factor being set by the `Force scale` parameter."
            }
            Self::Torque => {
                "\
                When enabled, a cyan arrow aligned with the torque axis is rendered \
                from the center of mass of rigid bodies. The length of the arrow is \
                proportional to the magnitude of the axis, with the proportionality \
                factor being set by the `Torque scale` parameter."
            }
        }
    }

    /// Returns the [`GizmoModel`]s defining the geometric and visual attributes
    /// of this gizmo.
    pub fn models(&self) -> &'static [GizmoModel] {
        &gizmo_models()[*self as usize]
    }

    /// Returns the single [`GizmoModel`] defining the geometric and visual
    /// attributes of this gizmo.
    ///
    /// # Panics
    /// If this gizmo does not have exactly one model.
    pub fn only_model(&self) -> &'static GizmoModel {
        assert_eq!(self.models().len(), 1);
        &self.models()[0]
    }

    /// Returns the [`ModelID`] of the single [`GizmoModel`] defining the
    /// geometric and visual attributes of this gizmo.
    ///
    /// # Panics
    /// If this gizmo does not have exactly one model.
    pub fn only_model_id(&self) -> &'static ModelID {
        &self.only_model().model_id
    }

    /// Returns the [`MeshID`] of the single [`GizmoModel`] defining the
    /// geometric and visual attributes of this gizmo.
    ///
    /// # Panics
    /// If this gizmo does not have exactly one model.
    pub fn only_mesh_id(&self) -> MeshID {
        self.only_model_id().mesh_id()
    }
}

impl GizmoVisibilities {
    /// Returns the visibility of the given gizmo.
    pub fn get_for(&self, gizmo: GizmoType) -> GizmoVisibility {
        match gizmo {
            GizmoType::ReferenceFrameAxes => self.reference_frame_axes,
            GizmoType::BoundingSphere => self.bounding_sphere,
            GizmoType::LightSphere => self.light_sphere,
            GizmoType::ShadowCubemapFaces => self.shadow_cubemap_face,
            GizmoType::ShadowMapCascades => self.shadow_map_cascade,
            GizmoType::CenterOfMass => self.center_of_mass,
            GizmoType::LinearVelocity => self.linear_velocity,
            GizmoType::AngularVelocity => self.angular_velocity,
            GizmoType::AngularMomentum => self.angular_momentum,
            GizmoType::Force => self.force,
            GizmoType::Torque => self.torque,
        }
    }

    /// Returns a mutable reference to the visibility of the given gizmo.
    pub fn get_mut_for(&mut self, gizmo: GizmoType) -> &mut GizmoVisibility {
        match gizmo {
            GizmoType::ReferenceFrameAxes => &mut self.reference_frame_axes,
            GizmoType::BoundingSphere => &mut self.bounding_sphere,
            GizmoType::LightSphere => &mut self.light_sphere,
            GizmoType::ShadowCubemapFaces => &mut self.shadow_cubemap_face,
            GizmoType::ShadowMapCascades => &mut self.shadow_map_cascade,
            GizmoType::CenterOfMass => &mut self.center_of_mass,
            GizmoType::LinearVelocity => &mut self.linear_velocity,
            GizmoType::AngularVelocity => &mut self.angular_velocity,
            GizmoType::AngularMomentum => &mut self.angular_momentum,
            GizmoType::Force => &mut self.force,
            GizmoType::Torque => &mut self.torque,
        }
    }
}

impl Default for GizmoParameters {
    fn default() -> Self {
        Self {
            center_of_mass_sphere_density: 1e3,
            linear_velocity_scale: 5.0,
            angular_velocity_scale: 10.0,
            angular_momentum_scale: 0.01,
            force_scale: 10.0,
            torque_scale: 10.0,
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

impl GizmoManager {
    pub fn new(config: GizmoConfig) -> Self {
        Self {
            config,
            gizmos_with_new_global_visibility: GizmoSet::all(),
        }
    }

    pub fn visibilities(&self) -> &GizmoVisibilities {
        &self.config.visibilities
    }

    pub fn parameters(&self) -> &GizmoParameters {
        &self.config.parameters
    }

    pub fn parameters_mut(&mut self) -> &mut GizmoParameters {
        &mut self.config.parameters
    }

    /// Sets the visibility of the specified gizmo.
    pub fn set_visibility_for_gizmo(&mut self, gizmo: GizmoType, visibility: GizmoVisibility) {
        let current_visibility = self.config.visibilities.get_mut_for(gizmo);

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

/// Initializes the instance buffers used for the model-view transforms of the
/// gizmo instances.
pub fn initialize_buffers_for_gizmo_models(instance_feature_manager: &mut InstanceFeatureManager) {
    for model_id in gizmo_models().iter().flatten().map(|model| model.model_id) {
        instance_feature_manager
            .initialize_instance_buffer(model_id, &[InstanceModelViewTransform::FEATURE_TYPE_ID]);
    }
}
