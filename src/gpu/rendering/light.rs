//! Management of light source data for rendering.

use crate::{
    assert_uniform_valid,
    geometry::CollectionChange,
    gpu::{
        rendering::{
            texture::{CascadedShadowMapTexture, ShadowCubemapTexture},
            CascadeIdx, RenderingConfig,
        },
        shader::{
            AmbientLightShaderInput, LightShaderInput, OmnidirectionalLightShaderInput,
            UnidirectionalLightShaderInput,
        },
        uniform::{
            self, MultiUniformRenderBuffer, UniformBuffer, UniformBufferable, UniformTransferResult,
        },
        GraphicsDevice,
    },
    scene::{
        AmbientLight, LightID, LightStorage, LightType, OmnidirectionalLight, UnidirectionalLight,
        MAX_SHADOW_MAP_CASCADES,
    },
};
use impact_utils::ConstStringHash64;
use std::mem;

/// Manager of the set of uniform render buffers holding light source render
/// data. Also manages the bind groups for these buffers and for associated
/// shadow map textures.
#[derive(Debug)]
pub struct LightRenderBufferManager {
    ambient_light_render_buffer: UniformRenderBufferWithLightIDs,
    omnidirectional_light_render_buffer: UniformRenderBufferWithLightIDs,
    unidirectional_light_render_buffer: UniformRenderBufferWithLightIDs,
    omnidirectional_light_shadow_map_texture: ShadowCubemapTexture,
    unidirectional_light_shadow_map_texture: CascadedShadowMapTexture,
    light_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group: wgpu::BindGroup,
    omnidirectional_light_shadow_map_bind_group_layout: wgpu::BindGroupLayout,
    omnidirectional_light_shadow_map_bind_group: wgpu::BindGroup,
    unidirectional_light_shadow_map_bind_group_layout: wgpu::BindGroupLayout,
    unidirectional_light_shadow_map_bind_group: wgpu::BindGroup,
    ambient_light_shader_input: LightShaderInput,
    omnidirectional_light_shader_input: LightShaderInput,
    unidirectional_light_shader_input: LightShaderInput,
}

#[derive(Debug)]
struct UniformRenderBufferWithLightIDs {
    uniform_render_buffer: MultiUniformRenderBuffer,
    light_ids: Vec<LightID>,
}

impl LightRenderBufferManager {
    const AMBIENT_LIGHT_BINDING: u32 = 0;

    const OMNIDIRECTIONAL_LIGHT_BINDING: u32 = 1;
    const OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING: u32 = 2;
    const OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING: u32 = 3;
    const OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_COMPARISON_SAMPLER_BINDING: u32 = 4;

    const UNIDIRECTIONAL_LIGHT_BINDING: u32 = 5;
    const UNIDIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING: u32 = 6;
    const UNIDIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING: u32 = 7;
    const UNIDIRECTIONAL_LIGHT_SHADOW_MAP_COMPARISON_SAMPLER_BINDING: u32 = 8;

    const VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::VERTEX_FRAGMENT;

    pub const LIGHT_IDX_PUSH_CONSTANT_SIZE: u32 = mem::size_of::<u32>() as u32;
    pub const CASCADE_IDX_PUSH_CONSTANT_SIZE: u32 = mem::size_of::<CascadeIdx>() as u32;

