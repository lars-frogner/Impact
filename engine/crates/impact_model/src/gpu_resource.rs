//! GPU resources for model instance data.

use crate::{
    DynamicInstanceFeatureBuffer, InstanceFeature, InstanceFeatureBufferRangeID,
    InstanceFeatureBufferRangeMap, InstanceFeatureTypeID,
};
use impact_containers::HashMap;
use impact_gpu::{
    buffer::{GPUBuffer, GPUBufferType},
    device::GraphicsDevice,
    wgpu,
};
use std::{borrow::Cow, hash::Hash, ops::Range};

/// Map of GPU buffers with per-instance data for each model.
#[derive(Debug)]
pub struct ModelInstanceGPUBufferMap<MID> {
    /// For each model ID, holds a set of GPU buffers with instance data, one
    /// buffer for each type of instance data (feature).
    pub(crate) buffers: HashMap<MID, Vec<InstanceFeatureGPUBuffer>>,
}

/// A vertex GPU buffer for model instance features.
#[derive(Debug)]
pub struct InstanceFeatureGPUBuffer {
    feature_gpu_buffer: GPUBuffer,
    vertex_buffer_layout: wgpu::VertexBufferLayout<'static>,
    feature_type_id: InstanceFeatureTypeID,
    n_features: u32,
    range_map: InstanceFeatureBufferRangeMap,
}

impl<MID: Eq + Hash> ModelInstanceGPUBufferMap<MID> {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::default(),
        }
    }

    /// Whether GPU buffers exist for the given model.
    pub fn contains(&self, model_id: &MID) -> bool {
        self.buffers.contains_key(model_id)
    }

    /// Returns an iterator over all model IDs in the map.
    pub fn model_ids(&self) -> impl Iterator<Item = &MID> {
        self.buffers.keys()
    }

    /// Returns an iterator over all model IDs in the map and their associated
    /// buffers.
    pub fn iter(&self) -> impl Iterator<Item = (&MID, &[InstanceFeatureGPUBuffer])> {
        self.buffers
            .iter()
            .map(|(model_id, buffer)| (model_id, buffer.as_slice()))
    }

    /// Returns the GPU buffers for the given model identifier if the model
    /// exists, otherwise returns [`None`].
    pub fn get_model_buffers(&self, model_id: &MID) -> Option<&[InstanceFeatureGPUBuffer]> {
        self.buffers
            .get(model_id)
            .map(|managers| managers.as_slice())
    }

    /// Returns the GPU buffer for features of type `Fe` for the given model if
    /// it exists, otherwise returns [`None`].
    pub fn get_model_buffer_for_feature_feature_type<Fe: InstanceFeature>(
        &self,
        model_id: &MID,
    ) -> Option<&InstanceFeatureGPUBuffer> {
        self.get_model_buffers(model_id).and_then(|buffers| {
            buffers
                .iter()
                .find(|buffer| buffer.is_for_feature_type::<Fe>())
        })
    }
}

impl<MID: Eq + Hash> Default for ModelInstanceGPUBufferMap<MID> {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceFeatureGPUBuffer {
    /// Creates a new manager with a vertex GPU buffer initialized from the
    /// given model instance feature buffer. Returns [`None`] if the buffer's
    /// instance feature type does not require GPU buffers.
    pub fn new(
        graphics_device: &GraphicsDevice,
        feature_buffer: &DynamicInstanceFeatureBuffer,
        label: Cow<'static, str>,
    ) -> Option<Self> {
        let vertex_buffer_layout = feature_buffer.vertex_buffer_layout()?;

        let raw_buffer = feature_buffer.raw_buffer();

        assert!(
            !raw_buffer.is_empty(),
            "Tried to create GPU buffer manager for empty instance feature buffer"
        );

        let feature_gpu_buffer = GPUBuffer::new(
            graphics_device,
            raw_buffer,
            feature_buffer.n_valid_bytes(),
            GPUBufferType::Vertex.usage(),
            label,
        );

        Some(Self {
            feature_gpu_buffer,
            vertex_buffer_layout,
            feature_type_id: feature_buffer.feature_type_id(),
            n_features: u32::try_from(feature_buffer.n_valid_features()).unwrap(),
            range_map: feature_buffer.create_range_map(),
        })
    }

