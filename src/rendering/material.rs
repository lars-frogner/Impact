//! Management of material data for rendering.

use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        Assets, CoreRenderingSystem, ImageTexture, MaterialPropertyTextureSetShaderInput,
        MaterialShaderInput, TextureID,
    },
    scene::{MaterialPropertyTextureSet, MaterialSpecification},
};
use anyhow::{anyhow, Result};

/// Manager of a render resources for a material type.
#[derive(Debug)]
pub struct MaterialRenderResourceManager {
    vertex_attribute_requirements: VertexAttributeSet,
    shader_input: MaterialShaderInput,
    label: String,
}

/// Manager of a set of textures used for material properties.
#[derive(Debug)]
pub struct MaterialPropertyTextureManager {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    shader_input: MaterialPropertyTextureSetShaderInput,
}

impl MaterialRenderResourceManager {
    /// Creates a new manager with render resources initialized from the given
    /// material specification.
    pub fn for_material_specification(
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        material_specification: &MaterialSpecification,
        label: String,
    ) -> Result<Self> {
        Ok(Self {
            vertex_attribute_requirements: material_specification.vertex_attribute_requirements(),
            shader_input: material_specification.shader_input().clone(),
            label,
        })
    }

    /// Returns a [`VertexAttributeSet`] encoding the vertex attributes required
    /// for rendering the material.
    pub fn vertex_attribute_requirements(&self) -> VertexAttributeSet {
        self.vertex_attribute_requirements
    }

    /// Returns the input required for generating shaders for the material.
    pub fn shader_input(&self) -> &MaterialShaderInput {
        &self.shader_input
    }
}

impl MaterialPropertyTextureManager {
    /// Creates a new manager for the given set of material property textures.
    pub fn for_texture_set(
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        texture_set: &MaterialPropertyTextureSet,
        label: String,
    ) -> Result<Self> {
        let image_texture_ids = texture_set.image_texture_ids().to_vec();

        let bind_group_layout = Self::create_texture_bind_group_layout(
            core_system.device(),
            image_texture_ids.len(),
            &label,
        );

        let bind_group = Self::create_texture_bind_group(
            core_system.device(),
            assets,
            &image_texture_ids,
            &bind_group_layout,
            &label,
        )?;

        Ok(Self {
            bind_group_layout,
            bind_group,
            shader_input: texture_set.shader_input().clone(),
        })
    }

    /// Returns the binding that will be used for the texture at the given index
    /// and its sampler in the bind group.
    pub const fn get_texture_and_sampler_bindings(texture_idx: usize) -> (u32, u32) {
        let texture_binding = (2 * texture_idx) as u32;
        let sampler_binding = texture_binding + 1;
        (texture_binding, sampler_binding)
    }

    /// Returns a reference to the bind group layout for the set of textures.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns a reference to the bind group for the set of textures.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    // Returns the input required for using the texture set in a shader.
    pub fn shader_input(&self) -> &MaterialPropertyTextureSetShaderInput {
        &self.shader_input
    }

    fn create_texture_bind_group_layout(
        device: &wgpu::Device,
        n_textures: usize,
        label: &str,
    ) -> wgpu::BindGroupLayout {
        let n_entries = 2 * n_textures;
        let mut bind_group_layout_entries = Vec::with_capacity(n_entries);

        for idx in 0..n_textures {
            let (texture_binding, sampler_binding) = Self::get_texture_and_sampler_bindings(idx);
            bind_group_layout_entries.push(ImageTexture::create_texture_bind_group_layout_entry(
                texture_binding,
            ));
            bind_group_layout_entries.push(ImageTexture::create_sampler_bind_group_layout_entry(
                sampler_binding,
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

            let (texture_binding, sampler_binding) = Self::get_texture_and_sampler_bindings(idx);
            bind_group_entries.push(image_texture.create_texture_bind_group_entry(texture_binding));
            bind_group_entries.push(image_texture.create_sampler_bind_group_entry(sampler_binding));
        }

        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &bind_group_entries,
            label: Some(&format!("{} bind group", label)),
        }))
    }
}