    /// Creates a new manager with render buffers initialized from the given
    /// [`LightStorage`].
    pub fn for_light_storage(
        graphics_device: &GraphicsDevice,
        light_storage: &LightStorage,
        config: &RenderingConfig,
    ) -> Self {
        let ambient_light_render_buffer = UniformRenderBufferWithLightIDs::for_uniform_buffer(
            graphics_device,
            light_storage.ambient_light_buffer(),
            Self::VISIBILITY,
        );
        let omnidirectional_light_render_buffer =
            UniformRenderBufferWithLightIDs::for_uniform_buffer(
                graphics_device,
                light_storage.omnidirectional_light_buffer(),
                Self::VISIBILITY,
            );
        let unidirectional_light_render_buffer =
            UniformRenderBufferWithLightIDs::for_uniform_buffer(
                graphics_device,
                light_storage.unidirectional_light_buffer(),
                Self::VISIBILITY,
            );

        let omnidirectional_light_shadow_map_texture = ShadowCubemapTexture::new(
            graphics_device,
            config.omnidirectional_light_shadow_map_resolution,
            "Omnidirectional light shadow cubemap texture",
        );

        let unidirectional_light_shadow_map_texture = CascadedShadowMapTexture::new(
            graphics_device,
            config.unidirectional_light_shadow_map_resolution,
            MAX_SHADOW_MAP_CASCADES,
            "Unidirectional light cascaded shadow map texture",
        );

        let light_bind_group_layout = Self::create_light_bind_group_layout(
            graphics_device.device(),
            &ambient_light_render_buffer,
            &omnidirectional_light_render_buffer,
            &unidirectional_light_render_buffer,
        );

        let light_bind_group = Self::create_light_bind_group(
            graphics_device.device(),
            &ambient_light_render_buffer,
            &omnidirectional_light_render_buffer,
            &unidirectional_light_render_buffer,
            &light_bind_group_layout,
        );

        let omnidirectional_light_shadow_map_bind_group_layout =
            Self::create_omnidirectional_light_shadow_map_bind_group_layout(
                graphics_device.device(),
            );

        let omnidirectional_light_shadow_map_bind_group =
            Self::create_omnidirectional_light_shadow_map_bind_group(
                graphics_device.device(),
                &omnidirectional_light_shadow_map_texture,
                &omnidirectional_light_shadow_map_bind_group_layout,
            );

        let unidirectional_light_shadow_map_bind_group_layout =
            Self::create_unidirectional_light_shadow_map_bind_group_layout(
                graphics_device.device(),
            );

        let unidirectional_light_shadow_map_bind_group =
            Self::create_unidirectional_light_shadow_map_bind_group(
                graphics_device.device(),
                &unidirectional_light_shadow_map_texture,
                &unidirectional_light_shadow_map_bind_group_layout,
            );

        let ambient_light_shader_input =
            Self::create_ambient_light_shader_input(&ambient_light_render_buffer);

        let omnidirectional_light_shader_input =
            Self::create_omnidirectional_light_shader_input(&omnidirectional_light_render_buffer);

        let unidirectional_light_shader_input =
            Self::create_unidirectional_light_shader_input(&unidirectional_light_render_buffer);

        Self {
            ambient_light_render_buffer,
            omnidirectional_light_render_buffer,
            unidirectional_light_render_buffer,
            omnidirectional_light_shadow_map_texture,
            unidirectional_light_shadow_map_texture,
            light_bind_group_layout,
            light_bind_group,
            omnidirectional_light_shadow_map_bind_group_layout,
            omnidirectional_light_shadow_map_bind_group,
            unidirectional_light_shadow_map_bind_group_layout,
            unidirectional_light_shadow_map_bind_group,
            ambient_light_shader_input,
            omnidirectional_light_shader_input,
            unidirectional_light_shader_input,
        }
    }

    /// Returns the slice of IDs of all the [`AmbientLight`]s currently residing
    /// in the ambient light render buffer.
    pub fn ambient_light_ids(&self) -> &[LightID] {
        self.ambient_light_render_buffer.light_ids()
    }

    /// Returns the slice of IDs of all the [`OmnidirectionalLight`]s currently residing
    /// in the omnidirectional light render buffer.
    pub fn omnidirectional_light_ids(&self) -> &[LightID] {
        self.omnidirectional_light_render_buffer.light_ids()
    }

    /// Returns the slice of IDs of all the [`UnidirectionalLight`]s currently
    /// residing in the unidirectional light render buffer.
    pub fn unidirectional_light_ids(&self) -> &[LightID] {
        self.unidirectional_light_render_buffer.light_ids()
    }

    /// Returns a reference to the shadow cubemap texture for omnidirectional
    /// lights.
    pub fn omnidirectional_light_shadow_map_texture(&self) -> &ShadowCubemapTexture {
        &self.omnidirectional_light_shadow_map_texture
    }

    /// Returns a reference to the cascaded shadow map texture for
    /// unidirectional lights.
    pub fn unidirectional_light_shadow_map_texture(&self) -> &CascadedShadowMapTexture {
        &self.unidirectional_light_shadow_map_texture
    }

