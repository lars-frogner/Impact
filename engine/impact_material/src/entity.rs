//! Management of materials for entities.

pub mod fixed;
pub mod physical;

use crate::components::MaterialComp;
use impact_ecs::world::EntityEntry;
use impact_model::InstanceFeatureManager;
use std::{hash::Hash, sync::RwLock};

/// Checks if the given entity has a [`MaterialComp`], and if so, removes the
/// assocated instance features from the given [`InstanceFeatureManager`].
pub fn cleanup_material_for_removed_entity<MID: Eq + Hash>(
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut bool,
) {
    if let Some(material) = entity.get_component::<MaterialComp>() {
        let material = material.access();

        if let Some(feature_id) = material.material_handle().material_property_feature_id() {
            instance_feature_manager
                .write()
                .unwrap()
                .get_storage_mut_for_feature_type_id(feature_id.feature_type_id())
                .expect("Missing storage for material feature")
                .remove_feature(feature_id);

            *desynchronized = true;
        }
    }
}
