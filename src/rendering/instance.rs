//! Management of model instance data for rendering.

use crate::{
    geometry::{DynamicInstanceFeatureBuffer, InstanceFeatureTypeID, ModelInstanceTransform},
    rendering::{
        buffer::{self, RenderBuffer, RenderBufferType, VertexBufferable},
        fre, CoreRenderingSystem,
    },
};

/// Owner and manager of a vertex render buffer for model instance
/// features.
#[derive(Debug)]
pub struct InstanceFeatureRenderBufferManager {
    feature_render_buffer: RenderBuffer,
    vertex_buffer_layout: wgpu::VertexBufferLayout<'static>,
    feature_type_id: InstanceFeatureTypeID,
    n_features: usize,
    label: String,
}

impl InstanceFeatureRenderBufferManager {
    /// Creates a new manager with a vertex render buffer initialized
    /// from the given model instance feature buffer.
    pub fn new(
        core_system: &CoreRenderingSystem,
        feature_buffer: &DynamicInstanceFeatureBuffer,
        label: String,
    ) -> Self {
        let feature_render_buffer = RenderBuffer::new(
            core_system,
            RenderBufferType::Vertex,
            feature_buffer.raw_buffer(),
            feature_buffer.n_valid_bytes(),
            &label,
        );

        Self {
            feature_render_buffer,
            vertex_buffer_layout: feature_buffer.vertex_buffer_layout().clone(),
            feature_type_id: feature_buffer.feature_type_id(),
            n_features: feature_buffer.n_valid_features(),
            label,
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

    /// Returns the number of features in the render buffer.
    pub fn n_features(&self) -> usize {
        self.n_features
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
            self.feature_render_buffer = RenderBuffer::new(
                core_system,
                RenderBufferType::Vertex,
                bytemuck::cast_slice(feature_buffer.raw_buffer()),
                n_valid_bytes,
                &self.label,
            );
        } else {
            self.feature_render_buffer
                .update_valid_bytes(core_system, valid_bytes);
        }

        self.n_features = feature_buffer.n_valid_features();
    }
}

impl VertexBufferable for ModelInstanceTransform<fre> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        buffer::create_vertex_buffer_layout_for_instance::<Self>(
            &wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4],
        );
}
