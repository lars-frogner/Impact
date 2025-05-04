//! [`Component`](impact_ecs::component::Component)s related to the detailed
//! drag model.

use crate::{mesh::MeshID, physics::fph};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, SetupComponent};
use roc_codegen::roc;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that should be affected by a drag force and torque computed from
/// aggregating drag on each point on the body.
///
/// The purpose of this component is to aid in constructing a
/// [`DragLoadMapComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(prefix = "Comp", name = "DetailedDrag")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct DetailedDragComp {
    /// The drag coefficient of the body.
    pub drag_coefficient: fph,
}

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// associated [`DragLoadMap`](crate::physics::DragLoadMap) in the
/// [`DragLoadMapRepository`](crate::physics::rigid_body::forces::DragLoadMapRepository).
#[roc(prefix = "Comp", name = "DragLoadMap")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DragLoadMapComp {
    /// The ID of the mesh from which the drag load map was computed.
    pub mesh_id: MeshID,
    /// The drag coefficient of the body.
    pub drag_coefficient: fph,
}

#[roc]
impl DetailedDragComp {
    /// Creates a new component for detailed drag with the given drag
    /// coefficient.
    #[roc(body = "{ drag_coefficient }")]
    pub fn new(drag_coefficient: fph) -> Self {
        Self { drag_coefficient }
    }
}