    /// Returns a reference to the bind group layout for the set of light
    /// uniform buffers.
    pub fn light_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.light_bind_group_layout
    }

    /// Returns a reference to the bind group for the set of light uniform
    /// buffers.
    pub fn light_bind_group(&self) -> &wgpu::BindGroup {
        &self.light_bind_group
    }

    /// Returns the bind group layout for the shadow map texture for the given
    /// light type, or [`None`] if the light type does not use shadow mapping.
    pub fn shadow_map_bind_group_layout_for_light_type(
        &self,
        light_type: LightType,
    ) -> Option<&wgpu::BindGroupLayout> {
        match light_type {
            LightType::AmbientLight => None,
            LightType::OmnidirectionalLight => {
                Some(&self.omnidirectional_light_shadow_map_bind_group_layout)
            }
            LightType::UnidirectionalLight => {
                Some(&self.unidirectional_light_shadow_map_bind_group_layout)
            }
        }
    }

    /// Returns the bind group for the shadow map texture for the given light
    /// type, or [`None`] if the light type does not use shadow mapping.
    pub fn shadow_map_bind_group_for_light_type(
        &self,
        light_type: LightType,
    ) -> Option<&wgpu::BindGroup> {
        match light_type {
            LightType::AmbientLight => None,
            LightType::OmnidirectionalLight => {
                Some(&self.omnidirectional_light_shadow_map_bind_group)
            }
            LightType::UnidirectionalLight => {
                Some(&self.unidirectional_light_shadow_map_bind_group)
            }
        }
    }

    /// Returns the input required for accessing light data of the given type in
    /// a shader.
    pub fn shader_input_for_light_type(&self, light_type: LightType) -> &LightShaderInput {
        match light_type {
            LightType::AmbientLight => &self.ambient_light_shader_input,
            LightType::OmnidirectionalLight => &self.omnidirectional_light_shader_input,
            LightType::UnidirectionalLight => &self.unidirectional_light_shader_input,
        }
    }

    /// Finds and returns the index of the light with the given ID in the light
    /// type's uniform buffer, for use as a push constant.
    ///
    /// # Panics
    /// If no light with the given ID is present in the relevant uniform buffer.
    pub fn get_light_idx_push_constant(&self, light_type: LightType, light_id: LightID) -> u32 {
        let light_idx = match light_type {
            LightType::AmbientLight => &self.ambient_light_render_buffer,
            LightType::OmnidirectionalLight => &self.omnidirectional_light_render_buffer,
            LightType::UnidirectionalLight => &self.unidirectional_light_render_buffer,
        }
        .find_idx_of_light_with_id(light_id)
        .expect("Tried to set light index push constant for missing light");

        u32::try_from(light_idx).unwrap()
    }

    /// Ensures that the light uniform buffers are in sync with the light data
    /// in the given light storage. Will also recreate the bind group and update
    /// the shader input if any of the render buffers had to be reallocated.
    pub fn sync_with_light_storage(
        &mut self,
        graphics_device: &GraphicsDevice,
        light_storage: &LightStorage,
    ) {
        let ambient_light_transfer_result = self
            .ambient_light_render_buffer
            .transfer_uniforms_to_render_buffer(
                graphics_device,
                light_storage.ambient_light_buffer(),
            );

        let omnidirectional_light_transfer_result = self
            .omnidirectional_light_render_buffer
            .transfer_uniforms_to_render_buffer(
                graphics_device,
                light_storage.omnidirectional_light_buffer(),
            );

        let unidirectional_light_transfer_result = self
            .unidirectional_light_render_buffer
            .transfer_uniforms_to_render_buffer(
                graphics_device,
                light_storage.unidirectional_light_buffer(),
            );

        if ambient_light_transfer_result == UniformTransferResult::CreatedNewBuffer
            || omnidirectional_light_transfer_result == UniformTransferResult::CreatedNewBuffer
            || unidirectional_light_transfer_result == UniformTransferResult::CreatedNewBuffer
        {
            // Recreate light bind group and shader input
            self.light_bind_group = Self::create_light_bind_group(
                graphics_device.device(),
                &self.ambient_light_render_buffer,
                &self.omnidirectional_light_render_buffer,
                &self.unidirectional_light_render_buffer,
                &self.light_bind_group_layout,
            );

            if ambient_light_transfer_result == UniformTransferResult::CreatedNewBuffer {
                self.ambient_light_shader_input =
                    Self::create_ambient_light_shader_input(&self.ambient_light_render_buffer);
            }

            if omnidirectional_light_transfer_result == UniformTransferResult::CreatedNewBuffer {
                self.omnidirectional_light_shader_input =
                    Self::create_omnidirectional_light_shader_input(
                        &self.omnidirectional_light_render_buffer,
                    );
            }

            if unidirectional_light_transfer_result == UniformTransferResult::CreatedNewBuffer {
                self.unidirectional_light_shader_input =
                    Self::create_unidirectional_light_shader_input(
                        &self.unidirectional_light_render_buffer,
                    );
            }
        }
    }

    fn create_light_bind_group_layout(
        device: &wgpu::Device,
        ambient_light_render_buffer: &UniformRenderBufferWithLightIDs,
        omnidirectional_light_render_buffer: &UniformRenderBufferWithLightIDs,
        unidirectional_light_render_buffer: &UniformRenderBufferWithLightIDs,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                ambient_light_render_buffer
                    .buffer()
                    .create_bind_group_layout_entry(Self::AMBIENT_LIGHT_BINDING),
                omnidirectional_light_render_buffer
                    .buffer()
                    .create_bind_group_layout_entry(Self::OMNIDIRECTIONAL_LIGHT_BINDING),
                unidirectional_light_render_buffer
                    .buffer()
                    .create_bind_group_layout_entry(Self::UNIDIRECTIONAL_LIGHT_BINDING),
            ],
            label: Some("Light bind group layout"),
        })
    }

    fn create_light_bind_group(
        device: &wgpu::Device,
        ambient_light_render_buffer: &UniformRenderBufferWithLightIDs,
        omnidirectional_light_render_buffer: &UniformRenderBufferWithLightIDs,
        unidirectional_light_render_buffer: &UniformRenderBufferWithLightIDs,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                ambient_light_render_buffer
                    .buffer()
                    .create_bind_group_entry(Self::AMBIENT_LIGHT_BINDING),
                omnidirectional_light_render_buffer
                    .buffer()
                    .create_bind_group_entry(Self::OMNIDIRECTIONAL_LIGHT_BINDING),
                unidirectional_light_render_buffer
                    .buffer()
                    .create_bind_group_entry(Self::UNIDIRECTIONAL_LIGHT_BINDING),
            ],
            label: Some("Light bind group"),
        })
    }

    fn create_omnidirectional_light_shadow_map_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                ShadowCubemapTexture::create_texture_bind_group_layout_entry(
                    Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                ),
                ShadowCubemapTexture::create_sampler_bind_group_layout_entry(
                    Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                ),
                ShadowCubemapTexture::create_comparison_sampler_bind_group_layout_entry(
                    Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_COMPARISON_SAMPLER_BINDING,
                ),
            ],
            label: Some("Omnidirectional light shadow cubemap bind group layout"),
        })
    }

    fn create_omnidirectional_light_shadow_map_bind_group(
        device: &wgpu::Device,
        omnidirectional_light_shadow_map_texture: &ShadowCubemapTexture,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                omnidirectional_light_shadow_map_texture.create_texture_bind_group_entry(
                    Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                ),
                omnidirectional_light_shadow_map_texture.create_sampler_bind_group_entry(
                    Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                ),
                omnidirectional_light_shadow_map_texture
                    .create_comparison_sampler_bind_group_entry(
                        Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_COMPARISON_SAMPLER_BINDING,
                    ),
            ],
            label: Some("Omnidirectional light shadow cubemap bind group"),
        })
    }

    fn create_unidirectional_light_shadow_map_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                CascadedShadowMapTexture::create_texture_bind_group_layout_entry(
                    Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                ),
                CascadedShadowMapTexture::create_sampler_bind_group_layout_entry(
                    Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                ),
                CascadedShadowMapTexture::create_comparison_sampler_bind_group_layout_entry(
                    Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_COMPARISON_SAMPLER_BINDING,
                ),
            ],
            label: Some("Unidirectional light shadow map bind group layout"),
        })
    }

    fn create_unidirectional_light_shadow_map_bind_group(
        device: &wgpu::Device,
        unidirectional_light_shadow_map_texture: &CascadedShadowMapTexture,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                unidirectional_light_shadow_map_texture.create_texture_bind_group_entry(
                    Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                ),
                unidirectional_light_shadow_map_texture.create_sampler_bind_group_entry(
                    Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                ),
                unidirectional_light_shadow_map_texture.create_comparison_sampler_bind_group_entry(
                    Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_COMPARISON_SAMPLER_BINDING,
                ),
            ],
            label: Some("Unidirectional light shadow map bind group"),
        })
    }

    fn create_ambient_light_shader_input(
        ambient_light_render_buffer: &UniformRenderBufferWithLightIDs,
    ) -> LightShaderInput {
        LightShaderInput::AmbientLight(AmbientLightShaderInput {
            uniform_binding: Self::AMBIENT_LIGHT_BINDING,
            max_light_count: ambient_light_render_buffer.buffer().max_uniform_count() as u64,
        })
    }

    fn create_omnidirectional_light_shader_input(
        omnidirectional_light_render_buffer: &UniformRenderBufferWithLightIDs,
    ) -> LightShaderInput {
        LightShaderInput::OmnidirectionalLight(OmnidirectionalLightShaderInput {
            uniform_binding: Self::OMNIDIRECTIONAL_LIGHT_BINDING,
            max_light_count: omnidirectional_light_render_buffer
                .buffer()
                .max_uniform_count() as u64,
            shadow_map_texture_and_sampler_bindings: (
                Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                Self::OMNIDIRECTIONAL_LIGHT_SHADOW_MAP_COMPARISON_SAMPLER_BINDING,
            ),
        })
    }

    fn create_unidirectional_light_shader_input(
        unidirectional_light_render_buffer: &UniformRenderBufferWithLightIDs,
    ) -> LightShaderInput {
        LightShaderInput::UnidirectionalLight(UnidirectionalLightShaderInput {
            uniform_binding: Self::UNIDIRECTIONAL_LIGHT_BINDING,
            max_light_count: unidirectional_light_render_buffer
                .buffer()
                .max_uniform_count() as u64,
            shadow_map_texture_and_sampler_bindings: (
                Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_TEXTURE_BINDING,
                Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_SAMPLER_BINDING,
                Self::UNIDIRECTIONAL_LIGHT_SHADOW_MAP_COMPARISON_SAMPLER_BINDING,
            ),
        })
    }
}

