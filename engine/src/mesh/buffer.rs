//! Buffering of mesh data for rendering.

use crate::{
    gpu::{
        GraphicsDevice,
        buffer::{GPUBuffer, GPUBufferType},
    },
    mesh::{
        MeshID, N_VERTEX_ATTRIBUTES, TriangleMesh, VERTEX_ATTRIBUTE_FLAGS, VertexAttribute,
        VertexAttributeSet, VertexColor, VertexNormalVector, VertexPosition,
        VertexTangentSpaceQuaternion, VertexTextureCoords, line_segment::LineSegmentMesh,
    },
};
use anyhow::{Result, anyhow};
use bytemuck::Pod;
use impact_containers::CollectionChange;
use std::{borrow::Cow, mem};

/// Represents types that can be written to a vertex buffer.
pub trait VertexBufferable: Pod {
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

/// Owner and manager of GPU buffers for triangle or line segment meshes.
#[derive(Debug)]
pub struct MeshGPUBufferManager {
    available_attributes: VertexAttributeSet,
    vertex_buffers: [Option<GPUBuffer>; N_VERTEX_ATTRIBUTES],
    vertex_buffer_layouts: [Option<wgpu::VertexBufferLayout<'static>>; N_VERTEX_ATTRIBUTES],
    index_buffer: Option<GPUBuffer>,
    index_format: Option<wgpu::IndexFormat>,
    n_vertices: usize,
    n_indices: usize,
    mesh_id: MeshID,
}

const MESH_VERTEX_BINDING_START: u32 = 10;

/// Binding location of a specific type of mesh vertex attribute.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MeshVertexAttributeLocation {
    Position = MESH_VERTEX_BINDING_START,
    NormalVector = (MESH_VERTEX_BINDING_START + 1),
    TextureCoords = (MESH_VERTEX_BINDING_START + 2),
    TangentSpaceQuaternion = (MESH_VERTEX_BINDING_START + 3),
    Color = (MESH_VERTEX_BINDING_START + 4),
}

impl MeshGPUBufferManager {
    /// Creates a new manager with GPU buffers initialized from the given
    /// triangle mesh.
    pub fn for_triangle_mesh(
        graphics_device: &GraphicsDevice,
        mesh_id: MeshID,
        mesh: &TriangleMesh<f32>,
    ) -> Self {
        assert!(
            mesh.has_indices(),
            "Tried to create GPU buffer manager for triangle mesh with no indices"
        );

        let mut available_attributes = VertexAttributeSet::empty();
        let mut vertex_buffers = [None, None, None, None, None];
        let mut vertex_buffer_layouts = [None, None, None, None, None];

        let indices = mesh.indices();
        let (index_format, index_buffer) =
            Self::create_index_buffer(graphics_device, mesh_id, indices);

        let n_vertices = mesh.n_vertices();
        let n_indices = indices.len();

        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            mesh_id,
            mesh.positions(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            mesh_id,
            mesh.normal_vectors(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            mesh_id,
            mesh.texture_coords(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            mesh_id,
            mesh.tangent_space_quaternions(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            mesh_id,
            mesh.colors(),
        );

        Self {
            available_attributes,
            vertex_buffers,
            vertex_buffer_layouts,
            index_buffer: Some(index_buffer),
            index_format: Some(index_format),
            n_vertices,
            n_indices,
            mesh_id,
        }
    }

    /// Creates a new manager with GPU buffers initialized from the given
    /// line segment mesh.
    pub fn for_line_segment_mesh(
        graphics_device: &GraphicsDevice,
        mesh_id: MeshID,
        mesh: &LineSegmentMesh<f32>,
    ) -> Self {
        let mut available_attributes = VertexAttributeSet::empty();
        let mut vertex_buffers = [None, None, None, None, None];
        let mut vertex_buffer_layouts = [None, None, None, None, None];

        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            mesh_id,
            mesh.positions(),
        );
        Self::add_vertex_attribute_if_available(
            graphics_device,
            &mut available_attributes,
            &mut vertex_buffers,
            &mut vertex_buffer_layouts,
            mesh_id,
            mesh.colors(),
        );

        Self {
            available_attributes,
            vertex_buffers,
            vertex_buffer_layouts,
            index_buffer: None,
            index_format: None,
            n_vertices: mesh.n_vertices(),
            n_indices: 0,
            mesh_id,
        }
    }

    /// Ensures that the GPU buffers are in sync with the given triangle mesh.
    pub fn sync_with_triangle_mesh(
        &mut self,
        graphics_device: &GraphicsDevice,
        mesh: &TriangleMesh<f32>,
    ) {
        self.sync_vertex_buffer(graphics_device, mesh.positions(), mesh.position_change());
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
        self.sync_vertex_buffer(graphics_device, mesh.colors(), mesh.color_change());

        self.sync_index_buffer(graphics_device, mesh.indices(), mesh.index_change());

        mesh.reset_change_tracking();
    }

    /// Ensures that the GPU buffers are in sync with the given line segment
    /// mesh.
    pub fn sync_with_line_segment_mesh(
        &mut self,
        graphics_device: &GraphicsDevice,
        mesh: &LineSegmentMesh<f32>,
    ) {
        self.sync_vertex_buffer(graphics_device, mesh.positions(), mesh.position_change());
        self.sync_vertex_buffer(graphics_device, mesh.colors(), mesh.color_change());
        mesh.reset_change_tracking();
    }

    /// Returns an iterator over the layouts of the GPU buffers for the
    /// requested set of vertex attributes.
    ///
    /// # Errors
    /// Returns an error if any of the requested vertex attributes are missing.
    pub fn request_vertex_buffer_layouts(
        &self,
        requested_attributes: VertexAttributeSet,
    ) -> Result<impl Iterator<Item = wgpu::VertexBufferLayout<'static>>> {
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

    /// Returns an iterator over the GPU buffers for the requested set of
    /// vertex attributes.
    ///
    /// # Errors
    /// Returns an error if any of the requested vertex attributes are missing.
    pub fn request_vertex_gpu_buffers(
        &self,
        requested_attributes: VertexAttributeSet,
    ) -> Result<impl Iterator<Item = &GPUBuffer>> {
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

    /// Returns an iterator over the layouts of the GPU buffers for the
    /// requested set of vertex attributes in addition to position, which is
    /// always included.
    ///
    /// # Errors
    /// Returns an error if any of the requested vertex attributes are missing.
    pub fn request_vertex_buffer_layouts_including_position(
        &self,
        requested_attributes: VertexAttributeSet,
    ) -> Result<impl Iterator<Item = wgpu::VertexBufferLayout<'static>>> {
        self.request_vertex_buffer_layouts(requested_attributes | VertexAttributeSet::POSITION)
    }

    /// Returns an iterator over the GPU buffers for the requested set of
    /// vertex attributes in addition to position, which is always included.
    ///
    /// # Errors
    /// Returns an error if any of the requested vertex attributes are missing.
    pub fn request_vertex_gpu_buffers_including_position(
        &self,
        requested_attributes: VertexAttributeSet,
    ) -> Result<impl Iterator<Item = &GPUBuffer>> {
        self.request_vertex_gpu_buffers(requested_attributes | VertexAttributeSet::POSITION)
    }

    /// Returns the GPU buffer of indices, assuming this is for a triangle mesh,
    /// which will always have an index buffer.
    ///
    /// # Panics
    /// If there is no index buffer.
    pub fn triangle_mesh_index_gpu_buffer(&self) -> &GPUBuffer {
        self.index_buffer
            .as_ref()
            .expect("Triangle meshes should have an index buffer")
    }

    /// Returns the format of the indices in the index buffer, assuming this is
    /// for a triangle mesh, which will always have an index buffer.
    ///
    /// # Panics
    /// If there is no index format.
    pub fn triangle_mesh_index_format(&self) -> wgpu::IndexFormat {
        self.index_format
            .expect("Triangle meshes should have an index buffer")
    }

    /// Returns the number of vertices in the vertex buffers.
    pub fn n_vertices(&self) -> usize {
        self.n_vertices
    }

    /// Returns the number of indices in the index buffer.
    pub fn n_indices(&self) -> usize {
        self.n_indices
    }

    fn add_vertex_attribute_if_available<V>(
        graphics_device: &GraphicsDevice,
        available_attributes: &mut VertexAttributeSet,
        vertex_buffers: &mut [Option<GPUBuffer>; N_VERTEX_ATTRIBUTES],
        vertex_buffer_layouts: &mut [Option<wgpu::VertexBufferLayout<'static>>;
                 N_VERTEX_ATTRIBUTES],
        mesh_id: MeshID,
        data: &[V],
    ) where
        V: VertexAttribute + VertexBufferable,
    {
        if !data.is_empty() {
            *available_attributes |= V::FLAG;

            vertex_buffers[V::GLOBAL_INDEX] = Some(GPUBuffer::new_full_vertex_buffer(
                graphics_device,
                data,
                Cow::Owned(format!("{} {}", mesh_id, V::NAME)),
            ));

            vertex_buffer_layouts[V::GLOBAL_INDEX] = Some(V::BUFFER_LAYOUT);
        }
    }

    fn remove_vertex_attribute<V>(&mut self)
    where
        V: VertexAttribute,
    {
        self.available_attributes -= V::FLAG;
        self.vertex_buffers[V::GLOBAL_INDEX] = None;
        self.vertex_buffer_layouts[V::GLOBAL_INDEX] = None;
    }

    fn create_index_buffer<I>(
        graphics_device: &GraphicsDevice,
        mesh_id: MeshID,
        indices: &[I],
    ) -> (wgpu::IndexFormat, GPUBuffer)
    where
        I: IndexBufferable,
    {
        (
            I::INDEX_FORMAT,
            GPUBuffer::new_full_index_buffer(
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
                        *vertex_buffer = GPUBuffer::new_full_vertex_buffer(
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
                    self.mesh_id,
                    data,
                );
            }

            self.n_vertices = data.len();
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
            if indices.is_empty() {
                self.index_buffer = None;
                self.index_format = None;
            } else if let Some(index_buffer) = &mut self.index_buffer {
                let index_bytes = bytemuck::cast_slice(indices);

                if index_bytes.len() > index_buffer.buffer_size() {
                    // If the new number of indices exceeds the size of the existing buffer,
                    // we create a new one that is large enough
                    *index_buffer = GPUBuffer::new_full_index_buffer(
                        graphics_device,
                        indices,
                        index_buffer.label().clone(),
                    );
                } else {
                    index_buffer.update_valid_bytes(graphics_device, index_bytes);
                }
            } else {
                let (index_format, index_buffer) =
                    Self::create_index_buffer(graphics_device, self.mesh_id, indices);
                self.index_buffer = Some(index_buffer);
                self.index_format = Some(index_format);
            }

            self.n_indices = indices.len();
        }
    }
}

impl GPUBuffer {
    /// Creates a vertex GPU buffer initialized with the given vertex data,
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

    /// Creates a vertex GPU buffer initialized with the given vertex
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

    /// Creates a vertex GPU buffer with capacity for the given number of
    /// vertices, with the start of the buffer initialized with the given
    /// vertices.
    ///
    /// # Panics
    /// - If `total_vertex_capacity` is zero.
    /// - If the length of the `initial_vertices` slice exceeds
    ///   `total_vertex_capacity`.
    pub fn new_vertex_buffer_with_spare_capacity<V>(
        graphics_device: &GraphicsDevice,
        total_vertex_capacity: usize,
        initial_vertices: &[V],
        label: Cow<'static, str>,
    ) -> Self
    where
        V: VertexBufferable,
    {
        let buffer_size = mem::size_of::<V>()
            .checked_mul(total_vertex_capacity)
            .unwrap();
        let valid_bytes = bytemuck::cast_slice(initial_vertices);
        Self::new_vertex_buffer_with_bytes_and_spare_capacity(
            graphics_device,
            buffer_size,
            valid_bytes,
            label,
        )
    }

    /// Creates a vertex GPU buffer initialized with the given bytes
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
        assert!(!bytes.is_empty(), "Tried to create empty vertex GPU buffer");
        Self::new(
            graphics_device,
            bytes,
            n_valid_bytes,
            GPUBufferType::Vertex.usage(),
            label,
        )
    }

    /// Creates a vertex GPU buffer with the given size. The given slice of
    /// valid bytes will be written into the beginning of the buffer.
    ///
    /// # Panics
    /// - If `buffer_size` is zero.
    /// - If the size of the `valid_bytes` slice exceeds `buffer_size`.
    pub fn new_vertex_buffer_with_bytes_and_spare_capacity(
        graphics_device: &GraphicsDevice,
        buffer_size: usize,
        valid_bytes: &[u8],
        label: Cow<'static, str>,
    ) -> Self {
        Self::new_with_spare_capacity(
            graphics_device,
            buffer_size,
            valid_bytes,
            GPUBufferType::Vertex.usage(),
            label,
        )
    }

    /// Creates an index GPU buffer initialized with the given index
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
            "Tried to create empty index GPU buffer"
        );

