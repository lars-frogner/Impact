//! Management of material data for rendering.

use crate::{
    rendering::{Assets, CoreRenderingSystem, ImageTexture, Shader, TextureID},
    scene::MaterialSpecification,
};
use anyhow::{anyhow, Result};
use std::sync::Arc;

/// Owner and manager of a render resources for a material,
/// including a bind group for the set of textures used for
/// the material.
#[derive(Debug)]
pub struct MaterialRenderResourceManager {
    shader: Arc<Shader>,
    image_texture_ids: Vec<TextureID>,
    texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    texture_bind_group: Option<wgpu::BindGroup>,
    label: String,
}

impl MaterialRenderResourceManager {
    /// Creates a new manager with render resources initialized
    /// from the given material specification.
    pub fn for_material_specification(
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        material_specification: &MaterialSpecification,
        label: String,
    ) -> Result<Self> {
        let shader = Arc::clone(
            assets
                .shaders
                .get(&material_specification.shader_id())
                .ok_or_else(|| {
                    anyhow!(
                        "Shader {} missing from assets",
                        material_specification.shader_id()
                    )
                })?,
        );

        let image_texture_ids = material_specification.image_texture_ids().to_vec();

        let (texture_bind_group_layout, texture_bind_group) = if image_texture_ids.is_empty() {
            (None, None)
        } else {
            let texture_bind_group_layout = Self::create_texture_bind_group_layout(
                core_system.device(),
                image_texture_ids.len(),
                &label,
            );
            let texture_bind_group = Self::create_texture_bind_group(
                core_system.device(),
                assets,
                &image_texture_ids,
                &texture_bind_group_layout,
                &label,
            )?;
            (Some(texture_bind_group_layout), Some(texture_bind_group))
        };

        Ok(Self {
            shader,
            image_texture_ids,
            texture_bind_group_layout,
            texture_bind_group,
            label,
        })
    }

    /// Returns a reference to the compiled shader module used
    /// for the material.
    pub fn shader_module(&self) -> &wgpu::ShaderModule {
        self.shader.module()
    }

    /// Returns a reference to the bind group layout for the
    /// set of textures used for the material.
    pub fn texture_bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.texture_bind_group_layout.as_ref()
    }

    /// Returns a reference to the bind group for the set of
    /// textures used for the material.
    pub fn texture_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.texture_bind_group.as_ref()
    }

    /// Ensures that the render resources are in sync with the
    /// given material specification. This includes recreating
    /// the bind group if the set of textures has changed.
    pub fn sync_with_material_specification(
        &mut self,
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        material_specification: &MaterialSpecification,
    ) -> Result<()> {
        assert_eq!(
            self.image_texture_ids.len(),
            material_specification.image_texture_ids().len(),
            "Changed number of textures in material specification"
        );
        if let Some(layout) = &self.texture_bind_group_layout {
            if material_specification.image_texture_ids() != self.image_texture_ids {
                self.image_texture_ids = material_specification.image_texture_ids().to_vec();
                self.texture_bind_group = Some(Self::create_texture_bind_group(
                    core_system.device(),
                    assets,
                    &self.image_texture_ids,
                    layout,
                    &self.label,
                )?);
            }
        }
        Ok(())
    }

    fn create_texture_bind_group_layout(
        device: &wgpu::Device,
        n_textures: usize,
        label: &str,
    ) -> wgpu::BindGroupLayout {
        let n_entries = 2 * n_textures;
        let mut bind_group_layout_entries = Vec::with_capacity(n_entries);

        for idx in 0..n_textures {
            let binding = (2 * idx).try_into().unwrap();
            bind_group_layout_entries.push(ImageTexture::create_texture_bind_group_layout_entry(
                binding,
            ));
            bind_group_layout_entries.push(ImageTexture::create_sampler_bind_group_layout_entry(
                binding + 1,
            ));
        }

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &bind_group_layout_entries,
            label: Some(&format!("{} bind group layout", label)),
        })
    }

    fn create_texture_bind_group(
        device: &wgpu::Device,
        assets: &Assets,
        texture_ids: &[TextureID],
        layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> Result<wgpu::BindGroup> {
        let n_entries = 2 * texture_ids.len();
        let mut bind_group_entries = Vec::with_capacity(n_entries);

        for (idx, texture_id) in texture_ids.iter().enumerate() {
            let image_texture = assets
                .image_textures
                .get(texture_id)
                .ok_or_else(|| anyhow!("Texture {} missing from assets", texture_id))?;

            let binding = (2 * idx).try_into().unwrap();
            bind_group_entries.push(image_texture.create_texture_bind_group_entry(binding));
            bind_group_entries.push(image_texture.create_sampler_bind_group_entry(binding + 1));
        }

        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &bind_group_entries,
            label: Some(&format!("{} bind group", label)),
        }))
    }
}
