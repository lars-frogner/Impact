//! Buffering of mesh data for rendering.

use crate::{
    geometry::CollectionChange,
    gpu::{
        rendering::{
            buffer::{RenderBuffer, RenderBufferType},
            fre,
        },
        shader::MeshShaderInput,
        GraphicsDevice,
    },
    mesh::{
        MeshID, TriangleMesh, VertexAttribute, VertexAttributeSet, VertexColor, VertexNormalVector,
        VertexPosition, VertexTangentSpaceQuaternion, VertexTextureCoords, N_VERTEX_ATTRIBUTES,
        VERTEX_ATTRIBUTE_FLAGS,
    },
};
use anyhow::{anyhow, Result};
use bytemuck::Pod;
use std::{borrow::Cow, mem};

/// Represents types that can be written to a vertex buffer.
pub trait VertexBufferable: Pod {
    /// The location index of the vertex attribute binding.
    const BINDING_LOCATION: u32;

    /// The layout of buffers made up of this vertex type.
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static>;
}

/// Represents types that can be written to an index buffer.
pub trait IndexBufferable: Pod {
    /// The data format of the index type.
    const INDEX_FORMAT: wgpu::IndexFormat;
}

impl IndexBufferable for u16 {
    const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}

impl IndexBufferable for u32 {
    const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

/// Owner and manager of render buffers for mesh geometry.
#[derive(Debug)]
pub struct MeshRenderBufferManager {
    available_attributes: VertexAttributeSet,
    vertex_buffers: [Option<RenderBuffer>; N_VERTEX_ATTRIBUTES],
    vertex_buffer_layouts: [Option<wgpu::VertexBufferLayout<'static>>; N_VERTEX_ATTRIBUTES],
    shader_input: MeshShaderInput,
    index_buffer: RenderBuffer,
    index_format: wgpu::IndexFormat,
    n_indices: usize,
    mesh_id: MeshID,
}

const MESH_VERTEX_BINDING_START: u32 = 10;

impl MeshRenderBufferManager {
    /// Creates a new manager with render buffers initialized
    /// from the given mesh.
    pub fn for_mesh(
        graphics_device: &GraphicsDevice,
        mesh_id: MeshID,
        mesh: &TriangleMesh<fre>,
    ) -> Self {
        assert!(
            mesh.has_indices(),
            "Tried to create render buffer manager for mesh with no indices"
        );

        let mut available_attributes = VertexAttributeSet::empty();
        let mut vertex_buffers = [None, None, None, None, None];
        let mut vertex_buffer_layouts = [None, None, None, None, None];
        let mut shader_input = MeshShaderInput {
            locations: [None, None, None, None, None],
        };

        let indices = mesh.indices();
        let n_indices = indices.len();
        let (index_format, index_buffer) =
            Self::create_index_buffer(graphics_device, mesh_id, indices);

        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            &mut shader_input,
            mesh_id,
            mesh.positions(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            &mut shader_input,
            mesh_id,
            mesh.colors(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            &mut shader_input,
            mesh_id,
            mesh.normal_vectors(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            &mut shader_input,
            mesh_id,
            mesh.texture_coords(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            &mut shader_input,
            mesh_id,
            mesh.tangent_space_quaternions(),
        );

        Self {
            available_attributes,
            vertex_buffers,
            vertex_buffer_layouts,
            shader_input,
            index_buffer,
            index_format,
            n_indices,
            mesh_id,
        }
    }

    /// Ensures that the render buffers are in sync with the given mesh.
    pub fn sync_with_mesh(&mut self, graphics_device: &GraphicsDevice, mesh: &TriangleMesh<fre>) {
        self.sync_vertex_buffer(graphics_device, mesh.positions(), mesh.position_change());
        self.sync_vertex_buffer(graphics_device, mesh.colors(), mesh.color_change());
        self.sync_vertex_buffer(
            graphics_device,
            mesh.normal_vectors(),
            mesh.normal_vector_change(),
        );
        self.sync_vertex_buffer(
            graphics_device,
            mesh.texture_coords(),
            mesh.texture_coord_change(),
        );
        self.sync_vertex_buffer(
            graphics_device,
            mesh.tangent_space_quaternions(),
            mesh.tangent_space_quaternion_change(),
        );

        self.sync_index_buffer(graphics_device, mesh.indices(), mesh.index_change());

        mesh.reset_change_tracking();
    }

    /// Returns an iterator over the layouts of the render buffers for the
    /// requested set of vertex attributes.
    ///
    /// # Errors
    /// Returns an error if any of the requested vertex attributes are missing.
    pub fn request_vertex_buffer_layouts(
        &self,
        requested_attributes: VertexAttributeSet,
    ) -> Result<impl Iterator<Item = wgpu::VertexBufferLayout<'static>> + '_> {
        if self.available_attributes.contains(requested_attributes) {
            Ok(VERTEX_ATTRIBUTE_FLAGS
                .iter()
                .zip(self.vertex_buffer_layouts.iter())
                .filter_map(move |(&attribute, layout)| {
                    if requested_attributes.contains(attribute) {
                        Some(layout.as_ref().unwrap().clone())
                    } else {
                        None
                    }
                }))
        } else {
            Err(anyhow!(
                "Mesh `{}` missing requested vertex attributes: {}",
                self.mesh_id,
                requested_attributes.difference(self.available_attributes)
            ))
        }
    }

    /// Returns an iterator over the render buffers for the requested set of
    /// vertex attributes.
    ///
    /// # Errors
    /// Returns an error if any of the requested vertex attributes are missing.
    pub fn request_vertex_render_buffers(
        &self,
        requested_attributes: VertexAttributeSet,
    ) -> Result<impl Iterator<Item = &RenderBuffer>> {
        if self.available_attributes.contains(requested_attributes) {
            Ok(VERTEX_ATTRIBUTE_FLAGS
                .iter()
                .zip(self.vertex_buffers.iter())
                .filter_map(move |(&attribute, buffer)| {
                    if requested_attributes.contains(attribute) {
                        Some(buffer.as_ref().unwrap())
                    } else {
                        None
                    }
                }))
        } else {
            Err(anyhow!(
                "Mesh `{}` missing requested vertex attributes: {}",
                self.mesh_id,
                requested_attributes.difference(self.available_attributes)
            ))
        }
    }

    /// Returns an iterator over the layouts of the render buffers for the
    /// requested set of vertex attributes in addition to position, which is
    /// always included.
    ///
    /// # Errors
    /// Returns an error if any of the requested vertex attributes are missing.
    pub fn request_vertex_buffer_layouts_including_position(
        &self,
        requested_attributes: VertexAttributeSet,
    ) -> Result<impl Iterator<Item = wgpu::VertexBufferLayout<'static>> + '_> {
        self.request_vertex_buffer_layouts(requested_attributes | VertexAttributeSet::POSITION)
    }

    /// Returns an iterator over the render buffers for the requested set of
    /// vertex attributes in addition to position, which is always included.
    ///
    /// # Errors
    /// Returns an error if any of the requested vertex attributes are missing.
    pub fn request_vertex_render_buffers_including_position(
        &self,
        requested_attributes: VertexAttributeSet,
    ) -> Result<impl Iterator<Item = &RenderBuffer>> {
        self.request_vertex_render_buffers(requested_attributes | VertexAttributeSet::POSITION)
    }

    /// The input required for accessing the vertex attributes
    /// in a shader.
    pub fn shader_input(&self) -> &MeshShaderInput {
        &self.shader_input
    }

    /// Returns the render buffer of indices.
    pub fn index_render_buffer(&self) -> &RenderBuffer {
        &self.index_buffer
    }

    /// Returns the format of the indices in the index buffer.
    pub fn index_format(&self) -> wgpu::IndexFormat {
        self.index_format
    }

    /// Returns the number of indices in the index buffer.
    pub fn n_indices(&self) -> usize {
        self.n_indices
    }

    fn add_vertex_attribute_if_available<V>(
        graphics_device: &GraphicsDevice,
        available_attributes: &mut VertexAttributeSet,
        vertex_buffers: &mut [Option<RenderBuffer>; N_VERTEX_ATTRIBUTES],
        vertex_buffer_layouts: &mut [Option<wgpu::VertexBufferLayout<'static>>;
                 N_VERTEX_ATTRIBUTES],
        shader_input: &mut MeshShaderInput,
        mesh_id: MeshID,
        data: &[V],
    ) where
        V: VertexAttribute + VertexBufferable,
    {
        if !data.is_empty() {
            *available_attributes |= V::FLAG;

            vertex_buffers[V::GLOBAL_INDEX] = Some(RenderBuffer::new_full_vertex_buffer(
                graphics_device,
                data,
                Cow::Owned(format!("{} {}", mesh_id, V::NAME)),
            ));

            vertex_buffer_layouts[V::GLOBAL_INDEX] = Some(V::BUFFER_LAYOUT);

            shader_input.locations[V::GLOBAL_INDEX] = Some(V::BINDING_LOCATION);
        }
    }

    fn remove_vertex_attribute<V>(&mut self)
    where
        V: VertexAttribute,
    {
        self.available_attributes -= V::FLAG;
        self.vertex_buffers[V::GLOBAL_INDEX] = None;
        self.vertex_buffer_layouts[V::GLOBAL_INDEX] = None;
        self.shader_input.locations[V::GLOBAL_INDEX] = None;
    }

    fn create_index_buffer<I>(
        graphics_device: &GraphicsDevice,
        mesh_id: MeshID,
        indices: &[I],
    ) -> (wgpu::IndexFormat, RenderBuffer)
    where
        I: IndexBufferable,
    {
        (
            I::INDEX_FORMAT,
            RenderBuffer::new_full_index_buffer(
                graphics_device,
                indices,
                Cow::Owned(format!("{} index", mesh_id)),
            ),
        )
    }

    fn sync_vertex_buffer<V>(
        &mut self,
        graphics_device: &GraphicsDevice,
        data: &[V],
        attribute_change: CollectionChange,
    ) where
        V: VertexAttribute + VertexBufferable,
    {
        if attribute_change != CollectionChange::None {
            let vertex_buffer = self.vertex_buffers[V::GLOBAL_INDEX].as_mut();

            if let Some(vertex_buffer) = vertex_buffer {
                if data.is_empty() {
                    self.remove_vertex_attribute::<V>();
                } else {
                    let vertex_bytes = bytemuck::cast_slice(data);

                    if vertex_bytes.len() > vertex_buffer.buffer_size() {
                        // If the new number of vertices exceeds the size of the existing buffer,
                        // we create a new one that is large enough
                        *vertex_buffer = RenderBuffer::new_full_vertex_buffer(
                            graphics_device,
                            data,
                            vertex_buffer.label().clone(),
                        );
                    } else {
                        vertex_buffer.update_valid_bytes(graphics_device, vertex_bytes);
                    }
                }
            } else {
                Self::add_vertex_attribute_if_available(
                    graphics_device,
                    &mut self.available_attributes,
                    &mut self.vertex_buffers,
                    &mut self.vertex_buffer_layouts,
                    &mut self.shader_input,
                    self.mesh_id,
                    data,
                );
            }
        }
    }

    fn sync_index_buffer<I>(
        &mut self,
        graphics_device: &GraphicsDevice,
        indices: &[I],
        index_change: CollectionChange,
    ) where
        I: IndexBufferable,
    {
        if index_change != CollectionChange::None {
            let index_bytes = bytemuck::cast_slice(indices);

            if index_bytes.len() > self.index_buffer.buffer_size() {
                // If the new number of indices exceeds the size of the existing buffer,
                // we create a new one that is large enough
                self.index_buffer = RenderBuffer::new_full_index_buffer(
                    graphics_device,
                    indices,
                    self.index_buffer.label().clone(),
                );
            } else {
                self.index_buffer
                    .update_valid_bytes(graphics_device, index_bytes);
            }

            self.n_indices = indices.len();
        }
    }
}

impl RenderBuffer {
    /// Creates a vertex render buffer initialized with the given vertex data,
    /// with the first `n_valid_vertices` considered valid data.
    ///
    /// # Panics
    /// - If `vertices` is empty.
    /// - If `n_valid_vertices` exceeds the number of items in the `vertices`
    ///   slice.
    pub fn new_vertex_buffer<V>(
        graphics_device: &GraphicsDevice,
        vertices: &[V],
        n_valid_vertices: usize,
        label: Cow<'static, str>,
    ) -> Self
    where
        V: VertexBufferable,
    {
        let n_valid_bytes = mem::size_of::<V>().checked_mul(n_valid_vertices).unwrap();

        let bytes = bytemuck::cast_slice(vertices);

        Self::new_vertex_buffer_with_bytes(graphics_device, bytes, n_valid_bytes, label)
    }

    /// Creates a vertex render buffer initialized with the given vertex
    /// data.
    ///
    /// # Panics
    /// If `vertices` is empty.
    pub fn new_full_vertex_buffer<V>(
        graphics_device: &GraphicsDevice,
        vertices: &[V],
        label: Cow<'static, str>,
    ) -> Self
    where
        V: VertexBufferable,
    {
        Self::new_vertex_buffer(graphics_device, vertices, vertices.len(), label)
    }

    /// Creates a vertex render buffer initialized with the given bytes
    /// representing vertex data, with the first `n_valid_bytes` considered
    /// valid data.
    ///
    /// # Panics
    /// - If `bytes` is empty.
    /// - If `n_valid_bytes` exceeds the size of the `bytes` slice.
    pub fn new_vertex_buffer_with_bytes(
        graphics_device: &GraphicsDevice,
        bytes: &[u8],
        n_valid_bytes: usize,
        label: Cow<'static, str>,
    ) -> Self {
        assert!(
            !bytes.is_empty(),
            "Tried to create empty vertex render buffer"
        );
        Self::new(
            graphics_device,
            RenderBufferType::Vertex,
            bytes,
            n_valid_bytes,
            label,
        )
    }

    /// Creates an index render buffer initialized with the given index
    /// data, with the first `n_valid_indices` considered valid data.
    ///
    /// # Panics
    /// - If `indices` is empty.
    /// - If `n_valid_indices` exceeds the number of items in the `indices`
    ///   slice.
    pub fn new_index_buffer<I>(
        graphics_device: &GraphicsDevice,
        indices: &[I],
        n_valid_indices: usize,
        label: Cow<'static, str>,
    ) -> Self
    where
        I: IndexBufferable,
    {
        assert!(
            !indices.is_empty(),
            "Tried to create empty index render buffer"
        );

        let n_valid_bytes = mem::size_of::<I>().checked_mul(n_valid_indices).unwrap();

        let bytes = bytemuck::cast_slice(indices);

        Self::new(
            graphics_device,
            RenderBufferType::Index,
            bytes,
            n_valid_bytes,
            label,
        )
    }

    /// Creates an index render buffer initialized with the given index
    /// data.
    ///
    /// # Panics
    /// If `indices` is empty.
    pub fn new_full_index_buffer<I>(
        graphics_device: &GraphicsDevice,
        indices: &[I],
        label: Cow<'static, str>,
    ) -> Self
    where
        I: IndexBufferable,
    {
        Self::new_index_buffer(graphics_device, indices, indices.len(), label)
    }
}

impl VertexBufferable for VertexPosition<fre> {
    const BINDING_LOCATION: u32 = MESH_VERTEX_BINDING_START;

    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            Self::BINDING_LOCATION => Float32x3,
        ]);
}

impl VertexBufferable for VertexColor<fre> {
    const BINDING_LOCATION: u32 = MESH_VERTEX_BINDING_START + 1;

    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            Self::BINDING_LOCATION => Float32x3,
        ]);
}

impl VertexBufferable for VertexNormalVector<fre> {
    const BINDING_LOCATION: u32 = MESH_VERTEX_BINDING_START + 2;

    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            Self::BINDING_LOCATION => Float32x3,
        ]);
}

impl VertexBufferable for VertexTextureCoords<fre> {
    const BINDING_LOCATION: u32 = MESH_VERTEX_BINDING_START + 3;

    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            Self::BINDING_LOCATION => Float32x2,
        ]);
}

impl VertexBufferable for VertexTangentSpaceQuaternion<fre> {
    const BINDING_LOCATION: u32 = MESH_VERTEX_BINDING_START + 4;

    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            Self::BINDING_LOCATION => Float32x4,
        ]);
}

/// Creates a [`VertexBufferLayout`](wgpu::VertexBufferLayout) for
/// vertex data of type `T`, with data layout defined by the given
/// vertex attributes.
pub const fn create_vertex_buffer_layout_for_vertex<T>(
    attributes: &'static [wgpu::VertexAttribute],
) -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<T>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    }
}

/// Creates a [`VertexBufferLayout`](wgpu::VertexBufferLayout) for
/// instance data of type `T`, with data layout defined by the given
/// instance attributes.
pub const fn create_vertex_buffer_layout_for_instance<T>(
    attributes: &'static [wgpu::VertexAttribute],
) -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<T>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes,
    }
}
