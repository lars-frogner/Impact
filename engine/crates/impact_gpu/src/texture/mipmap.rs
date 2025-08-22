//! Mipmapping.

use crate::device::GraphicsDevice;
use allocator_api2::{alloc::Allocator, vec::Vec as AVec};
use impact_containers::HashMap;
use std::{borrow::Cow, sync::Arc};

/// Helper for generating [`Mipmapper`]s for specific textures.
#[derive(Debug)]
pub struct MipmapperGenerator {
    _shader: wgpu::ShaderModule,
    sampler: wgpu::Sampler,
    pipelines_and_bind_group_layouts:
        HashMap<wgpu::TextureFormat, (Arc<wgpu::RenderPipeline>, wgpu::BindGroupLayout)>,
}

/// Mipmap generator for a specific texture.
#[derive(Debug)]
pub struct Mipmapper<A: Allocator> {
    pipeline: Arc<wgpu::RenderPipeline>,
    // It is important that the wgpu handles are dropped, so when we use an
    // arena allocator, we must make sure that `Mipmapper` is dropped before we
    // reset the arena
    texture_views_for_array_layers_and_mip_levels: AVec<AVec<wgpu::TextureView, A>, A>,
    bind_groups_for_array_layers_and_previous_mip_levels: AVec<AVec<wgpu::BindGroup, A>, A>,
    label: Cow<'static, str>,
}

impl MipmapperGenerator {
    pub const DEFAULT_SUPPORTED_FORMATS: [wgpu::TextureFormat; 4] = [
        wgpu::TextureFormat::R8Unorm,
        wgpu::TextureFormat::R32Float,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ];

    /// Creates a new mipmapper builder that can generate [`Mipmapper`]s for
    /// textures with the given formats.
    pub fn new(
        graphics_device: &GraphicsDevice,
        formats: impl IntoIterator<Item = wgpu::TextureFormat>,
    ) -> Self {
        let device = graphics_device.device();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../../shaders/mipmap.wgsl"
            ))),
            label: Some("Mipmap shader"),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            label: Some("Mipmap sampler"),
            ..Default::default()
        });

        let pipelines_and_bind_group_layouts = formats
            .into_iter()
            .map(|format| {
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("mainVS"),
                        buffers: &[],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("mainFS"),
                        targets: &[Some(format.into())],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                    label: Some("Mipmap pipeline"),
                });

                // Get bind group layout determined from shader code
                let bind_group_layout = pipeline.get_bind_group_layout(0);

                (format, (Arc::new(pipeline), bind_group_layout))
            })
            .collect();

        Self {
            _shader: shader,
            sampler,
            pipelines_and_bind_group_layouts,
        }
    }

    /// Generates a [`Mipmapper`] for the given texture. Returns [`None`] if the
    /// texture only has one mip level.
    ///
    /// # Panics
    /// If the texture's dimension or format is not supported.
    pub fn generate_mipmapper<A>(
        &self,
        alloc: A,
        graphics_device: &GraphicsDevice,
        texture: &wgpu::Texture,
        label: Cow<'static, str>,
    ) -> Option<Mipmapper<A>>
    where
        A: Copy + Allocator,
    {
        if texture.mip_level_count() < 2 {
            return None;
        }

        if texture.dimension() != wgpu::TextureDimension::D2 {
            panic!(
                "Tried to create mipmapper for texture with unsupported dimension: {:?}",
                texture.dimension()
            );
        }

        let (pipeline, bind_group_layout) = self
            .pipelines_and_bind_group_layouts
            .get(&texture.format())
            .unwrap_or_else(|| {
                panic!(
                    "Tried to create mipmapper for texture with unsupported format: {:?}",
                    texture.format()
                )
            });

        let mut texture_views_for_array_layers_and_mip_levels = AVec::new_in(alloc);
        texture_views_for_array_layers_and_mip_levels.extend(
            (0..texture.depth_or_array_layers()).map(|array_layer| {
                let mut texture_views = AVec::new_in(alloc);
                texture_views.extend((0..texture.mip_level_count()).map(|mip_level| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_mip_level: mip_level,
                        mip_level_count: Some(1),
                        base_array_layer: array_layer,
                        array_layer_count: Some(1),
                        ..Default::default()
                    })
                }));
                texture_views
            }),
        );

        let mut bind_groups_for_array_layers_and_previous_mip_levels = AVec::new_in(alloc);
        bind_groups_for_array_layers_and_previous_mip_levels.extend(
            (0..texture.depth_or_array_layers()).map(|array_layer| {
                let mut bind_groups = AVec::new_in(alloc);
                bind_groups.extend((1..texture.mip_level_count() as usize).map(
                    |target_mip_level| {
                        graphics_device
                            .device()
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                layout: bind_group_layout,
                                entries: &[
                                    // Bind the view for the previous mip level
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(
                                            &texture_views_for_array_layers_and_mip_levels
                                                [array_layer as usize][target_mip_level - 1],
                                        ),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                                    },
                                ],
                                label: Some(&format!("{label} mipmap bind group")),
                            })
                    },
                ));
                bind_groups
            }),
        );

        Some(Mipmapper {
            pipeline: Arc::clone(pipeline),
            texture_views_for_array_layers_and_mip_levels,
            bind_groups_for_array_layers_and_previous_mip_levels,
            label,
        })
    }

    /// Populates all mipmap levels of the given texture with the appropriately
    /// mipmapped versions of the full texture.
    pub fn update_texture_mipmaps<A>(
        &self,
        arena: A,
        graphics_device: &GraphicsDevice,
        texture: &wgpu::Texture,
        label: Cow<'static, str>,
    ) where
        A: Copy + Allocator,
    {
        if let Some(mipmapper) = self.generate_mipmapper(arena, graphics_device, texture, label) {
            mipmapper.update_texture_mipmaps(graphics_device);
        }
    }
}

impl<A: Allocator> Mipmapper<A> {
    /// Populates all mipmap levels of this [`Mipmapper`]'s texture with the
    /// appropriately mipmapped versions of the full texture.
    pub fn update_texture_mipmaps(&self, graphics_device: &GraphicsDevice) {
        let mut command_encoder =
            graphics_device
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(&format!("{} mipmap command encoder", &self.label)),
                });
        self.encode_mipmap_passes(&mut command_encoder);
        graphics_device
            .queue()
            .submit(Some(command_encoder.finish()));
    }

    /// Encodes to the given encoder the render passes that populate each mipmap
    /// level of this [`Mipmapper`]'s texture with the appropriately mipmapped
    /// versions of the full texture.
    pub fn encode_mipmap_passes(&self, command_encoder: &mut wgpu::CommandEncoder) {
        for (texture_views_for_mip_levels, bind_groups_for_previous_mip_levels) in self
            .texture_views_for_array_layers_and_mip_levels
            .iter()
            .zip(&self.bind_groups_for_array_layers_and_previous_mip_levels)
        {
            for (texture_view, bind_group) in texture_views_for_mip_levels
                .iter()
                .skip(1)
                .zip(bind_groups_for_previous_mip_levels)
            {
                let mut render_pass =
                    command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        // Render to the view for the current mip level
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        label: Some(&format!("{} mipmap render pass", &self.label)),
                    });

                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);

                render_pass.draw(0..3, 0..1);
            }
        }
    }
}
