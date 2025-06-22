//! Light sources.

pub mod entity;

use crate::{model::InstanceFeatureBufferRangeID, scene::SceneEntityFlags};
use impact_light::{LightFlags, LightID};

/// Converts the light ID into an [`InstanceFeatureBufferRangeID`].
pub fn light_id_to_instance_feature_buffer_range_id(
    light_id: LightID,
) -> InstanceFeatureBufferRangeID {
    // Use a stride of 6 so that the ID can be incremented up to 5 times to
    // create additional ranges associated with the same light
    6 * light_id.to_u32()
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
