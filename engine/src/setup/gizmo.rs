//! Setup of gizmos for new entities.

use crate::lock_order::OrderedRwLock;
use impact_ecs::{setup, world::PrototypeEntities};
use impact_geometry::ReferenceFrame;
use impact_gizmo::{GizmoManager, Gizmos, setup};
use impact_light::{
    OmnidirectionalEmission, ShadowableOmnidirectionalEmission, ShadowableUnidirectionalEmission,
};
use impact_physics::collision::HasCollidable;
use impact_voxel::HasVoxelObject;
use parking_lot::RwLock;

/// Adds the [`Gizmos`] component to the new entities if they have any of
/// the relevant components. The components are initialized based on which
/// gizmos are currently configured to be globally visible.
pub fn setup_gizmos_for_new_entities(
    gizmo_manager: &RwLock<GizmoManager>,
    entities: &mut PrototypeEntities,
) {
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&Gizmos>| -> Gizmos { setup::prepare_gizmos(&gizmo_manager, gizmos) },
        [ReferenceFrame]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&Gizmos>| -> Gizmos { setup::prepare_gizmos(&gizmo_manager, gizmos) },
        [OmnidirectionalEmission],
        ![ReferenceFrame]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&Gizmos>| -> Gizmos { setup::prepare_gizmos(&gizmo_manager, gizmos) },
        [ShadowableOmnidirectionalEmission],
        ![ReferenceFrame, OmnidirectionalEmission]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&Gizmos>| -> Gizmos { setup::prepare_gizmos(&gizmo_manager, gizmos) },
        [ShadowableUnidirectionalEmission],
        ![
            ReferenceFrame,
            OmnidirectionalEmission,
            ShadowableOmnidirectionalEmission
        ]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&Gizmos>| -> Gizmos { setup::prepare_gizmos(&gizmo_manager, gizmos) },
        [HasCollidable],
        ![
            ReferenceFrame,
            OmnidirectionalEmission,
            ShadowableOmnidirectionalEmission,
            ShadowableUnidirectionalEmission
        ]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&Gizmos>| -> Gizmos { setup::prepare_gizmos(&gizmo_manager, gizmos) },
        [HasVoxelObject],
        ![
            ReferenceFrame,
            OmnidirectionalEmission,
            ShadowableOmnidirectionalEmission,
            ShadowableUnidirectionalEmission,
            HasCollidable
        ]
    );
}
