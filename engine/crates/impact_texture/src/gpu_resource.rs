//! GPU resources for textures and samplers.

use crate::{
    SamplerCreateInfo, SamplerID, TextureCreateInfo, TextureID,
    lookup_table::{LookupTableBindingInfo, LookupTableID, LookupTableTextureCreateInfo},
};
use anyhow::{Context, Result, anyhow};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    texture::{Sampler, Texture, mipmap::MipmapperGenerator},
    wgpu,
};
use impact_resource::gpu::{GPUResource, GPUResourceMap};

/// Textures on the GPU.
pub type TextureMap = GPUResourceMap<TextureCreateInfo, SamplingTexture>;

/// Texture samplers on the GPU.
pub type SamplerMap = GPUResourceMap<SamplerCreateInfo, Sampler>;

/// Bind groups for lookup table textures and samplers.
pub type LookupTableBindGroupMap = GPUResourceMap<LookupTableBindingInfo, LookupTableBindGroup>;

/// A texture optionally accompanied by the ID of the sampler that should be
/// used to sample it.
#[derive(Debug)]
pub struct SamplingTexture {
    pub texture: Texture,
    pub sampler_id: Option<SamplerID>,
}

/// Properties needed to create a bind group layout for a texture.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureBindGroupLayoutEntryProps {
    pub texture_format: wgpu::TextureFormat,
    pub view_dimension: wgpu::TextureViewDimension,
}

/// Properties needed to create a bind group layout for a sampler.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SamplerBindGroupLayoutEntryProps {
    pub sampler_binding_type: wgpu::SamplerBindingType,
}

/// Bind group for a lookup table's texture and sampler.
#[derive(Debug)]
pub struct LookupTableBindGroup {
    pub bind_group: wgpu::BindGroup,
}

impl TextureCreateInfo {
    /// Creates a layout entry for a bind group including this texture.
    pub fn create_bind_group_layout_entry(
        &self,
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        let TextureBindGroupLayoutEntryProps {
            texture_format,
            view_dimension,
        } = self.bind_group_layout_entry_props();

        impact_gpu::texture::create_texture_bind_group_layout_entry(
            binding,
            visibility,
            texture_format,
            view_dimension,
        )
    }

    /// Returns the properties needed to create a bind group layout for this
    /// texture.
    pub fn bind_group_layout_entry_props(&self) -> TextureBindGroupLayoutEntryProps {
        match self {
            Self::Image(image_texture_info) => {
                let texture_format = image_texture_info.texel_description().texture_format();

                let view_dimension = if image_texture_info.is_cubemap() {
                    wgpu::TextureViewDimension::Cube
                } else {
                    Texture::determine_texture_view_dimension(
                        image_texture_info.height(),
                        image_texture_info.depth_or_array_layers(),
                    )
                };

                TextureBindGroupLayoutEntryProps {
                    texture_format,
                    view_dimension,
                }
            }
            Self::LookupTable(LookupTableTextureCreateInfo { metadata, .. }) => {
                let texture_format = metadata.value_type.texel_description().texture_format();

                let view_dimension = Texture::determine_texture_view_dimension(
                    metadata.height,
                    metadata.depth_or_array_layers,
                );

                TextureBindGroupLayoutEntryProps {
                    texture_format,
                    view_dimension,
                }
            }
        }
    }
}

impl SamplerCreateInfo {
    /// Creates a layout entry for a bind group including this sampler.
    pub fn create_bind_group_layout_entry(
        &self,
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        let SamplerBindGroupLayoutEntryProps {
            sampler_binding_type,
        } = self.bind_group_layout_entry_props();

        impact_gpu::texture::create_sampler_bind_group_layout_entry(
            binding,
            visibility,
            sampler_binding_type,
        )
    }

    /// Returns the properties needed to create a bind group layout for this
    /// sampler.
    pub fn bind_group_layout_entry_props(&self) -> SamplerBindGroupLayoutEntryProps {
        let sampler_binding_type = if self.config.filtering.filtering_enabled() {
            wgpu::SamplerBindingType::Filtering
        } else {
            wgpu::SamplerBindingType::NonFiltering
        };

        SamplerBindGroupLayoutEntryProps {
            sampler_binding_type,
        }
    }
}

impl<'a> GPUResource<'a, TextureCreateInfo> for SamplingTexture {
    type GPUContext = (&'a GraphicsDevice, &'a MipmapperGenerator);