impl UniformRenderBufferWithLightIDs {
    /// Creates a new uniform render buffer together with a list of light IDs
    /// initialized from the given uniform buffer.
    fn for_uniform_buffer<U>(
        graphics_device: &GraphicsDevice,
        uniform_buffer: &UniformBuffer<LightID, U>,
        visibility: wgpu::ShaderStages,
    ) -> Self
    where
        U: UniformBufferable,
    {
        Self {
            uniform_render_buffer: MultiUniformRenderBuffer::for_uniform_buffer(
                graphics_device,
                uniform_buffer,
                visibility,
            ),
            light_ids: uniform_buffer.valid_uniform_ids().to_vec(),
        }
    }

    fn buffer(&self) -> &MultiUniformRenderBuffer {
        &self.uniform_render_buffer
    }

    fn light_ids(&self) -> &[LightID] {
        &self.light_ids
    }

    fn find_idx_of_light_with_id(&self, light_id: LightID) -> Option<usize> {
        self.light_ids.iter().position(|&id| id == light_id)
    }

    fn transfer_uniforms_to_render_buffer<U>(
        &mut self,
        graphics_device: &GraphicsDevice,
        uniform_buffer: &UniformBuffer<LightID, U>,
    ) -> UniformTransferResult
    where
        U: UniformBufferable,
    {
        match uniform_buffer.change() {
            CollectionChange::Count => {
                self.light_ids = uniform_buffer.valid_uniform_ids().to_vec();
            }
            CollectionChange::Contents => {
                self.light_ids
                    .copy_from_slice(uniform_buffer.valid_uniform_ids());
            }
            CollectionChange::None => {}
        }

        self.uniform_render_buffer
            .transfer_uniforms_to_render_buffer(graphics_device, uniform_buffer)
    }
}

impl UniformBufferable for AmbientLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Ambient light");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(AmbientLight);

impl UniformBufferable for OmnidirectionalLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Omnidirectional light");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(OmnidirectionalLight);

impl UniformBufferable for UnidirectionalLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Unidirectional light");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(UnidirectionalLight);
