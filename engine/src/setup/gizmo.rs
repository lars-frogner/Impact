//! Setup of gizmos for new entities.

use crate::{
    gizmo::{GizmoManager, GizmoSet, GizmoType, GizmoVisibility, components::GizmosComp},
    lock_order::OrderedRwLock,
};
use impact_ecs::{setup, world::PrototypeEntities};
use impact_geometry::ReferenceFrame;
use impact_light::{
    OmnidirectionalEmission, ShadowableOmnidirectionalEmission, ShadowableUnidirectionalEmission,
};
use impact_physics::collision::CollidableID;
use impact_voxel::HasVoxelObject;
use parking_lot::RwLock;

/// Adds the [`GizmosComp`] component to the new entities if they have any of
/// the relevant components. The components are initialized based on which
/// gizmos are currently configured to be globally visible.
pub fn setup_gizmos_for_new_entities(
    gizmo_manager: &RwLock<GizmoManager>,
    entities: &mut PrototypeEntities,
) {
    fn setup_gizmos(gizmo_manager: &GizmoManager, gizmos: Option<&GizmosComp>) -> GizmosComp {
        let mut visible_gizmos =
            gizmos.map_or_else(GizmoSet::empty, |gizmos| gizmos.visible_gizmos);

        for gizmo in GizmoType::all() {
            match gizmo_manager.visibilities().get_for(gizmo) {
                GizmoVisibility::Hidden => {
                    visible_gizmos.remove(gizmo.as_set());
                }
                GizmoVisibility::VisibleForAll => {
                    visible_gizmos.insert(gizmo.as_set());
                }
                GizmoVisibility::VisibleForSelected => {}
            }
        }

        GizmosComp { visible_gizmos }
    }
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [ReferenceFrame]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [OmnidirectionalEmission],
        ![ReferenceFrame]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [ShadowableOmnidirectionalEmission],
        ![ReferenceFrame, OmnidirectionalEmission]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        entities,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
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
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [CollidableID],
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
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [HasVoxelObject],
        ![
            ReferenceFrame,
            OmnidirectionalEmission,
            ShadowableOmnidirectionalEmission,
            ShadowableUnidirectionalEmission,
            CollidableID
        ]
    );
}
