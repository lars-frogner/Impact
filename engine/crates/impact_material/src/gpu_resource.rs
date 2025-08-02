//! GPU resources for materials.

use crate::{
    MaterialBindGroupTemplate, MaterialTemplate, MaterialTemplateID, MaterialTextureGroup,
    MaterialTextureGroupID,
};
use anyhow::{Result, anyhow};
use impact_gpu::{device::GraphicsDevice, wgpu};
use impact_resource::gpu::{GPUResource, GPUResourceMap};
use impact_texture::gpu_resource::{SamplerMap, TextureMap};

/// Bind group layouts for material templates with textures.
pub type MaterialTemplateBindGroupLayoutMap =
    GPUResourceMap<MaterialTemplate, MaterialTemplateBindGroupLayout>;

/// Bind groups for material texture groups.
pub type MaterialTextureBindGroupMap =
    GPUResourceMap<MaterialTextureGroup, MaterialTextureBindGroup>;

/// A texture and sampler bind group layout shared across all materials that use
/// the same [`MaterialTemplate`].
#[derive(Debug)]
pub struct MaterialTemplateBindGroupLayout {
    pub bind_group_layout: wgpu::BindGroupLayout,
}

/// A texture and sampler bind group for a [`MaterialTextureGroup`].
#[derive(Debug)]
pub struct MaterialTextureBindGroup {
    pub bind_group: wgpu::BindGroup,
}

impl MaterialTemplateBindGroupLayout {
    /// Creates a bind group layout for a material template. Returns [`None`] if
    /// the template is for a material using no textures.
    pub fn create(
        graphics_device: &GraphicsDevice,
        template: &MaterialTemplate,
        label: &str,
    ) -> Option<Self> {
        if template.bind_group_template.is_empty() {
            return None;
        }

        let mut bind_group_layout_entries =
            Vec::with_capacity(template.bind_group_template.n_entries());

        for (idx, slot) in template.bind_group_template.slots.iter().enumerate() {
            let (texture_binding, sampler_binding) =
                MaterialBindGroupTemplate::get_texture_and_sampler_bindings(idx);

            bind_group_layout_entries.push(
                impact_gpu::texture::create_texture_bind_group_layout_entry(
                    texture_binding,
                    MaterialBindGroupTemplate::visibility(),
                    slot.texture.texture_format,
                    slot.texture.view_dimension,
                ),
            );
            bind_group_layout_entries.push(
                impact_gpu::texture::create_sampler_bind_group_layout_entry(
                    sampler_binding,
                    MaterialBindGroupTemplate::visibility(),
                    slot.sampler.sampler_binding_type,
                ),
            );
        }

        let bind_group_layout =
            graphics_device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &bind_group_layout_entries,
                    label: Some(&format!("Material template {label} bind group layout")),
                });

        Some(Self { bind_group_layout })
    }
}

impl<'a> GPUResource<'a, MaterialTemplate> for MaterialTemplateBindGroupLayout {
    type GPUContext = GraphicsDevice;

    fn create(
        graphics_device: &GraphicsDevice,
        id: MaterialTemplateID,
        template: &MaterialTemplate,
    ) -> Result<Option<Self>> {
        Ok(Self::create(graphics_device, template, &id.to_string()))
    }

    fn cleanup(self, _graphics_device: &GraphicsDevice, _id: MaterialTemplateID) -> Result<()> {
        Ok(())
    }
}

impl MaterialTextureBindGroup {
    /// Creates a bind group from a material texture group.
    ///
    /// Combines the textures and samplers from the texture group with the bind
    /// group layout from the associated material template to create a GPU bind
    /// group ready for rendering.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The template bind group layout is missing.
    /// - Any required texture or sampler is missing.
    /// - A texture has no associated sampler.
    pub fn create(
        graphics_device: &GraphicsDevice,
        textures: &TextureMap,
        samplers: &SamplerMap,
        bind_group_layouts: &MaterialTemplateBindGroupLayoutMap,
        group: &MaterialTextureGroup,
        label: &str,
    ) -> Result<Self> {
        let bind_group_layout = bind_group_layouts.get(group.template_id).ok_or_else(|| {
            anyhow!(
                "Missing template bind group layout {} for material texture group {label}",
                group.template_id
            )
        })?;

        let mut bind_group_entries = Vec::with_capacity(2 * group.texture_ids.len());

        for (idx, texture_id) in group.texture_ids.iter().enumerate() {
            let sampling_texture = textures.get(*texture_id).ok_or_else(|| {
                anyhow!("Missing texture {texture_id} for material texture group {label}")
            })?;
            let texture = &sampling_texture.texture;

            let sampler = samplers
                .get(sampling_texture.sampler_id.ok_or_else(|| {
                    anyhow!("Material texture {texture_id} has no associated sampler")
                })?)
                .ok_or_else(|| anyhow!("Missing sampler for texture {texture_id}"))?;

            let (texture_binding, sampler_binding) =
                MaterialBindGroupTemplate::get_texture_and_sampler_bindings(idx);

            bind_group_entries.push(texture.create_bind_group_entry(texture_binding));
            bind_group_entries.push(sampler.create_bind_group_entry(sampler_binding));
        }

        let bind_group = graphics_device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout.bind_group_layout,
                entries: &bind_group_entries,
                label: Some(&format!("Material texture group {label} bind group")),
            });

        Ok(Self { bind_group })
    }
}

impl<'a> GPUResource<'a, MaterialTextureGroup> for MaterialTextureBindGroup {
    type GPUContext = (
        &'a GraphicsDevice,
        &'a TextureMap,
        &'a SamplerMap,
        &'a MaterialTemplateBindGroupLayoutMap,
    );

    fn create(
        (graphics_device, textures, samplers, bind_group_layouts): &Self::GPUContext,
        id: MaterialTextureGroupID,
        group: &MaterialTextureGroup,
    ) -> Result<Option<Self>> {
        Self::create(
            graphics_device,
            textures,
            samplers,
            bind_group_layouts,
            group,
            &id.to_string(),
        )
        .map(Some)
    }

    fn cleanup(
        self,
        (_graphics_device, _textures, _samplers, _bind_group_layouts): &Self::GPUContext,
        _id: MaterialTextureGroupID,
    ) -> Result<()> {
        Ok(())
    }
}
