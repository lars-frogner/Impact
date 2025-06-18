//! Management of gizmo visibility for entities.

use crate::{
    gizmo::{GizmoManager, GizmoSet, GizmoType, GizmoVisibility, components::GizmosComp},
    light::components::{
        OmnidirectionalLightComp, ShadowableOmnidirectionalLightComp,
        ShadowableUnidirectionalLightComp,
    },
    physics::motion::components::ReferenceFrameComp,
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};

/// Adds the [`GizmosComp`] component to the new entity if it has any of the
/// relevant components. The component is initialized based on which gizmos are
/// currently configured to be globally visible.
pub fn setup_gizmos_for_new_entity(
    gizmo_manager: &GizmoManager,
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
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(gizmo_manager, gizmos) },
        [ReferenceFrameComp]
    );
    setup!(
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(gizmo_manager, gizmos) },
        [OmnidirectionalLightComp],
        ![ReferenceFrameComp]
    );
    setup!(
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(gizmo_manager, gizmos) },
        [ShadowableOmnidirectionalLightComp],
        ![ReferenceFrameComp, OmnidirectionalLightComp]
    );
    setup!(
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp { setup_gizmos(gizmo_manager, gizmos) },
        [ShadowableUnidirectionalLightComp],
        ![
            ReferenceFrameComp,
            OmnidirectionalLightComp,
            ShadowableOmnidirectionalLightComp
        ]
    );
}
