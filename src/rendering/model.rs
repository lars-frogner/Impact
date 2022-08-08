use crate::{
    geometry::{MeshID, ModelID, ModelInstance, ModelInstanceBuffer, ModelInstancePool},
    rendering::{
        buffer::{BufferableInstance, BufferableVertex, InstanceBuffer},
        CoreRenderingSystem, MaterialID, MaterialLibrary,
    },
};
use std::{collections::HashMap, mem};

#[derive(Clone, Debug)]
pub struct ModelSpecification {
    pub material_id: MaterialID,
    pub mesh_id: MeshID,
}

#[derive(Clone, Debug)]
pub struct ModelLibrary {
    material_library: MaterialLibrary,
    model_specifications: HashMap<ModelID, ModelSpecification>,
}

/// Owner and manager of a render buffer for model instances.
#[derive(Debug)]
pub struct ModelInstanceRenderBufferManager {
    instance_render_buffer: InstanceBuffer,
    label: String,
}

impl ModelLibrary {
    pub fn new(material_library: MaterialLibrary) -> Self {
        Self {
            material_library,
            model_specifications: HashMap::new(),
        }
    }

    pub fn material_library(&self) -> &MaterialLibrary {
        &self.material_library
    }

    pub fn get_model(&self, model_id: ModelID) -> Option<&ModelSpecification> {
        self.model_specifications.get(&model_id)
    }

    pub fn add_model(
        &mut self,
        instance_pool: &mut ModelInstancePool<f32>,
        model_id: ModelID,
        model_spec: ModelSpecification,
    ) {
        self.model_specifications.insert(model_id, model_spec);

        instance_pool
            .model_instance_buffers
            .insert(model_id, ModelInstanceBuffer::new());
    }
}

impl ModelInstanceRenderBufferManager {
    /// Creates a new manager with a render buffer initialized
    /// from the given model instance buffer.
    pub fn new(
        core_system: &CoreRenderingSystem,
        model_instance_buffer: &ModelInstanceBuffer<f32>,
        label: String,
    ) -> Self {
        let n_valid_instances = u32::try_from(model_instance_buffer.n_valid_instances()).unwrap();

        let instance_render_buffer = InstanceBuffer::new(
            core_system,
            model_instance_buffer.raw_buffer(),
            n_valid_instances,
            &label,
        );

        Self {
            instance_render_buffer,
            label,
        }
    }

    /// Writes the valid instances in the given model instance
    /// buffer into the instance render buffer (reallocating
    /// the render buffer if required). The model instance
    /// buffer is then cleared.
    pub fn transfer_model_instances_to_render_buffer(
        &mut self,
        core_system: &CoreRenderingSystem,
        model_instance_buffer: &ModelInstanceBuffer<f32>,
    ) {
        let n_valid_instances = u32::try_from(model_instance_buffer.n_valid_instances()).unwrap();

        if n_valid_instances > self.instance_render_buffer.max_instances() {
            // Reallocate render buffer since it is too small
            self.instance_render_buffer = InstanceBuffer::new(
                core_system,
                model_instance_buffer.raw_buffer(),
                n_valid_instances,
                &self.label,
            );
        } else {
            // Write valid instances into the beginning of the render buffer
            self.instance_render_buffer
                .update_valid_instances(core_system, model_instance_buffer.valid_instances());
        }

        // Clear container so that it is ready for reuse
        model_instance_buffer.clear();
    }

    /// Returns the buffer of instances.
    pub fn instance_buffer(&self) -> &InstanceBuffer {
        &self.instance_render_buffer
    }
}

impl BufferableVertex for ModelInstance<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4],
    };
}

impl BufferableInstance for ModelInstance<f32> {}
