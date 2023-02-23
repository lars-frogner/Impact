//! Management of model instance data for rendering.

use crate::{
    geometry::{
        DynamicInstanceFeatureBuffer, InstanceFeatureBufferRangeID, InstanceFeatureBufferRangeMap,
        InstanceFeatureTypeID,
    },
    rendering::{buffer::RenderBuffer, CoreRenderingSystem, InstanceFeatureShaderInput},
};
use std::{borrow::Cow, ops::Range};

/// Owner and manager of a vertex render buffer for model instance
/// features.
#[derive(Debug)]
pub struct InstanceFeatureRenderBufferManager {
    feature_render_buffer: RenderBuffer,
    vertex_buffer_layout: wgpu::VertexBufferLayout<'static>,
    shader_input: InstanceFeatureShaderInput,
    feature_type_id: InstanceFeatureTypeID,
    n_features: u32,
    range_map: InstanceFeatureBufferRangeMap,
}

impl InstanceFeatureRenderBufferManager {
    /// Creates a new manager with a vertex render buffer initialized
    /// from the given model instance feature buffer.
    pub fn new(
        core_system: &CoreRenderingSystem,
        feature_buffer: &DynamicInstanceFeatureBuffer,
        label: Cow<'static, str>,
    ) -> Self {
        let feature_render_buffer = RenderBuffer::new_vertex_buffer_with_bytes(
            core_system,
            feature_buffer.raw_buffer(),
            feature_buffer.n_valid_bytes(),
            label,
        );

        Self {
            feature_render_buffer,
            vertex_buffer_layout: feature_buffer.vertex_buffer_layout().clone(),
            shader_input: feature_buffer.shader_input().clone(),
            feature_type_id: feature_buffer.feature_type_id(),
            n_features: u32::try_from(feature_buffer.n_valid_features()).unwrap(),
            range_map: feature_buffer.create_range_map(),
        }
    }

    /// Returns the layout of the vertex buffer.
    pub fn vertex_buffer_layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.vertex_buffer_layout
    }

    /// Returns the vertex render buffer of instance features.
    pub fn vertex_render_buffer(&self) -> &RenderBuffer {
        &self.feature_render_buffer
    }

    /// Returns the input required for accessing the features
    /// in a shader.
    pub fn shader_input(&self) -> &InstanceFeatureShaderInput {
        &self.shader_input
    }

    /// Returns the number of features in the render buffer.
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

    /// Writes the valid features in the given model instance feature
    /// buffer into the instance feature render buffer (reallocating the
    /// render buffer if required).
    ///
    /// # Panics
    /// If the given buffer stores features of a different type than the
    /// render buffer.
    pub fn copy_instance_features_to_render_buffer(
        &mut self,
        core_system: &CoreRenderingSystem,
        feature_buffer: &DynamicInstanceFeatureBuffer,
    ) {
        assert_eq!(feature_buffer.feature_type_id(), self.feature_type_id);

        let valid_bytes = feature_buffer.valid_bytes();
        let n_valid_bytes = valid_bytes.len();

        if n_valid_bytes > self.feature_render_buffer.buffer_size() {
            // If the number of valid features exceeds the capacity of the existing buffer,
            // we create a new one that is large enough for all the features (also the ones
            // not currently valid)
            self.feature_render_buffer = RenderBuffer::new_vertex_buffer_with_bytes(
                core_system,
                bytemuck::cast_slice(feature_buffer.raw_buffer()),
                n_valid_bytes,
                self.feature_render_buffer.label().clone(),
            );
        } else {
            self.feature_render_buffer
                .update_valid_bytes(core_system, valid_bytes);
        }

        self.n_features = u32::try_from(feature_buffer.n_valid_features()).unwrap();

        self.range_map = feature_buffer.create_range_map();
    }
}
