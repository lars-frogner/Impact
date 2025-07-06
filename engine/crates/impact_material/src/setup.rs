//! Material setup.

pub mod fixed;
pub mod physical;

use crate::MaterialHandle;
use impact_model::InstanceFeatureManager;
use std::{hash::Hash, sync::RwLock};

/// Removes the instance features assocated with the given [`MaterialHandle`]
/// from the [`InstanceFeatureManager`].
pub fn cleanup_material<MID: Eq + Hash>(
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    material_handle: &MaterialHandle,
    desynchronized: &mut bool,
) {
    if let Some(feature_id) = material_handle.material_property_feature_id() {
        instance_feature_manager
            .write()
            .unwrap()
            .get_storage_mut_for_feature_type_id(feature_id.feature_type_id())
            .expect("Missing storage for material feature")
            .remove_feature(feature_id);

        *desynchronized = true;
    }
}

/// Checks if the given entity has a [`MaterialComp`], and if so, removes the
/// assocated instance features from the given [`InstanceFeatureManager`].
#[cfg(feature = "ecs")]
pub fn cleanup_material_for_removed_entity<MID: Eq + Hash>(
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    entity: &impact_ecs::world::EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(material_handle) = entity.get_component::<MaterialHandle>() {
        cleanup_material(
            instance_feature_manager,
            material_handle.access(),
            desynchronized,
        );
    }
}
