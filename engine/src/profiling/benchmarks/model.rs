//! Benchmarks for model related functionality.

use crate::model::{
    DynamicInstanceFeatureBuffer, InstanceFeatureStorage, transform::InstanceModelViewTransform,
};
use impact_profiling::Profiler;

pub fn add_feature_to_dynamic_instance_buffer_from_storage(profiler: impl Profiler) {
    let mut storage = InstanceFeatureStorage::new::<InstanceModelViewTransform>();
    let id = storage.add_feature(&InstanceModelViewTransform::identity());
    profiler.profile(&mut || {
        let mut buffer = DynamicInstanceFeatureBuffer::new_for_storage(&storage);
        for _ in 0..200000 {
            buffer.add_feature_from_storage(&storage, id);
        }
    });
}

pub fn add_feature_to_dynamic_instance_buffer_from_storage_repeatedly(profiler: impl Profiler) {
    let mut storage = InstanceFeatureStorage::new::<InstanceModelViewTransform>();
    let id = storage.add_feature(&InstanceModelViewTransform::identity());
    profiler.profile(&mut || {
        let mut buffer = DynamicInstanceFeatureBuffer::new_for_storage(&storage);
        buffer.add_feature_from_storage_repeatedly(&storage, id, 200000);
    });
}
