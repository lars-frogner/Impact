//! Light sources.

use crate::SceneEntityFlags;
use impact_id::EntityID;
use impact_light::LightFlags;
use impact_math::random::splitmix;
use impact_model::InstanceFeatureBufferRangeID;

/// Converts the given entity ID for a light along with an offset (for cascades
/// or cubemap faces) into an [`InstanceFeatureBufferRangeID`].
pub fn light_entity_id_to_instance_feature_buffer_range_id(
    entity_id: EntityID,
    offset: u64,
) -> InstanceFeatureBufferRangeID {
    splitmix::random_u64_from_two_states(entity_id.as_u64(), offset)
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