    /// Whether this buffer is for instance features of type `Fe`.
    pub fn is_for_feature_type<Fe: InstanceFeature>(&self) -> bool {
        self.is_for_feature_type_with_id(Fe::FEATURE_TYPE_ID)
    }

    /// Whether this buffer is for instance features of the type with the given ID.
    pub fn is_for_feature_type_with_id(&self, feature_type_id: InstanceFeatureTypeID) -> bool {
        feature_type_id == self.feature_type_id
    }

    /// Returns the layout of the vertex buffer.
    pub fn vertex_buffer_layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.vertex_buffer_layout
    }

    /// Returns the vertex GPU buffer of instance features.
    pub fn vertex_gpu_buffer(&self) -> &GPUBuffer {
        &self.feature_gpu_buffer
    }

    /// Returns the number of features in the GPU buffer.
    pub fn n_features(&self) -> u32 {
        self.n_features
    }

    /// Returns the range of feature indices with the given
    /// [`InstanceFeatureBufferRangeID`]. See
    /// [`DynamicInstanceFeatureBuffer::valid_feature_range`] for more
    /// information.
    ///
    /// # Panics
    /// If no range with the given ID exists.
    pub fn feature_range(&self, range_id: InstanceFeatureBufferRangeID) -> Range<u32> {
        self.range_map.get_range(range_id, self.n_features)
    }

    /// Returns the range of feature indices encompassing all features added
    /// before defining any explicit ranges. See
    /// [`DynamicInstanceFeatureBuffer::initial_valid_feature_range`] for more
    /// information.
    pub fn initial_feature_range(&self) -> Range<u32> {
        self.feature_range(InstanceFeatureBufferRangeMap::INITIAL_RANGE_ID)
    }

    /// Whether the buffer has features in the initial feature range.
    pub fn has_features_in_initial_range(&self) -> bool {
        !self.initial_feature_range().is_empty()
    }

    /// Whether the buffer has features after the initial feature range.
    pub fn has_features_after_initial_range(&self) -> bool {
        self.n_features > self.initial_feature_range().end
    }

    /// Writes the valid features in the given model instance feature
    /// buffer into the instance feature GPU buffer (reallocating the
    /// GPU buffer if required).
    ///
    /// # Panics
    /// If the given buffer stores features of a different type than the
    /// GPU buffer.
    pub fn copy_instance_features_to_gpu_buffer(
        &mut self,
        graphics_device: &GraphicsDevice,
        feature_buffer: &DynamicInstanceFeatureBuffer,
    ) {
        assert_eq!(feature_buffer.feature_type_id(), self.feature_type_id);

        let valid_bytes = feature_buffer.valid_bytes();
        let n_valid_bytes = valid_bytes.len();

        if n_valid_bytes > self.feature_gpu_buffer.buffer_size() {
            // If the number of valid features exceeds the capacity of the existing buffer,
            // we create a new one that is large enough for all the features (also the ones
            // not currently valid)
            self.feature_gpu_buffer = GPUBuffer::new(
                graphics_device,
                bytemuck::cast_slice(feature_buffer.raw_buffer()),
                n_valid_bytes,
                GPUBufferType::Vertex.usage(),
                self.feature_gpu_buffer.label().clone(),
            );
        } else {
            self.feature_gpu_buffer
                .update_valid_bytes(graphics_device, valid_bytes);
        }

        self.n_features = u32::try_from(feature_buffer.n_valid_features()).unwrap();

        feature_buffer.update_range_map(&mut self.range_map);
    }
}
