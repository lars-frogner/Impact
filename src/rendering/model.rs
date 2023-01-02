//! Management of model instance data for rendering.

use crate::{
    geometry::{ModelInstanceTransform, ModelInstanceTransformBuffer},
    rendering::{
        buffer::{BufferableInstanceFeature, BufferableVertex, InstanceFeatureRenderBuffer},
        fre, CoreRenderingSystem,
    },
};
use std::mem;

/// Owner and manager of a render buffer for model instance transforms.
#[derive(Debug)]
pub struct ModelInstanceTransformRenderBufferManager {
    transform_render_buffer: InstanceFeatureRenderBuffer,
    label: String,
}

impl ModelInstanceTransformRenderBufferManager {
    /// Creates a new manager with a render buffer initialized
    /// from the given model instance transform buffer.
    pub fn new(
        core_system: &CoreRenderingSystem,
        transform_buffer: &ModelInstanceTransformBuffer<fre>,
        label: String,
    ) -> Self {
        let n_valid_transforms = u32::try_from(transform_buffer.n_valid_transforms()).unwrap();

        let transform_render_buffer = InstanceFeatureRenderBuffer::new(
            core_system,
            transform_buffer.raw_buffer(),
            n_valid_transforms,
            &label,
        );

        Self {
            transform_render_buffer,
            label,
        }
    }

    /// Writes the valid transforms in the given model instance transform
    /// buffer into the instance transform render buffer (reallocating the
    /// render buffer if required). The model instance transform buffer is
    /// then cleared.
    pub fn transfer_instance_transforms_to_render_buffer(
        &mut self,
        core_system: &CoreRenderingSystem,
        instance_transform_buffer: &ModelInstanceTransformBuffer<fre>,
    ) {
        let n_valid_transforms =
            u32::try_from(instance_transform_buffer.n_valid_transforms()).unwrap();

        if n_valid_transforms > self.transform_render_buffer.max_instance_features() {
            // Reallocate render buffer since it is too small
            self.transform_render_buffer = InstanceFeatureRenderBuffer::new(
                core_system,
                instance_transform_buffer.raw_buffer(),
                n_valid_transforms,
                &self.label,
            );
        } else {
            // Write valid transforms into the beginning of the render buffer
            self.transform_render_buffer.update_valid_instance_features(
                core_system,
                instance_transform_buffer.valid_transforms(),
            );
        }

        // Clear container so that it is ready for reuse
        instance_transform_buffer.clear();
    }

    /// Returns the render buffer of instance transforms.
    pub fn transform_render_buffer(&self) -> &InstanceFeatureRenderBuffer {
        &self.transform_render_buffer
    }
}

impl BufferableVertex for ModelInstanceTransform<fre> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4],
    };
}

impl BufferableInstanceFeature for ModelInstanceTransform<fre> {}
