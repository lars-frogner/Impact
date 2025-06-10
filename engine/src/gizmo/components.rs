//! [`Component`](impact_ecs::component::Component)s related to gizmos.

use crate::gizmo::GizmoSet;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities for which one
/// or more gizmos may be visible.
///
/// This component is automatically added to any new entity that has a component
/// the relevant for a gizmo.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct GizmosComp {
    /// The gizmos currently visible for the entity.
    pub visible_gizmos: GizmoSet,
}
