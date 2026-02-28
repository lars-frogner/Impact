//! Resources management for scenes.

#[macro_use]
mod macros;

pub mod graph;
pub mod light;
pub mod model;
pub mod setup;
pub mod skybox;

#[cfg(feature = "ecs")]
pub mod systems;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use graph::SceneGraph;
use impact_camera::Camera;
use impact_id::EntityID;
use impact_intersection::IntersectionManager;
use impact_light::LightManager;
use impact_material::MaterialRegistry;
use impact_profiling::{TaskTimer, instrument_task};
use model::ModelInstanceManager;
use roc_integration::roc;

bitflags! {
    /// Bitflags encoding a set of binary states or properties for an entity in
    /// a scene.
    #[cfg_attr(
        feature = "ecs",
        doc = concat!(
            "\n\n\
            This is an ECS [`Component`](impact_ecs::component::Component). \
            If not specified, this component is automatically added to any \
            new entity that has a model, light or rigid body."
        )
    )]
    #[roc(parents="Comp", category="bitflags", flags=[IS_DISABLED=0, CASTS_NO_SHADOWS=1])]
    #[repr(transparent)]
    #[cfg_attr(feature = "ecs", derive(impact_ecs::Component))]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct SceneEntityFlags: u8 {
        /// The entity should not affect the scene in any way.
        const IS_DISABLED      = 1 << 0;
        /// The entity should not participate in shadow maps.
        const CASTS_NO_SHADOWS = 1 << 1;
    }
}

define_component_type! {
    /// A parent entity.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ParentEntity(pub EntityID);
}

define_component_type! {
    /// Marks that an entity can have children, meaning it has a group node in
    /// the scene graph identified by the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct CanBeParent;
}

define_component_type! {
    /// Rules defining entity behavior when it exceeds certain distances from an
    /// anchor entity.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DistanceTriggeredRules {
        /// The ID of the entity the distance is measured from.
        pub anchor_id: EntityID,
        /// The square of the distance beyond which the entity will no longer
        /// cast shadows.
        pub no_shadowing_dist_squared: f64,
        /// The square of the distance at which the entity will be removed.
        pub removal_dist_squared: f64,
    }
}

#[cfg(feature = "ecs")]
impact_ecs::declare_component_flags! {
    SceneEntityFlags => impact_ecs::component::ComponentFlags::INHERITABLE,
    ParentEntity => impact_ecs::component::ComponentFlags::INHERITABLE,
    DistanceTriggeredRules => impact_ecs::component::ComponentFlags::INHERITABLE,
}

impl SceneEntityFlags {
    /// Whether the [`SceneEntityFlags::IS_DISABLED`] flag is set.
    pub fn is_disabled(&self) -> bool {
        self.contains(Self::IS_DISABLED)
    }
}

#[roc]
impl DistanceTriggeredRules {
    /// Creates new rules for disabling shadowing and removal beyond the given
    /// distances from the given anchor entity.
    #[roc(body = r#"{
        anchor_id,
        no_shadowing_dist_squared: Num.to_f64(no_shadowing_distance * no_shadowing_distance),
        removal_dist_squared: Num.to_f64(removal_distance * removal_distance),
    }
    "#)]
    pub fn new(anchor_id: EntityID, no_shadowing_distance: f32, removal_distance: f32) -> Self {
        Self {
            anchor_id,
            no_shadowing_dist_squared: f64::from(no_shadowing_distance.powi(2)),
            removal_dist_squared: f64::from(removal_distance.powi(2)),
        }
    }

    /// Creates a new rule for removal beyond the given distance from the given
    /// anchor entity.
    #[roc(body = r#"{
        anchor_id,
        no_shadowing_dist_squared: Num.infinity_u64,
        removal_dist_squared: Num.to_f64(removal_distance * removal_distance),
    }
    "#)]
    pub fn removal(anchor_id: EntityID, removal_distance: f32) -> Self {
        Self {
            anchor_id,
            no_shadowing_dist_squared: f64::INFINITY,
            removal_dist_squared: f64::from(removal_distance.powi(2)),
        }
    }

    /// Creates a new rule for disabling shadowing beyond the given distance
    /// from the given anchor entity.
    #[roc(body = r#"{
        anchor_id,
        no_shadowing_dist_squared: Num.to_f64(no_shadowing_distance * no_shadowing_distance),
        removal_dist_squared: Num.infinity_u64,
    }
    "#)]
    pub fn no_shadowing(anchor_id: EntityID, no_shadowing_distance: f32) -> Self {
        Self {
            anchor_id,
            no_shadowing_dist_squared: f64::from(no_shadowing_distance.powi(2)),
            removal_dist_squared: f64::INFINITY,
        }
    }

    pub fn no_shadowing_dist_squared(&self) -> f32 {
        self.no_shadowing_dist_squared as f32
    }

    pub fn removal_dist_squared(&self) -> f32 {
        self.removal_dist_squared as f32
    }
}

pub fn buffer_model_instances_and_bound_lights(
    task_timer: &TaskTimer,
    material_registry: &MaterialRegistry,
    light_manager: &mut LightManager,
    model_instance_manager: &mut ModelInstanceManager,
    intersection_manager: &IntersectionManager,
    scene_graph: &SceneGraph,
    camera: &Camera,
    current_frame_number: u32,
    shadow_mapping_enabled: bool,
) {
    let camera_space_aabb_for_visible_models =
        instrument_task!("Buffering model instances for rendering", task_timer, {
            model::buffer_model_instances_for_rendering(
                material_registry,
                model_instance_manager,
                intersection_manager,
                scene_graph,
                camera,
                current_frame_number,
            )
        });

    let Some(camera_space_aabb_for_visible_models) = camera_space_aabb_for_visible_models else {
        // No need to consider shadow mapping if no models are visible
        return;
    };

    instrument_task!(
        "Buffering model instances for omnidirectional shadow mapping",
        task_timer,
        {
            light::bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
                light_manager,
                model_instance_manager,
                intersection_manager,
                scene_graph,
                camera,
                &camera_space_aabb_for_visible_models,
                shadow_mapping_enabled,
            );
        }
    );

    instrument_task!(
        "Buffering model instances for unidirectional shadow mapping",
        task_timer,
        {
            light::bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
                light_manager,
                model_instance_manager,
                intersection_manager,
                scene_graph,
                camera,
                shadow_mapping_enabled,
            );
        }
    );
}
