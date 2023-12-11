//! [`Component`](impact_ecs::component::Component)s related to the detailed
//! drag model.

use crate::{physics::fph, scene::MeshID};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that should be
/// affected by a drag force and torque computed from aggregating drag on each
/// point on the body.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DetailedDragComp {
    /// The drag coefficient of the body.
    pub drag_coefficient: fph,
}

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// associated [`DragLoadMap`] in the [`DragLoadMapRepository`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DragLoadMapComp {
    /// The ID of the mesh from which the drag load map was computed.
    pub mesh_id: MeshID,
}

impl DetailedDragComp {
    /// Creates a new component for detailed drag with the given drag
    /// coefficient.
    pub fn new(drag_coefficient: fph) -> Self {
        Self { drag_coefficient }
    }
}
