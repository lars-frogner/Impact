//! Setup of gizmos for new entities.

use crate::{
    gizmo::{GizmoManager, GizmoSet, GizmoType, GizmoVisibility, components::GizmosComp},
    lock_order::OrderedRwLock,
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::ReferenceFrame;
use impact_light::{
    OmnidirectionalLightID, ShadowableOmnidirectionalLightID, ShadowableUnidirectionalLightID,
};
use impact_physics::collision::CollidableID;
use impact_voxel::VoxelObjectID;
use parking_lot::RwLock;

/// Adds the [`GizmosComp`] component to the new entities if they have any of
/// the relevant components. The components are initialized based on which
/// gizmos are currently configured to be globally visible.
pub fn setup_gizmos_for_new_entities(
    gizmo_manager: &RwLock<GizmoManager>,
    components: &mut ArchetypeComponentStorage,
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
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [ReferenceFrame]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [OmnidirectionalLightID],
        ![ReferenceFrame]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [ShadowableOmnidirectionalLightID],
        ![ReferenceFrame, OmnidirectionalLightID]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [ShadowableUnidirectionalLightID],
        ![
            ReferenceFrame,
            OmnidirectionalLightID,
            ShadowableOmnidirectionalLightID
        ]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [CollidableID],
        ![
            ReferenceFrame,
            OmnidirectionalLightID,
            ShadowableOmnidirectionalLightID,
            ShadowableUnidirectionalLightID
        ]
    );
    setup!(
        {
            let gizmo_manager = gizmo_manager.oread();
        },
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(&gizmo_manager, gizmos) },
        [VoxelObjectID],
        ![
            ReferenceFrame,
            OmnidirectionalLightID,
            ShadowableOmnidirectionalLightID,
            ShadowableUnidirectionalLightID,
            CollidableID
        ]
    );
}
