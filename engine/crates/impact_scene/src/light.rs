//! Light sources.

use crate::SceneEntityFlags;
use impact_id::EntityID;
use impact_light::LightFlags;
use impact_model::InstanceFeatureBufferRangeID;

/// Converts the given entity ID for a light into an
/// [`InstanceFeatureBufferRangeID`].
pub fn light_entity_id_to_instance_feature_buffer_range_id(
    entity_id: EntityID,
) -> InstanceFeatureBufferRangeID {
    // Use a stride of 6 so that the ID can be incremented up to 5 times to
    // create additional ranges associated with the same light
    6 * entity_id.as_u64()
}

impl From<SceneEntityFlags> for LightFlags {
    fn from(scene_entity_flags: SceneEntityFlags) -> Self {
        let mut light_flags = Self::empty();
        if scene_entity_flags.contains(SceneEntityFlags::IS_DISABLED) {
            light_flags |= Self::IS_DISABLED;
        }
        light_flags
    }
}