    fn create(
        (graphics_device, mipmapper_generator): &Self::GPUContext,
        id: TextureID,
        texture_info: &TextureCreateInfo,
    ) -> Result<Option<Self>> {
        let label = id.to_string();
        log::debug!("Creating texture `{label}`");
        crate::create_texture_from_info(graphics_device, mipmapper_generator, texture_info, &label)
            .with_context(|| format!("Failed creating texture: {label}"))
            .map(Some)
    }

    fn cleanup(self, _gpu_context: &Self::GPUContext, _id: TextureID) -> Result<()> {
        Ok(())
    }
}

impl GPUResource<'_, SamplerCreateInfo> for Sampler {
    type GPUContext = GraphicsDevice;

    fn create(
        graphics_device: &GraphicsDevice,
        _id: SamplerID,
        sampler_info: &SamplerCreateInfo,
    ) -> Result<Option<Self>> {
        Ok(Some(Sampler::create(
            graphics_device,
            sampler_info.config.clone(),
        )))
    }

    fn cleanup(self, _gpu_context: &Self::GPUContext, _id: SamplerID) -> Result<()> {
        Ok(())
    }
}

impl LookupTableBindingInfo {
    /// Returns the bind group layout for the lookup table texture and sampler. The
    /// bind group layout is created and cached if it does not already exist.
    pub fn get_or_create_bind_group_layout(
        &self,
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> wgpu::BindGroupLayout {
        const VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::FRAGMENT;

        let bind_group_layout_id = self.id().0.hash();

        bind_group_layout_registry.get_or_create_layout(bind_group_layout_id, || {
            let meta = self.metadata();

            let texture_format = meta.value_type.texel_description().texture_format();

            let view_dimension =
                Texture::determine_texture_view_dimension(meta.height, meta.depth_or_array_layers);

            let sampler_binding_type = if self.sampler_config().filtering.filtering_enabled() {
                wgpu::SamplerBindingType::Filtering
            } else {
                wgpu::SamplerBindingType::NonFiltering
            };

            let texture_entry = impact_gpu::texture::create_texture_bind_group_layout_entry(
                0,
                VISIBILITY,
                texture_format,
                view_dimension,
            );
            let sampler_entry = impact_gpu::texture::create_sampler_bind_group_layout_entry(
                1,
                VISIBILITY,
                sampler_binding_type,
            );

            graphics_device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[texture_entry, sampler_entry],
                    label: Some(&format!("{} bind group layout", self.id())),
                })
        })
    }
}

impl LookupTableBindGroup {
    /// Creates the bind group for the lookup table with the given binding info.
    ///
    /// # Errors
    /// Returns an error if the lookup table's texture or sampler is missing
    /// from the respective map.
    pub fn create_from_info(
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        textures: &TextureMap,
        samplers: &SamplerMap,
        binding_info: &LookupTableBindingInfo,
        label: &str,
    ) -> Result<Self> {
        let texture = &textures
            .get(binding_info.texture_id())
            .ok_or_else(|| anyhow!("Missing texture for lookup table {label}"))?
            .texture;

        let sampler = samplers
            .get(binding_info.sampler_id())
            .ok_or_else(|| anyhow!("Missing sampler for lookup table {label}"))?;

        let bind_group_layout = binding_info
            .get_or_create_bind_group_layout(graphics_device, bind_group_layout_registry);

        let bind_group = graphics_device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    texture.create_bind_group_entry(0),
                    sampler.create_bind_group_entry(1),
                ],
                label: Some(&format!("{label} bind group")),
            });

        Ok(Self { bind_group })
    }
}

impl<'a> GPUResource<'a, LookupTableBindingInfo> for LookupTableBindGroup {
    type GPUContext = (
        &'a GraphicsDevice,
        &'a BindGroupLayoutRegistry,
        &'a TextureMap,
        &'a SamplerMap,
    );

    fn create(
        (graphics_device, bind_group_layout_registry, textures, samplers): &Self::GPUContext,
        id: LookupTableID,
        binding_info: &LookupTableBindingInfo,
    ) -> Result<Option<Self>> {
        Self::create_from_info(
            graphics_device,
            bind_group_layout_registry,
            textures,
            samplers,
            binding_info,
            &id.to_string(),
        )
        .map(Some)
    }

    fn cleanup(
        self,
        (_graphics_device, bind_group_layout_registry, _textures, _samplers): &Self::GPUContext,
        id: LookupTableID,
    ) -> Result<()> {
        bind_group_layout_registry.remove_layout(id.0.hash());
        Ok(())
    }
}