        let n_valid_bytes = mem::size_of::<I>().checked_mul(n_valid_indices).unwrap();

        let bytes = bytemuck::cast_slice(indices);

        Self::new(
            graphics_device,
            bytes,
            n_valid_bytes,
            GPUBufferType::Index.usage(),
            label,
        )
    }

    /// Creates an index GPU buffer initialized with the given index
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

    /// Creates a index GPU buffer with capacity for the given number of
    /// index, with the start of the buffer initialized with the given
    /// index.
    ///
    /// # Panics
    /// - If `total_index_capacity` is zero.
    /// - If the length of the `initial_indices` slice exceeds
    ///   `total_index_capacity`.
    pub fn new_index_buffer_with_spare_capacity<I>(
        graphics_device: &GraphicsDevice,
        total_index_capacity: usize,
        initial_indices: &[I],
        label: Cow<'static, str>,
    ) -> Self
    where
        I: IndexBufferable,
    {
        let buffer_size = mem::size_of::<I>()
            .checked_mul(total_index_capacity)
            .unwrap();

        let valid_bytes = bytemuck::cast_slice(initial_indices);

        Self::new_with_spare_capacity(
            graphics_device,
            buffer_size,
            valid_bytes,
            GPUBufferType::Index.usage(),
            label,
        )
    }
}

impl VertexBufferable for VertexPosition<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            MeshVertexAttributeLocation::Position as u32 => Float32x3,
        ]);
}

impl VertexBufferable for VertexNormalVector<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            MeshVertexAttributeLocation::NormalVector as u32 => Float32x3,
        ]);
}

impl VertexBufferable for VertexTextureCoords<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            MeshVertexAttributeLocation::TextureCoords as u32 => Float32x2,
        ]);
}

impl VertexBufferable for VertexTangentSpaceQuaternion<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            MeshVertexAttributeLocation::TangentSpaceQuaternion as u32 => Float32x4,
        ]);
}

impl VertexBufferable for VertexColor<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            MeshVertexAttributeLocation::Color as u32 => Float32x4,
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
