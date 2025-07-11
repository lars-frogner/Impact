//! Light sources.

use crate::SceneEntityFlags;
use impact_light::LightFlags;
use impact_model::InstanceFeatureBufferRangeID;

/// Converts the light ID into an [`InstanceFeatureBufferRangeID`].
pub fn light_id_to_instance_feature_buffer_range_id<ID>(
    light_id: ID,
) -> InstanceFeatureBufferRangeID
where
    ID: Into<u32>,
{
    // Use a stride of 6 so that the ID can be incremented up to 5 times to
    // create additional ranges associated with the same light
    6 * light_id.into()
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
